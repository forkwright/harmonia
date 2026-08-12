use std::time::{Duration, Instant};

use serde::Deserialize;
use snafu::ResultExt;
use tokio::sync::RwLock;
use tracing::instrument;

use super::{MetadataProvider, MetadataProviderId, ProviderMetadata, ProviderResult, SearchQuery};
use crate::error::{EpignosisError, ProviderParseSnafu, ProviderRequestSnafu};

const BASE_URL: &str = "https://api4.thetvdb.com/v4";

// NOTE: TVDB JWTs are valid for roughly a month; one day is a conservative
// reuse window that keeps a stale token from ever reaching the API.
const TOKEN_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub struct TvdbProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    // NOTE: interior mutability — MetadataProvider methods take &self.
    // Holds the bearer token and the instant it stops being valid.
    token: RwLock<Option<(String, Instant)>>,
    pub(crate) max_body_bytes: u64,
}

impl TvdbProvider {
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
            token: RwLock::new(None),
            max_body_bytes: super::DEFAULT_MAX_BODY_BYTES,
        }
    }

    /// Returns the cached bearer token, logging in only on a cache miss or
    /// after the reuse window has elapsed.
    async fn bearer_token(&self) -> Result<String, EpignosisError> {
        if let Some((token, valid_until)) = self.token.read().await.as_ref()
            && Instant::now() < *valid_until
        {
            return Ok(token.clone());
        }

        let mut slot = self.token.write().await;
        // WHY: double-checked — a concurrent caller may have refreshed the
        // token while this one waited for the write lock.
        if let Some((token, valid_until)) = slot.as_ref()
            && Instant::now() < *valid_until
        {
            return Ok(token.clone());
        }

        let url = format!("{}/login", self.base_url);
        let body = serde_json::json!({ "apikey": self.api_key });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        let text = super::read_body_limited(response, "tvdb", self.max_body_bytes).await?;

        let parsed: TvdbLoginResponse =
            serde_json::from_str(&text).context(ProviderParseSnafu { provider: "tvdb" })?;

        let token = parsed.data.token;
        *slot = Some((token.clone(), Instant::now() + TOKEN_TTL));
        Ok(token)
    }

    /// Drops the cached bearer when the API answers 401 Unauthorized.
    ///
    /// WHY: a 401 means the server no longer honors the cached token; keeping
    /// it would fail every call until the TTL expires. Clearing it makes the
    /// next call re-authenticate.
    async fn invalidate_token_on_unauthorized(&self, status: reqwest::StatusCode) {
        if status == reqwest::StatusCode::UNAUTHORIZED {
            *self.token.write().await = None;
        }
    }

    #[cfg(test)]
    async fn seed_token(&self, token: &str, valid_until: Instant) {
        *self.token.write().await = Some((token.to_string(), valid_until));
    }
}

#[derive(Debug, Deserialize)]
struct TvdbLoginResponse {
    data: TvdbToken,
}

#[derive(Debug, Deserialize)]
struct TvdbToken {
    token: String,
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
        let url = format!("{}/search", self.base_url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .query(&[("query", &query.title), ("type", &"series".to_string())])
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        self.invalidate_token_on_unauthorized(response.status())
            .await;
        let text = super::read_body_limited(response, "tvdb", self.max_body_bytes).await?;

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
                    provider: "tvdb".to_string(),
                    provider_id: MetadataProviderId(id),
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
        let url = format!("{}/series/{provider_id}", self.base_url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context(ProviderRequestSnafu { provider: "tvdb" })?;

        self.invalidate_token_on_unauthorized(response.status())
            .await;
        let text = super::read_body_limited(response, "tvdb", self.max_body_bytes).await?;

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
            provider_id: MetadataProviderId(detail.data.id.to_string()),
            title: detail.data.name,
            artist: None,
            year,
            extra,
        })
    }
}

#[cfg(test)]
mod tests {
    use aggelmata::MediaType;

    use super::*;
    use crate::test_support::spawn_sequential_http;

    fn tv_query(title: &str) -> SearchQuery {
        SearchQuery {
            media_type: MediaType::Tv,
            title: title.to_string(),
            artist: None,
            year: None,
            isbn: None,
            extra: None,
        }
    }

    fn login_body(token: &str) -> String {
        serde_json::json!({ "data": { "token": token } }).to_string()
    }

    fn empty_search_body() -> String {
        serde_json::json!({ "data": [] }).to_string()
    }

