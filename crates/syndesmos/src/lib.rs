//! Syndesmos — external API integration for Harmonia (Plex, Last.fm, Tidal).

pub mod error;
pub mod events;
pub mod lastfm;
pub mod plex;
pub mod retry;
pub(crate) mod test_support;
pub mod tidal;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

pub use error::SyndesmodError;
pub use lastfm::artist::ArtistInfo as ArtistData;
use snafu::{OptionExt, ResultExt};
use sqlx::SqlitePool;
use themelion::{EventSender, MediaId, MediaType, UserId};
use tracing::instrument;

use crate::error::{DatabaseSnafu, ScrobbleMetadataMissingSnafu, WantProfileMissingSnafu};
use crate::lastfm::scrobble::TrackMetadata;
use crate::lastfm::{LastfmApi, LastfmClient};
use crate::plex::{PlexApi, PlexClient};
use crate::retry::CircuitBreaker;
use crate::tidal::wantlist::{NewFavorite, TIDAL_WANT_SOURCE};
use crate::tidal::{TidalApi, TidalClient, TidalId};

// WHY: Tidal favorites are tracks, but the `wants` schema has no track-level
// music type — an album want is the acquisition unit, matching the request
// path's `music_album` convention.
const TIDAL_WANT_MEDIA_TYPE: &str = "music_album";
const TIDAL_WANT_PROFILE_TYPE: &str = "music";

/// Trait implemented by `ScrobbleClient` — one method per external integration.
///
/// When a service is unconfigured, the method returns `Ok(())` or `Ok(None)`.
/// Unconfigured integrations are a valid operational state, not an error.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait is stable since Rust 1.75; Send bound concern deferred"
)]
pub trait ExternalIntegration: Send + Sync {
    async fn notify_plex_import(
        &self,
        media_id: MediaId,
        media_type: MediaType,
    ) -> Result<(), SyndesmodError>;
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
pub struct ScrobbleClient {
    plex_api: Option<Arc<dyn PlexApi>>,
    // WHY: section mapping is stored separately so mock tests can inject
    // a MockPlexApi alongside a custom section map without a real PlexClient.
    plex_sections: HashMap<MediaType, u32>,
    lastfm_api: Option<Arc<dyn LastfmApi>>,
    tidal_api: Option<Arc<dyn TidalApi>>,
    // WHY: read access to the local catalog — scrobbles resolve track
    // metadata and Tidal sync diffs against persisted wants.
    db: SqlitePool,
    event_tx: EventSender,
    plex_circuit: CircuitBreaker,
    lastfm_circuit: CircuitBreaker,
    tidal_circuit: CircuitBreaker,
}

impl ScrobbleClient {
    /// Persists newly synced Tidal favorites as `searching` wants keyed on the
    /// Tidal ID, so the next sync's diff baseline is populated.
    ///
    /// Uses the request path's `music_album` + first-`music`-profile convention;
    /// a favorite becoming a want is the intended acquisition path.
    async fn persist_tidal_wants(&self, favorites: &[NewFavorite]) -> Result<(), SyndesmodError> {
        let profile_id =
            apotheke::repo::quality::list_profiles_for_type(&self.db, TIDAL_WANT_PROFILE_TYPE)
                .await
                .context(DatabaseSnafu)?
                .into_iter()
                .next()
                .context(WantProfileMissingSnafu {
                    media_type: TIDAL_WANT_PROFILE_TYPE,
                })?
                .id;

        let added_at = jiff::Timestamp::now().to_string();
        for nf in favorites {
            let want = apotheke::repo::want::Want {
                id: nf.media_id.as_bytes().to_vec(),
                media_type: TIDAL_WANT_MEDIA_TYPE.to_string(),
                title: format!("{} - {}", nf.favorite.artist, nf.favorite.title),
                registry_id: None,
                quality_profile_id: profile_id,
                status: "searching".to_string(),
                source: Some(TIDAL_WANT_SOURCE.to_string()),
                source_ref: Some(nf.favorite.tidal_id.as_str().to_string()),
                added_at: added_at.clone(),
                fulfilled_at: None,
            };
            apotheke::repo::want::insert_want(&self.db, &want)
                .await
                .context(DatabaseSnafu)?;
        }
        Ok(())
    }
}

impl ExternalIntegration for ScrobbleClient {
    #[instrument(skip(self), fields(media_id = %media_id, media_type = %media_type))]
    async fn notify_plex_import(
        &self,
        media_id: MediaId,
        media_type: MediaType,
    ) -> Result<(), SyndesmodError> {
        let api = match &self.plex_api {
            Some(a) => a.clone(),
            None => return Ok(()),
        };
        plex::notify::notify_library_scan(
            api.as_ref(),
            &self.plex_sections,
            media_type,
            &self.plex_circuit,
        )
        .await
    }

