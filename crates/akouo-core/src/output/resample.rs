use rubato::audioadapter::{Adapter, AdapterMut};
use rubato::{
    Async, FixedAsync, Resampler as _, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

use crate::error::OutputError;

// ---------------------------------------------------------------------------
// Lightweight audioadapter wrappers for interleaved f64 slices
// ---------------------------------------------------------------------------

struct InterleavedIn<'a> {
    data: &'a [f64],
    channels: usize,
    frames: usize,
}

unsafe impl<'a> Adapter<'a, f64> for InterleavedIn<'a> {
    unsafe fn read_sample_unchecked(&self, channel: usize, frame: usize) -> f64 {
        // SAFETY: caller guarantees channel < channels and frame < frames
        unsafe { *self.data.get_unchecked(frame * self.channels + channel) }
    }
    fn channels(&self) -> usize {
        self.channels
    }
    fn frames(&self) -> usize {
        self.frames
    }
}

struct InterleavedOut<'a> {
    data: &'a mut [f64],
    channels: usize,
    frames: usize,
}

unsafe impl<'a> Adapter<'a, f64> for InterleavedOut<'a> {
    unsafe fn read_sample_unchecked(&self, channel: usize, frame: usize) -> f64 {
        // SAFETY: caller guarantees channel < channels and frame < frames
        unsafe { *self.data.get_unchecked(frame * self.channels + channel) }
    }
    fn channels(&self) -> usize {
        self.channels
    }
    fn frames(&self) -> usize {
        self.frames
    }
}

unsafe impl<'a> AdapterMut<'a, f64> for InterleavedOut<'a> {
    unsafe fn write_sample_unchecked(&mut self, channel: usize, frame: usize, value: &f64) -> bool {
        // SAFETY: caller guarantees channel < channels and frame < frames
        unsafe { *self.data.get_unchecked_mut(frame * self.channels + channel) = *value };
        false
    }
}

// ---------------------------------------------------------------------------
// Resampler
// ---------------------------------------------------------------------------

/// Sinc resampler for converting interleaved f64 audio between sample rates.
///
/// Wraps rubato's `Async` sinc resampler with pre-allocated interleaved buffers so
/// that `process_interleaved` is allocation-free after construction.
pub struct Resampler {
    inner: Async<f64>,
    channels: usize,
    /// Pre-allocated interleaved output buffer; capacity = `output_frames_max * channels`.
    output_buf: Vec<f64>,
}

impl Resampler {
    /// Creates a sinc resampler converting FROM `source_rate` to `target_rate`.
    ///
    /// `chunk_frames` is the fixed number of input frames per call to
    /// `process_interleaved`. The output buffer is pre-allocated to the worst-case
    /// output size.
    pub fn new(
        source_rate: u32,
        target_rate: u32,
        channels: usize,
        chunk_frames: usize,
    ) -> Result<Self, OutputError> {
        let ratio = f64::from(target_rate) / f64::from(source_rate);

        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let inner = Async::<f64>::new_sinc(
            ratio,
            2.0,
            &params,
            chunk_frames,
            channels,
            FixedAsync::Input,
        )
        .map_err(|e| OutputError::FormatUnsupported {
            message: format!("resampler init failed: {e}"),
        })?;

        let max_output = inner.output_frames_max();
        let output_buf = vec![0.0f64; max_output * channels];

        Ok(Self {
            inner,
            channels,
            output_buf,
        })
    }

    /// Number of input frames expected by the next `process_interleaved` call.
    pub fn input_frames_next(&self) -> usize {
        self.inner.input_frames_next()
    }

    /// Maximum number of output frames the next `process_interleaved` call may produce.
    pub fn output_frames_max(&self) -> usize {
        self.inner.output_frames_max()
    }

