use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};

const BASE_URL: &str = "https://itunes.apple.com";

pub struct ItunesProvider {
    client: reqwest::Client,
}

impl ItunesProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItunesSearchResponse {
    results: Vec<ItunesPodcast>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItunesPodcast {
    collection_id: Option<u64>,
    collection_name: Option<String>,
    track_name: Option<String>,
    artist_name: Option<String>,
    release_date: Option<String>,
    feed_url: Option<String>,
    artwork_url600: Option<String>,
    genres: Option<Vec<String>>,
}

impl MetadataProvider for ItunesProvider {
    fn name(&self) -> &str {
        "itunes"
    }

    #[instrument(skip(self), fields(provider = "itunes"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let url = format!("{BASE_URL}/search");
        let response = self
            .client
            .get(&url)
            .query(&[
                ("term", query.title.as_str()),
                ("media", "podcast"),
                ("LIMIT", "10"),
            ])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "itunes" })?;

        let text = response
            .text()
            .await
            .context(ProviderRequestSnafu { provider: "itunes" })?;

        let parsed: ItunesSearchResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "itunes" })?;

        let results = parsed
            .results
            .into_iter()
            .filter_map(|pod| {
                let id = pod.collection_id?.to_string();
                let title = pod.collection_name.or(pod.track_name).unwrap_or_default();
                let year = pod
                    .release_date
                    .as_deref()
                    .and_then(|d| d.split('-').next())
                    .and_then(|y| y.parse().ok());
                let raw = serde_json::json!({
                    "feed_url": pod.feed_url,
                    "artwork": pod.artwork_url600,
                    "genres": pod.genres.unwrap_or_default(),
                });
                Some(ProviderResult {
                    provider_id: id,
                    title,
                    artist: pod.artist_name,
                    year,
                    score: 1.0,
                    raw,
                })
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "itunes"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let url = format!("{BASE_URL}/lookup");
        let response = self
            .client
            .get(&url)
            .query(&[("id", provider_id), ("media", "podcast")])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "itunes" })?;

        let text = response
            .text()
            .await
            .context(ProviderRequestSnafu { provider: "itunes" })?;

        let parsed: ItunesSearchResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "itunes" })?;

        let pod = parsed.results.into_iter().next().unwrap_or(ItunesPodcast {
            collection_id: None,
            collection_name: Some(String::new()),
            track_name: None,
            artist_name: None,
            release_date: None,
            feed_url: None,
            artwork_url600: None,
            genres: None,
        });

        let title = pod.collection_name.or(pod.track_name).unwrap_or_default();
        let year = pod
            .release_date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let extra = serde_json::json!({
            "feed_url": pod.feed_url,
            "artwork": pod.artwork_url600,
            "genres": pod.genres.unwrap_or_default(),
        });

        Ok(ProviderMetadata {
            provider_id: provider_id.to_string(),
            title,
            artist: pod.artist_name,
            year,
            extra,
        })
    }
}
