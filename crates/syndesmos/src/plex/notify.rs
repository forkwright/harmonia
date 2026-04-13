//! Plex library scan notification — triggers a section refresh on media import.

use themelion::MediaType;
use tracing::instrument;

use crate::error::SyndesmodError;
use crate::plex::{PlexApi, PlexClient};
use crate::retry::{CircuitBreaker, with_retry};

/// Triggers a Plex library scan for the section that corresponds to `media_type`.
///
/// Returns `Ok(())` silently when `media_type` has no configured section ID.
// WHY: dead in lib builds (PlexNotifyRequired doesn't carry MediaType yet) but used from tests.
// cfg_attr restricts the expect to non-test builds where the lint fires.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "primary entry point once PlexNotifyRequired carries MediaType"
    )
)]
#[instrument(skip(client, api, circuit))]
pub(crate) async fn notify_library_scan(
    client: &PlexClient,
    api: &dyn PlexApi,
    media_type: MediaType,
    circuit: &CircuitBreaker,
) -> Result<(), SyndesmodError> {
    let section_id = match client.section_id_for(media_type) {
        Some(id) => id,
        None => {
            tracing::debug!(
                media_type = %media_type,
                "no Plex section configured for media type; skipping notify"
            );
            return Ok(());
        }
    };

    notify_library_scan_by_section(api, section_id, circuit).await
}

/// Triggers a Plex library scan for a known section ID directly.
#[instrument(skip(api, circuit), fields(section_id))]
pub(crate) async fn notify_library_scan_by_section(
    api: &dyn PlexApi,
    section_id: u32,
    circuit: &CircuitBreaker,
) -> Result<(), SyndesmodError> {
    with_retry(|| api.refresh_library_section(section_id), circuit).await
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use themelion::MediaType;
    use horismos::PlexConfig;

    use super::*;
    use crate::plex::tests::MockPlexApi;
    use crate::retry::CircuitBreaker;

    fn make_client(sections: HashMap<MediaType, u32>) -> PlexClient {
        PlexClient::new(PlexConfig {
            url: "http://plex.test:32400".to_string(),
            token: "test-token".to_string(),
            library_sections: sections,
        })
    }

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new("plex", 5, std::time::Duration::from_secs(300))
    }

    #[tokio::test]
    async fn notifies_correct_section_id_for_media_type() {
        let mut sections = HashMap::new();
        sections.insert(MediaType::Music, 1u32);
        sections.insert(MediaType::Movie, 2u32);

        let client = make_client(sections);
        let mock = MockPlexApi::new();
        let circuit = breaker();

        notify_library_scan(&client, &mock, MediaType::Music, &circuit)
            .await
            .unwrap();

        assert_eq!(mock.refreshed_sections(), vec![1u32]);
    }

    #[tokio::test]
    async fn returns_ok_when_media_type_has_no_configured_section() {
        let client = make_client(HashMap::new());
        let mock = MockPlexApi::new();
        let circuit = breaker();

        let result = notify_library_scan(&client, &mock, MediaType::Music, &circuit).await;
        assert!(result.is_ok());
        assert!(mock.refreshed_sections().is_empty());
    }

    #[tokio::test]
    async fn notifies_movie_section_with_correct_id() {
        let mut sections = HashMap::new();
        sections.insert(MediaType::Movie, 42u32);

        let client = make_client(sections);
        let mock = MockPlexApi::new();
        let circuit = breaker();

        notify_library_scan(&client, &mock, MediaType::Movie, &circuit)
            .await
            .unwrap();

        assert_eq!(mock.refreshed_sections(), vec![42u32]);
    }

    #[tokio::test]
    async fn notify_by_section_calls_api_with_section_id() {
        let mock = MockPlexApi::new();
        let circuit = breaker();

        notify_library_scan_by_section(&mock, 99u32, &circuit)
            .await
            .unwrap();

        assert_eq!(mock.refreshed_sections(), vec![99u32]);
    }
}
