use std::path::PathBuf;

use snafu::Snafu;
use themelion::ids::DownloadId;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum ErgasiaError {
    #[snafu(display("failed to initialize librqbit session"))]
    SessionInit {
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to add torrent: {reason}"))]
    AddTorrent {
        reason: String,
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("torrent not found: {download_id}"))]
    TorrentNotFound {
        download_id: DownloadId,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to query torrent stats for {download_id}"))]
    StatsQuery {
        download_id: DownloadId,
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to pause torrent {download_id}"))]
    PauseAction {
        download_id: DownloadId,
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to open archive at {}", path.display()))]
    OpenArchive {
        path: PathBuf,
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to extract file {}", path.display()))]
    ExtractFile {
        path: PathBuf,
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("nested archive depth {depth} exceeds maximum {max}"))]
    NestingDepthExceeded {
        depth: u8,
        max: u8,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("insufficient disk space: need {needed} bytes, have {available} bytes"))]
    InsufficientDiskSpace {
        needed: u64,
        available: u64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unsupported archive format at {}: magic bytes {magic_bytes:02X?}", path.display()))]
    UnsupportedFormat {
        path: PathBuf,
        magic_bytes: [u8; 4],
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("invalid state transition from {from} to {to}"))]
    InvalidStateTransition {
        from: String,
        to: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
