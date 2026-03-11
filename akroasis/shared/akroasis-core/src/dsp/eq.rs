// P1-05: Parametric EQ DSP stage — biquad filter bank.

use crate::config::EqConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

pub struct ParametricEq {
    config: EqConfig,
}

impl ParametricEq {
    pub fn new(config: EqConfig) -> Self {
        Self { config }
    }
}

impl DspStage for ParametricEq {
    fn name(&self) -> &str {
        "parametric-eq"
    }

    fn process(&mut self, _samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        todo!("P1-05: apply biquad peaking filter per band; bypass when !enabled")
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        let bands = self
            .config
            .bands
            .iter()
            .map(|b| (b.frequency, b.gain_db, b.q))
            .collect();
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::Eq { bands },
            tier_impact: None,
        }
    }
}
