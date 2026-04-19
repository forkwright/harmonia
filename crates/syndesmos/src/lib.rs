//! Syndesmos — external API integration for Harmonia (Plex, Last.fm, Tidal).

pub mod error;
pub mod events;
pub mod lastfm;
pub mod plex;
pub mod retry;
pub mod tidal;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub use error::SyndesmodError;
pub use lastfm::artist::ArtistInfo as ArtistData;
use themelion::{EventSender, MediaId, MediaType, UserId};
use tracing::instrument;

use crate::lastfm::{LastfmApi, LastfmClient};
use crate::plex::{PlexApi, PlexClient};
use crate::retry::CircuitBreaker;
use crate::tidal::{TidalApi, TidalClient};

/// Trait implemented by `SyndesmosService` — one method per external integration.
///
/// When a service is unconfigured, the method returns `Ok(())` or `Ok(None)`.
/// Unconfigured integrations are a valid operational state, not an error.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait is stable since Rust 1.75; Send bound concern deferred"
)]
pub trait ExternalIntegration: Send + Sync {
    async fn notify_plex_import(&self, media_id: MediaId) -> Result<(), SyndesmodError>;
    async fn scrobble(&self, track_id: MediaId, user_id: UserId) -> Result<(), SyndesmodError>;
    async fn sync_tidal_want_list(&self) -> Result<Vec<MediaId>, SyndesmodError>;
    async fn get_artist_data(
        &self,
        artist_name: &str,
    ) -> Result<Option<ArtistData>, SyndesmodError>;
}

/// Live implementation of all external integrations.
///
/// Each integration is optional; missing config means the corresponding
/// method degrades gracefully rather than returning an error.
pub struct SyndesmosService {
    plex_api: Option<Arc<dyn PlexApi>>,
    // WHY: section mapping is stored separately so mock tests can inject
    // a MockPlexApi alongside a custom section map without a real PlexClient.
    plex_sections: HashMap<MediaType, u32>,
    lastfm_api: Option<Arc<dyn LastfmApi>>,
    tidal_api: Option<Arc<dyn TidalApi>>,
    event_tx: EventSender,
    plex_circuit: CircuitBreaker,
    lastfm_circuit: CircuitBreaker,
    tidal_circuit: CircuitBreaker,
}

impl SyndesmosService {
    /// Refreshes all configured Plex library sections.
    ///
    /// WHY: `PlexNotifyRequired` carries only a `MediaId`, not a `MediaType`.
    /// Without a DB lookup, the service cannot determine the exact section.
    /// Refreshing all configured sections is correct for v1; a future version
    /// can narrow this once the event includes the media type.
    async fn refresh_all_plex_sections(&self) -> Result<(), SyndesmodError> {
        let api = match &self.plex_api {
            Some(a) => a.clone(),
            None => return Ok(()),
        };

        for &section_id in self.plex_sections.values() {
            plex::notify::notify_library_scan_by_section(
                api.as_ref(),
                section_id,
                &self.plex_circuit,
            )
            .await?;
        }
        Ok(())
    }
}

impl ExternalIntegration for SyndesmosService {
    #[instrument(skip(self), fields(media_id = %media_id))]
    async fn notify_plex_import(&self, media_id: MediaId) -> Result<(), SyndesmodError> {
        if self.plex_api.is_none() {
            return Ok(());
        }
        self.refresh_all_plex_sections().await
    }

    #[instrument(skip(self), fields(track_id = %track_id, user_id = %user_id))]
    async fn scrobble(&self, track_id: MediaId, user_id: UserId) -> Result<(), SyndesmodError> {
        let api = match &self.lastfm_api {
            Some(a) => a.clone(),
            None => return Ok(()),
        };
        lastfm::scrobble::scrobble(api.as_ref(), track_id, user_id, &self.lastfm_circuit).await
    }

    #[instrument(skip(self))]
    async fn sync_tidal_want_list(&self) -> Result<Vec<MediaId>, SyndesmodError> {
        let api = match &self.tidal_api {
            Some(a) => a.clone(),
            None => return Ok(vec![]),
        };

        use std::collections::HashSet;
        tidal::wantlist::sync_want_list(
            api.as_ref(),
            &self.event_tx,
            &HashSet::new(),
            &self.tidal_circuit,
        )
        .await
    }

    #[instrument(skip(self), fields(artist = %artist_name))]
    async fn get_artist_data(
        &self,
        artist_name: &str,
    ) -> Result<Option<ArtistData>, SyndesmodError> {
        let api = match &self.lastfm_api {
            Some(a) => a.clone(),
            None => return Ok(None),
        };

        lastfm::artist::fetch_artist_data(api.as_ref(), artist_name, &self.lastfm_circuit).await
    }
}

/// Builds a `SyndesmosService` from real config or injected mocks.
pub struct SyndesmosServiceBuilder {
    event_tx: EventSender,
    plex_api: Option<Arc<dyn PlexApi>>,
    plex_sections: HashMap<MediaType, u32>,
    lastfm_api: Option<Arc<dyn LastfmApi>>,
    tidal_api: Option<Arc<dyn TidalApi>>,
    circuit_break_minutes: u64,
}

