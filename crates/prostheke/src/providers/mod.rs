//! SubtitleProvider trait and concrete provider implementations.

pub mod addic7ed;
pub mod opensubtitles;

use harmonia_common::{MediaId, MediaType};
use horismos::OpenSubtitlesConfig;

use self::addic7ed::Addic7edProvider;
use self::opensubtitles::OpenSubtitlesProvider;
use crate::error::ProsthekeError;
use crate::types::SubtitleMatch;

/// A subtitle source capable of searching for and downloading subtitle files.
///
/// Async fn in trait is stable since Rust 1.75. This trait is used via
/// generics, not `dyn`, because async methods are not dyn-compatible.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since Rust 1.75; used generically, not via dyn"
)]
pub trait SubtitleProvider: Send + Sync {
    /// Short stable identifier, e.g. "opensubtitles" or "addic7ed".
    fn name(&self) -> &str;

    /// Search for subtitle candidates.
    ///
    /// Returns an empty `Vec` when the provider is not configured or finds no
    /// results — never errors on missing configuration.
    #[allow(clippy::too_many_arguments)]
    async fn search(
        &self,
        media_id: &MediaId,
        media_type: MediaType,
        title: &str,
        year: Option<u16>,
        season: Option<u32>,
        episode: Option<u32>,
        languages: &[String],
        file_hash: Option<&str>,
    ) -> Result<Vec<SubtitleMatch>, ProsthekeError>;

    /// Download the subtitle file bytes for a given match.
    async fn download(&self, subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError>;
}

/// Compile-time dispatch over all supported providers.
///
/// Add a new variant when introducing an additional subtitle source. Static
/// dispatch avoids vtable overhead and dyn-compatibility concerns with async fn.
pub enum Provider {
    OpenSubtitles(OpenSubtitlesProvider),
    Addic7ed(Addic7edProvider),
}

impl Provider {
    /// Build the default production provider list from configuration.
    pub fn default_providers(opensubtitles: Option<OpenSubtitlesConfig>) -> Vec<Provider> {
        vec![
            Provider::OpenSubtitles(OpenSubtitlesProvider::new(opensubtitles)),
            Provider::Addic7ed(Addic7edProvider::new()),
        ]
    }
}

impl SubtitleProvider for Provider {
    fn name(&self) -> &str {
        match self {
            Self::OpenSubtitles(p) => p.name(),
            Self::Addic7ed(p) => p.name(),
        }
    }

    async fn search(
        &self,
        media_id: &MediaId,
        media_type: MediaType,
        title: &str,
        year: Option<u16>,
        season: Option<u32>,
        episode: Option<u32>,
        languages: &[String],
        file_hash: Option<&str>,
    ) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
        match self {
            Self::OpenSubtitles(p) => {
                p.search(
                    media_id, media_type, title, year, season, episode, languages, file_hash,
                )
                .await
            }
            Self::Addic7ed(p) => {
                p.search(
                    media_id, media_type, title, year, season, episode, languages, file_hash,
                )
                .await
            }
        }
    }

    async fn download(&self, subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
        match self {
            Self::OpenSubtitles(p) => p.download(subtitle).await,
            Self::Addic7ed(p) => p.download(subtitle).await,
        }
    }
}
