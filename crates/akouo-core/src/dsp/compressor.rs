// Stage 5: Dynamics compressor / limiter.

use crate::config::CompressorConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{QualityTier, SignalStageInfo, StageParams};

fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

/// One-pole RC coefficient FROM a time constant in milliseconds.
fn time_coeff(time_ms: f64, sample_rate: u32) -> f64 {
    if time_ms <= 0.0 {
        return 0.0;
    }
    (-1.0_f64 / (time_ms * f64::try_from(sample_rate).unwrap_or_default() / 1000.0)).exp()
}

pub struct Compressor {
    config: CompressorConfig,
    /// Current smoothed gain reduction in dB (always >= 0).
    gain_reduction_db: f64,
    attack_coeff: f64,
    release_coeff: f64,
    last_sample_rate: u32,
}

impl Compressor {
    pub fn new(config: CompressorConfig) -> Self {
        Self {
            config,
            gain_reduction_db: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            last_sample_rate: 0,
        }
    }

    fn update_coeffs(&mut self, sample_rate: u32) {
        if sample_rate != self.last_sample_rate {
            self.attack_coeff = time_coeff(self.config.attack_ms, sample_rate);
            self.release_coeff = time_coeff(self.config.release_ms, sample_rate);
            self.last_sample_rate = sample_rate;
        }
    }
}

impl DspStage for Compressor {
    fn name(&self) -> &str {
        "compressor"
    }

