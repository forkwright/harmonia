use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use super::{MetadataProvider, MetadataProviderId, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://musicbrainz.org/ws/2";
const USER_AGENT: &str = "Harmonia/0.1 (https://github.com/harmonia)";

pub struct MusicBrainzProvider {
    client: reqwest::Client,
    pub(crate) max_body_bytes: u64,
}

impl MusicBrainzProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            max_body_bytes: super::DEFAULT_MAX_BODY_BYTES,
        }
    }
}

/// Escape Lucene query syntax characters in a user-supplied term.
///
/// WHY: title/artist come FROM user-controlled file tags; unescaped quotes or
/// operators would alter the structure of the query sent to MusicBrainz.
fn lucene_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '+' | '-'
                | '&'
                | '|'
                | '!'
                | '('
                | ')'
                | '{'
                | '}'
                | '['
                | ']'
                | '^'
                | '"'
                | '~'
                | '*'
                | '?'
                | ':'
                | '\\'
                | '/'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Build the Lucene search expression for a recording query.
fn build_lucene_query(query: &SearchQuery) -> String {
    let mut lucene = format!("recording:\"{}\"", lucene_escape(&query.title));
    if let Some(artist) = &query.artist {
        lucene.push_str(&format!(" AND artist:\"{}\"", lucene_escape(artist)));
    }
    lucene
}

#[derive(Debug, Deserialize)]
struct MbRecording {
    id: String,
    title: String,
    score: Option<u32>,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MbArtistCredit>>,
    #[serde(rename = "first-release-date")]
    first_release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MbArtistCredit {
    artist: MbArtist,
}

#[derive(Debug, Deserialize)]
struct MbArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MbSearchResponse {
    recordings: Vec<MbRecording>,
}

#[derive(Debug, Deserialize)]
struct MbRelease {
    id: String,
    title: String,
    date: Option<String>,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MbArtistCredit>>,
}

impl MetadataProvider for MusicBrainzProvider {
    fn name(&self) -> &str {
        "musicbrainz"
    }

    #[instrument(skip(self), fields(provider = "musicbrainz"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let lucene = build_lucene_query(query);

        let url = format!("{BASE_URL}/recording");
        let response = self
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .query(&[
                ("query", &lucene),
                ("fmt", &"json".to_string()),
                ("LIMIT", &"10".to_string()),
            ])
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "musicbrainz",
            })?;

        let text = super::read_body_limited(response, "musicbrainz", self.max_body_bytes).await?;

        let parsed: MbSearchResponse = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "musicbrainz",
        })?;

        let results = parsed
            .recordings
            .into_iter()
            .map(|rec| {
                let artist = rec
                    .artist_credit
                    .as_deref()
                    .and_then(|ac| ac.first())
                    .map(|ac| ac.artist.name.clone());
                let year = rec
                    .first_release_date
                    .as_deref()
                    .and_then(|d| d.split('-').next())
                    .and_then(|y| y.parse().ok());
                let score = rec.score.unwrap_or(0) as f64 / 100.0;
                let raw = serde_json::json!({ "mb_recording_id": rec.id });
                ProviderResult {
                    provider: "musicbrainz".to_string(),
                    provider_id: MetadataProviderId(rec.id),
                    title: rec.title,
                    artist,
                    year,
                    score,
                    raw,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "musicbrainz"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let url = format!("{BASE_URL}/release/{provider_id}");
        let response = self
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .query(&[("fmt", "json"), ("inc", "artist-credits recordings")])
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "musicbrainz",
            })?;

        let text = super::read_body_limited(response, "musicbrainz", self.max_body_bytes).await?;

        let release: MbRelease = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "musicbrainz",
        })?;

        let artist = release
            .artist_credit
            .as_deref()
            .and_then(|ac| ac.first())
            .map(|ac| ac.artist.name.clone());
        let year = release
            .date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        Ok(ProviderMetadata {
            provider_id: MetadataProviderId(release.id),
            title: release.title,
            artist,
            year,
            extra: serde_json::Value::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    use aggelmata::MediaType;

    use super::*;

    fn query(title: &str, artist: Option<&str>) -> SearchQuery {
        SearchQuery {
            media_type: MediaType::Music,
            title: title.to_string(),
            artist: artist.map(str::to_owned),
            year: None,
            isbn: None,
            extra: None,
        }
    }

    #[test]
    fn build_lucene_query_plain_terms_unchanged() {
        let q = query("Song Title", Some("The Artist"));
        assert_eq!(
            build_lucene_query(&q),
            "recording:\"Song Title\" AND artist:\"The Artist\""
        );
    }

    #[test]
    fn build_lucene_query_escapes_embedded_quotes() {
        let q = query("foo\" OR *:*", None);
        let lucene = build_lucene_query(&q);
        assert_eq!(lucene, "recording:\"foo\\\" OR \\*\\:\\*\"");
        assert!(
            !lucene.contains("foo\" OR"),
            "an unescaped quote must not break out of the recording clause"
        );
    }

    #[test]
    fn build_lucene_query_escapes_artist_operators() {
        let q = query("Song", Some("a && b (c)"));
        assert_eq!(
            build_lucene_query(&q),
            "recording:\"Song\" AND artist:\"a \\&\\& b \\(c\\)\""
        );
    }

    #[test]
    fn lucene_escape_passes_through_safe_text() {
        assert_eq!(lucene_escape("plain text 123"), "plain text 123");
    }
}
