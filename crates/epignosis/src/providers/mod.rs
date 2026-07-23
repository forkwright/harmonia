use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};
use themelion::MediaType;

use crate::error::{
    EpignosisError, ProviderHttpStatusSnafu, ProviderRequestSnafu, ProviderResponseTooLargeSnafu,
};

pub mod acoustid;
pub mod audnexus;
pub mod comicvine;
pub mod googlebooks;
pub mod itunes;
pub mod musicbrainz;
pub mod openlibrary;
pub mod tmdb;
pub mod tvdb;

pub use crate::identity::MetadataProviderId;

/// Default cap on a buffered provider response body.
///
/// Overridden per resolver FROM `horismos::EpignosisConfig::provider_response_max_bytes`.
pub(crate) const DEFAULT_MAX_BODY_BYTES: u64 = 10 * 1024 * 1024;

/// Reject a non-2xx response with `ProviderHttpStatus`, then read the body up
/// to `max_bytes`, failing with `ProviderResponseTooLarge` once the declared
/// or accumulated size exceeds it.
///
/// WHY: `reqwest::Response::text()` buffers unbounded — a misbehaving provider
/// response must not be able to exhaust memory. The status guard keeps a 4xx/
/// 5xx error page from reaching `serde_json` and surfacing as a parse error.
///
/// NOTE: callers that treat a specific status as data (the 404 = clean-miss
/// paths in audnexus and openlibrary) must branch on `response.status()`
/// BEFORE calling this.
pub(crate) async fn read_body_limited(
    mut response: reqwest::Response,
    provider: &str,
    max_bytes: u64,
) -> Result<String, EpignosisError> {
    let status = response.status();
    ensure!(
        status.is_success(),
        ProviderHttpStatusSnafu { provider, status }
    );

    if let Some(declared) = response.content_length() {
        ensure!(
            declared <= max_bytes,
            ProviderResponseTooLargeSnafu {
                provider,
                limit: max_bytes,
            }
        );
    }

    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.context(ProviderRequestSnafu {
        provider: provider.to_string(),
    })? {
        let total = (bytes.len() as u64).saturating_add(chunk.len() as u64);
        ensure!(
            total <= max_bytes,
            ProviderResponseTooLargeSnafu {
                provider,
                limit: max_bytes,
            }
        );
        bytes.extend_from_slice(&chunk);
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

// WHY: pure data — query parameters for a metadata provider search.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub media_type: MediaType,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub isbn: Option<String>,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResult {
    pub provider: String,
    pub provider_id: MetadataProviderId,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub score: f64,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub provider_id: MetadataProviderId,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub extra: serde_json::Value,
}

#[expect(
    async_fn_in_trait,
    reason = "async fn in trait is stable since Rust 1.75; suppressed until Send bound concern is resolved"
)]
pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError>;

    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::spawn_sequential_http;

    #[tokio::test]
    async fn read_body_limited_accepts_body_within_cap() {
        let (base_url, handle) = spawn_sequential_http(vec![(200, "small body".to_string())]).await;
        let response = reqwest::get(&base_url).await.unwrap();

        let text = read_body_limited(response, "test", 1024).await.unwrap();

        assert_eq!(text, "small body");
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn read_body_limited_rejects_non_success_status() {
        let (base_url, handle) =
            spawn_sequential_http(vec![(500, "upstream error page".to_string())]).await;
        let response = reqwest::get(&base_url).await.unwrap();

        let err = read_body_limited(response, "test", 1024).await.unwrap_err();

        assert!(
            matches!(
                &err,
                EpignosisError::ProviderHttpStatus { status, .. }
                    if *status == reqwest::StatusCode::INTERNAL_SERVER_ERROR
            ),
            "a non-2xx response must surface as a status error, not a parse error: {err:?}"
        );
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn read_body_limited_rejects_client_error_status() {
        let (base_url, handle) =
            spawn_sequential_http(vec![(404, "{\"error\": \"missing\"}".to_string())]).await;
        let response = reqwest::get(&base_url).await.unwrap();

        let err = read_body_limited(response, "test", 1024).await.unwrap_err();

        assert!(
            matches!(
                &err,
                EpignosisError::ProviderHttpStatus { status, .. }
                    if *status == reqwest::StatusCode::NOT_FOUND
            ),
            "a 4xx that reaches the general guard must fail on status: {err:?}"
        );
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn read_body_limited_rejects_oversized_body() {
        let big = "x".repeat(2048);
        let (base_url, handle) = spawn_sequential_http(vec![(200, big)]).await;
        let response = reqwest::get(&base_url).await.unwrap();

        let err = read_body_limited(response, "test", 1024).await.unwrap_err();

        assert!(
            matches!(
                err,
                EpignosisError::ProviderResponseTooLarge { limit: 1024, .. }
            ),
            "oversized body must abort before buffering: {err:?}"
        );
        // WHY: the server may see a reset mid-write once the client aborts;
        // await it only to avoid a task leak, without asserting on its result.
        handle.await.ok();
    }
}
