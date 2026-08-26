//! Tidal API integration.

pub mod scheduler;
pub mod wantlist;

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use horismos::TidalConfig;
use snafu::{OptionExt, ResultExt};
use tokio::sync::RwLock;

use crate::error::{
    AuthenticationFailedSnafu, ConfigMissingSnafu, SyndesmodError, TidalApiCallSnafu,
};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// WHY: refresh slightly before the server-side expiry so an in-flight call
// never carries a token that dies mid-request.
const TOKEN_EXPIRY_MARGIN_SECS: u64 = 60;

// WHY: domain newtype — external API identifier, not a semantic struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TidalId(pub String);

impl TidalId {
    /// Returns the raw string identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// WHY: API schema — Tidal favorites endpoint response item.
/// A Tidal favorite track entry returned by the favorites endpoint.
#[derive(Debug, Clone)]
pub struct TidalFavorite {
    pub tidal_id: TidalId,
    pub title: String,
    pub artist: String,
}

/// Abstraction over the Tidal HTTP API, injectable for testing.
pub(crate) trait TidalApi: Send + Sync {
    fn fetch_favorites(&self) -> BoxFuture<'_, Result<Vec<TidalFavorite>, SyndesmodError>>;
}

/// A bearer token held by `TidalClient`'s cache.
///
/// `valid_until: None` marks the statically configured access token — its
/// expiry is unknown to the client, so it is trusted until the API rejects
/// it (401), at which point the cache is dropped and the refresh token takes
/// over. Refreshed tokens carry the instant they stop being valid.
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    valid_until: Option<Instant>,
}

impl CachedToken {
    fn is_valid(&self) -> bool {
        self.valid_until.is_none_or(|until| Instant::now() < until)
    }
}

/// Production Tidal API client backed by reqwest.
pub struct TidalClient {
    http: reqwest::Client,
    pub(crate) config: TidalConfig,
    base_url: String,
    auth_base_url: String,
    // WHY: interior mutability — TidalApi methods take &self. Token cache
    // pattern mirrors the TVDB provider's (epignosis).
    token: RwLock<Option<CachedToken>>,
}

impl TidalClient {
    const DEFAULT_BASE_URL: &'static str = "https://openapi.tidal.com";
    const DEFAULT_AUTH_BASE_URL: &'static str = "https://auth.tidal.com";