impl SyndesmosServiceBuilder {
    pub fn new(event_tx: EventSender) -> Self {
        Self {
            event_tx,
            plex_api: None,
            plex_sections: HashMap::new(),
            lastfm_api: None,
            tidal_api: None,
            circuit_break_minutes: 5,
        }
    }

    pub fn with_plex(mut self, client: PlexClient) -> Self {
        self.plex_sections = client.config.library_sections.clone();
        self.plex_api = Some(Arc::new(client));
        self
    }

    pub fn with_lastfm(mut self, client: LastfmClient) -> Self {
        self.lastfm_api = Some(Arc::new(client));
        self
    }

    pub fn with_tidal(mut self, client: TidalClient) -> Self {
        self.tidal_api = Some(Arc::new(client));
        self
    }

    pub fn circuit_break_minutes(mut self, minutes: u64) -> Self {
        self.circuit_break_minutes = minutes;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_mock_plex(
        mut self,
        mock: Arc<dyn PlexApi>,
        sections: HashMap<MediaType, u32>,
    ) -> Self {
        self.plex_api = Some(mock);
        self.plex_sections = sections;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_mock_lastfm(mut self, mock: Arc<dyn LastfmApi>) -> Self {
        self.lastfm_api = Some(mock);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_mock_tidal(mut self, mock: Arc<dyn TidalApi>) -> Self {
        self.tidal_api = Some(mock);
        self
    }

    pub fn build(self) -> SyndesmosService {
        let cooldown = Duration::from_secs(self.circuit_break_minutes * 60);
        SyndesmosService {
            plex_api: self.plex_api,
            plex_sections: self.plex_sections,
            lastfm_api: self.lastfm_api,
            tidal_api: self.tidal_api,
            event_tx: self.event_tx,
            plex_circuit: CircuitBreaker::new("plex", 5, cooldown),
            lastfm_circuit: CircuitBreaker::new("lastfm", 5, cooldown),
            tidal_circuit: CircuitBreaker::new("tidal", 5, cooldown),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use themelion::{MediaId, MediaType, UserId, create_event_bus};

    use super::*;
    use crate::lastfm::artist::ArtistInfo;
    use crate::lastfm::tests::MockLastfmApi;
    use crate::plex::tests::MockPlexApi;
    use crate::tidal::TidalFavorite;
    use crate::tidal::tests::MockTidalApi;

    fn build_service(event_tx: EventSender) -> SyndesmosService {
        SyndesmosServiceBuilder::new(event_tx).build()
    }

    // ── Unconfigured degradation ──────────────────────────────────────────────

    #[tokio::test]
    async fn notify_plex_returns_ok_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx);
        let result = service.notify_plex_import(MediaId::new()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn scrobble_returns_ok_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx);
        let result = service.scrobble(MediaId::new(), UserId::new()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn sync_tidal_returns_empty_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx);
        let result = service.sync_tidal_want_list().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn get_artist_data_returns_none_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx);
        let result = service.get_artist_data("Aphex Twin").await.unwrap();
        assert!(result.is_none());
    }

    // ── Plex configured ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn notify_plex_calls_refresh_when_configured() {
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockPlexApi::new());
        let sections_ref = mock.sections_refreshed.clone();

        let mut sections = HashMap::new();
        sections.insert(MediaType::Music, 7u32);

        let service = SyndesmosServiceBuilder::new(tx)
            .with_mock_plex(mock, sections)
            .build();

        service.notify_plex_import(MediaId::new()).await.unwrap();

        assert_eq!(*sections_ref.lock().unwrap(), vec![7u32]);
    }

    // ── Last.fm configured ────────────────────────────────────────────────────

    #[tokio::test]
    async fn scrobble_submits_when_lastfm_configured() {
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockLastfmApi::new());
        let submitted = mock.scrobbles_submitted.clone();

        let service = SyndesmosServiceBuilder::new(tx)
            .with_mock_lastfm(mock)
            .build();

        service
            .scrobble(MediaId::new(), UserId::new())
            .await
            .unwrap();

        assert_eq!(submitted.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_artist_data_returns_info_when_lastfm_configured() {
        let (tx, _rx) = create_event_bus(32);
        let expected = ArtistInfo {
            name: "Autechre".to_string(),
            bio: Some("Electronic duo FROM Rochdale.".to_string()),
            similar_artists: vec!["Boards of Canada".to_string()],
            tags: vec!["IDM".to_string()],
        };
        let mock = Arc::new(MockLastfmApi::with_artist_info(expected));

        let service = SyndesmosServiceBuilder::new(tx)
            .with_mock_lastfm(mock)
            .build();

        let result = service.get_artist_data("Autechre").await.unwrap();
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.name, "Autechre");
        assert_eq!(data.tags, vec!["IDM"]);
    }

    // ── Tidal configured ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn sync_tidal_returns_new_items_when_configured() {
        let (tx, _rx) = create_event_bus(32);
        let favorites = vec![TidalFavorite {
            tidal_id: "t1".to_string(),
            title: "Track One".to_string(),
            artist: "Artist A".to_string(),
        }];
        let mock = Arc::new(MockTidalApi::new(favorites));

        let service = SyndesmosServiceBuilder::new(tx)
            .with_mock_tidal(mock)
            .build();

        let added = service.sync_tidal_want_list().await.unwrap();
        assert_eq!(added.len(), 1);
    }
}
