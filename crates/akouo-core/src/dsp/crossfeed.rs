use std::f64::consts::PI;

use crate::config::CrossfeedConfig;
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{QualityTier, SignalStageInfo, StageParams};

/// bs2b preset parameters derived FROM `strength` via linear interpolation.
///
/// strength 0.0 → Easy  (700 Hz, −4.5 dB)
/// strength 0.5 → Normal (650 Hz, −6.0 dB)  -  Bauer default
/// strength 1.0 → Extreme (500 Hz, −9.0 dB)
fn preset_params(strength: f64) -> (f64, f64) {
    let s = strength.clamp(0.0, 1.0);
    let (cutoff, level_db) = if s <= 0.5 {
        let t = s * 2.0;
        (700.0 + t * (650.0 - 700.0), -4.5 + t * (-6.0 - -4.5))
    } else {
        let t = (s - 0.5) * 2.0;
        (650.0 + t * (500.0 - 650.0), -6.0 + t * (-9.0 - -6.0))
    };
    (cutoff, level_db)
}

/// Normalized 2nd-ORDER Butterworth low-pass biquad coefficients.
fn butterworth_lp(cutoff_hz: f64, sample_rate: u32) -> [f64; 5] {
    let q = 1.0 / 2f64.sqrt(); // Butterworth Q
    let w0 = 2.0 * PI * cutoff_hz / f64::from(sample_rate);
    let cos_w0 = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let b0 = (1.0 - cos_w0) / 2.0;
    let b1 = 1.0 - cos_w0;
    let b2 = (1.0 - cos_w0) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

pub(crate) struct Crossfeed {
    config: CrossfeedConfig,
    /// Direct Form II Transposed state [z1, z2] for each channel's LP filter.
    /// Index: [channel][delay element]  (0 = L→cross-INTO-R, 1 = R→cross-INTO-L)
    lp_state: [[f64; 2]; 2],
    /// 1-sample delay of the LP-filtered signal per channel, modeling ~23 µs ITD.
    lp_delayed: [f64; 2],
    /// Cached filter coefficients: [b0, b1, b2, a1, a2].
    lp_coeffs: [f64; 5],
    cross_gain: f64,
    last_sample_rate: u32,
}

impl Crossfeed {
    pub(crate) fn new(config: CrossfeedConfig) -> Self {
        Self {
            config,
            lp_state: [[0.0; 2]; 2],
            lp_delayed: [0.0; 2],
            lp_coeffs: [1.0, 0.0, 0.0, 0.0, 0.0], // passthrough until first process()
            cross_gain: 0.0,
            last_sample_rate: 0,
        }
    }

    fn recompute(&mut self, sample_rate: u32) {
        self.last_sample_rate = sample_rate;
        let (cutoff, level_db) = preset_params(self.config.strength);
        self.lp_coeffs = butterworth_lp(cutoff, sample_rate);
        self.cross_gain = 10f64.powf(level_db / 20.0);
        // Reset filter and delay state when parameters change.
        self.lp_state = [[0.0; 2]; 2];
        self.lp_delayed = [0.0; 2];
    }

    #[inline]
    fn lp_sample(&mut self, x: f64, ch: usize) -> f64 {
        let [b0, b1, b2, a1, a2] = self.lp_coeffs;
        let [z1, z2] = self.lp_state[ch];
        let y = b0 * x + z1;
        self.lp_state[ch] = [b1 * x - a1 * y + z2, b2 * x - a2 * y];
        y
    }
}

impl DspStage for Crossfeed {
    fn name(&self) -> &str {
        "crossfeed"
    }

    fn process(&mut self, samples: &mut [f64], channels: u16, sample_rate: u32) -> StageResult {
        if !self.config.enabled || channels != 2 {
            return StageResult {
                meta: self.signal_stage_meta(),
            };
        }

        if self.last_sample_rate != sample_rate {
            self.recompute(sample_rate);
        }

        for frame in samples.chunks_mut(2) {
            let l = frame.first().copied().unwrap_or_default();
            let r = frame.get(1).copied().unwrap_or_default();

            // LP-filter each channel (models the spectral shaping of head diffraction).
            let lp_l = self.lp_sample(l, 0);
            let lp_r = self.lp_sample(r, 1);

            // Mix: direct signal + 1-sample-delayed LP-filtered opposite channel.
            // The 1-sample delay (~22 µs at 44.1 kHz) models the interaural time difference.
            frame[0] = l + self.cross_gain * self.lp_delayed[1];
            frame[1] = r + self.cross_gain * self.lp_delayed[0];

            self.lp_delayed = [lp_l, lp_r];
        }

        StageResult {
            meta: self.signal_stage_meta(),
        }
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::Crossfeed {
                strength: self.config.strength,
            },
            // Crossfeed modifies the stereo image; no longer bit-perfect when active.
            tier_impact: self.config.enabled.then_some(QualityTier::Lossless),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(strength: f64) -> Crossfeed {
        Crossfeed::new(CrossfeedConfig {
            enabled: true,
            strength,
        })
    }

    fn measure_cross(strength: f64) -> f64 {
        let mut stage = make(strength);
        // Hard-panned LEFT signal: [1.0, 0.0] repeated
        let frames = 4410usize; // 100 ms at 44.1 kHz
        let mut samples: Vec<f64> = (0..frames).flat_map(|_| [0.5_f64, 0.0_f64]).collect();
        stage.process(&mut samples, 2, 44100);

        // Measure energy in the RIGHT channel after settling (last 50 ms)
        let skip = frames / 2 * 2;
        let right_energy: f64 = samples[skip..].chunks(2).map(|f| f[1] * f[1]).sum();
        right_energy.sqrt()
    }

    #[test]
    fn mono_signal_unchanged() {
        // L == R → crossfeed is identity in the sense that the mix is symmetric
        let mut stage = make(0.5);
        let signal = 0.5_f64;
        let original: Vec<f64> = (0..100).flat_map(|_| [signal, signal]).collect();
        let mut buf = original.clone();
        stage.process(&mut buf, 2, 44100);

        // After crossfeed, L and R should remain equal (symmetric signal stays symmetric).
        for frame in buf.chunks(2) {
            assert!(
                (frame.first().copied().unwrap_or_default()
                    - frame.get(1).copied().unwrap_or_default())
                .abs()
                    < 1e-9,
                "L and R should remain equal for mono signal"
            );
        }
    }

    #[test]
    fn hard_panned_left_appears_in_right() {
        let cross = measure_cross(0.5);
        assert!(
            cross > 0.0,
            "crossfeed signal should appear in opposite channel"
        );
    }

    #[test]
    fn three_presets_produce_different_levels() {
        let s_easy = measure_cross(0.0); // Easy:    cutoff 700 Hz, level −4.5 dB
        let s_normal = measure_cross(0.5); // Normal:  cutoff 650 Hz, level −6.0 dB
        let s_extreme = measure_cross(1.0); // Extreme: cutoff 500 Hz, level −9.0 dB

        // −4.5 dB > −6 dB > −9 dB in linear gain, so Easy mixes the most raw energy.
        // The important property is that all three are distinct.
        assert!(
            s_easy > s_normal && s_normal > s_extreme,
            "each preset should produce a distinct crossfeed level: \
             easy={s_easy:.4} normal={s_normal:.4} extreme={s_extreme:.4}"
        );
    }

    #[test]
    fn disabled_passes_through_unchanged() {
        let mut stage = Crossfeed::new(CrossfeedConfig {
            enabled: false,
            strength: 1.0,
        });
        let input: Vec<f64> = (0..200)
            .flat_map(|i| [f64::from(i) * 0.01, -(f64::from(i) * 0.01)])
            .collect();
        let mut buf = input.clone();
        stage.process(&mut buf, 2, 44100);
        assert_eq!(buf, input);
    }

    #[test]
    fn mono_channel_count_passes_through() {
        let mut stage = make(0.5);
        let input: Vec<f64> = (0..100).map(|i| f64::from(i) * 0.01).collect();
        let mut buf = input.clone();
        stage.process(&mut buf, 1, 44100);
        assert_eq!(buf, input, "mono (1 channel) should be a passthrough");
    }
}
