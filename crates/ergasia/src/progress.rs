use serde::{Deserialize, Serialize};
use themelion::ids::DownloadId;

use crate::state::DownloadState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub download_id: DownloadId,
    pub state: DownloadState,
    pub percent_complete: u8,
    pub download_speed_bps: u64,
    pub upload_speed_bps: u64,
    pub peers_connected: u32,
    pub seeders: u32,
    pub eta_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_progress_serde_roundtrip() {
        let progress = DownloadProgress {
            download_id: DownloadId::new(),
            state: DownloadState::Downloading,
            percent_complete: 42,
            download_speed_bps: 1_000_000,
            upload_speed_bps: 500_000,
            peers_connected: 10,
            seeders: 5,
            eta_seconds: Some(300),
        };
        let json = serde_json::to_string(&progress).unwrap();
        let recovered: DownloadProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.percent_complete, 42);
        assert_eq!(recovered.download_speed_bps, 1_000_000);
    }
}
