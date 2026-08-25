use aggelmata::ids::DownloadId;
use librqbit::TorrentStats;
use serde::{Deserialize, Serialize};

use crate::state::{DownloadState, map_torrent_stats};

// WHY: librqbit's `Speed.mbps` is bytes/s divided by 1024^2 — MiB/s despite
// the name (librqbit-core speed_estimator.rs `mbps()`); converting back with
// the same factor keeps the reported bytes/s honest.
const MIB_PER_SECOND_TO_BPS: f64 = 1024.0 * 1024.0;

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
    /// Engine-reported failure detail; `Some` only when `state` is `Failed`.
    pub error: Option<String>,
}

impl DownloadProgress {
    /// Assembles a progress report from a librqbit stats snapshot.
    pub fn from_stats(download_id: DownloadId, stats: &TorrentStats) -> Self {
        let (download_speed_bps, upload_speed_bps, peers_connected) = match &stats.live {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "speeds are non-negative and far below u64::MAX"
            )]
            // WHY: librqbit 9's AggregatePeerStats.live is already u32
            // (stat_gen.rs's gen_stats! macro expands `live u32` to a plain
            // u32 snapshot field, no atomic-to-narrower-int narrowing left);
            // librqbit 8's version was `usize`, which is what the removed
            // u32::try_from(...).unwrap_or(u32::MAX) saturating conversion
            // was guarding against. No conversion needed or possible now.
            Some(live) => (
                (live.download_speed.mbps * MIB_PER_SECOND_TO_BPS) as u64,
                (live.upload_speed.mbps * MIB_PER_SECOND_TO_BPS) as u64,
                live.snapshot.peer_stats.live,
            ),
            None => (0, 0, 0),
        };

        let remaining = stats.total_bytes.saturating_sub(stats.progress_bytes);
        // WHY: recomputed from remaining/speed — librqbit's own estimate
        // (`LiveStats.time_remaining`) wraps a Duration that is private to
        // the crate, so it cannot be read back here.
        let eta_seconds = if stats.finished || download_speed_bps == 0 {
            None
        } else {
            Some(remaining / download_speed_bps)
        };

        Self {
            download_id,
            state: map_torrent_stats(stats),
            percent_complete: percent_complete(
                stats.progress_bytes,
                stats.total_bytes,
                stats.finished,
            ),
            download_speed_bps,
            upload_speed_bps,
            peers_connected,
            // NOTE: librqbit exposes no per-torrent seeder count; 0 is the
            // honest value, not a fabricated one.
            seeders: 0,
            eta_seconds,
            error: stats.error.clone(),
        }
    }
}

fn percent_complete(progress: u64, total: u64, finished: bool) -> u8 {
    if finished {
        100
    } else if total > 0 {
        // SAFETY: progress <= total keeps the ratio in [0, 100]; floor of a
        // value in that range always fits u8.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "floored ratio is bounded to [0, 100]"
        )]
        {
            ((progress as f64 / total as f64) * 100.0).floor() as u8
        }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use librqbit::TorrentStatsState;
    use librqbit::api::LiveStats;

    use super::*;

    fn stats(state: TorrentStatsState, finished: bool) -> TorrentStats {
        TorrentStats {
            state,
            file_progress: Vec::new(),
            error: None,
            progress_bytes: 0,
            uploaded_bytes: 0,
            total_bytes: 0,
            finished,
            live: None,
        }
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
            error: None,
        };
        let json = serde_json::to_string(&progress).unwrap();
        let recovered: DownloadProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.percent_complete, 42);
        assert_eq!(recovered.download_speed_bps, 1_000_000);
        assert!(recovered.error.is_none());
    }

    #[test]
    fn percent_is_zero_when_total_unknown() {
        assert_eq!(percent_complete(0, 0, false), 0);
        assert_eq!(percent_complete(500, 0, false), 0);
    }

    #[test]
    fn percent_is_hundred_when_finished_regardless_of_bytes() {
        assert_eq!(percent_complete(0, 0, true), 100);
        assert_eq!(percent_complete(50, 100, true), 100);
    }

    #[test]
    fn percent_floors_the_ratio() {
        assert_eq!(percent_complete(999, 1000, false), 99);
        assert_eq!(percent_complete(1, 3, false), 33);
        assert_eq!(percent_complete(1000, 1000, false), 100);
    }

    #[test]
    fn from_stats_propagates_engine_error() {
        let mut s = stats(TorrentStatsState::Error, false);
        s.error = Some("disk gone".to_string());
        let progress = DownloadProgress::from_stats(DownloadId::new(), &s);
        assert_eq!(progress.state, DownloadState::Failed);
        assert_eq!(progress.error.as_deref(), Some("disk gone"));
    }

    #[test]
    fn from_stats_reads_live_speeds_and_peers() {
        let mut live = LiveStats {
            download_speed: 2.0.into(),
            upload_speed: 0.5.into(),
            ..LiveStats::default()
        };
        live.snapshot.peer_stats.live = 7;
        let mut s = stats(TorrentStatsState::Live, false);
        s.progress_bytes = 25;
        s.total_bytes = 100;
        s.live = Some(live);

        let progress = DownloadProgress::from_stats(DownloadId::new(), &s);
        assert_eq!(progress.state, DownloadState::Downloading);
        assert_eq!(progress.percent_complete, 25);
        assert_eq!(progress.download_speed_bps, 2 * 1024 * 1024);
        assert_eq!(progress.upload_speed_bps, 512 * 1024);
        assert_eq!(progress.peers_connected, 7);
        assert_eq!(progress.seeders, 0);
        // 75 remaining bytes at 2 MiB/s floors to 0 seconds.
        assert_eq!(progress.eta_seconds, Some(0));
    }

    #[test]
    fn from_stats_eta_absent_when_finished_or_idle() {
        let mut s = stats(TorrentStatsState::Live, true);
        s.progress_bytes = 100;
        s.total_bytes = 100;
        s.live = Some(LiveStats::default());
        let progress = DownloadProgress::from_stats(DownloadId::new(), &s);
        assert_eq!(progress.state, DownloadState::Seeding);
        assert_eq!(progress.percent_complete, 100);
        assert_eq!(progress.eta_seconds, None);

        let mut idle = stats(TorrentStatsState::Live, false);
        idle.total_bytes = 100;
        idle.live = Some(LiveStats::default());
        let progress = DownloadProgress::from_stats(DownloadId::new(), &idle);
        assert_eq!(
            progress.eta_seconds, None,
            "zero speed must not fabricate an ETA"
        );
    }
}
