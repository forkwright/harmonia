//! Prostheke — subtitle management for Harmonia.
//!
//! Replaces Bazarr. Acquires subtitle files for video media, stores them
//! alongside library files, and emits `SubtitleAcquired` events.

pub mod download;
pub mod error;
pub mod events;
pub mod language;
pub mod providers;
pub mod rate_limit;
pub mod repo;
pub mod search;
pub mod timing;
pub mod types;

use std::path::Path;
use std::sync::Arc;

pub use error::ProsthekeError;
use horismos::{ProsthekeConfig, Section};
use themelion::{EventSender, HarmoniaEvent, MediaId, MediaType};
use tracing::instrument;
pub use types::{
    LanguagePreference, SubtitleFormat, SubtitleMatch, SubtitleProviderId, SubtitleTrack,
};
use uuid::Uuid;

use crate::download::{detect_format_from_name, subtitle_path, write_subtitle_file};
use crate::providers::{Provider, SubtitleProvider};
use crate::search::search_all_providers;

/// The primary trait surface for subtitle acquisition.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since Rust 1.75; dyn dispatch not required here"
)]
pub trait SubtitleService: Send + Sync {
    /// Search all configured providers, download best matches, store files,
    /// and emit a `SubtitleAcquired` event on success.
    async fn acquire_subtitles(
        &self,
        media_id: MediaId,
        media_type: MediaType,
        path: &Path,
    ) -> Result<(), ProsthekeError>;

    /// Return all subtitle tracks stored for a media item.
    async fn list_for_media(&self, media_id: MediaId)
    -> Result<Vec<SubtitleTrack>, ProsthekeError>;
}

/// Live implementation backed by SQLite and configured providers.
///
/// Generic over `P` so that tests can inject a `MockProvider` without
/// needing `dyn SubtitleProvider` (which is not object-safe due to async fn).
/// Production code uses the default `P = Provider` enum.
pub struct SubtitleManager<P: SubtitleProvider = Provider> {
    read: sqlx::SqlitePool,
    write: sqlx::SqlitePool,
    // WHY: a live `Section` (not a frozen `ProsthekeConfig`) — #529 step 8
    // makes the per-op language/score preferences live.
    config: Section<ProsthekeConfig>,
    // WHY: swappable behind a std RwLock (never held across an .await — the
    // one read site takes the lock, clones the Arc, and drops the guard
    // before any async work) so a `prostheke.opensubtitles` subtree change
    // can swap the live provider set without a service rebuild. Arc-wrapped
    // (not `RwLock<Vec<P>>`) because provider variants hold non-Clone
    // internal rate-limiter state (`OpenSubtitlesProvider`'s `RateLimiter`
    // wraps a `tokio::sync::Mutex`).
    providers: std::sync::RwLock<Arc<Vec<P>>>,
    event_tx: EventSender,
}

impl<P: SubtitleProvider> SubtitleManager<P> {
    pub fn new(
        read: sqlx::SqlitePool,
        write: sqlx::SqlitePool,
        config: Section<ProsthekeConfig>,
        providers: Vec<P>,
        event_tx: EventSender,
    ) -> Self {
        Self {
            read,
            write,
            config,
            providers: std::sync::RwLock::new(Arc::new(providers)),
            event_tx,
        }
    }

    /// Swaps the live provider set. Called by archon's prostheke supervisor
    /// on a `prostheke.opensubtitles` subtree change (INCLUDING None↔Some
    /// presence). The OpenSubtitles rate limiter resets with the provider —
    /// acceptable (it's a politeness limiter, not a 429 embargo).
    pub fn set_providers(&self, providers: Vec<P>) {
        let mut guard = self.providers.write().unwrap_or_else(|e| e.into_inner());
        *guard = Arc::new(providers);
    }

    fn providers_snapshot(&self) -> Arc<Vec<P>> {
        let guard = self.providers.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard)
    }
}

