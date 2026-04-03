use std::time::Instant;

use harmonia_common::ids::{DownloadId, WantId};
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
    Completed,
    Seeding,
    SeedPolicySatisfied,
    Failed,
    Deleted,
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
                | (Seeding, SeedPolicySatisfied)
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
            (DownloadState::Seeding, DownloadState::SeedPolicySatisfied),
            (DownloadState::SeedPolicySatisfied, DownloadState::Deleted),
            (DownloadState::Queued, DownloadState::Failed),
        ];

        for (from, to) in cases {
            let mut entry = DownloadEntry::new(DownloadId::new(), WantId::new());
            entry.state = from;
            assert!(
                entry.transition_to(to).is_ok(),
                "expected {FROM} -> {to} to succeed"
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
                "expected {FROM} -> {to} to fail"
            );
            assert_eq!(entry.state, from);
        }
    }

    #[test]
    fn display_state() {
        assert_eq!(DownloadState::Queued.to_string(), "queued");
        assert_eq!(DownloadState::Downloading.to_string(), "downloading");
        assert_eq!(
            DownloadState::SeedPolicySatisfied.to_string(),
            "seed_policy_satisfied"
        );
    }
}
