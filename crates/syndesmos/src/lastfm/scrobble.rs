//! Last.fm track scrobble submission.

use themelion::{MediaId, UserId};
use jiff::Timestamp;
use tracing::instrument;

use crate::error::SyndesmodError;
use crate::lastfm::{LastfmApi, ScrobbleParams};
use crate::retry::{CircuitBreaker, with_retry};

/// Submits a scrobble to Last.fm for the given track and user.
///
/// Returns `Ok(())` immediately when Last.fm is unconfigured.
#[instrument(skip(api, circuit), fields(track_id = %track_id, user_id = %user_id))]
pub(crate) async fn scrobble(
    api: &dyn LastfmApi,
    track_id: MediaId,
    user_id: UserId,
    circuit: &CircuitBreaker,
) -> Result<(), SyndesmodError> {
    let params = ScrobbleParams {
        artist: String::new(),
        track: track_id.to_string(),
        album: None,
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
    async fn scrobble_function_calls_api_with_track_id() {
        let mock = MockLastfmApi::new();
        let circuit = breaker();
        let track_id = MediaId::new();
        let user_id = UserId::new();

        scrobble(&mock, track_id, user_id, &circuit).await.unwrap();

        let submitted = mock.submitted_scrobbles();
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].track, track_id.to_string());
        assert!(submitted[0].timestamp > 0);
    }
}
