//! Plex Media Server API integration.

pub mod collections;
pub mod notify;
pub mod stats;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use horismos::PlexConfig;
use snafu::ResultExt;
use themelion::MediaType;

use crate::error::{PlexApiCallSnafu, SyndesmodError};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Abstraction over the Plex HTTP API, injectable for testing.
pub(crate) trait PlexApi: Send + Sync {
    fn refresh_library_section(&self, section_id: u32)
    -> BoxFuture<'_, Result<(), SyndesmodError>>;
}

/// Production Plex API client backed by reqwest.
pub struct PlexClient {
    http: reqwest::Client,
    pub(crate) config: PlexConfig,
}

impl PlexClient {
    pub fn new(config: PlexConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config
        Self { http, config }
    }

    /// Resolves the Plex library section ID for the given media type.
    pub(crate) fn section_id_for(&self, media_type: MediaType) -> Option<u32> {
        self.config.library_sections.get(&media_type).copied()
    }
}

impl PlexApi for PlexClient {
    fn refresh_library_section(
        &self,
        section_id: u32,
    ) -> BoxFuture<'_, Result<(), SyndesmodError>> {
        Box::pin(async move {
            let url = format!(
                "{}/library/sections/{}/refresh",
                self.config.url.trim_end_matches('/'),
                section_id,
            );
            self.http
                .get(&url)
                .header("X-Plex-Token", &self.config.token)
                .send()
                .await
                .context(PlexApiCallSnafu)?
                .error_for_status()
                .context(PlexApiCallSnafu)?;
            Ok(())
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    pub(crate) struct MockPlexApi {
        pub(crate) sections_refreshed: Arc<Mutex<Vec<u32>>>,
        pub(crate) fail_count: Arc<std::sync::atomic::AtomicU32>,
        pub(crate) delay_ms: Arc<std::sync::atomic::AtomicU64>,
    }

    impl MockPlexApi {
        pub(crate) fn new() -> Self {
            Self {
                sections_refreshed: Arc::new(Mutex::new(Vec::new())),
                fail_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
                delay_ms: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            }
        }

        /// Simulates a slow Plex endpoint — each call sleeps first.
        pub(crate) fn with_delay_ms(delay_ms: u64) -> Self {
            let mock = Self::new();
            mock.delay_ms
                .store(delay_ms, std::sync::atomic::Ordering::SeqCst);
            mock
        }

        #[expect(
            dead_code,
            reason = "available for future tests requiring pre-configured failures"
        )]
        pub(crate) fn with_failures(failures: u32) -> Self {
            let mock = Self::new();
            mock.fail_count
                .store(failures, std::sync::atomic::Ordering::SeqCst);
            mock
        }

        pub(crate) fn refreshed_sections(&self) -> Vec<u32> {
            self.sections_refreshed.lock().unwrap().clone()
        }
    }

    impl PlexApi for MockPlexApi {
        fn refresh_library_section(
            &self,
            section_id: u32,
        ) -> BoxFuture<'_, Result<(), SyndesmodError>> {
            let sections = self.sections_refreshed.clone();
            let fail_count = self.fail_count.clone();
            let delay_ms = self.delay_ms.load(std::sync::atomic::Ordering::SeqCst);
            Box::pin(async move {
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                let remaining = fail_count.fetch_update(
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                    |n| if n > 0 { Some(n - 1) } else { None },
                );
                if remaining.is_ok() {
                    return Err(SyndesmodError::PlexApiCall {
                        source: reqwest::Client::new()
                            .get("http://invalid.test/")
                            .build()
                            .unwrap_err(),
                        location: snafu::location!(),
                    });
                }
                sections.lock().unwrap().push(section_id);
                Ok(())
            })
        }
    }

    fn test_config(url: String) -> PlexConfig {
        PlexConfig {
            url,
            token: "plextok".to_string(),
            library_sections: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn refresh_library_section_errors_on_http_error_status() {
        let (base_url, server) =
            crate::test_support::spawn_one_shot_http(401, "Unauthorized", "").await;
        let client = PlexClient::new(test_config(base_url));

        let result = client.refresh_library_section(7).await;

        assert!(matches!(result, Err(SyndesmodError::PlexApiCall { .. })));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refresh_library_section_succeeds_on_ok_status() {
        let (base_url, server) = crate::test_support::spawn_one_shot_http(200, "OK", "").await;
        let client = PlexClient::new(test_config(base_url));

        client.refresh_library_section(7).await.unwrap();
        server.await.unwrap();
    }
}
