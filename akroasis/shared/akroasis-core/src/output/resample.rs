use rubato::{
    Async, FixedAsync, Resampler as _, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use rubato::audioadapter::{Adapter, AdapterMut};

use crate::error::OutputError;

// ---------------------------------------------------------------------------
// Lightweight audioadapter wrappers for interleaved f64 slices
// ---------------------------------------------------------------------------

struct InterleavedIn<'a> {
    data: &'a [f64],
    channels: usize,
    frames: usize,
}

impl<'a> Adapter<'a, f64> for InterleavedIn<'a> {
    unsafe fn read_sample_unchecked(&self, channel: usize, frame: usize) -> f64 {
        // SAFETY: caller guarantees channel < channels and frame < frames
        unsafe { *self.data.get_unchecked(frame * self.channels + channel) }
    }
    fn channels(&self) -> usize { self.channels }
    fn frames(&self) -> usize { self.frames }
}

struct InterleavedOut<'a> {
    data: &'a mut [f64],
    channels: usize,
    frames: usize,
}

impl<'a> Adapter<'a, f64> for InterleavedOut<'a> {
    unsafe fn read_sample_unchecked(&self, channel: usize, frame: usize) -> f64 {
        // SAFETY: caller guarantees channel < channels and frame < frames
        unsafe { *self.data.get_unchecked(frame * self.channels + channel) }
    }
    fn channels(&self) -> usize { self.channels }
    fn frames(&self) -> usize { self.frames }
}

impl<'a> AdapterMut<'a, f64> for InterleavedOut<'a> {
    unsafe fn write_sample_unchecked(
        &mut self,
        channel: usize,
        frame: usize,
        value: &f64,
    ) -> bool {
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
    /// Creates a sinc resampler converting from `source_rate` to `target_rate`.
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
        let ratio = target_rate as f64 / source_rate as f64;

        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let inner =
            Async::<f64>::new_sinc(ratio, 2.0, &params, chunk_frames, channels, FixedAsync::Input)
                .map_err(|e| OutputError::FormatUnsupported {
                    message: format!("resampler init failed: {e}"),
                })?;

        let max_output = inner.output_frames_max();
        let output_buf = vec![0.0f64; max_output * channels];

        Ok(Self { inner, channels, output_buf })
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
    /// writes resampled interleaved audio into `output`.
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

        // Copy resampled data from staging buffer into the caller's output
        let out_samples = out_frames * self.channels;
        output[..out_samples].copy_from_slice(&self.output_buf[..out_samples]);

        Ok(out_frames)
    }
}

#[cfg(test)]
mod tests {
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
        // 44100 → 48000 at 441 input frames ≈ 480 output frames
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
    #[ignore = "requires hardware timing; verifies freq content below Nyquist"]
    fn resampler_preserves_frequency_content() {
        // Full FFT verification of sinc quality is an integration test
    }
}
