pub mod alias;
pub mod error;
pub mod event;
pub mod import;
pub mod sanitize;
pub mod scanner;
pub mod sidecar;
pub mod template;

use std::path::Path;

use themelion::{MediaId, MediaType};

use crate::error::TaxisError;
use crate::import::{CompletedDownload, ImportResult, PendingImport};

pub use alias::{
    AliasError, create_artist_alias, list_artist_aliases, remove_artist_alias, resolve_artist,
};
pub use error::{EpignosisError, TaxisError as Error};
pub use event::{Debouncer, WatchEvent, WatchEventKind};
pub use import::{
    ImportOperation, ImportOrigin, ImportPipeline, ImportResult as ImportResultPub, ImportSource,
    MetadataResolver, PendingImport as PendingImportPub, ResolvedMetadata,
};
pub use sanitize::{sanitize_component, sanitize_path};
pub use scanner::ScannerManager;
pub use sidecar::{
    AlbumSidecar, ArtistSidecar, AudiobookSidecar, BookSidecar, Meta, ShowSidecar, SidecarError,
    read_sidecar, write_sidecar,
};
pub use template::{
    ReleaseType, audiobook_path, book_path, music_release_path, music_track_filename,
    podcast_episode_filename,
};

/// The primary service interface for Taxis.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait is stable since Rust 1.75; suppressed until Send bound concern is resolved"
)]
pub trait ImportService: Send + Sync {
    async fn import_download(
        &self,
        download: CompletedDownload,
    ) -> Result<ImportResult, TaxisError>;

    async fn import_scan(&self, path: &Path, library: &str) -> Result<ImportResult, TaxisError>;

    async fn get_import_queue(&self) -> Result<Vec<PendingImport>, TaxisError>;

    async fn manual_match(
        &self,
        item_id: MediaId,
        media_type: MediaType,
    ) -> Result<ImportResult, TaxisError>;
}
