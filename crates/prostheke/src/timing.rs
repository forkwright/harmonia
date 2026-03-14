//! Timing synchronization — placeholder pending full implementation.
//!
//! Full timing sync requires audio fingerprinting to detect offset between the
//! subtitle track and the video stream. Deferred to a future phase.

use std::path::Path;

use crate::error::ProsthekeError;

/// Adjust subtitle timestamps to match a video stream.
///
/// The offset is detected by comparing audio silence patterns or scene-change
/// timing against subtitle cue points.
///
/// Not yet implemented — returns `TimingSyncFailed` unconditionally.
pub trait TimingSync: Send + Sync {
    fn detect_offset(
        &self,
        video_path: &Path,
        subtitle_content: &[u8],
    ) -> Result<f64, ProsthekeError>;

    fn adjust_timestamps(
        &self,
        subtitle_content: &[u8],
        offset_secs: f64,
    ) -> Result<Vec<u8>, ProsthekeError>;
}

/// Placeholder implementation that always returns `TimingSyncFailed`.
pub struct NoopTimingSync;

impl TimingSync for NoopTimingSync {
    fn detect_offset(
        &self,
        _video_path: &Path,
        _subtitle_content: &[u8],
    ) -> Result<f64, ProsthekeError> {
        Err(ProsthekeError::TimingSyncFailed {
            location: snafu::location!(),
        })
    }

    fn adjust_timestamps(
        &self,
        _subtitle_content: &[u8],
        _offset_secs: f64,
    ) -> Result<Vec<u8>, ProsthekeError> {
        Err(ProsthekeError::TimingSyncFailed {
            location: snafu::location!(),
        })
    }
}
