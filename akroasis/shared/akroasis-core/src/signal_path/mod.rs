pub mod tier;

pub use tier::{source_tier, QualityTier};

use std::time::Instant;

/// A snapshot of the signal path state at a point in time.
/// Sent via `watch::Sender<SignalPathSnapshot>` whenever any stage changes.
#[derive(Debug, Clone)]
pub struct SignalPathSnapshot {
    /// Aggregate quality tier for the entire path (minimum across all stages).
    pub tier: QualityTier,
    pub source: Option<SourceInfo>,
    pub stages: Vec<SignalStageInfo>,
    pub output: Option<OutputInfo>,
    pub timestamp: Instant,
}

impl SignalPathSnapshot {
    /// Constructs an idle snapshot with no source or output.
    pub fn idle() -> Self {
        Self {
            tier: QualityTier::Lossless,
            source: None,
            stages: Vec::new(),
            output: None,
            timestamp: Instant::now(),
        }
    }
}

/// Describes the source audio stream in the signal path.
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    /// Original bit depth of the source. `None` for lossy codecs.
    pub bit_depth: Option<u32>,
    pub tier: QualityTier,
}

/// Describes one DSP stage's contribution to the signal path.
#[derive(Debug, Clone)]
pub struct SignalStageInfo {
    pub name: String,
    pub enabled: bool,
    pub params: StageParams,
    /// The tier impact of this stage when enabled. `None` = no impact.
    pub tier_impact: Option<QualityTier>,
}

/// Describes the audio output in the signal path.
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub channels: u16,
    pub exclusive_mode: bool,
}

/// Parameters captured for display / logging from a DSP stage.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum StageParams {
    SkipSilence {
        threshold_db: f64,
    },
    Eq {
        /// (frequency_hz, gain_db, q) per band.
        bands: Vec<(f64, f64, f64)>,
    },
    Crossfeed {
        strength: f64,
    },
    ReplayGain {
        mode: String,
        gain_db: f64,
    },
    Compressor {
        threshold_db: f64,
        ratio: f64,
    },
    Convolution {
        ir_name: Option<String>,
    },
    Volume {
        level_db: f64,
        dither: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_snapshot_has_no_source_or_output() {
        let snap = SignalPathSnapshot::idle();
        assert!(snap.source.is_none());
        assert!(snap.output.is_none());
        assert!(snap.stages.is_empty());
    }
}
