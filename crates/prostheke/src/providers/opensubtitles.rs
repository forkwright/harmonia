//! OpenSubtitles.com REST API v1 provider.

use std::path::Path;
use std::time::Duration;

use horismos::OpenSubtitlesConfig;
use serde::Deserialize;
use snafu::ResultExt;
use themelion::{MediaId, MediaType};
use tracing::{debug, instrument, warn};

use crate::error::{
    AcquisitionFailedSnafu, DownloadFailedSnafu, InvalidProviderIdSnafu, ProsthekeError,
    ProviderDownSnafu,
};
use crate::providers::SubtitleProvider;
use crate::types::{SubtitleMatch, SubtitleProviderId};

const BASE_URL: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = "Harmonia/1.0";
/// Fallback download cap when the provider is exercised without a config.
const DEFAULT_MAX_DOWNLOAD_BYTES: u64 = 10 * 1024 * 1024;

/// Computes the OpenSubtitles-specific file hash.
///
/// The hash is the wrapping sum of the file size and the first/last 64 KB of
/// the file, treating each 8-byte chunk as a little-endian u64. For files
/// smaller than 128 KB the two windows overlap; bytes between the windows
/// never affect the hash.
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
    let tail_offset = file_size.saturating_sub(u64::try_from(CHUNK_SIZE).unwrap_or_default()); // WHY: CHUNK_SIZE is a compile-time constant that fits in u64
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
    pub fn new(config: Option<OpenSubtitlesConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config
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

// ── Download safety ───────────────────────────────────────────────────────────

/// Validate a provider-supplied download link before fetching it.
///
/// WHY: the link comes FROM the OpenSubtitles API response body — a
/// compromised or spoofed response must not be able to redirect the fetch at
/// internal hosts (SSRF) or downgrade to plaintext.
fn validate_download_url(link: &str) -> Result<url::Url, ProsthekeError> {
    let parsed = url::Url::parse(link).map_err(|e| {
        DownloadFailedSnafu {
            detail: format!("invalid download link: {e}"),
        }
        .build()
    })?;

    if parsed.scheme() != "https" {
        return DownloadFailedSnafu {
            detail: format!(
                "download link scheme must be https, got {}",
                parsed.scheme()
            ),
        }
        .fail();
    }

    let host_ok = parsed
        .host_str()
        .is_some_and(|host| host == "opensubtitles.com" || host.ends_with(".opensubtitles.com"));
    if !host_ok {
        return DownloadFailedSnafu {
            detail: format!(
                "download link host is not an opensubtitles.com domain: {}",
                parsed.host_str().unwrap_or("<none>")
            ),
        }
        .fail();
    }

    Ok(parsed)
}

/// Read a response body up to `max_bytes`, failing once the declared or
/// accumulated size exceeds the cap.
async fn read_body_capped(
    mut response: reqwest::Response,
    max_bytes: u64,
) -> Result<Vec<u8>, ProsthekeError> {
    if let Some(declared) = response.content_length()
        && declared > max_bytes
    {
        return DownloadFailedSnafu {
            detail: format!("subtitle download exceeds the {max_bytes}-byte cap"),
        }
        .fail();
    }

    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.context(ProviderDownSnafu)? {
        let total = (bytes.len() as u64).saturating_add(chunk.len() as u64);
        if total > max_bytes {
            return DownloadFailedSnafu {
                detail: format!("subtitle download exceeds the {max_bytes}-byte cap"),
            }
            .fail();
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
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
                provider_id: SubtitleProviderId(item.id),
                language: item.attributes.language,
                hearing_impaired: item.attributes.hearing_impaired,
                forced: item.attributes.foreign_parts_only,
                score,
                download_url,
            });
        }

        Ok(matches)
    }

    #[instrument(skip(self, subtitle), fields(provider = "opensubtitles", provider_id = %subtitle.provider_id.0))]
    async fn download(&self, subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
        let Some(api_key) = self.api_key() else {
            return DownloadFailedSnafu {
                detail: "opensubtitles not configured".to_string(),
            }
            .fail();
        };

        // First request the download link FROM the API.
        // WHY: a non-numeric provider id must fail the request — a silent
        // file_id=0 fallback would download the wrong subtitle.
        let file_id: u64 = subtitle
            .provider_id
            .0
            .parse()
            .context(InvalidProviderIdSnafu {
                provider_id: subtitle.provider_id.0.clone(),
            })?;

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
        let download_url = validate_download_url(&dl_info.link)?;
        let max_bytes = self
            .config
            .as_ref()
            .map_or(DEFAULT_MAX_DOWNLOAD_BYTES, |c| c.max_download_bytes);
        let response = self
            .client
            .get(download_url)
            .send()
            .await
            .context(ProviderDownSnafu)?;

        read_body_capped(response, max_bytes).await
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
            ..OpenSubtitlesConfig::default()
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

    // NOTE: known-answer vector, hand-computed from the published
    // OpenSubtitles algorithm. 16 bytes = two u64 words (w1 = 8 bytes of
    // 0x01, w2 = 8 bytes of 0x02); head and tail windows both start at
    // offset 0 for a file under 64 KB, so each word is summed twice:
    // 16 + 2*0x0101010101010101 + 2*0x0202020202020202 = 0x0606060606060616.
    #[test]
    fn compute_file_hash_small_file_known_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.bin");
        let mut content = vec![0x01u8; 8];
        content.extend_from_slice(&[0x02u8; 8]);
        std::fs::write(&path, &content).unwrap();

        assert_eq!(compute_file_hash(&path).unwrap(), "0606060606060616");
    }

    // NOTE: known-answer vector, hand-computed. 128 KB file: head window is
    // all zeros (contributes nothing), tail window seeks to offset 65536 and
    // reads 8192 words of 0xFFFFFFFFFFFFFFFF, contributing
    // 8192 * (2^64 - 1) = -8192 (mod 2^64). Hash = 0x20000 - 0x2000 = 0x1e000.
    #[test]
    fn compute_file_hash_large_file_tail_seek_known_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        let mut content = vec![0u8; 128 * 1024];
        content[64 * 1024..].fill(0xFF);
        std::fs::write(&path, &content).unwrap();

        assert_eq!(compute_file_hash(&path).unwrap(), "000000000001e000");
    }

    #[test]
    fn compute_file_hash_ignores_bytes_between_the_windows() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain.bin");
        let mutated = dir.path().join("mutated.bin");

        let content = vec![0u8; 192 * 1024];
        std::fs::write(&plain, &content).unwrap();

        let mut content = content;
        // WHY: only the region outside both 64 KB windows is changed — the
        // hash must not see it.
        content[64 * 1024..128 * 1024].fill(0xAB);
        std::fs::write(&mutated, &content).unwrap();

        let plain_hash = compute_file_hash(&plain).unwrap();
        assert_eq!(plain_hash, compute_file_hash(&mutated).unwrap());
        // NOTE: both windows are zero, so the hash is the file size alone.
        assert_eq!(plain_hash, "0000000000030000");
    }

    #[test]
    fn compute_file_hash_empty_file_is_size_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.bin");
        std::fs::write(&path, b"").unwrap();

        assert_eq!(compute_file_hash(&path).unwrap(), "0000000000000000");
    }

    #[test]
    fn compute_file_hash_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(compute_file_hash(&dir.path().join("absent.bin")).is_err());
    }

    #[test]
    fn validate_download_url_accepts_opensubtitles_hosts() {
        assert!(validate_download_url("https://www.opensubtitles.com/download/abc.srt").is_ok());
        assert!(validate_download_url("https://opensubtitles.com/download/abc.srt").is_ok());
    }

    #[test]
    fn validate_download_url_rejects_foreign_and_internal_hosts() {
        for link in [
            "http://169.254.169.254/latest/meta-data",
            "http://localhost:1234/x.srt",
            "https://evil.com/x.srt",
            "https://evilopensubtitles.com/x.srt",
            "https://opensubtitles.com.evil.com/x.srt",
            "not a url",
        ] {
            let err = validate_download_url(link).unwrap_err();
            assert!(
                matches!(err, ProsthekeError::DownloadFailed { .. }),
                "{link} must be rejected before any request is issued"
            );
        }
    }

    #[test]
    fn validate_download_url_rejects_plain_http_even_on_allowed_host() {
        assert!(validate_download_url("https://www.opensubtitles.com/x.srt").is_ok());
        assert!(validate_download_url("http://www.opensubtitles.com/x.srt").is_err());
    }

    #[tokio::test]
    async fn download_rejects_non_numeric_provider_id_before_any_request() {
        let provider = OpenSubtitlesProvider::new(Some(OpenSubtitlesConfig {
            api_key: "key".to_string(),
            ..OpenSubtitlesConfig::default()
        }));
        let subtitle = SubtitleMatch {
            provider: "opensubtitles".to_string(),
            provider_id: SubtitleProviderId("abc".to_string()),
            language: "en".to_string(),
            hearing_impaired: false,
            forced: false,
            score: 0.9,
            download_url: "https://www.opensubtitles.com/x.srt".to_string(),
        };

        let err = provider.download(&subtitle).await.unwrap_err();
        assert!(
            matches!(err, ProsthekeError::InvalidProviderId { .. }),
            "non-numeric provider id must error, not silently become file_id=0: {err:?}"
        );
    }

    #[tokio::test]
    async fn read_body_capped_rejects_oversized_declared_body() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            // WHY: the request content is irrelevant — one read drains
            // enough of it to keep the client happy before responding.
            let _bytes_read = stream.read(&mut buf).await.unwrap();
            let body = "x".repeat(2048);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            stream.write_all(response.as_bytes()).await.ok();
        });

        let response = reqwest::get(format!("http://{addr}")).await.unwrap();
        let err = read_body_capped(response, 1024).await.unwrap_err();

        assert!(
            matches!(err, ProsthekeError::DownloadFailed { .. }),
            "oversized body must abort before buffering: {err:?}"
        );
        server.await.ok();
    }

    #[tokio::test]
    async fn read_body_capped_accepts_body_within_cap() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            // WHY: the request content is irrelevant — one read drains
            // enough of it to keep the client happy before responding.
            let _bytes_read = stream.read(&mut buf).await.unwrap();
            let body = "subtitle content";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let response = reqwest::get(format!("http://{addr}")).await.unwrap();
        let bytes = read_body_capped(response, 1024).await.unwrap();

        assert_eq!(bytes, b"subtitle content");
        server.await.unwrap();
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
