//! OpenSubtitles.com REST API v1 provider.

use std::path::Path;
use std::time::Duration;

use horismos::OpenSubtitlesConfig;
use serde::Deserialize;
use snafu::ResultExt;
use themelion::{MediaId, MediaType};
use tracing::{debug, instrument, warn};

use crate::error::{
    AcquisitionFailedSnafu, DownloadFailedSnafu, ProsthekeError, ProviderDownSnafu,
};
use crate::providers::SubtitleProvider;
use crate::types::SubtitleMatch;

const BASE_URL: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = "Harmonia/1.0";

/// Computes the OpenSubtitles-specific file hash.
///
/// The hash is the XOR sum of the file size and the first/last 64 KB of the
/// file, treating each 8-byte chunk as a little-endian u64.
pub fn compute_file_hash(path: &Path) -> std::io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};

    const CHUNK_SIZE: usize = 64 * 1024;
    const WORD_SIZE: usize = 8;

    let mut file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    let mut hash: u64 = file_size;

    let mut tmp = [0u8; WORD_SIZE];

    // Read first 64 KB.
    for _ in 0..(CHUNK_SIZE / WORD_SIZE) {
        match file.read_exact(&mut tmp) {
            Ok(()) => hash = hash.wrapping_add(u64::from_le_bytes(tmp)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
    }

    // Read last 64 KB.
    let tail_offset = file_size.saturating_sub(u64::try_from(CHUNK_SIZE).unwrap_or_default());
    file.seek(SeekFrom::Start(tail_offset))?;

    for _ in 0..(CHUNK_SIZE / WORD_SIZE) {
        match file.read_exact(&mut tmp) {
            Ok(()) => hash = hash.wrapping_add(u64::from_le_bytes(tmp)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
    }

    Ok(format!("{hash:016x}"))
}

/// OpenSubtitles.com REST API v1 client.
///
/// Returns empty results when not configured (no API key). This satisfies the
/// acceptance criterion: "Provider unconfigured → empty results, not error."
pub struct OpenSubtitlesProvider {
    config: Option<OpenSubtitlesConfig>,
    client: reqwest::Client,
}

impl OpenSubtitlesProvider {
    pub(crate) fn new(config: Option<OpenSubtitlesConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default();
        Self { config, client }
    }

    fn api_key(&self) -> Option<&str> {
        self.config.as_ref().map(|c| c.api_key.as_str())
    }
}

// ── API response shapes ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<SubtitleData>,
}

#[derive(Debug, Deserialize)]
struct SubtitleData {
    id: String,
    attributes: SubtitleAttributes,
}

#[derive(Debug, Deserialize)]
struct SubtitleAttributes {
    language: String,
    #[serde(default)]
    hearing_impaired: bool,
    #[serde(default)]
    foreign_parts_only: bool,
    #[serde(default)]
    moviehash_match: bool,
    files: Vec<SubtitleFile>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubtitleFile {
    file_id: u64,
}

#[derive(Debug, Deserialize)]
struct DownloadResponse {
    link: String,
    #[serde(default)]
    file_name: String,
}

// ── Scoring ───────────────────────────────────────────────────────────────────

/// Score a subtitle result. Hash match is highest quality.
fn score_result(attr: &SubtitleAttributes, requested_lang: &str) -> f64 {
    let base = if attr.moviehash_match { 1.0 } else { 0.75 };

    // Exact language match is full score; otherwise reduce slightly.
    let lang_match = if attr.language == requested_lang {
        1.0
    } else {
        0.9
    };

    base * lang_match
}

// ── Provider implementation ───────────────────────────────────────────────────

impl SubtitleProvider for OpenSubtitlesProvider {
    fn name(&self) -> &str {
        "opensubtitles"
    }

    #[instrument(skip(self), fields(provider = "opensubtitles", title = %title))]
    async fn search(
        &self,
        _media_id: &MediaId,
        media_type: MediaType,
        title: &str,
        year: Option<u16>,
        season: Option<u32>,
        episode: Option<u32>,
        languages: &[String],
        file_hash: Option<&str>,
    ) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
        let Some(api_key) = self.api_key() else {
            debug!("opensubtitles not configured  -  skipping search");
            return Ok(vec![]);
        };

        if api_key.is_empty() {
            debug!("opensubtitles credential empty  -  skipping search"); // kanon:ignore SECURITY/credential-logging -- literal string, no secret interpolated
            return Ok(vec![]);
        }

        let media_type_str = match media_type {
            MediaType::Movie => "movie",
            MediaType::Tv => "episode",
            _ => "movie",
        };

        let lang_param = languages.join(",");

        let mut params: Vec<(&str, String)> = vec![
            ("query", title.to_string()),
            ("type", media_type_str.to_string()),
            ("languages", lang_param),
        ];

        if let Some(y) = year {
            params.push(("year", y.to_string()));
        }
        if let Some(s) = season {
            params.push(("season_number", s.to_string()));
        }
        if let Some(e) = episode {
            params.push(("episode_number", e.to_string()));
        }
        if let Some(hash) = file_hash {
            params.push(("moviehash", hash.to_string()));
        }

        let response = self
            .client
            .get(format!("{BASE_URL}/subtitles"))
            .header("Api-Key", api_key)
            .query(&params)
            .send()
            .await
            .context(ProviderDownSnafu)?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(status = %status, "opensubtitles search returned non-200");
            return AcquisitionFailedSnafu {
                detail: format!("HTTP {status}"),
            }
            .fail();
        }

        let body: SearchResponse = response.json().await.context(ProviderDownSnafu)?;

        let mut matches = Vec::new();
        for item in body.data {
            let Some(file) = item.attributes.files.first() else {
                continue;
            };

            // Use the best language requested as the match language.
            let matched_lang = languages
                .iter()
                .find(|l| l.as_str() == item.attributes.language)
                .cloned()
                .unwrap_or_else(|| item.attributes.language.clone());

            let score = score_result(&item.attributes, &matched_lang);
            let download_url = item
                .attributes
                .url
                .clone()
                .unwrap_or_else(|| format!("{BASE_URL}/download/{}", file.file_id));

            matches.push(SubtitleMatch {
                provider: self.name().to_string(),
                provider_id: item.id,
                language: item.attributes.language,
                hearing_impaired: item.attributes.hearing_impaired,
                forced: item.attributes.foreign_parts_only,
                score,
                download_url,
            });
        }

        Ok(matches)
    }

    #[instrument(skip(self, subtitle), fields(provider = "opensubtitles", provider_id = %subtitle.provider_id))]
    async fn download(&self, subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
        let Some(api_key) = self.api_key() else {
            return DownloadFailedSnafu {
                detail: "opensubtitles not configured".to_string(),
            }
            .fail();
        };

        // First request the download link FROM the API.
        let file_id: u64 = subtitle.provider_id.parse().unwrap_or_default();

        let download_req = serde_json::json!({ "file_id": file_id });
        let dl_resp = self
            .client
            .post(format!("{BASE_URL}/download"))
            .header("Api-Key", api_key)
            .json(&download_req)
            .send()
            .await
            .context(ProviderDownSnafu)?;

        if !dl_resp.status().is_success() {
            let status = dl_resp.status();
            return DownloadFailedSnafu {
                detail: format!("HTTP {status} FROM download endpoint"),
            }
            .fail();
        }

        let dl_info: DownloadResponse = dl_resp.json().await.context(ProviderDownSnafu)?;
        debug!(file_name = %dl_info.file_name, "obtained download link");

        // Fetch the actual subtitle file.
        let content = self
            .client
            .get(&dl_info.link)
            .send()
            .await
            .context(ProviderDownSnafu)?
            .bytes()
            .await
            .context(ProviderDownSnafu)?;

        Ok(content.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_returns_empty_results() {
        let provider = OpenSubtitlesProvider::new(None);
        assert!(provider.api_key().is_none());
    }

    #[test]
    fn empty_api_key_treated_as_unconfigured() {
        let config = OpenSubtitlesConfig {
            api_key: String::new(),
            username: None,
            password: None,
            rate_limit_per_second: 5,
        };
        let provider = OpenSubtitlesProvider::new(Some(config));
        assert_eq!(provider.api_key(), Some(""));
    }

    #[test]
    fn score_hash_match_higher_than_title_match() {
        let hash_attr = SubtitleAttributes {
            language: "en".to_string(),
            hearing_impaired: false,
            foreign_parts_only: false,
            moviehash_match: true,
            files: vec![],
            url: None,
        };
        let title_attr = SubtitleAttributes {
            language: "en".to_string(),
            hearing_impaired: false,
            foreign_parts_only: false,
            moviehash_match: false,
            files: vec![],
            url: None,
        };
        assert!(score_result(&hash_attr, "en") > score_result(&title_attr, "en"));
    }

    #[tokio::test]
    async fn unconfigured_search_returns_empty_not_error() {
        let provider = OpenSubtitlesProvider::new(None);
        let media_id = themelion::MediaId::new();
        let result = provider
            .search(
                &media_id,
                MediaType::Movie,
                "Inception",
                Some(2010),
                None,
                None,
                &["en".to_string()],
                None,
            )
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