    pub fn new(config: TidalConfig) -> Self {
        Self::with_base_url(config, Self::DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(config: TidalConfig, base_url: String) -> Self {
        Self::with_base_urls(config, base_url, Self::DEFAULT_AUTH_BASE_URL.to_string())
    }

    pub fn with_base_urls(config: TidalConfig, base_url: String, auth_base_url: String) -> Self {
        // WHY unwrap_or_default: only catches a genuinely-invalid TLS
        // config (an Err). It does NOT catch reqwest's rustls-no-provider
        // "no crypto provider installed" failure — that's a panic!, not an
        // Err, and unwrap_or_default() cannot intercept a panic. Safe here
        // only because every caller (production via main.rs, tests via
        // install_test_crypto_provider) installs the provider first.
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        let seeded = config.access_token.as_ref().map(|token| CachedToken {
            access_token: token.clone(),
            valid_until: None,
        });
        Self {
            http,
            config,
            base_url,
            auth_base_url,
            token: RwLock::new(seeded),
        }
    }

    /// True when the OAuth client credentials and refresh token needed to
    /// rotate the access token are all configured.
    fn refresh_configured(&self) -> bool {
        !self.config.client_id.is_empty()
            && !self.config.client_secret.is_empty()
            && self.config.refresh_token.is_some()
    }

    /// Returns a bearer token for the API, refreshing first when the cached
    /// token is absent or past its reuse window. Returns `None` when no
    /// token is configured at all.
    async fn bearer_token(&self) -> Result<Option<String>, SyndesmodError> {
        if let Some(cached) = self.token.read().await.as_ref()
            && cached.is_valid()
        {
            return Ok(Some(cached.access_token.clone()));
        }

        let mut slot = self.token.write().await;
        // WHY: double-checked — a concurrent caller may have refreshed the
        // token while this one waited for the write lock.
        if let Some(cached) = slot.as_ref()
            && cached.is_valid()
        {
            return Ok(Some(cached.access_token.clone()));
        }

        if self.refresh_configured() {
            let refreshed = self.refresh_access_token().await?;
            let token = refreshed.access_token.clone();
            *slot = Some(refreshed);
            return Ok(Some(token));
        }

        Ok(None)
    }

    /// Exchanges the configured refresh token for a fresh access token at
    /// the Tidal OAuth token endpoint.
    async fn refresh_access_token(&self) -> Result<CachedToken, SyndesmodError> {
        let url = format!("{}/v1/oauth2/token", self.auth_base_url);
        let response = self
            .http
            .post(&url)
            .form(&[
                ("grant_type", "refresh_token"),
                (
                    "refresh_token",
                    self.config.refresh_token.as_deref().unwrap_or_default(),
                ),
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
            ])
            .send()
            .await
            .context(TidalApiCallSnafu)?;

        // WHY: a 401 here means the refresh token itself is rejected — that
        // is an authentication failure, not a transient API error.
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return AuthenticationFailedSnafu {
                service: "tidal".to_string(),
            }
            .fail();
        }

        let parsed: TidalTokenResponse = response
            .error_for_status()
            .context(TidalApiCallSnafu)?
            .json()
            .await
            .context(TidalApiCallSnafu)?;

        Ok(CachedToken {
            access_token: parsed.access_token,
            valid_until: Some(
                Instant::now()
                    + Duration::from_secs(
                        parsed.expires_in.saturating_sub(TOKEN_EXPIRY_MARGIN_SECS),
                    ),
            ),
        })
    }

    /// Drops the cached token after the API answers 401, so the next call
    /// re-authenticates instead of replaying a rejected bearer.
    async fn invalidate_token(&self) {
        *self.token.write().await = None;
    }

    async fn get_favorites(
        &self,
        url: &str,
        token: &str,
    ) -> Result<reqwest::Response, SyndesmodError> {
        self.http
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .context(TidalApiCallSnafu)
    }

    #[cfg(test)]
    async fn seed_token(&self, token: &str, valid_until: Option<Instant>) {
        *self.token.write().await = Some(CachedToken {
            access_token: token.to_string(),
            valid_until,
        });
    }
}

// WHY: API schema — Tidal OAuth token endpoint response.
#[derive(Debug, serde::Deserialize)]
struct TidalTokenResponse {
    access_token: String,
    expires_in: u64,
}

impl TidalApi for TidalClient {
    fn fetch_favorites(&self) -> BoxFuture<'_, Result<Vec<TidalFavorite>, SyndesmodError>> {
        Box::pin(async move {
            let Some(token) = self.bearer_token().await? else {
                return Ok(vec![]);
            };

            let url = format!("{}/v2/my-collection/tracks/favoriteTracks", self.base_url);
            let response = self.get_favorites(&url, &token).await?;

            // WHY: a 401 means the server no longer honors the cached token;
            // drop it, re-authenticate via the refresh token, and retry once
            // with the fresh bearer. Without refresh credentials the 401
            // surfaces unchanged, preserving static-token behavior.
            let response = if response.status() == reqwest::StatusCode::UNAUTHORIZED
                && self.refresh_configured()
            {
                self.invalidate_token().await;
                let fresh = self.bearer_token().await?.context(ConfigMissingSnafu {
                    service: "tidal".to_string(),
                })?;
                self.get_favorites(&url, &fresh).await?
            } else {
                response
            };

            let body: serde_json::Value = response
                .error_for_status()
                .context(TidalApiCallSnafu)?
                .json()
                .await
                .context(TidalApiCallSnafu)?;
            Ok(parse_favorites(&body))
        })
    }
}

