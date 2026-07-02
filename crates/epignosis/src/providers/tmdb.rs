use serde::Deserialize;
use snafu::ResultExt;
use tracing::instrument;

use super::{MetadataProvider, MetadataProviderId, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://api.themoviedb.org/3";

pub struct TmdbProvider {
    client: reqwest::Client,
    /// TMDB API Read Access Token (v4), sent as a Bearer header.
    ///
    /// WHY: a v3 `api_key` query parameter leaks the credential INTO server
    /// and proxy access logs; header auth keeps it out of the URL.
    api_key: String,
    base_url: String,
    pub(crate) max_body_bytes: u64,
}

impl TmdbProvider {
    pub fn new(client: reqwest::Client, api_key: impl Into<String>) -> Self {
        Self::with_base_url(client, api_key, BASE_URL.to_string())
    }

    pub fn with_base_url(
        client: reqwest::Client,
        api_key: impl Into<String>,
        base_url: String,
    ) -> Self {
        Self {
            client,
            api_key: api_key.into(),
            base_url,
            max_body_bytes: super::DEFAULT_MAX_BODY_BYTES,
        }
    }

    /// Cross-references an external provider ID (e.g. a TVDB series id) to a
    /// TMDB TV id via `/find/{external_id}`. Returns `None` when TMDB holds
    /// no mapping for that external id.
    #[instrument(skip(self), fields(provider = "tmdb"))]
    pub async fn find_by_external_id(
        &self,
        external_id: &str,
        external_source: &str,
    ) -> Result<Option<MetadataProviderId>, EpignosisError> {
        let url = format!("{}/find/{external_id}", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[("external_source", external_source)])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let text = super::read_body_limited(response, "tmdb", self.max_body_bytes).await?;

        let parsed: TmdbFindResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tmdb" })?;

        Ok(parsed
            .tv_results
            .into_iter()
            .next()
            .map(|tv| MetadataProviderId(tv.id.to_string())))
    }

    /// Fetches TV-series detail from `/tv/{id}`.
    ///
    /// TMDB's TV ids are a separate namespace from the movie ids served by
    /// `get_metadata`, with a distinct response shape (`name` and
    /// `first_air_date` instead of `title` and `release_date`).
    #[instrument(skip(self), fields(provider = "tmdb"))]
    pub async fn get_tv_metadata(
        &self,
        provider_id: &str,
    ) -> Result<ProviderMetadata, EpignosisError> {
        let url = format!("{}/tv/{provider_id}", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let text = super::read_body_limited(response, "tmdb", self.max_body_bytes).await?;

        let series: TmdbTvDetail =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tmdb" })?;

        let year = series
            .first_air_date
            .as_deref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let genres: Vec<String> = series
            .genres
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.name)
            .collect();

        let extra = serde_json::json!({
            "overview": series.overview,
            "seasons": series.number_of_seasons,
            "genres": genres,
        });

        Ok(ProviderMetadata {
            provider_id: MetadataProviderId(series.id.to_string()),
            title: series.name,
            artist: None,
            year,
            extra,
        })
    }
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbMovie>,
}

#[derive(Debug, Deserialize)]
struct TmdbFindResponse {
    #[serde(default)]
    tv_results: Vec<TmdbTvResult>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvResult {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct TmdbTvDetail {
    id: u64,
    name: String,
    first_air_date: Option<String>,
    overview: Option<String>,
    number_of_seasons: Option<u32>,
    genres: Option<Vec<TmdbGenre>>,
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
        let url = format!("{}/search/movie", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[("query", query.title.as_str()), ("page", "1")])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let text = super::read_body_limited(response, "tmdb", self.max_body_bytes).await?;

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
                    provider_id: MetadataProviderId(movie.id.to_string()),
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
        let url = format!("{}/movie/{provider_id}", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tmdb" })?;

        let text = super::read_body_limited(response, "tmdb", self.max_body_bytes).await?;

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
            provider_id: MetadataProviderId(movie.id.to_string()),
            title: movie.title,
            artist: None,
            year,
            extra,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::spawn_sequential_http;

    #[tokio::test]
    async fn find_by_external_id_returns_tv_namespace_id() {
        let body = serde_json::json!({
            "movie_results": [],
            "tv_results": [{ "id": 1396, "name": "Breaking Bad" }],
        })
        .to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, body)]).await;
        let provider = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let found = provider
            .find_by_external_id("81189", "tvdb_id")
            .await
            .unwrap();

        assert_eq!(found, Some(MetadataProviderId("1396".to_string())));
        let requests = handle.await.unwrap();
        assert!(requests[0].starts_with("GET /find/81189"));
        assert!(
            requests[0].contains("external_source=tvdb_id"),
            "cross-reference must name the source namespace"
        );
    }

    #[tokio::test]
    async fn find_by_external_id_no_mapping_returns_none() {
        let body = serde_json::json!({ "movie_results": [], "tv_results": [] }).to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, body)]).await;
        let provider = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let found = provider
            .find_by_external_id("999999", "tvdb_id")
            .await
            .unwrap();

        assert_eq!(found, None);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn find_by_external_id_tolerates_missing_tv_results_field() {
        let body = serde_json::json!({ "movie_results": [] }).to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, body)]).await;
        let provider = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let found = provider
            .find_by_external_id("81189", "tvdb_id")
            .await
            .unwrap();

        assert_eq!(found, None);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn get_tv_metadata_hits_tv_endpoint_and_parses_tv_shape() {
        let body = serde_json::json!({
            "id": 1396,
            "name": "Breaking Bad",
            "first_air_date": "2008-01-20",
            "overview": "A chemistry teacher turns to crime.",
            "number_of_seasons": 5,
            "genres": [{ "name": "Drama" }, { "name": "Crime" }],
        })
        .to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, body)]).await;
        let provider = TmdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        let meta = provider.get_tv_metadata("1396").await.unwrap();

        assert_eq!(meta.provider_id, MetadataProviderId("1396".to_string()));
        assert_eq!(meta.title, "Breaking Bad");
        assert_eq!(meta.year, Some(2008));
        assert_eq!(meta.extra["seasons"], 5);
        assert_eq!(meta.extra["genres"][0], "Drama");
        let requests = handle.await.unwrap();
        assert!(
            requests[0].starts_with("GET /tv/1396"),
            "TV detail must address the /tv namespace, not /movie"
        );
    }

    #[tokio::test]
    async fn requests_carry_bearer_auth_and_no_api_key_query_param() {
        let body = serde_json::json!({ "movie_results": [], "tv_results": [] }).to_string();
        let (base_url, handle) = spawn_sequential_http(vec![(200, body)]).await;
        let provider =
            TmdbProvider::with_base_url(reqwest::Client::new(), "secret-token", base_url);

        provider
            .find_by_external_id("81189", "tvdb_id")
            .await
            .unwrap();

        let requests = handle.await.unwrap();
        assert!(
            requests[0].contains("authorization: Bearer secret-token"),
            "the credential must travel in the Authorization header"
        );
        assert!(
            !requests[0].contains("api_key"),
            "the credential must not appear in the URL query string"
        );
    }
}
