// P1-09: Convolution reverb / room-correction stage.

use crate::config::ConvolutionConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{QualityTier, SignalStageInfo, StageParams};

pub struct Convolution {
    config: ConvolutionConfig,
}

impl Convolution {
    pub fn new(config: ConvolutionConfig) -> Self {
        Self { config }
    }
}

impl DspStage for Convolution {
    fn name(&self) -> &str {
        "convolution"
    }

    fn process(&mut self, _samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        todo!(
            "P1-09: partition-overlap-save FFT convolution against loaded IR; passthrough when !enabled"
        )
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        let ir_name = self
            .config
            .ir_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::Convolution { ir_name },
            // Convolution with a room IR lowers quality tier to HighQuality.
            tier_impact: self.config.enabled.then_some(QualityTier::HighQuality),
        }
    }
}
