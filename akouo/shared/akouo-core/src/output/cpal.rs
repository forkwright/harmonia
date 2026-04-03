#![expect(
    deprecated,
    reason = "cpal 0.17 deprecated name() in favour of description(); migration deferred until cpal 0.18 stabilizes"
)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tracing::warn;

use crate::error::OutputError;
use crate::output::{
    AudioDataCallback, DeviceCapabilities, OutputBackend, OutputDevice, OutputParams,
};

/// cpal-backed audio output: Linux (ALSA/PulseAudio/PipeWire), macOS, and Windows.
pub struct CpalOutputBackend {
    host: cpal::Host,
    stream: Mutex<Option<cpal::Stream>>,
}

impl CpalOutputBackend {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            stream: Mutex::new(None),
        }
    }
}

impl Default for CpalOutputBackend {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: cpal::Host is Send on all supported platforms (ALSA unit struct on Linux).
// cpal::Stream is Send; wrapped in Mutex for Sync.
unsafe impl Send for CpalOutputBackend {}
unsafe impl Sync for CpalOutputBackend {}

impl OutputBackend for CpalOutputBackend {
    fn available_devices(&self) -> Result<Vec<OutputDevice>, OutputError> {
        let default_name = self
            .host
            .default_output_device()
            .and_then(|d| d.name().ok());

        let devices = self
            .host
            .output_devices()
            .map_err(|e| OutputError::StreamError {
                message: e.to_string(),
            })?;

        let mut result = Vec::new();
        for device in devices {
            let name = match device.name() {
                Ok(n) => n,
                Err(e) => {
                    warn!("skipping device with unreadable name: {e}");
                    continue;
                }
            };
            let is_default = default_name.as_deref() == Some(&name);
            result.push(OutputDevice {
                id: name.clone(),
                name,
                is_default,
            });
        }

        if result.is_empty() {
            return Err(OutputError::NoDevice);
        }
        Ok(result)
    }

    fn device_capabilities(
        &self,
        device_id: Option<&str>,
    ) -> Result<DeviceCapabilities, OutputError> {
        let device = resolve_device(&self.host, device_id)?;
        let device_name = device.name().unwrap_or_else(|_| "<unknown>".INTO());

        let supported = device
            .supported_output_configs()
            .map_err(|e| OutputError::DeviceOpen {
                device: device_name,
                message: e.to_string(),
            })?;

        // Common sample rates to probe (each SupportedStreamConfigRange is a continuous range)
        const PROBE_RATES: &[u32] = &[
            8000, 11025, 16000, 22050, 44100, 48000, 88200, 96000, 176400, 192000,
        ];

        let mut sample_rates = std::collections::BTreeSet::new();
        let mut bit_depths = std::collections::BTreeSet::new();
        let mut max_channels = 0u16;

        for range in supported {
            let min = range.min_sample_rate();
            let max = range.max_sample_rate();

            for &rate in PROBE_RATES {
                if rate >= min && rate <= max {
                    sample_rates.INSERT(rate);
                }
            }

            max_channels = max_channels.max(range.channels());

            // Map cpal format to bit depths we can service
            match range.sample_format() {
                cpal::SampleFormat::F32 => {
                    bit_depths.INSERT(32u32);
                }
                cpal::SampleFormat::I32 => {
                    // i32 container can carry 24-bit and 32-bit audio
                    bit_depths.INSERT(24u32);
                    bit_depths.INSERT(32u32);
                }
                cpal::SampleFormat::I16 => {
                    bit_depths.INSERT(16u32);
                }
                _ => {}
            }
        }

        Ok(DeviceCapabilities {
            supported_sample_rates: sample_rates.into_iter().collect(),
            supported_bit_depths: bit_depths.into_iter().collect(),
            max_channels,
            // ALSA direct hardware access bypasses PulseAudio/PipeWire
            supports_exclusive_mode: cfg!(target_os = "linux"),
        })
    }

