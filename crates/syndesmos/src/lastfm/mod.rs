//! Last.fm API integration.

pub mod artist;
pub mod auth;
pub mod scrobble;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use horismos::LastfmConfig;
use snafu::ResultExt;

use crate::error::{LastfmApiCallSnafu, SyndesmodError};
use crate::lastfm::artist::ArtistInfo;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Parameters required to scrobble a single track.
#[derive(Debug, Clone)]
pub struct ScrobbleParams {
    pub artist: String,
    pub track: String,
    pub album: Option<String>,
    pub timestamp: i64,
}

/// Abstraction over the Last.fm HTTP API, injectable for testing.
pub(crate) trait LastfmApi: Send + Sync {
    fn submit_scrobble(&self, params: ScrobbleParams) -> BoxFuture<'_, Result<(), SyndesmodError>>;

    fn fetch_artist_info(
        &self,
        artist_name: &str,
    ) -> BoxFuture<'_, Result<Option<ArtistInfo>, SyndesmodError>>;
}

/// Production Last.fm API client backed by reqwest.
pub struct LastfmClient {
    http: reqwest::Client,
    pub(crate) config: LastfmConfig,
    base_url: String,
}

impl LastfmClient {
    const DEFAULT_BASE_URL: &'static str = "https://ws.audioscrobbler.com/2.0";

    pub fn new(config: LastfmConfig) -> Self {
        Self::with_base_url(config, Self::DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(config: LastfmConfig, base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            http,
            config,
            base_url,
        }
    }

    fn session_key(&self) -> Option<&str> {
        self.config.session_key.as_deref()
    }
}

impl LastfmApi for LastfmClient {
    fn submit_scrobble(&self, params: ScrobbleParams) -> BoxFuture<'_, Result<(), SyndesmodError>> {
        Box::pin(async move {
            let session_key = match self.session_key() {
                Some(k) => k.to_string(),
                None => return Ok(()),
            };
            let mut form = vec![
                ("method", "track.scrobble"),
                ("api_key", self.config.api_key.as_str()),
                ("sk", session_key.as_str()),
                ("format", "json"),
            ];
            let artist = params.artist.as_str();
            let track = params.track.as_str();
            let timestamp = params.timestamp.to_string();
            form.push(("artist[0]", artist));
            form.push(("track[0]", track));
            form.push(("timestamp[0]", &timestamp));

            let album_ref;
            if let Some(album) = &params.album {
                album_ref = album.clone();
                form.push(("album[0]", album_ref.as_str()));
            }

            self.http
                .post(&self.base_url)
                .form(&form)
                .send()
                .await
                .context(LastfmApiCallSnafu)?;
            Ok(())
        })
    }

    fn fetch_artist_info(
        &self,
        artist_name: &str,
    ) -> BoxFuture<'_, Result<Option<ArtistInfo>, SyndesmodError>> {
        let artist_name = artist_name.to_string();
        Box::pin(async move {
            let response = self
                .http
                .get(&self.base_url)
                .query(&[
                    ("method", "artist.getinfo"),
                    ("artist", &artist_name),
                    ("api_key", &self.config.api_key),
                    ("format", "json"),
                ])
                .send()
                .await
                .context(LastfmApiCallSnafu)?;

            let body: serde_json::Value = response.json().await.context(LastfmApiCallSnafu)?;

            if body.get("error").is_some() {
                return Ok(None);
            }

            let info = parse_artist_info(&body);
            Ok(info)
        })
    }
}

fn parse_artist_info(body: &serde_json::Value) -> Option<ArtistInfo> {
    let artist = body.get("artist")?;
    let name = artist.get("name")?.as_str()?.to_string();
    let bio = artist
        .get("bio")
        .and_then(|b| b.get("summary"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let similar_artists = artist
        .get("similar")
        .and_then(|s| s.get("artist"))
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| entry.get("name")?.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let tags = artist
        .get("tags")
        .and_then(|t| t.get("tag"))
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| entry.get("name")?.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some(ArtistInfo {
        name,
        bio,
        similar_artists,
        tags,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    pub(crate) struct MockLastfmApi {
        pub(crate) scrobbles_submitted: Arc<Mutex<Vec<ScrobbleParams>>>,
        pub(crate) artist_info_response: Option<ArtistInfo>,
    }

    impl MockLastfmApi {
        pub(crate) fn new() -> Self {
            Self {
                scrobbles_submitted: Arc::new(Mutex::new(Vec::new())),
                artist_info_response: None,
            }
        }

        pub(crate) fn with_artist_info(info: ArtistInfo) -> Self {
            Self {
                scrobbles_submitted: Arc::new(Mutex::new(Vec::new())),
                artist_info_response: Some(info),
            }
        }

        pub(crate) fn submitted_scrobbles(&self) -> Vec<ScrobbleParams> {
            self.scrobbles_submitted.lock().unwrap().clone()
        }
    }

    impl LastfmApi for MockLastfmApi {
        fn submit_scrobble(
            &self,
            params: ScrobbleParams,
        ) -> BoxFuture<'_, Result<(), SyndesmodError>> {
            let submitted = self.scrobbles_submitted.clone();
            Box::pin(async move {
                submitted.lock().unwrap().push(params);
                Ok(())
            })
        }

        fn fetch_artist_info(
            &self,
            _artist_name: &str,
        ) -> BoxFuture<'_, Result<Option<ArtistInfo>, SyndesmodError>> {
            let info = self.artist_info_response.clone();
            Box::pin(async move { Ok(info) })
        }
    }
}
