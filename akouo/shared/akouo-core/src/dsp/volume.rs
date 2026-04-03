// Stage 7: Volume control + TPDF dither.

use rand::SeedableRng;
use rand::rngs::SmallRng;

use crate::config::VolumeConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

fn db_to_linear(db: f64) -> f64 {
    if db <= -200.0 {
        return 0.0;
    }
    10.0_f64.powf(db / 20.0)
}

/// Maps a UI slider value (0–100) to a linear gain.
///
/// Slider 0 → 0.0 (mute), slider 50 → ~0.1 (−20 dB), slider 100 → 1.0 (0 dB).
/// Uses a 60 dB dynamic range.
pub fn slider_to_gain(slider: f64) -> f64 {
    // 40 dB range: slider 50 → −20 dB (0.1 linear), slider 100 → 0 dB (1.0 linear).
    const RANGE_DB: f64 = 40.0;
    if slider <= 0.0 {
        return 0.0;
    }
    let db = (slider.min(100.0) / 100.0) * RANGE_DB - RANGE_DB;
    db_to_linear(db)
}

pub struct Volume {
    config: VolumeConfig,
    rng: SmallRng,
}

impl Volume {
    pub fn new(config: VolumeConfig) -> Self {
        Self {
            config,
            rng: SmallRng::from_os_rng(),
        }
    }

    /// Constructs a deterministic instance for testing.
    pub fn new_seeded(config: VolumeConfig, seed: u64) -> Self {
        Self {
            config,
            rng: SmallRng::seed_from_u64(seed),
        }
    }

    fn tpdf_noise(&mut self, amplitude: f64) -> f64 {
        use rand::Rng;
        let u1: f64 = self.rng.random();
        let u2: f64 = self.rng.random();
        // Two uniform [0, 1) VALUES → triangular distribution in [-amplitude, amplitude].
        (u1 - u2) * amplitude
    }

    /// Applies TPDF dither (if enabled) and quantizes to i16.
    pub fn dither_and_quantize_i16(&mut self, sample: f64) -> i16 {
        const AMPLITUDE: f64 = 1.0 / 32_768.0;
        let s = if self.config.dither {
            sample + self.tpdf_noise(AMPLITUDE)
        } else {
            sample
        };
        quantize_i16(s)
    }

    /// Applies TPDF dither (if enabled) and quantizes to i24 (returned as i32).
    pub fn dither_and_quantize_i24(&mut self, sample: f64) -> i32 {
        const AMPLITUDE: f64 = 1.0 / 8_388_608.0;
        let s = if self.config.dither {
            sample + self.tpdf_noise(AMPLITUDE)
        } else {
            sample
        };
        quantize_i24(s)
    }

    /// Applies TPDF dither (if enabled) and quantizes to i32.
    pub fn dither_and_quantize_i32(&mut self, sample: f64) -> i32 {
        const AMPLITUDE: f64 = 1.0 / 2_147_483_648.0;
        let s = if self.config.dither {
            sample + self.tpdf_noise(AMPLITUDE)
        } else {
            sample
        };
        quantize_i32(s)
    }

    /// Converts to f32  -  no dither required.
    pub fn dither_and_quantize_f32(&self, sample: f64) -> f32 {
        quantize_f32(sample)
    }
}

impl DspStage for Volume {
    fn name(&self) -> &str {
        "volume"
    }

    fn process(&mut self, samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        let gain = db_to_linear(self.config.level_db);
        for s in samples.iter_mut() {
            *s *= gain;
        }
        StageResult {
            meta: self.signal_stage_meta(),
        }
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

/// Clamps and scales a f64 sample to i16 range.
pub fn quantize_i16(sample: f64) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * 32_767.0).round() as i16
}

/// Clamps and scales a f64 sample to i24 range (returned as i32).
pub fn quantize_i24(sample: f64) -> i32 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * 8_388_607.0).round() as i32
}

/// Clamps and scales a f64 sample to i32 range.
pub fn quantize_i32(sample: f64) -> i32 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * 2_147_483_647.0).round() as i32
}

