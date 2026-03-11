use crate::config::SkipSilenceConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

pub struct SkipSilence {
    config: SkipSilenceConfig,
    /// Consecutive silent frames seen so far (reset on any non-silent frame).
    consecutive_silent: usize,
}

impl SkipSilence {
    pub fn new(config: SkipSilenceConfig) -> Self {
        Self { config, consecutive_silent: 0 }
    }

    fn rms_db(samples: &[f64]) -> f64 {
        if samples.is_empty() {
            return f64::NEG_INFINITY;
        }
        let mean_sq = samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64;
        if mean_sq == 0.0 {
            return f64::NEG_INFINITY;
        }
        20.0 * mean_sq.sqrt().log10()
    }
}

impl DspStage for SkipSilence {
    fn name(&self) -> &str {
        "skip-silence"
    }

    fn process(&mut self, samples: &mut [f64], channels: u16, _sample_rate: u32) -> StageResult {
        if self.config.enabled {
            let frame_count = samples.len() / channels.max(1) as usize;
            let is_silent = Self::rms_db(samples) < self.config.threshold_db;

            if is_silent {
                let prev_silent = self.consecutive_silent;
                self.consecutive_silent = self.consecutive_silent.saturating_add(frame_count);

                // Zero the frame while we are in [min, max) of consecutive silence.
                // Frames beyond max_silence_samples pass through to preserve intentional pauses.
                if self.consecutive_silent >= self.config.min_silence_samples
                    && prev_silent < self.config.max_silence_samples
                {
                    samples.fill(0.0);
                }
            } else {
                self.consecutive_silent = 0;
            }
        }

        StageResult { meta: self.signal_stage_meta() }
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::SkipSilence { threshold_db: self.config.threshold_db },
            tier_impact: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(enabled: bool, threshold_db: f64, min_ms: f64, max_ms: f64) -> SkipSilenceConfig {
        let sr = 44100.0_f64;
        SkipSilenceConfig {
            enabled,
            threshold_db,
            min_silence_samples: (min_ms * sr / 1000.0) as usize,
            max_silence_samples: (max_ms * sr / 1000.0) as usize,
        }
    }

    /// A whisper-quiet signal well below any reasonable threshold (-160 dBFS).
    fn near_silence(len: usize) -> Vec<f64> {
        vec![1e-8_f64; len]
    }

    fn audible_tone(len: usize) -> Vec<f64> {
        (0..len)
            .map(|i| (i as f64 * std::f64::consts::TAU / 100.0).sin() * 0.5)
            .collect()
    }

    fn process_all(stage: &mut SkipSilence, samples: &mut [f64], frame_size: usize) {
        for chunk in samples.chunks_mut(frame_size) {
            stage.process(chunk, 1, 44100);
        }
    }

    #[test]
    fn detects_silence_region() {
        let sr = 44100_usize;
        let frame = 1024;
        // min 200 ms, max 3 s — sustained 1 s silence should be removed
        let mut stage = SkipSilence::new(cfg(true, -50.0, 200.0, 3000.0));

        // Process 1 s of near-silence.
        let mut silence = near_silence(sr);
        process_all(&mut stage, &mut silence, frame);

        // After >200 ms of accumulated silence, frames should be zeroed.
        let mut extra = near_silence(frame);
        stage.process(&mut extra, 1, 44100);
        assert!(extra.iter().all(|&s| s == 0.0), "silence should be zeroed after min threshold");
    }

    #[test]
    fn passes_tone_unchanged() {
        let frame = 1024;
        let mut stage = SkipSilence::new(cfg(true, -50.0, 200.0, 3000.0));
        let tone = audible_tone(frame);
        let mut buf = tone.clone();
        stage.process(&mut buf, 1, 44100);
        assert_eq!(buf, tone, "audible tone should pass through unchanged");
    }

    #[test]
    fn respects_min_silence_ms() {
        // min = 200 ms → 8820 frames at 44.1 kHz
        // A 10 ms burst of near-silence is too short to be removed.
        let mut stage = SkipSilence::new(cfg(true, -50.0, 200.0, 3000.0));
        let mut short = near_silence(441); // 10 ms
        let original = short.clone();
        stage.process(&mut short, 1, 44100);
        assert_eq!(short, original, "short silence should pass through (below min threshold)");
    }

    #[test]
    fn respects_max_silence_ms() {
        // max = 500 ms → 22050 frames at 44.1 kHz
        let frame = 1024;
        let mut stage = SkipSilence::new(cfg(true, -50.0, 10.0, 500.0));

        // Drain well past 500 ms worth of frames.
        let mut bulk = near_silence(44100); // 1 s
        process_all(&mut stage, &mut bulk, frame);

        // At this point consecutive_silent >> max; further silence passes through.
        let original = near_silence(frame);
        let mut extra = original.clone();
        stage.process(&mut extra, 1, 44100);
        assert_eq!(extra, original, "silence beyond max should pass through unchanged");
    }

    #[test]
    fn resets_on_tone() {
        let frame = 1024;
        // min = 200 ms (8820 frames at 44.1 kHz)
        let mut stage = SkipSilence::new(cfg(true, -50.0, 200.0, 3000.0));

        // Build up enough consecutive silence to be well into the removal region.
        let mut silence = near_silence(44100);
        process_all(&mut stage, &mut silence, frame);

        // Interrupt with a tone — should reset the counter to zero.
        let mut tone_buf = audible_tone(frame);
        stage.process(&mut tone_buf, 1, 44100);

        // Following silence of only 10 ms (441 frames) should NOT be zeroed:
        // consecutive_silent just reset and 441 < 8820 (min).
        let short = near_silence(441); // 10 ms << 200 ms min
        let original = short.clone();
        let mut buf = short;
        stage.process(&mut buf, 1, 44100);
        assert_eq!(buf, original, "silence counter should reset after tone");
    }

    #[test]
    fn disabled_passes_all() {
        let mut stage = SkipSilence::new(cfg(false, -50.0, 10.0, 3000.0));
        let near = near_silence(1024);
        let mut buf = near.clone();
        stage.process(&mut buf, 1, 44100);
        assert_eq!(buf, near);
    }
}
