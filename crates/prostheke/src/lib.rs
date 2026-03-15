//! Prostheke — subtitle management for Harmonia.
//!
//! Replaces Bazarr. Acquires subtitle files for video media, stores them
//! alongside library files, and emits `SubtitleAcquired` events.

pub mod download;
pub mod error;
pub mod events;
pub mod language;
pub mod providers;
pub mod repo;
pub mod search;
pub mod timing;
pub mod types;

pub use error::ProsthekeError;
pub use types::{LanguagePreference, SubtitleFormat, SubtitleMatch, SubtitleTrack};

use std::path::Path;

use harmonia_common::{EventSender, HarmoniaEvent, MediaId, MediaType};
use horismos::ProsthekeConfig;
use tracing::instrument;
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
pub struct ProsthekeService<P: SubtitleProvider = Provider> {
    read: sqlx::SqlitePool,
    write: sqlx::SqlitePool,
    config: ProsthekeConfig,
    providers: Vec<P>,
    event_tx: EventSender,
}

impl<P: SubtitleProvider> ProsthekeService<P> {
    pub fn new(
        read: sqlx::SqlitePool,
        write: sqlx::SqlitePool,
        config: ProsthekeConfig,
        providers: Vec<P>,
        event_tx: EventSender,
    ) -> Self {
        Self {
            read,
            write,
            config,
            providers,
            event_tx,
        }
    }
}

impl<P: SubtitleProvider> SubtitleService for ProsthekeService<P> {
    #[instrument(skip(self), fields(media_id = %media_id, media_type = ?media_type))]
    async fn acquire_subtitles(
        &self,
        media_id: MediaId,
        media_type: MediaType,
        path: &Path,
    ) -> Result<(), ProsthekeError> {
        let preferences = LanguagePreference {
            languages: self.config.languages.clone(),
            include_hearing_impaired: self.config.include_hearing_impaired,
            include_forced: self.config.include_forced,
        };

        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let matches = search_all_providers(
            &self.providers,
            &media_id,
            media_type,
            title,
            None,
            None,
            None,
            &preferences,
            None,
            self.config.min_match_score,
        )
        .await?;

        if matches.is_empty() {
            return Ok(());
        }

        let mut acquired_languages: Vec<String> = Vec::new();

        for subtitle_match in &matches {
            let provider = self
                .providers
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
    use harmonia_common::{MediaId, MediaType, create_event_bus};
    use harmonia_db::migrate::MIGRATOR;
    use sqlx::SqlitePool;

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
            provider_id: "42".to_string(),
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
        ProsthekeService<MockProvider>,
        SqlitePool,
        harmonia_common::EventReceiver,
    ) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let (tx, rx) = create_event_bus(64);
        let svc = ProsthekeService::new(pool.clone(), pool.clone(), config, providers, tx);
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
}
