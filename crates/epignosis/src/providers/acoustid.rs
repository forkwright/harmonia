use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};
use crate::identity::FingerprintResult;

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};

const BASE_URL: &str = "https://api.acoustid.org/v2";

pub struct AcoustIdProvider {
    client: reqwest::Client,
    api_key: String,
}

impl AcoustIdProvider {
    pub fn new(client: reqwest::Client, api_key: impl Into<String>) -> Self {
        Self {
            client,
            api_key: api_key.into(),
        }
    }

    /// Look up a pre-computed chromaprint fingerprint against the AcoustID service.
    #[instrument(skip(self), fields(provider = "acoustid"))]
    pub async fn lookup_fingerprint(
        &self,
        fingerprint: &FingerprintResult,
    ) -> Result<Vec<ProviderResult>, EpignosisError> {
        let url = format!("{BASE_URL}/lookup");
        let duration_str = (fingerprint.duration_secs as u64).to_string();
        let response = self
            .client
            .get(&url)
            .query(&[
                ("client", self.api_key.as_str()),
                ("duration", &duration_str),
                ("fingerprint", &fingerprint.fingerprint),
                ("meta", "recordings"),
            ])
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "acoustid",
            })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "acoustid",
        })?;

        let parsed: AcoustIdResponse = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "acoustid",
        })?;

        let results = parsed
            .results
            .unwrap_or_default()
            .into_iter()
            .flat_map(|result| {
                let score = result.score.unwrap_or(0.0);
                let acoustid = result.id.clone();
                result
                    .recordings
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |rec| {
                        let artist = rec
                            .artists
                            .as_deref()
                            .and_then(|a| a.first())
                            .map(|a| a.name.clone());
                        let raw = serde_json::json!({
                            "acoustid": acoustid,
                            "mb_recording_id": rec.id,
                        });
                        ProviderResult {
                            provider_id: rec.id,
                            title: rec.title.unwrap_or_default(),
                            artist,
                            year: None,
                            score,
                            raw,
                        }
                    })
            })
            .collect();

        Ok(results)
    }
}

#[derive(Debug, Deserialize)]
struct AcoustIdResponse {
    results: Option<Vec<AcoustIdResult>>,
}

#[derive(Debug, Deserialize)]
struct AcoustIdResult {
    id: String,
    score: Option<f64>,
    recordings: Option<Vec<AcoustIdRecording>>,
}

#[derive(Debug, Deserialize)]
struct AcoustIdRecording {
    id: String,
    title: Option<String>,
    artists: Option<Vec<AcoustIdArtist>>,
}

#[derive(Debug, Deserialize)]
struct AcoustIdArtist {
    name: String,
}

impl MetadataProvider for AcoustIdProvider {
    fn name(&self) -> &str {
        "acoustid"
    }

    #[instrument(skip(self), fields(provider = "acoustid"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        // AcoustID is fingerprint-based; text search is not its primary mode.
        // Use MusicBrainz for text-based music search. Return empty here.
        let _ = query;
        Ok(vec![])
    }

    #[instrument(skip(self), fields(provider = "acoustid"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let url = format!("{BASE_URL}/lookup");
        let response = self
            .client
            .get(&url)
            .query(&[
                ("client", self.api_key.as_str()),
                ("trackid", provider_id),
                ("meta", "recordings"),
            ])
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "acoustid",
            })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "acoustid",
        })?;

        let parsed: AcoustIdResponse = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "acoustid",
        })?;

        let first_rec = parsed
            .results
            .unwrap_or_default()
            .into_iter()
            .next()
            .and_then(|r| r.recordings.unwrap_or_default().into_iter().next());

        match first_rec {
            Some(rec) => {
                let artist = rec
                    .artists
                    .as_deref()
                    .and_then(|a| a.first())
                    .map(|a| a.name.clone());
                Ok(ProviderMetadata {
                    provider_id: rec.id,
                    title: rec.title.unwrap_or_default(),
                    artist,
                    year: None,
                    extra: serde_json::Value::Null,
                })
            }
            None => Ok(ProviderMetadata {
                provider_id: provider_id.to_string(),
                title: String::new(),
                artist: None,
                year: None,
                extra: serde_json::Value::Null,
            }),
        }
    }
}