impl<P: SubtitleProvider> SubtitleService for SubtitleManager<P> {
    #[instrument(skip(self), fields(media_id = %media_id, media_type = ?media_type))]
    async fn acquire_subtitles(
        &self,
        media_id: MediaId,
        media_type: MediaType,
        path: &Path,
    ) -> Result<(), ProsthekeError> {
        // WHY: one snapshot for the whole operation — a mid-acquire reload
        // cannot mix an old language preference with a new `min_match_score`
        // FROM after it.
        let config = self.config.get();
        let preferences = LanguagePreference {
            languages: config.languages.clone(),
            include_hearing_impaired: config.include_hearing_impaired,
            include_forced: config.include_forced,
        };

        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // WHY: snapshot-clone-drop before the awaits below — never hold the
        // providers RwLock guard across an .await.
        let providers = self.providers_snapshot();

        let matches = search_all_providers(
            providers.as_slice(),
            &media_id,
            media_type,
            title,
            None,
            None,
            None,
            &preferences,
            None,
            config.min_match_score,
        )
        .await?;

        if matches.is_empty() {
            return Ok(());
        }

        let mut acquired_languages: Vec<String> = Vec::new();

        for subtitle_match in &matches {
            let provider = providers
                .iter()
                .find(|p| p.name() == subtitle_match.provider)
                .ok_or_else(|| ProsthekeError::AcquisitionFailed {
                    detail: format!("provider '{}' not found", subtitle_match.provider),
                    location: snafu::location!(),
                })?;

            let content = provider.download(subtitle_match).await?;

            let format = detect_format_from_name(&subtitle_match.download_url)
                .unwrap_or(SubtitleFormat::Srt);

            let dest = subtitle_path(path, &subtitle_match.language, format);
            write_subtitle_file(&dest, &content).await?;

            let track = SubtitleTrack {
                id: Uuid::now_v7(),
                media_id,
                language: subtitle_match.language.clone(),
                format,
                file_path: dest.clone(),
                provider: subtitle_match.provider.clone(),
                provider_id: subtitle_match.provider_id.clone(),
                hearing_impaired: subtitle_match.hearing_impaired,
                forced: subtitle_match.forced,
                score: subtitle_match.score,
                acquired_at: jiff::Timestamp::now(),
            };

            // Insert ignoring conflicts — idempotent re-runs are safe.
            match repo::insert_subtitle(&self.write, &track).await {
                Ok(()) => acquired_languages.push(subtitle_match.language.clone()),
                Err(e) => {
                    tracing::warn!(
                        language = %subtitle_match.language,
                        error = %e,
                        "subtitle already stored, skipping"
                    );
                }
            }
        }

        if !acquired_languages.is_empty() {
            let _ = self.event_tx.send(HarmoniaEvent::SubtitleAcquired {
                media_id,
                languages: acquired_languages,
            });
        }

        Ok(())
    }

    #[instrument(skip(self), fields(media_id = %media_id))]
    async fn list_for_media(
        &self,
        media_id: MediaId,
    ) -> Result<Vec<SubtitleTrack>, ProsthekeError> {
        repo::get_subtitles_for_media(&self.read, &media_id).await
    }
}

#[cfg(test)]
mod tests {
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;
    use themelion::{MediaId, MediaType, create_event_bus};

    use super::*;
    use crate::providers::SubtitleProvider;

    // ── Mock provider ─────────────────────────────────────────────────────────

    struct MockProvider {
        name: String,
        results: Vec<SubtitleMatch>,
        content: Vec<u8>,
    }

    impl MockProvider {
        fn new(name: &str, results: Vec<SubtitleMatch>, content: Vec<u8>) -> Self {
            Self {
                name: name.to_string(),
                results,
                content,
            }
        }
    }

