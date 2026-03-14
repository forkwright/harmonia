//! Tidal API integration.

pub mod wantlist;

use std::{future::Future, pin::Pin, time::Duration};

use horismos::TidalConfig;
use snafu::ResultExt;

use crate::error::{SyndesmodError, TidalApiCallSnafu};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A Tidal favorite track entry returned by the favorites endpoint.
#[derive(Debug, Clone)]
pub struct TidalFavorite {
    pub tidal_id: String,
    pub title: String,
    pub artist: String,
}

/// Abstraction over the Tidal HTTP API, injectable for testing.
pub(crate) trait TidalApi: Send + Sync {
    fn fetch_favorites(&self) -> BoxFuture<'_, Result<Vec<TidalFavorite>, SyndesmodError>>;
}

/// Production Tidal API client backed by reqwest.
pub struct TidalClient {
    http: reqwest::Client,
    pub(crate) config: TidalConfig,
    base_url: String,
}

impl TidalClient {
    const DEFAULT_BASE_URL: &'static str = "https://openapi.tidal.com";

    pub fn new(config: TidalConfig) -> Self {
        Self::with_base_url(config, Self::DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(config: TidalConfig, base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self {
            http,
            config,
            base_url,
        }
    }

    fn access_token(&self) -> Option<&str> {
        self.config.access_token.as_deref()
    }
}

impl TidalApi for TidalClient {
    fn fetch_favorites(&self) -> BoxFuture<'_, Result<Vec<TidalFavorite>, SyndesmodError>> {
        Box::pin(async move {
            let token = match self.access_token() {
                Some(t) => t.to_string(),
                None => return Ok(vec![]),
            };

            let url = format!("{}/v2/my-collection/tracks/favoriteTracks", self.base_url);
            let response = self
                .http
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .context(TidalApiCallSnafu)?;

            let body: serde_json::Value = response.json().await.context(TidalApiCallSnafu)?;
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
                    let tidal_id = resource.get("id")?.as_str()?.to_string();
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
        .unwrap_or_default()
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
}