    /// Resamples `input` (interleaved, `input_frames_next() * channels` samples) and
    /// writes resampled interleaved audio INTO `output`.
    ///
    /// Returns the number of output frames written. `output` must have capacity for at
    /// least `output_frames_max() * channels` samples.
    ///
    /// No allocation occurs after construction.
    pub fn process_interleaved(
        &mut self,
        input: &[f64],
        output: &mut [f64],
    ) -> Result<usize, OutputError> {
        let in_frames = input.len() / self.channels;
        let out_capacity = output.len() / self.channels;
        let max_out = self.inner.output_frames_max();

        if out_capacity < max_out {
            return Err(OutputError::StreamError {
                message: format!(
                    "output buffer too small: need {max_out} frames ({} samples), got {out_capacity} frames",
                    max_out * self.channels
                ),
            });
        }

        let buf_in = InterleavedIn {
            data: input,
            channels: self.channels,
            frames: in_frames,
        };
        let mut buf_out = InterleavedOut {
            data: &mut self.output_buf,
            channels: self.channels,
            frames: max_out,
        };

        let (_, out_frames) = self
            .inner
            .process_into_buffer(&buf_in, &mut buf_out, None)
            .map_err(|e| OutputError::StreamError {
                message: format!("resample failed: {e}"),
            })?;

        // Copy resampled data FROM staging buffer INTO the caller's output
        let out_samples = out_frames * self.channels;
        let output_slice =
            output
                .get_mut(..out_samples)
                .ok_or_else(|| OutputError::StreamError {
                    message: format!("output buffer too small for {out_samples} samples"),
                })?;
        let staged =
            self.output_buf
                .get(..out_samples)
                .ok_or_else(|| OutputError::StreamError {
                    message: format!(
                        "resampler staging buffer too small for {out_samples} samples"
                    ),
                })?;
        output_slice.copy_from_slice(staged);

        Ok(out_frames)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use rustfft::FftPlanner;
    use rustfft::num_complex::Complex;

    use super::*;

    #[test]
    fn resampler_new_same_rate() {
        let r = Resampler::new(44100, 44100, 2, 512);
        assert!(r.is_ok());
    }

    #[test]
    fn resampler_new_upsample() {
        let r = Resampler::new(44100, 48000, 2, 441);
        assert!(r.is_ok());
    }

    #[test]
    fn resampler_new_downsample() {
        let r = Resampler::new(96000, 48000, 2, 1024);
        assert!(r.is_ok());
    }

    #[test]
    fn resampler_produces_output_frames() {
        let chunk = 441;
        let channels = 2;
        let mut r = Resampler::new(44100, 48000, channels, chunk).unwrap();

        let input = vec![0.0f64; r.input_frames_next() * channels];
        let max_out = r.output_frames_max();
        let mut output = vec![0.0f64; max_out * channels];

        let out_frames = r.process_interleaved(&input, &mut output).unwrap();
        // 44100 -> 48000 at 441 input frames ~ 480 output frames
        assert!(out_frames > 0);
        assert!(out_frames <= max_out);
    }

    #[test]
    fn resampler_output_too_small_returns_error() {
        let chunk = 441;
        let channels = 2;
        let mut r = Resampler::new(44100, 48000, channels, chunk).unwrap();
        let input = vec![0.0f64; r.input_frames_next() * channels];
        // Deliberately undersized output
        let mut output = vec![0.0f64; 2];
        assert!(r.process_interleaved(&input, &mut output).is_err());
    }

    #[test]
    #[ignore = "should_run_ci = false: spectral threshold is calibration-only until CI baseline is established; see #299"]
    fn resampler_preserves_frequency_content() {
        let source_rate = 44_100_u32;
        let target_rate = 48_000_u32;
        let tone_hz = 1_000.0;
        let channels = 1;
        let chunk_frames = 2_048;
        let source_frames = source_rate as usize;
        let fft_len = 16_384;

        let mut resampler =
            Resampler::new(source_rate, target_rate, channels, chunk_frames).unwrap();
        let mut resampled = Vec::new();
        let mut source_frame = 0_usize;

        while source_frame < source_frames {
            let input_frames = resampler.input_frames_next();
            let mut input = vec![0.0; input_frames * channels];
            for (frame, sample) in input.iter_mut().enumerate().take(input_frames) {
                let phase = TAU * tone_hz * (source_frame + frame) as f64 / f64::from(source_rate);
                *sample = phase.sin();
            }
            source_frame += input_frames;

            let mut output = vec![0.0; resampler.output_frames_max() * channels];
            let out_frames = resampler.process_interleaved(&input, &mut output).unwrap();
            resampled.extend_from_slice(&output[..out_frames * channels]);
        }

        assert!(
            resampled.len() >= fft_len * 2,
            "resampled output should contain enough steady-state audio for spectral analysis"
        );

        let start = (resampled.len() / 2).saturating_sub(fft_len / 2);
        let spectrum = power_spectrum(&resampled[start..start + fft_len]);
        let nyquist_bin = spectrum.len() / 2;
        let target_bin = frequency_bin(tone_hz, target_rate, fft_len);
        let peak_bin = peak_bin(&spectrum[..nyquist_bin], 1);
        let peak_hz = bin_frequency(peak_bin, target_rate, fft_len);

        assert!(
            (peak_hz - tone_hz).abs() <= 12.0,
            "resampled peak {peak_hz:.2} Hz should remain near {tone_hz:.2} Hz"
        );

        let signal_power = band_power(&spectrum, target_bin, 3);
        let alias_power = spectrum[..nyquist_bin]
            .iter()
            .enumerate()
            .filter(|(bin, _)| target_bin.abs_diff(*bin) > 8)
            .map(|(_, power)| *power)
            .fold(0.0, f64::max);

        assert!(
            alias_power / signal_power < 0.005,
            "strongest non-target spectral component should stay below -23 dB of target power"
        );
    }

    fn power_spectrum(samples: &[f64]) -> Vec<f64> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(samples.len());
        let len_minus_one = samples.len().saturating_sub(1) as f64;
        let mut buffer: Vec<_> = samples
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                let window = 0.5 - 0.5 * (TAU * index as f64 / len_minus_one).cos();
                Complex::new(sample * window, 0.0)
            })
            .collect();
        fft.process(&mut buffer);
        buffer.iter().map(Complex::norm_sqr).collect()
    }

    fn frequency_bin(frequency: f64, sample_rate: u32, fft_len: usize) -> usize {
        ((frequency / f64::from(sample_rate)) * fft_len as f64).round() as usize
    }

    fn bin_frequency(bin: usize, sample_rate: u32, fft_len: usize) -> f64 {
        bin as f64 * f64::from(sample_rate) / fft_len as f64
    }

    fn peak_bin(spectrum: &[f64], start_bin: usize) -> usize {
        spectrum
            .iter()
            .enumerate()
            .skip(start_bin)
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(bin, _)| bin)
            .unwrap()
    }

    fn band_power(spectrum: &[f64], center: usize, radius: usize) -> f64 {
        let start = center.saturating_sub(radius);
        let end = (center + radius + 1).min(spectrum.len());
        spectrum[start..end].iter().sum()
    }
}
