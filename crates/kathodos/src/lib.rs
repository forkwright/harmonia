pub mod error;
pub mod event;
pub mod import;
pub mod scanner;

use std::path::Path;

use themelion::{MediaId, MediaType};

use crate::error::TaxisError;
use crate::import::{CompletedDownload, ImportResult, PendingImport};

pub use error::{EpignosisError, TaxisError as Error};
pub use event::{Debouncer, WatchEvent, WatchEventKind};
pub use import::{
    ImportOperation, ImportOrigin, ImportPipeline, ImportResult as ImportResultPub, ImportSource,
    MetadataResolver, PendingImport as PendingImportPub, ResolvedMetadata,
};
pub use scanner::ScannerManager;

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