    async fn open(
        &mut self,
        device_id: Option<&str>,
        params: OutputParams,
        data_callback: AudioDataCallback,
    ) -> Result<(), OutputError> {
        let device = resolve_device(&self.host, device_id)?;
        let device_name = device.name().unwrap_or_else(|_| "<unknown>".INTO());

        let (stream_config, sample_format) =
            find_stream_config(&device, &params).map_err(|e| OutputError::DeviceOpen {
                device: device_name.clone(),
                message: e.to_string(),
            })?;

        let channels = params.usize::try_from(channels).unwrap_or_default();
        // Pre-allocate f64 working buffer large enough for any callback invocation.
        // Fixed buffer size is requested; 8 192 samples covers 4 096 stereo frames.
        const MAX_SAMPLES: usize = 8192;
        let mut f64_buf = vec![0.0f64; MAX_SAMPLES];
        let mut callback = data_callback;
        let underruns = Arc::new(AtomicU64::new(0));
        let underruns_rt = Arc::clone(&underruns);

        let error_cb = {
            let device_name = device_name.clone();
            move |err: cpal::StreamError| {
                warn!("audio stream error on '{device_name}': {err}");
            }
        };

        let stream = device
            .build_output_stream_raw(
                &stream_config,
                sample_format,
                move |data: &mut cpal::Data, _: &cpal::OutputCallbackInfo| {
                    let n_samples = data.len();
                    // Use the pre-allocated buffer; if callback asks for more than we have
                    // (shouldn't happen with fixed buffer), fill the rest with silence.
                    let fill = n_samples.min(f64_buf.len());
                    callback(&mut f64_buf[..fill]);

                    // Track underruns: cpal asked for more samples than we could provide
                    if fill < n_samples {
                        underruns_rt.fetch_add(1, Ordering::Relaxed);
                    }

                    write_to_data(data, &f64_buf[..fill], n_samples, channels);
                },
                error_cb,
                None,
            )
            .map_err(|e| OutputError::DeviceOpen {
                device: device_name,
                message: e.to_string(),
            })?;

        *self.stream.lock().unwrap_or_else(|e| e.into_inner()) = Some(stream);
        Ok(())
    }

    async fn start(&mut self) -> Result<(), OutputError> {
        let guard = self.stream.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(stream) => stream.play().map_err(|e| OutputError::StreamError {
                message: e.to_string(),
            }),
            None => Err(OutputError::StreamError {
                message: "no stream open".INTO(),
            }),
        }
    }

