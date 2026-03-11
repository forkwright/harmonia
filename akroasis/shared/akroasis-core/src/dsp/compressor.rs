// P1-08: Compressor / limiter DSP stage.

use crate::config::CompressorConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{QualityTier, SignalStageInfo, StageParams};

pub struct Compressor {
    config: CompressorConfig,
}

impl Compressor {
    pub fn new(config: CompressorConfig) -> Self {
        Self { config }
    }
}

impl DspStage for Compressor {
    fn name(&self) -> &str {
        "compressor"
    }

    fn process(&mut self, _samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        todo!("P1-08: peak-mode compressor with attack/release envelope; hard limiter ceiling")
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::Compressor {
                threshold_db: self.config.threshold_db,
                ratio: self.config.ratio,
            },
            // Compression reduces dynamic range — lowers tier from BitPerfect to Lossless.
            tier_impact: self.config.enabled.then_some(QualityTier::Lossless),
        }
    }
}
