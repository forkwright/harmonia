use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tracing::{error, warn};

// WHY: quantization lives in dsp::volume — a second local implementation
// drifted to an asymmetric formula (-1.0 mapped to i32::MIN here but
// -2_147_483_647 in the DSP path), so the output stage shares the one
// canonical symmetric-scale quantizer family.
use crate::config::BufferSize;
use crate::dsp::volume::{quantize_i16, quantize_i32};
use crate::error::OutputError;
#[cfg(target_os = "linux")]
use crate::output::pipewire;
use crate::output::{
    AudioDataCallback, DeviceCapabilities, OutputBackend, OutputDevice, OutputParams,
};

/// cpal-backed audio output: Linux (ALSA/PulseAudio/PipeWire), macOS, and Windows.
pub struct CpalOutputBackend {
    host: cpal::Host,
    stream: Mutex<Option<cpal::Stream>>,
    // WHY: the counter must outlive open() — the engine polls it to emit
    // EngineEvent::Underrun; a counter local to open() was dropped
    // immediately and the event could never fire.
    underruns: Arc<AtomicU64>,
    #[cfg(target_os = "linux")]
    pipewire_rate_forced: Mutex<bool>,
}

impl CpalOutputBackend {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            stream: Mutex::new(None),
            underruns: Arc::new(AtomicU64::new(0)),
            #[cfg(target_os = "linux")]
            pipewire_rate_forced: Mutex::new(false),
        }
    }

    /// Cumulative underrun count for the currently open stream.
    #[must_use]
    pub fn underrun_count(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }
}

