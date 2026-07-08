pub mod error;
pub mod extract;
pub mod progress;
pub mod seeding;
pub mod session;
pub mod state;

use std::future::Future;
use std::path::{Path, PathBuf};

pub use error::ErgasiaError;
pub use extract::{
    ArchiveFormat, ExtractedFile, ExtractionLimits, ExtractionResult, extract_archives,
};
pub use progress::DownloadProgress;
pub use seeding::{SeedingPolicy, TrackerSeedPolicy};
pub use session::TorrentSession;
pub use state::{DownloadEntry, DownloadState, map_torrent_stats};
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

    /// Resolves the on-disk path holding the download's content: the
    /// containing directory for a multi-file download, the file itself for a
    /// single-file one.
    fn content_path(
        &self,
        download_id: DownloadId,
    ) -> impl Future<Output = Result<PathBuf, ErgasiaError>> + Send;

    fn extract(
        &self,
        download_path: &Path,
        output_dir: &Path,
    ) -> impl Future<Output = Result<Option<ExtractionResult>, ErgasiaError>> + Send;
}
