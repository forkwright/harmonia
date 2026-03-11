// P1-04: Skip-silence DSP stage.

use crate::config::SkipSilenceConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

pub struct SkipSilence {
    config: SkipSilenceConfig,
}

impl SkipSilence {
    pub fn new(config: SkipSilenceConfig) -> Self {
        Self { config }
    }
}

impl DspStage for SkipSilence {
    fn name(&self) -> &str {
        "skip-silence"
    }

    fn process(&mut self, _samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        todo!("P1-04: detect and trim silence at track boundaries")
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::SkipSilence {
                threshold_db: self.config.threshold_db,
            },
            tier_impact: None,
        }
    }
}