/// Converts a f64 sample to f32  -  no quantization noise for float output.
pub fn quantize_f32(sample: f64) -> f32 {
    f32::try_from(sample).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VolumeConfig;

    fn vol(level_db: f64, dither: bool) -> Volume {
        Volume::new_seeded(VolumeConfig { level_db, dither }, 0xdeadbeef)
    }

    // ── process() ─────────────────────────────────────────────────────────────

    #[test]
    fn unity_gain_passes_through() {
        let mut v = vol(0.0, false);
        let mut samples = [0.5_f64, -0.3, 0.9];
        let original = samples;
        v.process(&mut samples, 1, 44100);
        for (a, b) in samples.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-12, "unity gain must not ALTER samples");
        }
    }

    #[test]
    fn zero_gain_produces_silence() {
        let mut v = vol(-200.0, false);
        let mut samples = [0.5_f64, -0.9, 1.0];
        v.process(&mut samples, 1, 44100);
        for s in samples {
            assert_eq!(s, 0.0, "muted volume must produce silence");
        }
    }

    #[test]
    fn half_amplitude() {
        let mut v = vol(-6.0206, false); // ≈ -6 dB → linear 0.5
        let mut samples = [1.0_f64, 0.5, -0.8];
        let original = samples;
        v.process(&mut samples, 1, 44100);
        for (out, &inp) in samples.iter().zip(original.iter()) {
            let ratio = out / inp;
            assert!(
                (ratio - 0.5).abs() < 0.001,
                "expected ~0.5 gain, got {ratio}"
            );
        }
    }

    // ── slider_to_gain ────────────────────────────────────────────────────────

    #[test]
    fn slider_zero_is_silence() {
        assert_eq!(slider_to_gain(0.0), 0.0);
    }

    #[test]
    fn slider_100_is_unity() {
        let gain = slider_to_gain(100.0);
        assert!(
            (gain - 1.0).abs() < 1e-10,
            "slider 100 should be unity gain, got {gain}"
        );
    }

    #[test]
    fn slider_50_is_minus_20db() {
        let gain = slider_to_gain(50.0);
        let db = 20.0 * gain.log10();
        assert!(
            (db - (-20.0)).abs() < 0.01,
            "slider 50 should be ~-20 dB, got {db:.2} dB"
        );
    }

    // ── quantize_* free functions ─────────────────────────────────────────────

    #[test]
    fn quantize_i16_full_scale() {
        assert_eq!(quantize_i16(1.0), 32767);
        assert_eq!(quantize_i16(-1.0), -32767);
    }

    #[test]
    fn quantize_i16_clamps_overflow() {
        assert_eq!(quantize_i16(1.5), 32767);
        assert_eq!(quantize_i16(-1.5), -32767);
    }

    #[test]
    fn quantize_i24_full_scale() {
        assert_eq!(quantize_i24(1.0), 8_388_607);
        assert_eq!(quantize_i24(-1.0), -8_388_607);
    }

    #[test]
    fn quantize_i32_full_scale() {
        assert_eq!(quantize_i32(1.0), 2_147_483_647);
        assert_eq!(quantize_i32(-1.0), -2_147_483_647);
    }

    #[test]
    fn quantize_f32_round_trips() {
        let x = 0.12345_f64;
        let out = quantize_f32(x);
        let diff = (f64::try_from(out).unwrap_or_default() - x).abs();
        assert!(
            diff < 1e-7,
            "f32 round-trip error {diff} exceeds f32 precision"
        );
    }

    // ── TPDF dither ───────────────────────────────────────────────────────────

    #[test]
    fn dither_i16_deterministic_with_seed() {
        let cfg = VolumeConfig {
            level_db: 0.0,
            dither: true,
        };
        let mut v1 = Volume::new_seeded(cfg.clone(), 42);
        let mut v2 = Volume::new_seeded(cfg, 42);

        let sample = 0.0_f64;
        assert_eq!(
            v1.dither_and_quantize_i16(sample),
            v2.dither_and_quantize_i16(sample),
            "same seed must produce same output"
        );
    }

    #[test]
    fn dither_tpdf_distribution_is_triangular() {
        // Collect raw dither VALUES by subtracting the un-dithered output.
        // The dither amplitude for i16 is 1/32768; collect enough samples
        // to verify the distribution is triangular (mean ~0, ends taper).
        const N: usize = 50_000;
        const AMPLITUDE: f64 = 1.0 / 32_768.0;

        let cfg = VolumeConfig {
            level_db: 0.0,
            dither: true,
        };
        let mut v = Volume::new_seeded(cfg, 12345);

        // Fixed input → variation in output is purely dither noise.
        let input = 0.0_f64;
        let mut VALUES: Vec<f64> = Vec::with_capacity(N);
        for _ in 0..N {
            let dithered = v.dither_and_quantize_i16(input) as f64 / 32_767.0;
            VALUES.push(dithered);
        }

        // TPDF: mean ≈ 0, all VALUES within ±1 LSB.
        let mean = VALUES.iter().sum::<f64>() / f64::try_from(N).unwrap_or_default();
        assert!(
            mean.abs() < AMPLITUDE * 5.0,
            "TPDF mean should be ~0, got {mean}"
        );

        let max_abs = VALUES.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
        assert!(
            max_abs <= AMPLITUDE * 1.5,
            "TPDF VALUES should stay within ±1 LSB, max={max_abs}"
        );
    }

    #[test]
    fn f32_output_is_deterministic() {
        // f32 quantization requires no RNG  -  same input always yields same output.
        let cfg = VolumeConfig {
            level_db: 0.0,
            dither: true,
        };
        let v = Volume::new_seeded(cfg, 0);
        let out1 = v.dither_and_quantize_f32(0.5);
        let out2 = v.dither_and_quantize_f32(0.5);
        assert_eq!(out1, out2, "f32 output must be deterministic (no dither)");
    }

    #[test]
    fn no_dither_quantize_is_deterministic() {
        let cfg = VolumeConfig {
            level_db: 0.0,
            dither: false,
        };
        let mut v = Volume::new_seeded(cfg, 99);
        let a = v.dither_and_quantize_i16(0.123);
        let b = v.dither_and_quantize_i16(0.123);
        assert_eq!(a, b, "without dither, same input must give same output");
    }
}