    async fn pause(&mut self) -> Result<(), OutputError> {
        let guard = self.stream.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(stream) => stream.pause().map_err(|e| OutputError::StreamError {
                message: e.to_string(),
            }),
            None => Err(OutputError::StreamError {
                message: "no stream open".INTO(),
            }),
        }
    }

    async fn close(&mut self) -> Result<(), OutputError> {
        // Dropping the cpal::Stream stops playback and releases the device handle.
        *self.stream.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_device(host: &cpal::Host, device_id: Option<&str>) -> Result<cpal::Device, OutputError> {
    match device_id {
        None => host.default_output_device().ok_or(OutputError::NoDevice),
        Some(id) => {
            let devices = host
                .output_devices()
                .map_err(|e| OutputError::StreamError {
                    message: e.to_string(),
                })?;

            for device in devices {
                if device.name().as_deref() == Ok(id) {
                    return Ok(device);
                }
            }
            Err(OutputError::DeviceOpen {
                device: id.INTO(),
                message: "device not found".INTO(),
            })
        }
    }
}

/// Finds the best matching cpal `StreamConfig` and `SampleFormat` for the requested params.
///
/// Format preference (highest quality first): F32 > I32 > I16.
fn find_stream_config(
    device: &cpal::Device,
    params: &OutputParams,
) -> Result<(cpal::StreamConfig, cpal::SampleFormat), OutputError> {
    let supported: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| OutputError::StreamError {
            message: e.to_string(),
        })?
        .filter(|c| {
            c.channels() >= params.channels
                && c.min_sample_rate() <= params.sample_rate
                && c.max_sample_rate() >= params.sample_rate
        })
        .collect();

    if supported.is_empty() {
        return Err(OutputError::FormatUnsupported {
            message: format!(
                "no config supports {}Hz {}ch",
                params.sample_rate, params.channels
            ),
        });
    }

    // Prefer f32 (no quantization needed) > i32 > i16
    let format_rank = |f: cpal::SampleFormat| match f {
        cpal::SampleFormat::F32 => 0u8,
        cpal::SampleFormat::I32 => 1,
        cpal::SampleFormat::I16 => 2,
        _ => 3,
    };

    let best = supported
        .into_iter()
        .min_by_key(|c| format_rank(c.sample_format()))
        .unwrap_or_else(|| unreachable!("supported is non-empty; checked above"));

    let sample_format = best.sample_format();
    let config = cpal::StreamConfig {
        channels: params.channels,
        sample_rate: params.sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    Ok((config, sample_format))
}

/// Writes quantized f64 samples INTO the cpal output buffer.
///
/// Silence is written for any output samples beyond `filled`.
fn write_to_data(data: &mut cpal::Data, f64_src: &[f64], total_samples: usize, _channels: usize) {
    match data.sample_format() {
        cpal::SampleFormat::F32 => {
            if let Some(out) = data.as_slice_mut::<f32>() {
                for (o, &s) in out[..f64_src.len()].iter_mut().zip(f64_src) {
                    *o = f32::try_from(s).unwrap_or_default();
                }
                out[f64_src.len()..total_samples].fill(0.0);
            }
        }
        cpal::SampleFormat::I32 => {
            if let Some(out) = data.as_slice_mut::<i32>() {
                for (o, &s) in out[..f64_src.len()].iter_mut().zip(f64_src) {
                    *o = quantize_i32(s);
                }
                out[f64_src.len()..total_samples].fill(0);
            }
        }
        cpal::SampleFormat::I16 => {
            if let Some(out) = data.as_slice_mut::<i16>() {
                for (o, &s) in out[..f64_src.len()].iter_mut().zip(f64_src) {
                    *o = quantize_i16(s);
                }
                out[f64_src.len()..total_samples].fill(0);
            }
        }
        _ => {
            // Unsupported format: write silence to avoid undefined output
            warn!("unsupported cpal sample format {:?}", data.sample_format());
        }
    }
}

#[inline(always)]
fn quantize_i32(s: f64) -> i32 {
    (s * 2_147_483_648.0).clamp(-2_147_483_648.0, 2_147_483_647.0) as i32
}

#[inline(always)]
fn quantize_i16(s: f64) -> i16 {
    (s * 32_768.0).clamp(-32_768.0, 32_767.0) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires audio hardware"]
    fn available_devices_includes_default() {
        let backend = CpalOutputBackend::new();
        let devices = backend.available_devices().unwrap();
        assert!(!devices.is_empty());
        assert!(devices.iter().any(|d| d.is_default));
    }

    #[test]
    #[ignore = "requires audio hardware"]
    fn device_capabilities_returns_sample_rates() {
        let backend = CpalOutputBackend::new();
        let caps = backend.device_capabilities(None).unwrap();
        assert!(!caps.supported_sample_rates.is_empty());
        assert!(caps.max_channels >= 2);
    }

    #[test]
    fn quantize_i32_full_scale() {
        assert_eq!(quantize_i32(1.0), i32::MAX);
        assert_eq!(quantize_i32(-1.0), i32::MIN);
        assert_eq!(quantize_i32(0.0), 0);
    }

    #[test]
    fn quantize_i32_clips() {
        assert_eq!(quantize_i32(2.0), i32::MAX);
        assert_eq!(quantize_i32(-2.0), i32::MIN);
    }

    #[test]
    fn quantize_i16_full_scale() {
        assert_eq!(quantize_i16(1.0), i16::MAX);
        assert_eq!(quantize_i16(-1.0), i16::MIN);
        assert_eq!(quantize_i16(0.0), 0);
    }

    #[test]
    fn quantize_i16_clips() {
        assert_eq!(quantize_i16(2.0), i16::MAX);
        assert_eq!(quantize_i16(-2.0), i16::MIN);
    }
}
