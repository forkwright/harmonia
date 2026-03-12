use std::time::{Duration, Instant};

use harmonia_common::ids::DownloadId;
use serde::{Deserialize, Serialize};

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

#[derive(Debug)]
pub struct ProgressThrottle {
    last_emit: Instant,
    last_percent: u8,
    throttle_duration: Duration,
}

impl ProgressThrottle {
    pub fn new(throttle_duration: Duration) -> Self {
        Self {
            last_emit: Instant::now() - throttle_duration,
            last_percent: 0,
            throttle_duration,
        }
    }

    pub fn should_emit(&mut self, percent: u8) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_emit);
        let delta = percent.abs_diff(self.last_percent);

        if elapsed >= self.throttle_duration && delta >= 1 {
            self.last_emit = now;
            self.last_percent = percent;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_allows_first_emission_with_change() {
        let mut throttle = ProgressThrottle::new(Duration::from_secs(2));
        assert!(throttle.should_emit(1));
    }

    #[test]
    fn throttle_blocks_within_window() {
        let mut throttle = ProgressThrottle::new(Duration::from_secs(100));
        assert!(throttle.should_emit(1));
        assert!(!throttle.should_emit(5));
    }

    #[test]
    fn throttle_blocks_small_delta() {
        let mut throttle = ProgressThrottle::new(Duration::from_millis(0));
        assert!(throttle.should_emit(1));
        assert!(!throttle.should_emit(1));
    }

    #[test]
    fn throttle_allows_after_window_and_delta() {
        let mut throttle = ProgressThrottle::new(Duration::from_millis(0));
        assert!(throttle.should_emit(1));
        assert!(throttle.should_emit(2));
        assert!(throttle.should_emit(5));
    }

    #[test]
    fn throttle_blocks_zero_delta_even_after_window() {
        let mut throttle = ProgressThrottle::new(Duration::from_millis(0));
        assert!(throttle.should_emit(5));
        assert!(!throttle.should_emit(5));
    }

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