pub(crate) fn parse_favorites(body: &serde_json::Value) -> Vec<TidalFavorite> {
    body.get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let resource = item.get("resource")?;
                    let tidal_id = TidalId(resource.get("id")?.as_str()?.to_string());
                    let title = resource.get("title")?.as_str()?.to_string();
                    let artist = resource
                        .get("artists")
                        .and_then(|a| a.as_array())
                        .and_then(|a| a.first())
                        .and_then(|a| a.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    Some(TidalFavorite {
                        tidal_id,
                        title,
                        artist,
                    })
                })
                .collect()
        })
        .unwrap_or_default() // WHY: Option chain — .map from .and_then produces Option, not Result
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use super::*;

    pub(crate) struct MockTidalApi {
        pub(crate) favorites: Vec<TidalFavorite>,
        pub(crate) call_count: Arc<std::sync::atomic::AtomicU32>,
    }

    impl MockTidalApi {
        pub(crate) fn new(favorites: Vec<TidalFavorite>) -> Self {
            Self {
                favorites,
                call_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            }
        }
    }

    impl TidalApi for MockTidalApi {
        fn fetch_favorites(&self) -> BoxFuture<'_, Result<Vec<TidalFavorite>, SyndesmodError>> {
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let fav = self.favorites.clone();
            Box::pin(async move { Ok(fav) })
        }
    }

    /// Static-token-only config — no OAuth refresh credentials.
    fn test_config() -> TidalConfig {
        TidalConfig {
            access_token: Some("tidaltok".to_string()),
            ..TidalConfig::default()
        }
    }

    /// Full OAuth config: client credentials + refresh token, seeded with a
    /// static access token.
    fn oauth_config() -> TidalConfig {
        TidalConfig {
            client_id: "client-id".to_string(),
            client_secret: "client-secret".to_string(),
            access_token: Some("seeded-access".to_string()),
            refresh_token: Some("refresh-tok".to_string()),
            sync_interval_minutes: 60,
        }
    }

    fn token_body(token: &str) -> String {
        serde_json::json!({ "access_token": token, "token_type": "Bearer", "expires_in": 3600 })
            .to_string()
    }

    #[tokio::test]
    async fn fetch_favorites_errors_on_http_error_status() {
        let (base_url, server) =
            crate::test_support::spawn_one_shot_http(429, "Too Many Requests", "{}").await;
        let client = TidalClient::with_base_url(test_config(), base_url);

        let result = client.fetch_favorites().await;

        assert!(matches!(result, Err(SyndesmodError::TidalApiCall { .. })));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn fetch_favorites_succeeds_on_ok_status() {
        let (base_url, server) =
            crate::test_support::spawn_one_shot_http(200, "OK", r#"{"data":[]}"#).await;
        let client = TidalClient::with_base_url(test_config(), base_url);

        let favorites = client.fetch_favorites().await.unwrap();
        assert!(favorites.is_empty());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn fetch_favorites_returns_empty_when_no_token_configured() {
        // WHY: TidalClient::new builds a real reqwest client (unlike
        // with_base_url elsewhere in this file, this test never spawns a
        // mock server first); see test_support::install_test_crypto_provider's
        // WHY note.
        crate::test_support::install_test_crypto_provider();
        let client = TidalClient::new(TidalConfig::default());

        let favorites = client.fetch_favorites().await.unwrap();
        assert!(favorites.is_empty());
    }

    #[tokio::test]
    async fn expired_cached_token_refreshes_before_the_api_call() {
        let (base_url, auth_url, server) = crate::test_support::spawn_sequential_http(vec![
            (200, token_body("tok-fresh")),
            (200, r#"{"data":[]}"#.to_string()),
        ])
        .await;
        let client = TidalClient::with_base_urls(oauth_config(), base_url, auth_url);
        // NOTE: a deadline of "now" is already past by the time the cache is read.
        client.seed_token("tok-stale", Some(Instant::now())).await;

        client.fetch_favorites().await.unwrap();

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2, "one token refresh plus one API call");
        assert!(requests[0].starts_with("POST /v1/oauth2/token"));
        let body = requests[0].to_ascii_lowercase();
        assert!(
            body.contains("grant_type=refresh_token"),
            "refresh grant: {body}"
        );
        assert!(
            body.contains("refresh_token=refresh-tok"),
            "refresh token: {body}"
        );
        assert!(body.contains("client_id=client-id"), "client id: {body}");
        assert!(
            body.contains("client_secret=client-secret"),
            "client secret: {body}"
        );
        assert!(
            requests[1]
                .to_ascii_lowercase()
                .contains("authorization: bearer tok-fresh"),
            "the API call must carry the freshly refreshed token, not the stale one"
        );
    }

    #[tokio::test]
    async fn valid_cached_token_skips_refresh() {
        let (base_url, auth_url, server) =
            crate::test_support::spawn_sequential_http(vec![(200, r#"{"data":[]}"#.to_string())])
                .await;
        let client = TidalClient::with_base_urls(oauth_config(), base_url, auth_url);
        client
            .seed_token("tok-live", Some(Instant::now() + Duration::from_secs(600)))
            .await;

        client.fetch_favorites().await.unwrap();

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1, "no refresh request may be issued");
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("authorization: bearer tok-live")
        );
    }

    #[tokio::test]
    async fn unauthorized_response_refreshes_and_retries_once() {
        let (base_url, auth_url, server) = crate::test_support::spawn_sequential_http(vec![
            (401, "{}".to_string()),
            (200, token_body("tok-next")),
            (200, r#"{"data":[]}"#.to_string()),
        ])
        .await;
        // WHY: the seeded static token (no known expiry) is trusted until the
        // API rejects it — the 401 must drop it and rotate via refresh_token.
        let client = TidalClient::with_base_urls(oauth_config(), base_url, auth_url);

        client.fetch_favorites().await.unwrap();

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3, "rejected call, refresh, retried call");
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("authorization: bearer seeded-access"),
            "the first call carries the seeded static token"
        );
        assert!(
            requests[1].starts_with("POST /v1/oauth2/token"),
            "a 401 must force a token refresh"
        );
        assert!(
            requests[2]
                .to_ascii_lowercase()
                .contains("authorization: bearer tok-next"),
            "the retried call must carry the re-issued token"
        );
    }

    #[tokio::test]
    async fn unauthorized_static_token_without_refresh_credentials_surfaces_error() {
        let (base_url, server) =
            crate::test_support::spawn_one_shot_http(401, "Unauthorized", "{}").await;
        let client = TidalClient::with_base_url(test_config(), base_url);

        let result = client.fetch_favorites().await;

        assert!(
            matches!(result, Err(SyndesmodError::TidalApiCall { .. })),
            "without refresh credentials a 401 surfaces unchanged: {result:?}"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejected_refresh_token_is_an_authentication_failure() {
        let (base_url, auth_url, server) =
            crate::test_support::spawn_sequential_http(vec![(401, "{}".to_string())]).await;
        let mut config = oauth_config();
        config.access_token = None; // no seed — the first call must refresh
        let client = TidalClient::with_base_urls(config, base_url, auth_url);

        let result = client.fetch_favorites().await;

        assert!(
            matches!(result, Err(SyndesmodError::AuthenticationFailed { .. })),
            "a 401 from the token endpoint is an auth failure: {result:?}"
        );
        server.await.unwrap();
    }
}