    impl SubtitleProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn search(
            &self,
            _media_id: &MediaId,
            _media_type: MediaType,
            _title: &str,
            _year: Option<u16>,
            _season: Option<u32>,
            _episode: Option<u32>,
            _languages: &[String],
            _file_hash: Option<&str>,
        ) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
            Ok(self.results.clone())
        }

        async fn download(&self, _subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
            Ok(self.content.clone())
        }
    }

    fn make_match(provider: &str, lang: &str, score: f64) -> SubtitleMatch {
        SubtitleMatch {
            provider: provider.to_string(),
            provider_id: SubtitleProviderId("42".to_string()),
            language: lang.to_string(),
            hearing_impaired: false,
            forced: false,
            score,
            download_url: format!("https://example.com/sub.{lang}.srt"),
        }
    }

    async fn make_service(
        providers: Vec<MockProvider>,
        config: ProsthekeConfig,
    ) -> (
        SubtitleManager<MockProvider>,
        SqlitePool,
        themelion::EventReceiver,
    ) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let (tx, rx) = create_event_bus(64);
        let svc = SubtitleManager::new(
            pool.clone(),
            pool.clone(),
            Section::fixed(config),
            providers,
            tx,
        );
        (svc, pool, rx)
    }

    #[tokio::test]
    async fn subtitle_acquired_event_emitted_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let media_path = dir.path().join("movie.mkv");
        std::fs::write(&media_path, b"").unwrap();

        let provider = MockProvider::new(
            "mock",
            vec![make_match("mock", "en", 0.9)],
            b"1\n00:00:01,000 --> 00:00:02,000\nHello\n".to_vec(),
        );

        let config = ProsthekeConfig {
            languages: vec!["en".to_string()],
            include_hearing_impaired: false,
            include_forced: true,
            min_match_score: 0.7,
            opensubtitles: None,
        };

        let (svc, _pool, mut rx) = make_service(vec![provider], config).await;

        svc.acquire_subtitles(MediaId::new(), MediaType::Movie, &media_path)
            .await
            .unwrap();

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, HarmoniaEvent::SubtitleAcquired { .. }));
    }

    #[tokio::test]
    async fn no_event_when_no_matches_found() {
        let dir = tempfile::tempdir().unwrap();
        let media_path = dir.path().join("movie.mkv");

        let provider = MockProvider::new("mock", vec![], vec![]);

        let config = ProsthekeConfig::default();
        let (svc, _pool, mut rx) = make_service(vec![provider], config).await;

        svc.acquire_subtitles(MediaId::new(), MediaType::Movie, &media_path)
            .await
            .unwrap();

        assert!(rx.try_recv().is_err());
    }

    // ── Live config (#529 step 8) ────────────────────────────────────────────

    struct RecordingProvider {
        name: String,
        seen_languages: Arc<std::sync::Mutex<Vec<Vec<String>>>>,
    }

    impl SubtitleProvider for RecordingProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn search(
            &self,
            _media_id: &MediaId,
            _media_type: MediaType,
            _title: &str,
            _year: Option<u16>,
            _season: Option<u32>,
            _episode: Option<u32>,
            languages: &[String],
            _file_hash: Option<&str>,
        ) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
            self.seen_languages.lock().unwrap().push(languages.to_vec());
            Ok(Vec::new())
        }

        async fn download(&self, _subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
            Ok(Vec::new())
        }
    }

    // A `prostheke.languages` change made through a REAL
    // `ConfigManager::replace` must be visible on the NEXT `acquire_subtitles`
    // call — no service rebuild.
    #[tokio::test]
    async fn language_preference_change_is_visible_on_next_acquire() {
        let dir = tempfile::tempdir().unwrap();
        let media_path = dir.path().join("movie.mkv");
        std::fs::write(&media_path, b"").unwrap();

        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let (tx, _rx) = create_event_bus(64);

        let mut boot = horismos::Config::default();
        boot.exousia.jwt_secret = "test-secret-that-is-long-enough-for-hs256".to_string();
        boot.prostheke.languages = vec!["en".to_string()];
        let (manager, handle) = horismos::ConfigManager::new(
            boot.clone(),
            std::path::PathBuf::from("unused.toml"),
            horismos::ConfigOverrides::default(),
        );

        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let provider = RecordingProvider {
            name: "mock".to_string(),
            seen_languages: Arc::clone(&seen),
        };
        let svc = SubtitleManager::new(
            pool.clone(),
            pool.clone(),
            handle.section(|c| &c.prostheke),
            vec![provider],
            tx,
        );

        svc.acquire_subtitles(MediaId::new(), MediaType::Movie, &media_path)
            .await
            .unwrap();

        let mut raised = boot.clone();
        raised.prostheke.languages = vec!["fr".to_string()];
        manager
            .replace(raised)
            .expect("replace applies the language change");

        svc.acquire_subtitles(MediaId::new(), MediaType::Movie, &media_path)
            .await
            .unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2, "both acquire calls must have searched");
        assert!(
            seen[0].iter().any(|l| l == "en"),
            "first acquire must search the boot-time language, got {:?}",
            seen[0]
        );
        assert!(
            seen[1].iter().any(|l| l == "fr"),
            "second acquire must reflect the live language change, got {:?}",
            seen[1]
        );
    }

    // A `prostheke.opensubtitles` None↔Some swap (simulating what archon's
    // prostheke supervisor does on that subtree changing) must change the
    // live provider set without a service rebuild.
    #[tokio::test]
    async fn set_providers_swaps_the_live_provider_set() {
        let dir = tempfile::tempdir().unwrap();
        let media_path = dir.path().join("movie.mkv");
        std::fs::write(&media_path, b"").unwrap();

        let (svc, _pool, mut rx) = make_service(vec![], ProsthekeConfig::default()).await;

        svc.acquire_subtitles(MediaId::new(), MediaType::Movie, &media_path)
            .await
            .unwrap();
        assert!(
            rx.try_recv().is_err(),
            "an empty provider set must find nothing"
        );

        let provider = MockProvider::new(
            "mock",
            vec![make_match("mock", "en", 0.9)],
            b"1\n00:00:01,000 --> 00:00:02,000\nHello\n".to_vec(),
        );
        svc.set_providers(vec![provider]);

        svc.acquire_subtitles(MediaId::new(), MediaType::Movie, &media_path)
            .await
            .unwrap();

        let event = rx.try_recv().unwrap();
        assert!(
            matches!(event, HarmoniaEvent::SubtitleAcquired { .. }),
            "the swapped-in provider must be used by the NEXT acquire call"
        );
    }
}
