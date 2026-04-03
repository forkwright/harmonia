#[cfg(feature = "native-output")]
pub mod cpal;
pub mod format;
pub mod resample;

use crate::error::OutputError;
use crate::signal_path::QualityTier;

/// Callback type supplied to `OutputBackend::open`. Called by the audio backend whenever
/// it needs samples; must fill the provided buffer within the real-time deadline.
pub type AudioDataCallback = Box<dyn FnMut(&mut [f64]) + Send + 'static>;

/// Describes an enumerated audio output device.
///
/// `Clone` + no lifetimes: safe to pass across FFI/UniFFI boundaries in Phase 6.
#[derive(Debug, Clone)]
pub struct OutputDevice {
    /// OS-assigned device identifier (e.g. WASAPI device GUID, CoreAudio UID).
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Capabilities reported by an `OutputBackend` for a specific device.
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    pub supported_sample_rates: Vec<u32>,
    pub supported_bit_depths: Vec<u32>,
    pub max_channels: u16,
    pub supports_exclusive_mode: bool,
}

/// Parameters for an output stream, as determined by format negotiation.
#[derive(Debug, Clone)]
pub struct OutputParams {
    /// Hardware output sample rate in Hz.
    pub sample_rate: u32,
    /// Number of output channels.
    pub channels: u16,
    /// Output bit depth (16, 24, or 32).
    pub bit_depth: u32,
    /// Request exclusive device access (ALSA, WASAPI, CoreAudio).
    pub exclusive_mode: bool,
    /// True if the output sample rate differs FROM the source and resampling is needed.
    pub needs_resample: bool,
    /// Source sample rate before resampling. Equals `sample_rate` when `needs_resample` is false.
    pub source_sample_rate: u32,
    /// Signal quality tier after output negotiation.
    pub quality_tier: QualityTier,
}

/// Abstraction over platform audio backends (cpal, AAudio, CoreAudio, WASAPI).
///
/// Implementations are `Send + Sync` so they can be shared between the audio task and
/// the engine control task via `Arc<Mutex<dyn OutputBackend>>`.
pub trait OutputBackend: Send + Sync {
    /// Returns all available output devices.
    fn available_devices(&self) -> Result<Vec<OutputDevice>, OutputError>;

    /// Returns capabilities for the named device (or the default if `None`).
    fn device_capabilities(
        &self,
        device_id: Option<&str>,
    ) -> Result<DeviceCapabilities, OutputError>;

    /// Opens an output stream with the given parameters. The backend calls `data_callback`
    /// whenever it needs audio samples. `data_callback` receives a mutable interleaved f64
    /// buffer; it must fill it within the real-time deadline.
    fn open(
        &mut self,
        device_id: Option<&str>,
        params: OutputParams,
        data_callback: AudioDataCallback,
    ) -> impl std::future::Future<Output = Result<(), OutputError>> + Send;

    /// Starts the audio stream (begins calling `data_callback`).
    fn start(&mut self) -> impl std::future::Future<Output = Result<(), OutputError>> + Send;

    /// Pauses the stream without closing it.
    fn pause(&mut self) -> impl std::future::Future<Output = Result<(), OutputError>> + Send;

    /// Closes the stream and releases device resources.
    fn close(&mut self) -> impl std::future::Future<Output = Result<(), OutputError>> + Send;
}
