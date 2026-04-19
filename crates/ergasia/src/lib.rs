pub mod error;
pub mod extract;
pub mod progress;
pub mod seeding;
pub mod session;
pub mod state;

use std::path::Path;

pub use error::ErgasiaError;
pub use extract::{ArchiveFormat, ExtractedFile, ExtractionResult, extract_archives};
pub use progress::DownloadProgress;
pub use seeding::{SeedingPolicy, TrackerSeedPolicy};
pub use session::ErgasiaSession;
pub use state::{DownloadEntry, DownloadState};
use themelion::ids::{DownloadId, WantId};

pub struct DownloadRequest {
    pub download_url: String,
    pub protocol: DownloadProtocol,
    pub download_id: DownloadId,
    pub want_id: WantId,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadProtocol {
    Torrent,
}

pub trait DownloadEngine: Send + Sync {
    fn start_download(
        &self,
        request: DownloadRequest,
    ) -> impl Future<Output = Result<DownloadId, ErgasiaError>> + Send;

    fn cancel_download(
        &self,
        download_id: DownloadId,
    ) -> impl Future<Output = Result<(), ErgasiaError>> + Send;

    fn get_progress(
        &self,
        download_id: DownloadId,
    ) -> impl Future<Output = Result<DownloadProgress, ErgasiaError>> + Send;

    fn extract(
        &self,
        download_path: &Path,
        output_dir: &Path,
    ) -> Result<Option<ExtractionResult>, ErgasiaError>;
}

use std::future::Future;
