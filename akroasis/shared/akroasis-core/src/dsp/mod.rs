pub mod silence;
pub mod eq;
pub mod crossfeed;
pub mod replaygain;
pub mod compressor;
pub mod convolution;
pub mod volume;

use tokio::sync::watch;

use crate::config::DspConfig;
use crate::signal_path::{SignalStageInfo, SignalPathSnapshot};

/// Result returned by `DspStage::process` after each frame.
pub struct StageResult {
    /// Metadata snapshot for this stage, used to build the signal path.
    pub meta: SignalStageInfo,
}

/// A single processing stage in the DSP pipeline.
///
/// Implementations must be `Send + Sync` for use in the audio task.
/// `process` takes `&mut self` because stages maintain internal state (filters, gain
/// smoothing, lookahead buffers). Frame data is modified in place.
pub trait DspStage: Send + Sync {
    /// Short human-readable name used in signal path display.
    fn name(&self) -> &str;

    /// Processes `samples` in place (interleaved, f64, channel-major order).
    /// Returns metadata about this processing step.
    fn process(
        &mut self,
        samples: &mut [f64],
        channels: u16,
        sample_rate: u32,
    ) -> StageResult;

    /// Returns the current signal path metadata for this stage.
    fn signal_stage_meta(&self) -> SignalStageInfo;
}

/// The full DSP pipeline: an ordered list of stages with live config updates.
pub struct DspPipeline {
    stages: Vec<Box<dyn DspStage>>,
    config_rx: watch::Receiver<DspConfig>,
}

impl DspPipeline {
    /// Constructs the pipeline from the initial config. The `config_rx` channel is polled
    /// each frame so DSP settings can be updated without stopping playback.
    pub fn new(initial_config: DspConfig, config_rx: watch::Receiver<DspConfig>) -> Self {
        let stages = Self::build_stages(&initial_config);
        Self { stages, config_rx }
    }

    /// Processes one frame through all stages in order. Returns a snapshot of the signal path
    /// state after this frame, including per-stage metadata.
    pub fn process_frame(
        &mut self,
        samples: &mut [f64],
        channels: u16,
        sample_rate: u32,
    ) -> Vec<SignalStageInfo> {
        // Apply any pending config update.
        if self.config_rx.has_changed().unwrap_or(false) {
            let config = self.config_rx.borrow_and_update().clone();
            self.stages = Self::build_stages(&config);
        }

        self.stages
            .iter_mut()
            .map(|stage| {
                let result = stage.process(samples, channels, sample_rate);
                result.meta
            })
            .collect()
    }

    /// Returns the current signal path metadata for all stages without processing audio.
    pub fn stage_metas(&self) -> Vec<SignalStageInfo> {
        self.stages.iter().map(|s| s.signal_stage_meta()).collect()
    }

    /// Rebuilds the stage list from a config snapshot.
    fn build_stages(config: &DspConfig) -> Vec<Box<dyn DspStage>> {
        vec![
            Box::new(silence::SkipSilence::new(config.skip_silence.clone())),
            Box::new(eq::ParametricEq::new(config.eq.clone())),
            Box::new(crossfeed::Crossfeed::new(config.crossfeed.clone())),
            Box::new(replaygain::ReplayGainStage::new(config.replaygain.clone())),
            Box::new(compressor::Compressor::new(config.compressor.clone())),
            Box::new(convolution::Convolution::new(config.convolution.clone())),
            Box::new(volume::Volume::new(config.volume.clone())),
        ]
    }

    /// Convenience: build a snapshot from the current stage metadata.
    pub fn build_snapshot(
        &self,
        source_snapshot: &SignalPathSnapshot,
    ) -> Vec<SignalStageInfo> {
        let _ = source_snapshot; // used by callers to merge; pipeline provides stage list only
        self.stage_metas()
    }
}
