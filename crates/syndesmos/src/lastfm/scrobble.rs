//! Last.fm track scrobble submission.

use aggelmata::{MediaId, UserId};
use jiff::Timestamp;
use tracing::instrument;

use crate::error::SyndesmodError;
use crate::lastfm::{LastfmApi, ScrobbleParams};
use crate::retry::{CircuitBreaker, with_retry};

// WHY: catalog metadata resolved by the caller — Last.fm requires real
// artist/title names, and internal IDs carry neither.
/// Track metadata resolved from the local catalog for scrobble submission.
#[derive(Debug, Clone)]
pub(crate) struct TrackMetadata {
    pub(crate) artist: String,
    pub(crate) title: String,
    pub(crate) album: Option<String>,
}

/// Submits a scrobble to Last.fm for the given track and user.
///
/// `metadata` must carry the track's resolved artist/title/album; this
/// function performs no catalog lookups itself.
#[instrument(skip(api, circuit, metadata), fields(track_id = %track_id, user_id = %user_id))]
pub(crate) async fn scrobble(
    api: &dyn LastfmApi,
    track_id: MediaId,
    user_id: UserId,
    metadata: TrackMetadata,
    circuit: &CircuitBreaker,
) -> Result<(), SyndesmodError> {
    let params = ScrobbleParams {
        artist: metadata.artist,
        track: metadata.title,
        album: metadata.album,
        timestamp: Timestamp::now().as_second(),
    };

    with_retry(|| api.submit_scrobble(params.clone()), circuit).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lastfm::tests::MockLastfmApi;
    use crate::retry::CircuitBreaker;

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new("lastfm", 5, std::time::Duration::from_secs(300))
    }

    fn metadata() -> TrackMetadata {
        TrackMetadata {
            artist: "Boards of Canada".to_string(),
            title: "Roygbiv".to_string(),
            album: Some("Music Has the Right to Children".to_string()),
        }
    }

    #[tokio::test]
    async fn submit_scrobble_via_mock_records_correct_parameters() {
        let mock = MockLastfmApi::new();
        let params = ScrobbleParams {
            artist: "Boards of Canada".to_string(),
            track: "Roygbiv".to_string(),
            album: Some("Music Has the Right to Children".to_string()),
            timestamp: 1_700_000_000,
        };

        mock.submit_scrobble(params.clone()).await.unwrap();

        let submitted = mock.submitted_scrobbles();
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].artist, "Boards of Canada");
        assert_eq!(submitted[0].track, "Roygbiv");
        assert_eq!(
            submitted[0].album.as_deref(),
            Some("Music Has the Right to Children")
        );
        assert_eq!(submitted[0].timestamp, 1_700_000_000);
    }

    #[tokio::test]
    async fn scrobble_submits_resolved_metadata() {
        let mock = MockLastfmApi::new();
        let circuit = breaker();

        scrobble(&mock, MediaId::new(), UserId::new(), metadata(), &circuit)
            .await
            .unwrap();

        let submitted = mock.submitted_scrobbles();
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].artist, "Boards of Canada");
        assert_eq!(submitted[0].track, "Roygbiv");
        assert_eq!(
            submitted[0].album.as_deref(),
            Some("Music Has the Right to Children")
        );
        assert!(submitted[0].timestamp > 0);
    }

    #[tokio::test]
    async fn scrobble_submits_no_album_when_metadata_has_none() {
        let mock = MockLastfmApi::new();
        let circuit = breaker();
        let metadata = TrackMetadata {
            album: None,
            ..metadata()
        };

        scrobble(&mock, MediaId::new(), UserId::new(), metadata, &circuit)
            .await
            .unwrap();

        let submitted = mock.submitted_scrobbles();
        assert_eq!(submitted.len(), 1);
        assert!(submitted[0].album.is_none());
    }
}