    fn process(&mut self, samples: &mut [f64], channels: u16, sample_rate: u32) -> StageResult {
        if !self.config.enabled || channels == 0 {
            return StageResult {
                meta: self.signal_stage_meta(),
            };
        }

        self.update_coeffs(sample_rate);

        let ch = usize::try_from(channels).unwrap_or_default();
        let limiter_ceiling = db_to_linear(self.config.limiter_ceiling_db);

        for frame in samples.chunks_mut(ch) {
            // Peak detection across all channels preserves stereo image.
            let peak = frame.iter().map(|s| s.abs()).fold(0.0_f64, f64::max);

            let peak_db = if peak > 1e-10 {
                20.0 * peak.log10()
            } else {
                -200.0
            };

            // Target gain reduction (dB, always >= 0).
            let target_gr = if peak_db > self.config.threshold_db {
                (peak_db - self.config.threshold_db) * (1.0 - 1.0 / self.config.ratio)
            } else {
                0.0
            };

            // Attack when gain reduction is increasing; release when decreasing.
            let coeff = if target_gr > self.gain_reduction_db {
                self.attack_coeff
            } else {
                self.release_coeff
            };
            self.gain_reduction_db = coeff * self.gain_reduction_db + (1.0 - coeff) * target_gr;

            let gain = db_to_linear(-self.gain_reduction_db);

            for s in frame.iter_mut() {
                *s *= gain;
                // Brick-wall limiter ceiling.
                *s = s.clamp(-limiter_ceiling, limiter_ceiling);
            }
        }

        StageResult {
            meta: self.signal_stage_meta(),
        }
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::Compressor {
                threshold_db: self.config.threshold_db,
                ratio: self.config.ratio,
            },
            // Compression reduces dynamic range  -  lowers tier FROM BitPerfect to Lossless.
            tier_impact: self.config.enabled.then_some(QualityTier::Lossless),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CompressorConfig;

    fn config_enabled(
        threshold_db: f64,
        ratio: f64,
        attack_ms: f64,
        release_ms: f64,
    ) -> CompressorConfig {
        CompressorConfig {
            enabled: true,
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            limiter_ceiling_db: 0.0,
        }
    }

    fn run_mono(comp: &mut Compressor, amplitude: f64, n_samples: usize, sr: u32) -> Vec<f64> {
        let mut out = Vec::with_capacity(n_samples);
        for _ in 0..n_samples {
            let mut frame = [amplitude];
            comp.process(&mut frame, 1, sr);
            out.push(frame.get(0).copied().unwrap_or_default());
        }
        out
    }

    #[test]
    fn below_threshold_passes_through() {
        let cfg = config_enabled(-20.0, 4.0, 1.0, 10.0);
        let mut comp = Compressor::new(cfg);

        // Signal at -40 dBFS, well below -20 dB threshold.
        let amplitude = db_to_linear(-40.0);
        let out = run_mono(&mut comp, amplitude, 200, 44100);

        for s in &out {
            let diff = (s.abs() - amplitude).abs();
            assert!(diff < 1e-6, "expected passthrough, got diff {diff}");
        }
    }

    #[test]
    fn above_threshold_gain_reduced() {
        // 0 dBFS input, threshold -6 dB, ratio 4:1. After settling:
        // excess = 6 dB, gain reduction = 6 * (1 - 1/4) = 4.5 dB.
        let cfg = config_enabled(-6.0, 4.0, 1.0, 1000.0);
        let mut comp = Compressor::new(cfg);

        let amplitude = 1.0_f64; // 0 dBFS
        // Run enough samples to fully settle (>> attack time).
        let out = run_mono(&mut comp, amplitude, 5000, 44100);

        let final_level_db = 20.0 * out.last().copied().unwrap_or_default().abs().log10();
        let expected_db = -4.5_f64;
        assert!(
            (final_level_db - expected_db).abs() < 0.1,
            "expected ~{expected_db:.1} dB, got {final_level_db:.2} dB"
        );
    }

    #[test]
    fn attack_ramps_over_attack_time() {
        // After one time constant the gain reduction should be ~63.2% of target.
        let attack_ms = 10.0;
        let sr = 44100_u32;
        let cfg = config_enabled(-6.0, 100.0, attack_ms, 1000.0);
        let mut comp = Compressor::new(cfg);

        // Target gain reduction for 0 dBFS → threshold -6 dB, ratio 100 ≈ ∞:
        // gr_target ≈ (0 - (-6)) * (1 - 1/100) ≈ 5.94 dB.
        let target_gr = 6.0 * (1.0 - 1.0 / 100.0);
        let n_tc = (attack_ms * sr as f64 / 1000.0).round() as usize;

        let amplitude = 1.0_f64;
        let out = run_mono(&mut comp, amplitude, n_tc, sr);

        // Output amplitude after one time constant; gain reduction ≈ 63.2% of target.
        let actual_output = out.last().copied().unwrap_or_default().abs();
        let actual_gr = -20.0 * actual_output.log10();
        let expected_gr_at_tc = target_gr * (1.0 - (-1.0_f64).exp()); // = target * 0.632

        let ratio = actual_gr / expected_gr_at_tc;
        assert!(
            (ratio - 1.0).abs() < 0.05,
            "gain reduction after 1 TC should be ~63.2% of target; ratio={ratio:.3}"
        );
    }

    #[test]
    fn limiter_ceiling_clamps_output() {
        let cfg = CompressorConfig {
            enabled: true,
            threshold_db: -60.0, // very low threshold
            ratio: 1.0,          // no compression
            attack_ms: 0.0,
            release_ms: 0.0,
            limiter_ceiling_db: -6.0, // ceiling at ~0.5 linear
        };
        let mut comp = Compressor::new(cfg);
        let ceiling = db_to_linear(-6.0);

        let mut frame = [1.0_f64, -1.0]; // stereo, full scale
        comp.process(&mut frame, 2, 44100);

        assert!(
            frame.get(0).copied().unwrap_or_default() <= ceiling + 1e-10,
            "positive sample not clamped"
        );
        assert!(
            frame.get(1).copied().unwrap_or_default() >= -ceiling - 1e-10,
            "negative sample not clamped"
        );
    }

    #[test]
    fn disabled_passes_through() {
        let cfg = CompressorConfig {
            enabled: false,
            ..CompressorConfig::default()
        };
        let mut comp = Compressor::new(cfg);
        let mut samples = [0.9_f64, -0.8, 0.7];
        let original = samples;
        comp.process(&mut samples, 1, 44100);
        assert_eq!(samples, original);
    }

    #[test]
    fn signal_stage_meta_tier_impact() {
        let mut cfg = CompressorConfig {
            enabled: true,
            ..CompressorConfig::default()
        };
        let comp = Compressor::new(cfg.clone());
        assert_eq!(
            comp.signal_stage_meta().tier_impact,
            Some(QualityTier::Lossless)
        );

        cfg.enabled = false;
        let comp = Compressor::new(cfg);
        assert_eq!(comp.signal_stage_meta().tier_impact, None);
    }
}
