use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};

const BASE_URL: &str = "https://api4.thetvdb.com/v4";

pub struct TvdbProvider {
    client: reqwest::Client,
    api_key: SecretString,
}

impl TvdbProvider {
    pub fn new(client: reqwest::Client, api_key: impl Into<String>) -> Self {
        Self {
            client,
            api_key: api_key.INTO(),
        }
    }

    async fn bearer_token(&self) -> Result<String, EpignosisError> {
        let url = format!("{BASE_URL}/login");
        let body = serde_json::json!({ "apikey": self.api_key });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        let text = response
            .text()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        let parsed: TvdbLoginResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tvdb" })?;

        Ok(parsed.data.token)
    }
}

#[derive(Debug, Deserialize)]
struct TvdbLoginResponse {
    data: TvdbToken,
}

#[derive(Debug, Deserialize)]
struct TvdbToken {
    token: SecretString,
}

#[derive(Debug, Deserialize)]
struct TvdbSearchResponse {
    data: Option<Vec<TvdbSeries>>,
}

#[derive(Debug, Deserialize)]
struct TvdbSeries {
    #[serde(rename = "tvdb_id")]
    tvdb_id: Option<String>,
    name: String,
    year: Option<String>,
    overview: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TvdbSeriesDetail {
    data: TvdbSeriesData,
}

#[derive(Debug, Deserialize)]
struct TvdbSeriesData {
    id: u64,
    name: String,
    year: Option<String>,
    overview: Option<String>,
    genres: Option<Vec<TvdbGenre>>,
}

#[derive(Debug, Deserialize)]
struct TvdbGenre {
    name: String,
}

impl MetadataProvider for TvdbProvider {
    fn name(&self) -> &str {
        "tvdb"
    }

    #[instrument(skip(self), fields(provider = "tvdb"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let token = self.bearer_token().await?;
        let url = format!("{BASE_URL}/search");

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .query(&[("query", &query.title), ("type", &"series".to_string())])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        let text = response
            .text()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        let parsed: TvdbSearchResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tvdb" })?;

        let results = parsed
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|series| {
                let year: Option<u32> = series.year.as_deref().and_then(|y| y.parse().ok());
                let id = series.tvdb_id.unwrap_or_default();
                let raw = serde_json::json!({ "overview": series.overview, "tvdb_id": id });
                ProviderResult {
                    provider_id: id,
                    title: series.name,
                    artist: None,
                    year,
                    score: 1.0,
                    raw,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "tvdb"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let token = self.bearer_token().await?;
        let url = format!("{BASE_URL}/series/{provider_id}");

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        let text = response
            .text()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        let detail: TvdbSeriesDetail =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tvdb" })?;

        let year: Option<u32> = detail.data.year.as_deref().and_then(|y| y.parse().ok());
        let genres: Vec<String> = detail
            .data
            .genres
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.name)
            .collect();

        let extra = serde_json::json!({
            "overview": detail.data.overview,
            "genres": genres,
        });

        Ok(ProviderMetadata {
            provider_id: detail.data.id.to_string(),
            title: detail.data.name,
            artist: None,
            year,
            extra,
        })
    }
}
