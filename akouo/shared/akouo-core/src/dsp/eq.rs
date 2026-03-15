use std::f64::consts::PI;

use crate::config::{EqBand, EqConfig, FilterType};
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

/// Normalized biquad coefficients (b0, b1, b2 divided by a0; a1, a2 divided by a0).
/// Direct Form II Transposed: y = b0·x + z1; z1 = b1·x − a1·y + z2; z2 = b2·x − a2·y
#[derive(Clone, Copy)]
struct Coeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Coeffs {
    fn passthrough() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Compute RBJ Audio EQ Cookbook coefficients for the given band and sample rate.
    fn from_band(band: &EqBand, sample_rate: u32) -> Self {
        if band.q <= 0.0 || !band.frequency.is_finite() || !band.gain_db.is_finite() {
            return Self::passthrough();
        }

        let w0 = 2.0 * PI * band.frequency / sample_rate as f64;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * band.q);

        let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
            FilterType::Peaking => {
                let a = 10f64.powf(band.gain_db / 40.0);
                (
                    1.0 + alpha * a,
                    -2.0 * cos_w0,
                    1.0 - alpha * a,
                    1.0 + alpha / a,
                    -2.0 * cos_w0,
                    1.0 - alpha / a,
                )
            }
            FilterType::LowShelf => {
                let a = 10f64.powf(band.gain_db / 40.0);
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            FilterType::HighShelf => {
                let a = 10f64.powf(band.gain_db / 40.0);
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            FilterType::LowPass => (
                (1.0 - cos_w0) / 2.0,
                1.0 - cos_w0,
                (1.0 - cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::HighPass => (
                (1.0 + cos_w0) / 2.0,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::Notch => (
                1.0,
                -2.0 * cos_w0,
                1.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::AllPass => (
                1.0 - alpha,
                -2.0 * cos_w0,
                1.0 + alpha,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
        };

        if a0.abs() < f64::EPSILON {
            return Self::passthrough();
        }

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

/// Per-channel delay elements for one biquad (Direct Form II Transposed).
#[derive(Clone, Copy, Default)]
struct Chan {
    z1: f64,
    z2: f64,
}

struct BiquadBand {
    coeffs: Coeffs,
    chans: Vec<Chan>,
}

impl BiquadBand {
    fn new(band: &EqBand, num_channels: usize, sample_rate: u32) -> Self {
        Self {
            coeffs: Coeffs::from_band(band, sample_rate),
            chans: vec![Chan::default(); num_channels],
        }
    }

    fn ensure_channels(&mut self, n: usize) {
        if self.chans.len() < n {
            self.chans.resize(n, Chan::default());
        }
    }

    #[inline]
    fn process_sample(&mut self, x: f64, ch: usize) -> f64 {
        let c = &self.chans[ch];
        let y = self.coeffs.b0 * x + c.z1;
        let z1_next = self.coeffs.b1 * x - self.coeffs.a1 * y + c.z2;
        let z2_next = self.coeffs.b2 * x - self.coeffs.a2 * y;
        self.chans[ch] = Chan {
            z1: z1_next,
            z2: z2_next,
        };
        y
    }
}

pub struct ParametricEq {
    config: EqConfig,
    bands: Vec<BiquadBand>,
    last_sample_rate: u32,
}

impl ParametricEq {
    pub fn new(config: EqConfig) -> Self {
        Self {
            config,
            bands: Vec::new(),
            last_sample_rate: 0,
        }
    }

    fn rebuild(&mut self, channels: u16, sample_rate: u32) {
        self.last_sample_rate = sample_rate;
        self.bands = self
            .config
            .bands
            .iter()
            .map(|b| BiquadBand::new(b, channels as usize, sample_rate))
            .collect();
    }
}

impl DspStage for ParametricEq {
    fn name(&self) -> &str {
        "parametric-eq"
    }

    fn process(&mut self, samples: &mut [f64], channels: u16, sample_rate: u32) -> StageResult {
        if !self.config.enabled || self.config.bands.is_empty() {
            return StageResult {
                meta: self.signal_stage_meta(),
            };
        }

        if self.last_sample_rate != sample_rate || self.bands.len() != self.config.bands.len() {
            self.rebuild(channels, sample_rate);
        }

        let ch_count = channels.max(1) as usize;
        for band in &mut self.bands {
            band.ensure_channels(ch_count);
        }

        for (i, sample) in samples.iter_mut().enumerate() {
            let ch = i % ch_count;
            for band in &mut self.bands {
                *sample = band.process_sample(*sample, ch);
            }
        }

        StageResult {
            meta: self.signal_stage_meta(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EqBand;

    const SR: u32 = 44100;

    fn peaking(freq: f64, gain_db: f64, q: f64) -> EqBand {
        EqBand {
            frequency: freq,
            gain_db,
            q,
            filter_type: FilterType::Peaking,
        }
    }

    fn make_eq(band: EqBand) -> ParametricEq {
        ParametricEq::new(EqConfig {
            enabled: true,
            bands: vec![band],
        })
    }

    /// Measure steady-state gain (dB) at `freq_hz` by driving the filter with a sinusoid
    /// and measuring RMS of the last cycle after settling.
    fn measure_gain_db(eq: &mut ParametricEq, freq_hz: f64, input_amplitude: f64) -> f64 {
        let period = (SR as f64 / freq_hz).round() as usize;
        let warmup_cycles = 200usize;
        let measure_cycles = 4usize;
        let total = (warmup_cycles + measure_cycles) * period;

        let mut samples: Vec<f64> = (0..total)
            .map(|i| input_amplitude * (2.0 * PI * freq_hz * i as f64 / SR as f64).sin())
            .collect();

        // Process in frames of one period at a time.
        for chunk in samples.chunks_mut(period) {
            eq.process(chunk, 1, SR);
        }

        // Measure RMS over the last `measure_cycles` periods.
        let measure_start = warmup_cycles * period;
        let segment = &samples[measure_start..];
        let rms_out = (segment.iter().map(|s| s * s).sum::<f64>() / segment.len() as f64).sqrt();
        let rms_in = input_amplitude / 2f64.sqrt();
        20.0 * (rms_out / rms_in).log10()
    }

    #[test]
    fn bypass_when_disabled() {
        let mut eq = ParametricEq::new(EqConfig {
            enabled: false,
            bands: vec![peaking(1000.0, 12.0, 1.414)],
        });
        let input: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.01).sin()).collect();
        let mut buf = input.clone();
        eq.process(&mut buf, 1, SR);
        assert_eq!(buf, input);
    }

    #[test]
    fn bypass_when_all_gains_zero() {
        let mut eq = make_eq(peaking(1000.0, 0.0, 1.414));
        let input: Vec<f64> = (0..2048).map(|i| (i as f64 * 0.01).sin() * 0.5).collect();
        let mut buf = input.clone();
        eq.process(&mut buf, 1, SR);
        for (a, b) in buf.iter().zip(input.iter()) {
            assert!(
                (a - b).abs() < 1e-10,
                "zero-gain peaking should be transparent"
            );
        }
    }

    #[test]
    fn peaking_boost_at_center_frequency() {
        let gain_db = 6.0;
        let mut eq = make_eq(peaking(1000.0, gain_db, 1.414));
        let measured = measure_gain_db(&mut eq, 1000.0, 0.5);
        assert!(
            (measured - gain_db).abs() < 1.0,
            "peaking: measured {measured:.2} dB, expected ~{gain_db} dB at 1 kHz"
        );
    }

    #[test]
    fn peaking_unity_two_octaves_away() {
        let mut eq = make_eq(peaking(1000.0, 12.0, 1.414));
        for &freq in &[250.0_f64, 4000.0_f64] {
            let measured = measure_gain_db(&mut eq, freq, 0.5);
            assert!(
                measured.abs() < 3.0,
                "peaking at 1 kHz: {freq} Hz should be near unity, got {measured:.2} dB"
            );
        }
    }

    #[test]
    fn low_shelf_boosts_below_cutoff() {
        let mut eq = make_eq(EqBand {
            frequency: 200.0,
            gain_db: 9.0,
            q: 0.707,
            filter_type: FilterType::LowShelf,
        });
        let low = measure_gain_db(&mut eq, 50.0, 0.5);
        let high = measure_gain_db(&mut eq, 4000.0, 0.5);
        assert!(
            low > 5.0,
            "low shelf should boost low frequencies, got {low:.2} dB at 50 Hz"
        );
        assert!(
            high.abs() < 2.0,
            "low shelf should leave highs flat, got {high:.2} dB at 4 kHz"
        );
    }

    #[test]
    fn high_shelf_boosts_above_cutoff() {
        let mut eq = make_eq(EqBand {
            frequency: 4000.0,
            gain_db: 9.0,
            q: 0.707,
            filter_type: FilterType::HighShelf,
        });
        let low = measure_gain_db(&mut eq, 200.0, 0.5);
        let high = measure_gain_db(&mut eq, 16000.0, 0.5);
        assert!(
            low.abs() < 2.0,
            "high shelf should leave lows flat, got {low:.2} dB at 200 Hz"
        );
        assert!(
            high > 5.0,
            "high shelf should boost highs, got {high:.2} dB at 16 kHz"
        );
    }

    #[test]
    fn no_nan_or_inf_at_extreme_q() {
        for &q in &[0.1_f64, 100.0_f64] {
            let mut eq = make_eq(peaking(1000.0, 6.0, q));
            let mut buf: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.01).sin()).collect();
            eq.process(&mut buf, 1, SR);
            assert!(buf.iter().all(|s| s.is_finite()), "NaN/Inf at Q={q}");
        }
    }

    #[test]
    fn iso_10_band_default_all_zero_gain_is_transparent() {
        let cfg = EqConfig {
            enabled: true,
            ..EqConfig::iso_10_band_default()
        };
        let mut eq = ParametricEq::new(cfg);
        let input: Vec<f64> = (0..4096).map(|i| (i as f64 * 0.01).sin() * 0.5).collect();
        let mut buf = input.clone();
        eq.process(&mut buf, 1, SR);
        for (a, b) in buf.iter().zip(input.iter()) {
            assert!(
                (a - b).abs() < 1e-9,
                "10-band ISO default should be transparent at 0 dB gains"
            );
        }
    }

    #[test]
    fn stereo_independent_channels() {
        let mut eq = make_eq(peaking(1000.0, 6.0, 1.414));
        // Stereo: ch0 = sine, ch1 = silence. After EQ, ch1 should remain near-zero.
        let mut buf: Vec<f64> = (0..2048)
            .map(|i| {
                if i % 2 == 0 {
                    (i as f64 * 0.1).sin() * 0.5
                } else {
                    0.0
                }
            })
            .collect();
        eq.process(&mut buf, 2, SR);
        for i in (1..2048).step_by(2) {
            assert!(
                buf[i].abs() < 1e-10,
                "silent channel should stay silent, got {}",
                buf[i]
            );
        }
    }
}
