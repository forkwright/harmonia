//! Domain types for the download queue and post-processing pipeline.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use themelion::ids::{DownloadId, ReleaseId, WantId};
use uuid::Uuid;

/// Protocol used to retrieve the release.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadProtocol {
    Torrent,
    Usenet,
}

impl DownloadProtocol {
    /// Canonical string representation stored in the database.
    pub fn as_db_str(self) -> &'static str {
        match self {
            DownloadProtocol::Torrent => "torrent",
            DownloadProtocol::Usenet => "nzb",
        }
    }
}

impl std::fmt::Display for DownloadProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_db_str())
    }
}

/// A single entry waiting to be dispatched or actively downloading.
#[derive(Debug, Clone)]
pub struct QueueItem {
    /// UUIDv7 identifier; also used as `download_queue.id` in the DB.
    pub id: Uuid,
    pub want_id: WantId,
    pub release_id: ReleaseId,
    pub download_url: String,
    pub protocol: DownloadProtocol,
    /// 1–4; see priority tier table in orchestration.md.
    pub priority: u8,
    /// FK to `indexers.id` (torrent only).
    pub tracker_id: Option<i64>,
    /// Torrent info hash (torrent only).
    pub info_hash: Option<String>,
}

/// Passed from Syntaxis to the import service when post-processing is complete.
#[derive(Debug, Clone)]
pub struct CompletedDownload {
    pub download_id: DownloadId,
    /// Path to the completed download (or extracted files if archives were present).
    pub download_path: PathBuf,
    /// Original download directory; used for seeding cleanup and archive removal.
    pub source_path: PathBuf,
    pub want_id: WantId,
    pub release_id: ReleaseId,
    pub protocol: DownloadProtocol,
    /// Set when `std::fs::hard_link` fails with `EXDEV`; Taxis copies instead.
    pub requires_copy: bool,
}

/// Returned by `QueueManager::enqueue`.
#[derive(Debug, Clone)]
pub struct QueuePosition {
    /// Zero-based position in the priority queue (0 = next to dispatch).
    pub position: usize,
    /// Rough estimate in seconds; `None` when the queue is empty.
    pub estimated_wait_secs: Option<u64>,
}

/// Current queue state for API responses.
#[derive(Debug, Clone)]
pub struct QueueSnapshot {
    pub active_downloads: Vec<QueueItem>,
    pub queued_items: Vec<QueueItem>,
    pub completed_count: u64,
    pub failed_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_db_str_torrent() {
        assert_eq!(DownloadProtocol::Torrent.as_db_str(), "torrent");
    }

    #[test]
    fn protocol_db_str_usenet() {
        assert_eq!(DownloadProtocol::Usenet.as_db_str(), "nzb");
    }

    #[test]
    fn protocol_display_matches_db_str() {
        assert_eq!(
            DownloadProtocol::Torrent.to_string(),
            DownloadProtocol::Torrent.as_db_str()
        );
        assert_eq!(
            DownloadProtocol::Usenet.to_string(),
            DownloadProtocol::Usenet.as_db_str()
        );
    }

    #[test]
    fn completed_download_carries_required_fields() {
        let dl = CompletedDownload {
            download_id: DownloadId::new(),
            download_path: PathBuf::from("/data/downloads/album"),
            source_path: PathBuf::from("/data/downloads/album"),
            want_id: WantId::new(),
            release_id: ReleaseId::new(),
            protocol: DownloadProtocol::Torrent,
            requires_copy: false,
        };
        assert!(!dl.requires_copy);
        assert_eq!(dl.protocol, DownloadProtocol::Torrent);
    }
}