    fn series_body(id: u64) -> String {
        serde_json::json!({ "data": { "id": id, "name": "Show", "year": "2008" } }).to_string()
    }

    #[tokio::test]
    async fn token_fetched_once_and_reused_across_calls() {
        let (base_url, handle) = spawn_sequential_http(vec![
            (200, login_body("tok-1")),
            (200, empty_search_body()),
            (200, empty_search_body()),
            (200, series_body(42)),
        ])
        .await;
        let provider = TvdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        provider.search(&tv_query("Breaking Bad")).await.unwrap();
        provider.search(&tv_query("The Wire")).await.unwrap();
        provider.get_metadata("42").await.unwrap();

        let requests = handle.await.unwrap();
        assert_eq!(requests.len(), 4, "exactly one login plus three API calls");
        assert!(requests[0].starts_with("POST /login"));
        assert!(requests[1].starts_with("GET /search"));
        assert!(
            requests[2].starts_with("GET /search"),
            "second search must reuse the cached token, not re-login: {}",
            requests[2].lines().next().unwrap_or_default()
        );
        assert!(requests[3].starts_with("GET /series/42"));
        for request in &requests[1..] {
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer tok-1"),
                "cached token must be sent on every authenticated call"
            );
        }
    }

    #[tokio::test]
    async fn expired_token_triggers_fresh_login() {
        let (base_url, handle) = spawn_sequential_http(vec![
            (200, login_body("tok-fresh")),
            (200, empty_search_body()),
        ])
        .await;
        let provider = TvdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);
        // NOTE: a deadline of "now" is already past by the time the cache is read.
        provider.seed_token("tok-stale", Instant::now()).await;

        provider.search(&tv_query("Severance")).await.unwrap();

        let requests = handle.await.unwrap();
        assert!(requests[0].starts_with("POST /login"));
        assert!(
            requests[1]
                .to_ascii_lowercase()
                .contains("authorization: bearer tok-fresh"),
            "stale token must be replaced, not reused"
        );
    }

    #[tokio::test]
    async fn valid_cached_token_skips_login_entirely() {
        let (base_url, handle) = spawn_sequential_http(vec![(200, empty_search_body())]).await;
        let provider = TvdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);
        provider
            .seed_token("tok-live", Instant::now() + Duration::from_secs(60))
            .await;

        provider.search(&tv_query("Dark")).await.unwrap();

        let requests = handle.await.unwrap();
        assert_eq!(requests.len(), 1, "no login request may be issued");
        assert!(requests[0].starts_with("GET /search"));
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("authorization: bearer tok-live")
        );
    }

    #[tokio::test]
    async fn unauthorized_response_clears_cached_token() {
        let (base_url, handle) = spawn_sequential_http(vec![
            (401, "{}".to_string()),
            (200, login_body("tok-next")),
            (200, empty_search_body()),
        ])
        .await;
        let provider = TvdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);
        provider
            .seed_token("tok-revoked", Instant::now() + Duration::from_secs(60))
            .await;

        let err = provider.search(&tv_query("Andor")).await.unwrap_err();
        assert!(
            matches!(
                &err,
                EpignosisError::ProviderHttpStatus { status, .. }
                    if *status == reqwest::StatusCode::UNAUTHORIZED
            ),
            "a 401 must surface as a status error: {err:?}"
        );

        provider.search(&tv_query("Andor")).await.unwrap();

        let requests = handle.await.unwrap();
        assert!(requests[0].starts_with("GET /search"));
        assert!(
            requests[1].starts_with("POST /login"),
            "a 401 must clear the cached token and force a fresh login"
        );
        assert!(
            requests[2]
                .to_ascii_lowercase()
                .contains("authorization: bearer tok-next"),
            "the retried call must carry the re-issued token"
        );
    }

    #[tokio::test]
    async fn failed_login_caches_nothing() {
        let (base_url, handle) = spawn_sequential_http(vec![
            (200, "not json".to_string()),
            (200, login_body("tok-2")),
            (200, empty_search_body()),
        ])
        .await;
        let provider = TvdbProvider::with_base_url(reqwest::Client::new(), "key", base_url);

        assert!(provider.search(&tv_query("Lost")).await.is_err());
        provider.search(&tv_query("Lost")).await.unwrap();

        let requests = handle.await.unwrap();
        assert!(requests[0].starts_with("POST /login"));
        assert!(
            requests[1].starts_with("POST /login"),
            "a failed login must not poison the cache"
        );
        assert!(requests[2].starts_with("GET /search"));
    }
}
