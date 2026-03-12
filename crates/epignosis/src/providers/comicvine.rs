use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};

const BASE_URL: &str = "https://comicvine.gamespot.com/api";

pub struct ComicVineProvider {
    client: reqwest::Client,
    api_key: String,
}

impl ComicVineProvider {
    pub fn new(client: reqwest::Client, api_key: impl Into<String>) -> Self {
        Self {
            client,
            api_key: api_key.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CvSearchResponse {
    results: Vec<CvVolume>,
}

#[derive(Debug, Deserialize)]
struct CvVolume {
    id: u64,
    name: String,
    publisher: Option<CvPublisher>,
    start_year: Option<String>,
    description: Option<String>,
    image: Option<CvImage>,
}

#[derive(Debug, Deserialize)]
struct CvPublisher {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CvImage {
    medium_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CvVolumeDetail {
    results: CvVolume,
}

impl MetadataProvider for ComicVineProvider {
    fn name(&self) -> &str {
        "comicvine"
    }

    #[instrument(skip(self), fields(provider = "comicvine"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let url = format!("{BASE_URL}/search/");
        let response = self
            .client
            .get(&url)
            .query(&[
                ("api_key", self.api_key.as_str()),
                ("query", &query.title),
                ("resources", "volume"),
                ("format", "json"),
                ("limit", "10"),
            ])
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "comicvine",
            })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "comicvine",
        })?;

        let parsed: CvSearchResponse = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "comicvine",
        })?;

        let results = parsed
            .results
            .into_iter()
            .map(|vol| {
                let year: Option<u32> = vol.start_year.as_deref().and_then(|y| y.parse().ok());
                let artist = vol.publisher.map(|p| p.name);
                let raw = serde_json::json!({
                    "cv_id": vol.id,
                    "image": vol.image.and_then(|i| i.medium_url),
                });
                ProviderResult {
                    provider_id: format!("4050-{}", vol.id),
                    title: vol.name,
                    artist,
                    year,
                    score: 1.0,
                    raw,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "comicvine"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let url = format!("{BASE_URL}/volume/{provider_id}/");
        let response = self
            .client
            .get(&url)
            .query(&[("api_key", self.api_key.as_str()), ("format", "json")])
            .send()
            .await
            .context(ProviderRequestSnafu {
                provider: "comicvine",
            })?;

        let text = response.text().await.context(ProviderRequestSnafu {
            provider: "comicvine",
        })?;

        let detail: CvVolumeDetail = serde_json::from_str(&text).context(ProviderParseSnafu {
            provider: "comicvine",
        })?;

        let vol = detail.results;
        let year: Option<u32> = vol.start_year.as_deref().and_then(|y| y.parse().ok());
        let artist = vol.publisher.map(|p| p.name);

        let extra = serde_json::json!({
            "description": vol.description,
            "image": vol.image.and_then(|i| i.medium_url),
        });

        Ok(ProviderMetadata {
            provider_id: format!("4050-{}", vol.id),
            title: vol.name,
            artist,
            year,
            extra,
        })
    }
}
