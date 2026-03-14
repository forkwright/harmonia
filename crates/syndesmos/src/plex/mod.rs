//! Plex Media Server API integration.

pub mod collections;
pub mod notify;
pub mod stats;

use std::{future::Future, pin::Pin, time::Duration};

use harmonia_common::MediaType;
use horismos::PlexConfig;
use snafu::ResultExt;

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
            .unwrap_or_default();
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
    }

    impl MockPlexApi {
        pub(crate) fn new() -> Self {
            Self {
                sections_refreshed: Arc::new(Mutex::new(Vec::new())),
            }
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
            Box::pin(async move {
                sections.lock().unwrap().push(section_id);
                Ok(())
            })
        }
    }
}
