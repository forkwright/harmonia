// Stage 6: Convolution reverb / room-correction — Phase 5 passthrough.

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
        // Partition-overlap-save FFT convolution is a Phase 5 feature.
        // Passthrough: samples are unmodified regardless of enabled state.
        StageResult {
            meta: self.signal_stage_meta(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConvolutionConfig;

    #[test]
    fn passthrough_disabled() {
        let mut conv = Convolution::new(ConvolutionConfig::default());
        let original = [0.1_f64, -0.5, 0.9, -0.3];
        let mut samples = original;
        conv.process(&mut samples, 2, 44100);
        assert_eq!(samples, original, "disabled convolution must not modify samples");
    }

    #[test]
    fn passthrough_enabled() {
        let cfg = ConvolutionConfig {
            enabled: true,
            ir_path: Some("/tmp/room.wav".into()),
            output_gain_db: 0.0,
        };
        let mut conv = Convolution::new(cfg);
        let original = [0.2_f64, -0.4, 0.6];
        let mut samples = original;
        conv.process(&mut samples, 1, 48000);
        assert_eq!(samples, original, "enabled convolution passthrough must not modify samples");
    }

    #[test]
    fn signal_path_reports_inactive() {
        let conv = Convolution::new(ConvolutionConfig::default());
        let meta = conv.signal_stage_meta();
        assert!(!meta.enabled);
        assert_eq!(meta.tier_impact, None);
    }

    #[test]
    fn signal_path_enabled_reports_high_quality_tier() {
        let cfg = ConvolutionConfig {
            enabled: true,
            ir_path: None,
            output_gain_db: 0.0,
        };
        let conv = Convolution::new(cfg);
        let meta = conv.signal_stage_meta();
        assert!(meta.enabled);
        assert_eq!(meta.tier_impact, Some(QualityTier::HighQuality));
    }
}
