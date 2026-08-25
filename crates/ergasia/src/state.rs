use std::time::Instant;

use aggelmata::ids::{DownloadId, WantId};
use librqbit::{TorrentStats, TorrentStatsState};
use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::error::{ErgasiaError, InvalidStateTransitionSnafu};
use crate::progress::DownloadProgress;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    Queued,
    Initializing,
    Downloading,
    Paused,
    Completed,
    Seeding,
    SeedPolicySatisfied,
    Failed,
    Deleted,
}

/// Maps a librqbit stats snapshot onto the honest harmonia download state.
///
/// WHY: librqbit reports a coarse engine state (`initializing`/`live`/
/// `paused`/`error`) plus a `finished` flag; harmonia's lifecycle needs the
/// cross product spelled out so completion, seeding, and failure each surface
/// as themselves instead of a hardcoded `Downloading` (#602).
pub fn map_torrent_stats(stats: &TorrentStats) -> DownloadState {
    match (stats.state, stats.finished) {
        // WHY: librqbit folds its internal `None` state into `Error` with a
        // diagnostic message (torrent_state/mod.rs `stats()`), so this arm
        // covers both.
        (TorrentStatsState::Error, _) => DownloadState::Failed,
        // NOTE: librqbit 9 added a `paused` field to `Initializing` (whether
        // an initializing torrent is also paused) — ignored here, matching
        // librqbit 8's coarser (fieldless) variant: harmonia has no
        // `Initializing`+paused state of its own to report.
        (TorrentStatsState::Initializing { .. }, _) => DownloadState::Initializing,
        (TorrentStatsState::Live, false) => DownloadState::Downloading,
        (TorrentStatsState::Live, true) => DownloadState::Seeding,
        // NOTE: the only production pauser is the seed monitor (#590);
        // librqbit persists `is_paused`, so a restored paused+finished
        // torrent still reads as seed-policy-satisfied after a restart.
        (TorrentStatsState::Paused, true) => DownloadState::SeedPolicySatisfied,
        (TorrentStatsState::Paused, false) => DownloadState::Paused,
    }
}

impl DownloadState {
    pub fn can_transition_to(self, next: DownloadState) -> bool {
        use DownloadState::*;
        matches!(
            (self, next),
            (Queued, Initializing)
                | (Initializing, Downloading)
                | (Initializing, Failed)
                | (Downloading, Completed)
                | (Downloading, Failed)
                | (Completed, Seeding)
                | (Completed, Deleted)
                | (Seeding, SeedPolicySatisfied)
                | (Seeding, Failed)
                | (SeedPolicySatisfied, Deleted)
                | (Queued, Failed)
        )
    }
}

impl std::fmt::Display for DownloadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Queued => "queued",
            Self::Initializing => "initializing",
            Self::Downloading => "downloading",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Seeding => "seeding",
            Self::SeedPolicySatisfied => "seed_policy_satisfied",
            Self::Failed => "failed",
            Self::Deleted => "deleted",
        };
        f.write_str(s)
    }
}

#[derive(Debug)]
pub struct DownloadEntry {
    pub download_id: DownloadId,
    pub torrent_id: Option<usize>,
    pub state: DownloadState,
    pub want_id: WantId,
    pub started_at: Instant,
    pub progress: Option<DownloadProgress>,
    pub failure_reason: Option<String>,
}

impl DownloadEntry {
    pub fn new(download_id: DownloadId, want_id: WantId) -> Self {
        Self {
            download_id,
            torrent_id: None,
            state: DownloadState::Queued,
            want_id,
            started_at: Instant::now(),
            progress: None,
            failure_reason: None,
        }
    }

