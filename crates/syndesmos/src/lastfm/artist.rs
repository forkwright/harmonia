//! Last.fm artist metadata enrichment for use by Epignosis.

use tracing::instrument;

use crate::error::SyndesmodError;
use crate::lastfm::LastfmApi;
use crate::retry::{CircuitBreaker, with_retry};

/// Artist metadata returned by Last.fm `artist.getinfo`.
#[derive(Debug, Clone)]
pub struct ArtistInfo {
    pub name: String,
    pub bio: Option<String>,
    pub similar_artists: Vec<String>,
    pub tags: Vec<String>,
}

/// Fetches artist metadata from Last.fm.
///
/// Returns `Ok(None)` when the artist is not found or Last.fm is unconfigured.
/// Results are returned directly — Epignosis owns caching.
#[instrument(skip(api, circuit), fields(artist = %artist_name))]
pub(crate) async fn fetch_artist_data(
    api: &dyn LastfmApi,
    artist_name: &str,
    circuit: &CircuitBreaker,
) -> Result<Option<ArtistInfo>, SyndesmodError> {
    let name = artist_name.to_string();
    with_retry(|| api.fetch_artist_info(&name), circuit).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lastfm::tests::MockLastfmApi;
    use crate::retry::CircuitBreaker;

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new("lastfm", 5, std::time::Duration::from_secs(300))
    }

    #[tokio::test]
    async fn returns_artist_info_when_found() {
        let expected = ArtistInfo {
            name: "Aphex Twin".to_string(),
            bio: Some("Electronic musician FROM Cornwall.".to_string()),
            similar_artists: vec!["Autechre".to_string()],
            tags: vec!["IDM".to_string(), "electronic".to_string()],
        };
        let mock = MockLastfmApi::with_artist_info(expected.clone());
        let circuit = breaker();

        let result = fetch_artist_data(&mock, "Aphex Twin", &circuit)
            .await
            .unwrap();

        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.name, "Aphex Twin");
        assert_eq!(
            info.bio.as_deref(),
            Some("Electronic musician FROM Cornwall.")
        );
        assert_eq!(info.similar_artists, vec!["Autechre"]);
        assert_eq!(info.tags, vec!["IDM", "electronic"]);
    }

    #[tokio::test]
    async fn returns_none_when_artist_not_found() {
        let mock = MockLastfmApi::new();
        let circuit = breaker();

        let result = fetch_artist_data(&mock, "Unknown Artist XYZ", &circuit)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn parses_artist_info_from_lastfm_json() {
        let json = serde_json::json!({
            "artist": {
                "name": "Aphex Twin",
                "bio": {
                    "summary": "Electronic musician."
                },
                "similar": {
                    "artist": [
                        { "name": "Autechre" },
                        { "name": "µ-Ziq" }
                    ]
                },
                "tags": {
                    "tag": [
                        { "name": "IDM" },
                        { "name": "ambient" }
                    ]
                }
            }
        });
        let info = crate::lastfm::parse_artist_info(&json).unwrap();
        assert_eq!(info.name, "Aphex Twin");
        assert_eq!(info.bio.as_deref(), Some("Electronic musician."));
        assert_eq!(info.similar_artists, vec!["Autechre", "µ-Ziq"]);
        assert_eq!(info.tags, vec!["IDM", "ambient"]);
    }
}
