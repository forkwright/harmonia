use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://musicbrainz.org/ws/2";
const USER_AGENT: &str = "Harmonia/0.1 (https://github.com/harmonia)";

pub struct MusicBrainzProvider {
    client: reqwest::Client,
}

impl MusicBrainzProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
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
        let mut lucene = format!("recording:\"{}\"", query.title);
        if let Some(artist) = &query.artist {
            lucene.push_str(&format!(" AND artist:\"{}\"", artist));
        }

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

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "musicbrainz",
        })?;

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
                    provider_id: rec.id,
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

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "musicbrainz",
        })?;

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
            provider_id: release.id,
            title: release.title,
            artist,
            year,
            extra: serde_json::Value::Null,
        })
    }
}