    pub fn transition_to(&mut self, next: DownloadState) -> Result<(), ErgasiaError> {
        ensure!(
            self.state.can_transition_to(next),
            InvalidStateTransitionSnafu {
                from: self.state.to_string(),
                to: next.to_string(),
            }
        );
        self.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transitions_succeed() {
        let cases = [
            (DownloadState::Queued, DownloadState::Initializing),
            (DownloadState::Initializing, DownloadState::Downloading),
            (DownloadState::Initializing, DownloadState::Failed),
            (DownloadState::Downloading, DownloadState::Completed),
            (DownloadState::Downloading, DownloadState::Failed),
            (DownloadState::Completed, DownloadState::Seeding),
            (DownloadState::Completed, DownloadState::Deleted),
            (DownloadState::Seeding, DownloadState::SeedPolicySatisfied),
            (DownloadState::Seeding, DownloadState::Failed),
            (DownloadState::SeedPolicySatisfied, DownloadState::Deleted),
            (DownloadState::Queued, DownloadState::Failed),
        ];

        for (from, to) in cases {
            let mut entry = DownloadEntry::new(DownloadId::new(), WantId::new());
            entry.state = from;
            assert!(
                entry.transition_to(to).is_ok(),
                "expected {from} -> {to} to succeed"
            );
            assert_eq!(entry.state, to);
        }
    }

    #[test]
    fn invalid_transitions_fail() {
        let cases = [
            (DownloadState::Queued, DownloadState::Seeding),
            (DownloadState::Queued, DownloadState::Completed),
            (DownloadState::Queued, DownloadState::Downloading),
            (DownloadState::Downloading, DownloadState::Queued),
            (DownloadState::Completed, DownloadState::Downloading),
            (DownloadState::Seeding, DownloadState::Downloading),
            (DownloadState::Failed, DownloadState::Downloading),
            (DownloadState::Deleted, DownloadState::Queued),
        ];

        for (from, to) in cases {
            let mut entry = DownloadEntry::new(DownloadId::new(), WantId::new());
            entry.state = from;
            assert!(
                entry.transition_to(to).is_err(),
                "expected {from} -> {to} to fail"
            );
            assert_eq!(entry.state, from);
        }
    }

    #[test]
    fn display_state() {
        assert_eq!(DownloadState::Queued.to_string(), "queued");
        assert_eq!(DownloadState::Downloading.to_string(), "downloading");
        assert_eq!(DownloadState::Paused.to_string(), "paused");
        assert_eq!(
            DownloadState::SeedPolicySatisfied.to_string(),
            "seed_policy_satisfied"
        );
    }

    fn stats(state: TorrentStatsState, finished: bool, error: Option<&str>) -> TorrentStats {
        TorrentStats {
            state,
            file_progress: Vec::new(),
            error: error.map(str::to_owned),
            progress_bytes: 0,
            uploaded_bytes: 0,
            total_bytes: 0,
            finished,
            live: None,
        }
    }

    #[test]
    fn map_torrent_stats_covers_every_engine_state() {
        use TorrentStatsState as S;
        let cases = [
            (S::Error, false, None, DownloadState::Failed),
            (S::Error, true, None, DownloadState::Failed),
            // WHY: librqbit's internal None state surfaces as Error + message.
            (
                S::Error,
                false,
                Some("bug: torrent in broken \"None\" state"),
                DownloadState::Failed,
            ),
            // NOTE: the `paused` field here is librqbit 9's addition to
            // Initializing (see the WHY note on map_torrent_stats above) —
            // it is unrelated to the `finished` bool in the second tuple
            // slot (a torrent still initializing is never `finished`; these
            // rows exercise `finished` true/false anyway for exhaustive
            // coverage of the cross product). map_torrent_stats ignores
            // `paused` entirely, so its value here is inert; `false` is the
            // representative case.
            (
                S::Initializing { paused: false },
                false,
                None,
                DownloadState::Initializing,
            ),
            (
                S::Initializing { paused: false },
                true,
                None,
                DownloadState::Initializing,
            ),
            (S::Live, false, None, DownloadState::Downloading),
            (S::Live, true, None, DownloadState::Seeding),
            (S::Paused, true, None, DownloadState::SeedPolicySatisfied),
            (S::Paused, false, None, DownloadState::Paused),
        ];

        for (state, finished, error, expected) in cases {
            let mapped = map_torrent_stats(&stats(state, finished, error));
            assert_eq!(
                mapped, expected,
                "({state:?}, finished={finished}) must map to {expected:?}"
            );
        }
    }
}
