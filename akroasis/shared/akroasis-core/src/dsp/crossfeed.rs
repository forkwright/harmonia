// P1-06: Crossfeed DSP stage — Bauer/bs2b-style stereo crossfeed for headphone listening.

use crate::config::CrossfeedConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{QualityTier, SignalStageInfo, StageParams};

pub struct Crossfeed {
    config: CrossfeedConfig,
}

impl Crossfeed {
    pub fn new(config: CrossfeedConfig) -> Self {
        Self { config }
    }
}

impl DspStage for Crossfeed {
    fn name(&self) -> &str {
        "crossfeed"
    }

    fn process(&mut self, _samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        todo!("P1-06: apply frequency-dependent cross-channel blending; bypass when !enabled")
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::Crossfeed {
                strength: self.config.strength,
            },
            // Crossfeed modifies the stereo image but does not reduce frequency resolution.
            tier_impact: self.config.enabled.then_some(QualityTier::Lossless),
        }
    }
}
