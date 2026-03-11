// P1-07: ReplayGain / EBU R128 normalization stage.

use crate::config::ReplayGainConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

pub struct ReplayGainStage {
    config: ReplayGainConfig,
}

impl ReplayGainStage {
    pub fn new(config: ReplayGainConfig) -> Self {
        Self { config }
    }
}

impl DspStage for ReplayGainStage {
    fn name(&self) -> &str {
        "replaygain"
    }

    fn process(&mut self, _samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        todo!("P1-07: apply per-track or album gain scalar; compute ebur128 integrated loudness")
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::ReplayGain {
                mode: format!("{:?}", self.config.mode),
                gain_db: self.config.preamp_db,
            },
            tier_impact: None,
        }
    }
}
