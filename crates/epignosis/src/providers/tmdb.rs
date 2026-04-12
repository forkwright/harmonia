use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

use super::{MetadataProvider, ProviderMetadata, ProviderResult, SearchQuery};

const BASE_URL: &str = "https://api.themoviedb.org/3";

pub struct TmdbProvider {
    client: reqwest::Client,
    api_key: String,
}

impl TmdbProvider {
    pub fn new(client: reqwest::Client, api_key: impl Into<String>) -> Self {
        Self {
            client,
            api_key: api_key.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbMovie>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovie {
    id: u64,
    title: String,
    release_date: Option<String>,
    popularity: Option<f64>,
    overview: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieDetail {
    id: u64,
    title: String,
    release_date: Option<String>,
    overview: Option<String>,
    runtime: Option<u32>,
    genres: Option<Vec<TmdbGenre>>,
}

#[derive(Debug, Deserialize)]
struct TmdbGenre {
    name: String,
}

impl MetadataProvider for TmdbProvider {
    fn name(&self) -> &str {
        "tmdb"
    }

    #[instrument(skip(self), fields(provider = "tmdb"))]
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError> {
        let url = format!("{BASE_URL}/search/movie");
        let response = self
            .client
            .get(&url)
            .query(&[
                ("api_key", self.api_key.as_str()),
                ("query", &query.title),
                ("page", "1"),
            ])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let text = response
            .text()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let parsed: TmdbSearchResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tmdb" })?;

        let results = parsed
            .results
            .into_iter()
            .map(|movie| {
                let year = movie
                    .release_date
                    .as_deref()
                    .and_then(|d| d.split('-').next())
                    .and_then(|y| y.parse().ok());
                let score = movie.popularity.unwrap_or(0.0) / 1000.0;
                let raw = serde_json::json!({
                    "overview": movie.overview,
                    "tmdb_id": movie.id,
                });
                ProviderResult {
                    provider_id: movie.id.to_string(),
                    title: movie.title,
                    artist: None,
                    year,
                    score: score.clamp(0.0, 1.0),
                    raw,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self), fields(provider = "tmdb"))]
    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError> {
        let url = format!("{BASE_URL}/movie/{provider_id}");
        let response = self
            .client
            .get(&url)
            .query(&[("api_key", self.api_key.as_str())])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let text = response
            .text()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let movie: TmdbMovieDetail =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tmdb" })?;

        let year = movie
            .release_date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let genres: Vec<String> = movie
            .genres
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.name)
            .collect();

        let extra = serde_json::json!({
            "overview": movie.overview,
            "runtime_mins": movie.runtime,
            "genres": genres,
        });

        Ok(ProviderMetadata {
            provider_id: movie.id.to_string(),
            title: movie.title,
            artist: None,
            year,
            extra,
        })
    }
}