impl Default for CpalOutputBackend {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: cpal::Host is Send on all supported platforms (ALSA Host wraps an
// Arc around a unit context on Linux). cpal::Stream is Send (ALSA StreamInner
// is Send + Sync; WASAPI/AAudio declare Send); wrapped in Mutex for Sync.
unsafe impl Send for CpalOutputBackend {}
unsafe impl Sync for CpalOutputBackend {}

impl OutputBackend for CpalOutputBackend {
    fn available_devices(&self) -> Result<Vec<OutputDevice>, OutputError> {
        let default_name = self
            .host
            .default_output_device()
            .and_then(|d| device_name(&d).ok());

        let devices = self
            .host
            .output_devices()
            .map_err(|e| OutputError::StreamError {
                message: e.to_string(),
            })?;

        let mut result = Vec::new();
        for device in devices {
            let name = match device_name(&device) {
                Ok(n) => n,
                Err(e) => {
                    warn!("skipping device with unreadable name: {e}");
                    continue;
                }
            };
            let is_default = default_name.as_deref() == Some(&name);
            result.push(OutputDevice {
                id: crate::output::AudioDeviceId(name.clone()),
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
        let device_name = device_name(&device).unwrap_or_else(|_| "<unknown>".into());

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
            for &rate in PROBE_RATES {
                if range.contains_rate(rate) {
                    sample_rates.insert(rate);
                }
            }

            max_channels = max_channels.max(range.channels());

            // Map cpal format to bit depths we can service
            match range.sample_format() {
                cpal::SampleFormat::F32 => {
                    bit_depths.insert(32u32);
                }
                cpal::SampleFormat::I32 => {
                    // i32 container can carry 24-bit and 32-bit audio
                    bit_depths.insert(24u32);
                    bit_depths.insert(32u32);
                }
                cpal::SampleFormat::I16 => {
                    bit_depths.insert(16u32);
                }
                _ => {}
            }
        }

        #[cfg(target_os = "linux")]
        {
            for rate in pipewire::alsa_hardware_sample_rates()? {
                sample_rates.insert(rate);
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
        error_tx: tokio::sync::mpsc::Sender<OutputError>,
    ) -> Result<(), OutputError> {
        let device = resolve_device(&self.host, device_id)?;
        let device_name = device_name(&device).unwrap_or_else(|_| "<unknown>".into());

        let (stream_config, sample_format, supported_buffer_size) =
            find_stream_config(&device, &params).map_err(|e| OutputError::DeviceOpen {
                device: device_name.clone(),
                message: e.to_string(),
            })?;

        let pipewire_rate_forced = force_pipewire_rate(params.sample_rate)?;

        let channels = usize::from(params.channels);
        // WHY(#543): sized FROM the negotiated buffer_size (or the device's
        // reported max period when Default) instead of a fixed 8 192-sample
        // guess — a device requesting more per callback than the guess would
        // otherwise silently truncate real audio.
        let mut f64_buf =
            vec![
                0.0f64;
                scratch_buffer_samples(&params.buffer_size, supported_buffer_size, channels)
            ];
        let mut callback = data_callback;
        // Fresh stream, fresh counter.
        self.underruns.store(0, Ordering::Relaxed);
        let underruns_rt = Arc::clone(&self.underruns);

        let error_tx_rt = error_tx.clone();
        let error_cb = make_stream_error_callback(device_name.clone(), error_tx);

        let stream = match device.build_output_stream_raw(
            stream_config,
            sample_format,
            move |data: &mut cpal::Data, _: &cpal::OutputCallbackInfo| {
                let n_samples = data.len();
                match pull_samples(n_samples, &mut f64_buf, &mut callback, &underruns_rt) {
                    Ok(samples) => write_to_data(data, samples, n_samples, channels),
                    Err(message) => {
                        // WHY(#543): a scratch buffer sized below the negotiated
                        // device period must never silently truncate real audio —
                        // emit silence for this callback and surface the defect.
                        error!("{message}");
                        error_tx_rt
                            .try_send(OutputError::StreamError { message })
                            .ok();
                        write_to_data(data, &[], n_samples, channels);
                    }
                }
            },
            error_cb,
            None,
        ) {
            Ok(stream) => stream,
            Err(e) => {
                if pipewire_rate_forced {
                    // WHY: rate reset failure on error path is non-fatal; hardware rate may already be reset
                    reset_pipewire_rate().ok();
                }
                return Err(OutputError::DeviceOpen {
                    device: device_name,
                    message: e.to_string(),
                });
            }
        };

        if let Err(e) = reapply_pipewire_rate_if_forced(params.sample_rate, pipewire_rate_forced) {
            // WHY: rate reset failure on error path is non-fatal; hardware rate may already be reset
            reset_pipewire_rate().ok();
            return Err(e);
        }

        *self.stream.lock().unwrap_or_else(|e| e.into_inner()) = Some(stream);
        #[cfg(target_os = "linux")]
        {
            *self
                .pipewire_rate_forced
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = pipewire_rate_forced;
        }
        Ok(())
    }

    async fn start(&mut self) -> Result<(), OutputError> {
        let guard = self.stream.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(stream) => stream.play().map_err(|e| OutputError::StreamError {
                message: e.to_string(),
            }),
            None => Err(OutputError::StreamError {
                message: "no stream open".into(),
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
                message: "no stream open".into(),
            }),
        }
    }

    async fn close(&mut self) -> Result<(), OutputError> {
        // Dropping the cpal::Stream stops playback and releases the device handle.
        *self.stream.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.reset_pipewire_rate_if_forced()
    }
}

impl Drop for CpalOutputBackend {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        // WHY: drop cannot propagate errors; best-effort rate reset on backend teardown
        self.reset_pipewire_rate_if_forced().ok();
    }
}

impl CpalOutputBackend {
    #[cfg(target_os = "linux")]
    fn reset_pipewire_rate_if_forced(&self) -> Result<(), OutputError> {
        let mut forced = self
            .pipewire_rate_forced
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !*forced {
            return Ok(());
        }

        pipewire::reset_pipewire_rate()?;
        *forced = false;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn reset_pipewire_rate_if_forced(&self) -> Result<(), OutputError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds the cpal stream-error callback: logs the error and forwards it to the engine.
///
/// Runs on the audio backend's callback thread; `try_send` never blocks. A full channel
/// drops the report — the first error already triggers engine shutdown, so later ones
/// carry no additional signal.
fn make_stream_error_callback(
    device_name: String,
    error_tx: tokio::sync::mpsc::Sender<OutputError>,
) -> impl FnMut(cpal::Error) {
    move |err: cpal::Error| {
        warn!("audio stream error on '{device_name}': {err}");
        error_tx
            .try_send(OutputError::StreamError {
                message: format!("audio stream error on '{device_name}': {err}"),
            })
            .ok();
    }
}

/// Human-readable device name from the structured device description.
fn device_name(device: &cpal::Device) -> Result<String, cpal::Error> {
    device.description().map(|d| d.name().to_owned())
}

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
                if device_name(&device).is_ok_and(|n| n == id) {
                    return Ok(device);
                }
            }
            Err(OutputError::DeviceOpen {
                device: id.into(),
                message: "device not found".into(),
            })
        }
    }
}

#[cfg(target_os = "linux")]
fn force_pipewire_rate(sample_rate: u32) -> Result<bool, OutputError> {
    match pipewire::force_pipewire_rate_if_available(sample_rate) {
        Ok(false) => {
            warn!("pw-metadata not available; PipeWire clock rate was not forced");
            Ok(false)
        }
        other => other,
    }
}

#[cfg(not(target_os = "linux"))]
fn force_pipewire_rate(_sample_rate: u32) -> Result<bool, OutputError> {
    Ok(false)
}

#[cfg(target_os = "linux")]
fn reapply_pipewire_rate_if_forced(sample_rate: u32, forced: bool) -> Result<(), OutputError> {
    if forced {
        pipewire::force_pipewire_rate(sample_rate)?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn reapply_pipewire_rate_if_forced(_sample_rate: u32, _forced: bool) -> Result<(), OutputError> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn reset_pipewire_rate() -> Result<(), OutputError> {
    pipewire::reset_pipewire_rate()
}

#[cfg(not(target_os = "linux"))]
fn reset_pipewire_rate() -> Result<(), OutputError> {
    Ok(())
}

/// Finds the best matching cpal `StreamConfig` and `SampleFormat` for the requested params.
///
/// Format preference (highest quality first): F32 > I32 > I16. Honors
/// `params.buffer_size` (#543) instead of always requesting
/// `cpal::BufferSize::Default`; a `Fixed` request is validated against the
/// matched config's supported range before being handed to cpal. Also returns
/// the matched config's `SupportedBufferSize` so the caller can size its
/// scratch buffer FROM the negotiated period.
fn find_stream_config(
    device: &cpal::Device,
    params: &OutputParams,
) -> Result<
    (
        cpal::StreamConfig,
        cpal::SampleFormat,
        cpal::SupportedBufferSize,
    ),
    OutputError,
> {
    let supported: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| OutputError::StreamError {
            message: e.to_string(),
        })?
        .filter(|c| c.channels() >= params.channels && c.contains_rate(params.sample_rate))
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
        .ok_or_else(|| OutputError::FormatUnsupported {
            message: format!(
                "no config supports {}Hz {}ch",
                params.sample_rate, params.channels
            ),
        })?;

    let sample_format = best.sample_format();
    let supported_buffer_size = *best.buffer_size();

    let buffer_size = match params.buffer_size {
        BufferSize::Default => cpal::BufferSize::Default,
        BufferSize::Fixed(frames) => {
            validate_fixed_buffer_size(frames, supported_buffer_size)?;
            cpal::BufferSize::Fixed(frames)
        }
    };

    let config = cpal::StreamConfig {
        channels: params.channels,
        sample_rate: params.sample_rate,
        buffer_size,
    };

    Ok((config, sample_format, supported_buffer_size))
}

/// Validates a requested `BufferSize::Fixed` frame count against the device's
/// supported range. `Unknown` (device reports no range) passes through — cpal
/// itself rejects an unsupportable value when the stream is built.
///
/// Pure so the #543 validation logic is unit-testable without a real device.
fn validate_fixed_buffer_size(
    requested: cpal::FrameCount,
    supported: cpal::SupportedBufferSize,
) -> Result<(), OutputError> {
    if let cpal::SupportedBufferSize::Range { min, max } = supported
        && !(min..=max).contains(&requested)
    {
        return Err(OutputError::FormatUnsupported {
            message: format!(
                "requested fixed buffer size {requested} frames outside device-supported range {min}..={max}"
            ),
        });
    }
    Ok(())
}

/// Fallback per-callback frame budget when the device reports no buffer-size
/// range (`SupportedBufferSize::Unknown`) and the caller requested the default
/// buffer size — matches the scratch buffer's previous fixed allocation.
const DEFAULT_SCRATCH_FRAMES: usize = 4096;

/// Floor on the scratch buffer so a degenerate (zero-channel) negotiation
/// never leaves a too-small buffer.
const MIN_SCRATCH_SAMPLES: usize = 512;

/// Ceiling on the scratch buffer regardless of what the device reports — guards
/// against a driver advertising an unreasonable `SupportedBufferSize::Range.max`.
const MAX_SCRATCH_SAMPLES: usize = 1 << 20;

/// Computes the scratch-buffer sample capacity (across all channels) needed to
/// service one callback without truncation (#543).
///
/// Pure so the sizing logic is unit-testable without a real device.
fn scratch_buffer_samples(
    buffer_size: &BufferSize,
    supported: cpal::SupportedBufferSize,
    channels: usize,
) -> usize {
    let frames = match (buffer_size, supported) {
        (BufferSize::Fixed(frames), _) => *frames as usize,
        (BufferSize::Default, cpal::SupportedBufferSize::Range { max, .. }) => max as usize,
        (BufferSize::Default, cpal::SupportedBufferSize::Unknown) => DEFAULT_SCRATCH_FRAMES,
    };
    frames
        .saturating_mul(channels)
        .clamp(MIN_SCRATCH_SAMPLES, MAX_SCRATCH_SAMPLES)
}

/// Pulls one callback's worth of samples FROM `callback` into `f64_buf[..n_samples]`.
///
/// Tracks a genuine underrun in `underruns` whenever `callback` reports it could
/// not supply real audio (the ring buffer was empty) — this is the single source
/// of truth for underruns (#541); a callback that fills the buffer with real
/// audio never increments the counter, regardless of `n_samples`.
///
/// Returns `Err` when `n_samples` exceeds the pre-allocated scratch buffer — a
/// buffer-sizing defect (#543) that must be surfaced, never silently truncated.
fn pull_samples<'a>(
    n_samples: usize,
    f64_buf: &'a mut [f64],
    callback: &mut dyn FnMut(&mut [f64]) -> bool,
    underruns: &AtomicU64,
) -> Result<&'a [f64], String> {
    if n_samples > f64_buf.len() {
        return Err(format!(
            "audio callback requested {n_samples} samples, exceeding the {}-sample negotiated scratch buffer",
            f64_buf.len()
        ));
    }

    let filled = callback(&mut f64_buf[..n_samples]);
    if !filled {
        underruns.fetch_add(1, Ordering::Relaxed);
    }
    Ok(&f64_buf[..n_samples])
}

/// Writes quantized f64 samples INTO the cpal output buffer.
///
/// Silence is written for any output samples beyond `filled`.
fn write_to_data(data: &mut cpal::Data, f64_src: &[f64], total_samples: usize, _channels: usize) {
    match data.sample_format() {
        cpal::SampleFormat::F32 => {
            if let Some(out) = data.as_slice_mut::<f32>() {
                for (o, &s) in out[..f64_src.len()].iter_mut().zip(f64_src) {
                    *o = s as f32;
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
    fn stream_error_callback_forwards_error_to_channel() {
        let (error_tx, mut error_rx) = tokio::sync::mpsc::channel::<OutputError>(1);
        let mut cb = make_stream_error_callback("test-device".to_string(), error_tx);

        cb(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable));

        let err = error_rx.try_recv().expect("error must reach the channel");
        match err {
            OutputError::StreamError { message } => {
                assert!(message.contains("test-device"), "message: {message}");
            }
            other => panic!("expected StreamError, got {other:?}"),
        }
    }

    #[test]
    fn stream_error_callback_full_channel_drops_report_without_panic() {
        let (error_tx, mut error_rx) = tokio::sync::mpsc::channel::<OutputError>(1);
        let mut cb = make_stream_error_callback("test-device".to_string(), error_tx);

        // Second send hits a full channel; must be dropped silently, not panic.
        cb(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable));
        cb(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable));

        assert!(error_rx.try_recv().is_ok(), "first error must be delivered");
        assert!(
            error_rx.try_recv().is_err(),
            "second error must have been dropped by the full channel"
        );
    }

    // NOTE: quantize_i16/quantize_i32 are the canonical dsp::volume
    // symmetric-scale quantizers — -1.0 maps to -MAX (not MIN), matching
    // the i16/i24 convention used throughout the DSP pipeline.
    #[test]
    fn quantize_i32_full_scale() {
        assert_eq!(quantize_i32(1.0), i32::MAX);
        assert_eq!(quantize_i32(-1.0), -i32::MAX);
        assert_eq!(quantize_i32(0.0), 0);
    }

    #[test]
    fn quantize_i32_clips() {
        assert_eq!(quantize_i32(2.0), i32::MAX);
        assert_eq!(quantize_i32(-2.0), -i32::MAX);
    }

    #[test]
    fn quantize_i16_full_scale() {
        assert_eq!(quantize_i16(1.0), i16::MAX);
        assert_eq!(quantize_i16(-1.0), -i16::MAX);
        assert_eq!(quantize_i16(0.0), 0);
    }

    #[test]
    fn quantize_i16_clips() {
        assert_eq!(quantize_i16(2.0), i16::MAX);
        assert_eq!(quantize_i16(-2.0), -i16::MAX);
    }

    // --- #541: real (ring-empty) underruns must be counted, not near-unreachable
    // "cpal asked for more than the fixed scratch buffer" cases ---

    #[test]
    fn pull_samples_increments_underrun_on_empty_ring() {
        let underruns = AtomicU64::new(0);
        let mut buf = vec![0.0f64; 8];
        let mut cb = |b: &mut [f64]| -> bool {
            b.fill(0.0);
            false // simulates pop_frame() returning false (ring empty)
        };

        let result = pull_samples(4, &mut buf, &mut cb, &underruns);

        assert!(result.is_ok());
        assert_eq!(underruns.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn pull_samples_no_underrun_when_callback_supplies_real_audio() {
        let underruns = AtomicU64::new(0);
        let mut buf = vec![0.0f64; 8];
        let mut cb = |b: &mut [f64]| -> bool {
            b.fill(1.0);
            true
        };

        let result = pull_samples(4, &mut buf, &mut cb, &underruns);

        assert!(result.is_ok());
        assert_eq!(underruns.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn pull_samples_errors_without_counting_underrun_when_buffer_too_small() {
        let underruns = AtomicU64::new(0);
        let mut buf = vec![0.0f64; 4];
        let mut cb = |_: &mut [f64]| -> bool { true };

        let result = pull_samples(8, &mut buf, &mut cb, &underruns);

        assert!(result.is_err());
        // WHY: a buffer-sizing defect (#543) is a distinct failure mode FROM a
        // genuine ring-empty underrun (#541) — never conflate the two counters.
        assert_eq!(underruns.load(Ordering::Relaxed), 0);
    }

    // --- #543: buffer_size must be honored, never silently truncated ---

    #[test]
    fn validate_fixed_buffer_size_accepts_in_range() {
        let supported = cpal::SupportedBufferSize::Range { min: 64, max: 4096 };
        assert!(validate_fixed_buffer_size(1024, supported).is_ok());
    }

    #[test]
    fn validate_fixed_buffer_size_rejects_out_of_range() {
        let supported = cpal::SupportedBufferSize::Range { min: 64, max: 4096 };
        assert!(validate_fixed_buffer_size(8192, supported).is_err());
    }

    #[test]
    fn validate_fixed_buffer_size_passes_through_unknown() {
        assert!(validate_fixed_buffer_size(99_999, cpal::SupportedBufferSize::Unknown).is_ok());
    }

    #[test]
    fn scratch_buffer_samples_uses_fixed_request_regardless_of_supported() {
        let n = scratch_buffer_samples(
            &BufferSize::Fixed(2048),
            cpal::SupportedBufferSize::Range { min: 64, max: 512 },
            2,
        );
        assert_eq!(n, 2048 * 2);
    }

    #[test]
    fn scratch_buffer_samples_default_uses_device_max() {
        let n = scratch_buffer_samples(
            &BufferSize::Default,
            cpal::SupportedBufferSize::Range {
                min: 64,
                max: 16_384,
            },
            2,
        );
        assert_eq!(n, 16_384 * 2);
    }

    #[test]
    fn scratch_buffer_samples_default_unknown_uses_fallback() {
        let n = scratch_buffer_samples(&BufferSize::Default, cpal::SupportedBufferSize::Unknown, 2);
        assert_eq!(n, DEFAULT_SCRATCH_FRAMES * 2);
    }

    #[test]
    fn scratch_buffer_samples_never_below_floor() {
        let n =
            scratch_buffer_samples(&BufferSize::Fixed(1), cpal::SupportedBufferSize::Unknown, 1);
        assert!(n >= MIN_SCRATCH_SAMPLES);
    }
}