    #[instrument(skip(self), fields(track_id = %track_id, user_id = %user_id))]
    async fn scrobble(&self, track_id: MediaId, user_id: UserId) -> Result<(), SyndesmodError> {
        let api = match &self.lastfm_api {
            Some(a) => a.clone(),
            None => return Ok(()),
        };

        // WHY: Last.fm requires real artist/title names; a track with no
        // resolvable metadata is an error, never a blank scrobble.
        let resolved =
            apotheke::repo::music::get_track_scrobble_metadata(&self.db, track_id.as_bytes())
                .await
                .context(DatabaseSnafu)?
                .context(ScrobbleMetadataMissingSnafu { track_id })?;
        let artist = resolved
            .artist_name
            .context(ScrobbleMetadataMissingSnafu { track_id })?;
        let metadata = TrackMetadata {
            artist,
            title: resolved.track_title,
            album: Some(resolved.album_title),
        };

        lastfm::scrobble::scrobble(
            api.as_ref(),
            track_id,
            user_id,
            metadata,
            &self.lastfm_circuit,
        )
        .await
    }

    #[instrument(skip(self))]
    async fn sync_tidal_want_list(&self) -> Result<Vec<MediaId>, SyndesmodError> {
        let api = match &self.tidal_api {
            Some(a) => a.clone(),
            None => return Ok(vec![]),
        };

        // WHY: diff against persisted wants — an empty baseline would re-add
        // every favorite on every sync.
        let existing_tidal_ids: HashSet<TidalId> =
            apotheke::repo::want::list_want_source_refs(&self.db, TIDAL_WANT_SOURCE)
                .await
                .context(DatabaseSnafu)?
                .into_iter()
                .map(TidalId)
                .collect();

        let new_favorites = tidal::wantlist::sync_want_list(
            api.as_ref(),
            &self.event_tx,
            &existing_tidal_ids,
            &self.tidal_circuit,
        )
        .await?;

        // WHY: persist each fresh favorite as a want so the next sync's
        // baseline is populated — without this, `existing_tidal_ids` stays
        // empty and every favorite is re-added on every sync.
        if !new_favorites.is_empty() {
            self.persist_tidal_wants(&new_favorites).await?;
        }

        Ok(new_favorites.into_iter().map(|nf| nf.media_id).collect())
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

/// Builds a `ScrobbleClient` from real config or injected mocks.
pub struct ScrobbleClientBuilder {
    event_tx: EventSender,
    db: SqlitePool,
    plex_api: Option<Arc<dyn PlexApi>>,
    plex_sections: HashMap<MediaType, u32>,
    lastfm_api: Option<Arc<dyn LastfmApi>>,
    tidal_api: Option<Arc<dyn TidalApi>>,
    circuit_break_minutes: u64,
    circuit_break_failure_threshold: u32,
}

impl ScrobbleClientBuilder {
    #[must_use]
    pub fn new(event_tx: EventSender, db: SqlitePool) -> Self {
        Self {
            event_tx,
            db,
            plex_api: None,
            plex_sections: HashMap::new(),
            lastfm_api: None,
            tidal_api: None,
            circuit_break_minutes: 5,
            circuit_break_failure_threshold: horismos::SyndesmosConfig::default()
                .circuit_break_failure_threshold,
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

    pub fn circuit_break_failure_threshold(mut self, threshold: u32) -> Self {
        self.circuit_break_failure_threshold = threshold;
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

    pub fn build(self) -> ScrobbleClient {
        let cooldown = Duration::from_secs(self.circuit_break_minutes * 60);
        ScrobbleClient {
            plex_api: self.plex_api,
            plex_sections: self.plex_sections,
            lastfm_api: self.lastfm_api,
            tidal_api: self.tidal_api,
            db: self.db,
            event_tx: self.event_tx,
            plex_circuit: CircuitBreaker::new(
                "plex",
                self.circuit_break_failure_threshold,
                cooldown,
            ),
            lastfm_circuit: CircuitBreaker::new(
                "lastfm",
                self.circuit_break_failure_threshold,
                cooldown,
            ),
            tidal_circuit: CircuitBreaker::new(
                "tidal",
                self.circuit_break_failure_threshold,
                cooldown,
            ),
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
    use crate::test_support::{
        seed_scrobble_track, seed_tidal_want, seed_track_without_artist, test_pool,
    };
    use crate::tidal::TidalFavorite;
    use crate::tidal::tests::MockTidalApi;

    async fn build_service(event_tx: EventSender) -> ScrobbleClient {
        ScrobbleClientBuilder::new(event_tx, test_pool().await).build()
    }

    fn make_favorite(id: &str) -> TidalFavorite {
        TidalFavorite {
            tidal_id: crate::tidal::TidalId(id.to_string()),
            title: format!("Track {id}"),
            artist: "Artist A".to_string(),
        }
    }

    // ── Unconfigured degradation ──────────────────────────────────────────────

    #[tokio::test]
    async fn notify_plex_returns_ok_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx).await;
        let result = service
            .notify_plex_import(MediaId::new(), MediaType::Music)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn scrobble_returns_ok_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx).await;
        let result = service.scrobble(MediaId::new(), UserId::new()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn sync_tidal_returns_empty_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx).await;
        let result = service.sync_tidal_want_list().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn get_artist_data_returns_none_when_unconfigured() {
        let (tx, _rx) = create_event_bus(32);
        let service = build_service(tx).await;
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

        let service = ScrobbleClientBuilder::new(tx, test_pool().await)
            .with_mock_plex(mock, sections)
            .build();

        service
            .notify_plex_import(MediaId::new(), MediaType::Music)
            .await
            .unwrap();

        assert_eq!(*sections_ref.lock().unwrap(), vec![7u32]);
    }

    #[tokio::test]
    async fn notify_plex_refreshes_only_the_matching_section() {
        // WHY: #644's actual defect — the old handler refreshed EVERY
        // configured section on every import because PlexNotifyRequired
        // carried no media_type. With media_type threaded through, only
        // the section for the imported item's type may be touched.
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockPlexApi::new());
        let sections_ref = mock.sections_refreshed.clone();

        let mut sections = HashMap::new();
        sections.insert(MediaType::Music, 1u32);
        sections.insert(MediaType::Movie, 2u32);

        let service = ScrobbleClientBuilder::new(tx, test_pool().await)
            .with_mock_plex(mock, sections)
            .build();

        service
            .notify_plex_import(MediaId::new(), MediaType::Movie)
            .await
            .unwrap();

        assert_eq!(
            *sections_ref.lock().unwrap(),
            vec![2u32],
            "only the Movie section may be refreshed — the Music section must stay untouched"
        );
    }

    // ── Last.fm configured ────────────────────────────────────────────────────

    #[tokio::test]
    async fn scrobble_submits_resolved_metadata_when_lastfm_configured() {
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockLastfmApi::new());
        let submitted = mock.scrobbles_submitted.clone();

        let pool = test_pool().await;
        let track_id = seed_scrobble_track(
            &pool,
            "Boards of Canada",
            "Roygbiv",
            "Music Has the Right to Children",
        )
        .await;

        let service = ScrobbleClientBuilder::new(tx, pool)
            .with_mock_lastfm(mock)
            .build();

        service.scrobble(track_id, UserId::new()).await.unwrap();

        let submitted = submitted.lock().unwrap();
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].artist, "Boards of Canada");
        assert_eq!(submitted[0].track, "Roygbiv");
        assert_eq!(
            submitted[0].album.as_deref(),
            Some("Music Has the Right to Children")
        );
    }

    #[tokio::test]
    async fn scrobble_errors_when_track_missing_from_catalog() {
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockLastfmApi::new());
        let submitted = mock.scrobbles_submitted.clone();

        let service = ScrobbleClientBuilder::new(tx, test_pool().await)
            .with_mock_lastfm(mock)
            .build();

        let result = service.scrobble(MediaId::new(), UserId::new()).await;

        assert!(matches!(
            result,
            Err(SyndesmodError::ScrobbleMetadataMissing { .. })
        ));
        assert!(submitted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn scrobble_errors_when_track_has_no_artist() {
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockLastfmApi::new());
        let submitted = mock.scrobbles_submitted.clone();

        let pool = test_pool().await;
        let track_id = seed_track_without_artist(&pool, "Roygbiv", "Album").await;

        let service = ScrobbleClientBuilder::new(tx, pool)
            .with_mock_lastfm(mock)
            .build();

        let result = service.scrobble(track_id, UserId::new()).await;

        assert!(matches!(
            result,
            Err(SyndesmodError::ScrobbleMetadataMissing { .. })
        ));
        assert!(submitted.lock().unwrap().is_empty());
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

        let service = ScrobbleClientBuilder::new(tx, test_pool().await)
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
        let mock = Arc::new(MockTidalApi::new(vec![make_favorite("t1")]));

        let service = ScrobbleClientBuilder::new(tx, test_pool().await)
            .with_mock_tidal(mock)
            .build();

        let added = service.sync_tidal_want_list().await.unwrap();
        assert_eq!(added.len(), 1);
    }

    #[tokio::test]
    async fn sync_tidal_persists_wants_and_second_sync_adds_none() {
        // End-to-end: first sync of {t1,t2} on an empty catalog persists two
        // wants (source='tidal_sync'); a second sync of the same favorites
        // reads that baseline back and adds zero.
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockTidalApi::new(vec![
            make_favorite("t1"),
            make_favorite("t2"),
        ]));

        let pool = test_pool().await;
        let service = ScrobbleClientBuilder::new(tx, pool.clone())
            .with_mock_tidal(mock)
            .build();

        let first = service.sync_tidal_want_list().await.unwrap();
        assert_eq!(first.len(), 2);

        let mut persisted = apotheke::repo::want::list_want_source_refs(&pool, "tidal_sync")
            .await
            .unwrap();
        persisted.sort();
        assert_eq!(persisted, vec!["t1".to_string(), "t2".to_string()]);

        let second = service.sync_tidal_want_list().await.unwrap();
        assert!(second.is_empty());

        // Baseline unchanged — no duplicate wants written.
        let after = apotheke::repo::want::list_want_source_refs(&pool, "tidal_sync")
            .await
            .unwrap();
        assert_eq!(after.len(), 2);
    }

