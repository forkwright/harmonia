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

// WHY: wire DTO — Last.fm scrobble submission parameters.
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
            .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config
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
            let form = build_scrobble_form(&self.config, &session_key, &params);

            self.http
                .post(&self.base_url)
                .form(&form)
                .send()
                .await
                .context(LastfmApiCallSnafu)?
                .error_for_status()
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
                .context(LastfmApiCallSnafu)?
                .error_for_status()
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

/// Builds the `track.scrobble` POST form, signed per the Last.fm API spec.
///
/// `api_sig` must cover every parameter except `format` (`sign_params`
/// excludes `format` itself), so it is appended after all other parameters
/// are final. Last.fm rejects unsigned write calls with error 13.
fn build_scrobble_form(
    config: &LastfmConfig,
    session_key: &str,
    params: &ScrobbleParams,
) -> Vec<(String, String)> {
    let mut form = vec![
        ("method".to_string(), "track.scrobble".to_string()),
        ("api_key".to_string(), config.api_key.clone()),
        ("sk".to_string(), session_key.to_string()),
        ("format".to_string(), "json".to_string()),
        ("artist[0]".to_string(), params.artist.clone()),
        ("track[0]".to_string(), params.track.clone()),
        ("timestamp[0]".to_string(), params.timestamp.to_string()),
    ];
    if let Some(album) = &params.album {
        form.push(("album[0]".to_string(), album.clone()));
    }

    let view: Vec<(&str, &str)> = form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let api_sig = auth::sign_params(&view, &config.shared_secret);
    form.push(("api_sig".to_string(), api_sig));
    form
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
        .unwrap_or_default(); // WHY: Option chain — .map produces Option, not Result

    let tags = artist
        .get("tags")
        .and_then(|t| t.get("tag"))
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| entry.get("name")?.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default(); // WHY: Option chain — .map produces Option, not Result

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

    fn test_config() -> LastfmConfig {
        LastfmConfig {
            api_key: "key123".to_string(),
            shared_secret: "sekrit".to_string(),
            session_key: Some("sess789".to_string()),
        }
    }

    fn test_params() -> ScrobbleParams {
        ScrobbleParams {
            artist: "Boards of Canada".to_string(),
            track: "Roygbiv".to_string(),
            album: Some("Music Has the Right to Children".to_string()),
            timestamp: 1_700_000_000,
        }
    }

    #[test]
    fn scrobble_form_includes_api_sig_signed_over_params() {
        let form = build_scrobble_form(&test_config(), "sess789", &test_params());

        let (last_key, api_sig) = form.last().map(|(k, v)| (k.as_str(), v.clone())).unwrap();
        assert_eq!(last_key, "api_sig");

        let unsigned: Vec<(&str, &str)> = form
            .iter()
            .filter(|(k, _)| k != "api_sig")
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let expected = auth::sign_params(&unsigned, "sekrit");
        assert_eq!(api_sig, expected);
        assert_eq!(api_sig.len(), 32);
    }

    #[test]
    fn scrobble_form_carries_all_track_parameters() {
        let form = build_scrobble_form(&test_config(), "sess789", &test_params());

        let get = |key: &str| form.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
        assert_eq!(get("method"), Some("track.scrobble"));
        assert_eq!(get("api_key"), Some("key123"));
        assert_eq!(get("sk"), Some("sess789"));
        assert_eq!(get("artist[0]"), Some("Boards of Canada"));
        assert_eq!(get("track[0]"), Some("Roygbiv"));
        assert_eq!(get("timestamp[0]"), Some("1700000000"));
        assert_eq!(get("album[0]"), Some("Music Has the Right to Children"));
    }

    #[test]
    fn scrobble_form_omits_album_when_absent() {
        let params = ScrobbleParams {
            album: None,
            ..test_params()
        };
        let form = build_scrobble_form(&test_config(), "sess789", &params);
        assert!(form.iter().all(|(k, _)| k != "album[0]"));
    }

    #[tokio::test]
    async fn submit_scrobble_sends_api_sig_in_form_body() {
        let (base_url, server) = crate::test_support::spawn_one_shot_http(200, "OK", "{}").await;
        let client = LastfmClient::with_base_url(test_config(), base_url);

        client.submit_scrobble(test_params()).await.unwrap();

        let request = server.await.unwrap();
        let form = build_scrobble_form(&test_config(), "sess789", &test_params());
        let (_, api_sig) = form.last().unwrap();
        assert!(request.contains(&format!("api_sig={api_sig}")));
        assert!(request.contains("method=track.scrobble"));
        assert!(request.contains("sk=sess789"));
    }

    #[tokio::test]
    async fn submit_scrobble_errors_on_http_error_status() {
        let (base_url, server) =
            crate::test_support::spawn_one_shot_http(401, "Unauthorized", "{}").await;
        let client = LastfmClient::with_base_url(test_config(), base_url);

        let result = client.submit_scrobble(test_params()).await;

        assert!(matches!(result, Err(SyndesmodError::LastfmApiCall { .. })));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn fetch_artist_info_errors_on_http_error_status() {
        let (base_url, server) =
            crate::test_support::spawn_one_shot_http(500, "Internal Server Error", "{}").await;
        let client = LastfmClient::with_base_url(test_config(), base_url);

        let result = client.fetch_artist_info("Autechre").await;

        assert!(matches!(result, Err(SyndesmodError::LastfmApiCall { .. })));
        server.await.unwrap();
    }
}
