// P1-10: Volume control + TPDF dither stage.

use crate::config::VolumeConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

pub struct Volume {
    config: VolumeConfig,
}

impl Volume {
    pub fn new(config: VolumeConfig) -> Self {
        Self { config }
    }
}

impl DspStage for Volume {
    fn name(&self) -> &str {
        "volume"
    }

    fn process(&mut self, _samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        todo!("P1-10: apply linear gain scalar; apply TPDF dither before quantization")
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: true, // volume is always active
            params: StageParams::Volume {
                level_db: self.config.level_db,
                dither: self.config.dither,
            },
            tier_impact: None,
        }
    }
}