    #[tokio::test]
    async fn sync_tidal_skips_favorites_already_persisted_as_wants() {
        let (tx, _rx) = create_event_bus(32);
        let mock = Arc::new(MockTidalApi::new(vec![
            make_favorite("t1"),
            make_favorite("t2"),
        ]));

        let pool = test_pool().await;
        seed_tidal_want(&pool, "t1").await;

        let service = ScrobbleClientBuilder::new(tx, pool)
            .with_mock_tidal(mock)
            .build();

        let added = service.sync_tidal_want_list().await.unwrap();
        assert_eq!(added.len(), 1);
    }

    #[tokio::test]
    async fn sync_tidal_returns_empty_and_no_event_when_all_favorites_known() {
        let (tx, mut rx) = create_event_bus(32);
        let mock = Arc::new(MockTidalApi::new(vec![
            make_favorite("t1"),
            make_favorite("t2"),
        ]));

        let pool = test_pool().await;
        seed_tidal_want(&pool, "t1").await;
        seed_tidal_want(&pool, "t2").await;

        let service = ScrobbleClientBuilder::new(tx, pool)
            .with_mock_tidal(mock)
            .build();

        let added = service.sync_tidal_want_list().await.unwrap();
        assert!(added.is_empty());
        assert!(rx.try_recv().is_err());
    }
}
