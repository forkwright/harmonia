use std::time::Duration;

/// Top-level engine configuration.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Ring buffer capacity in samples per channel.
    pub ring_buffer_capacity: usize,
    /// Pre-buffer threshold before playback starts.
    pub prebuffer_threshold: Duration,
    /// DSP pipeline configuration.
    pub dsp: DspConfig,
    /// Audio output configuration.
    pub output: OutputConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            ring_buffer_capacity: 65536,
            prebuffer_threshold: Duration::from_millis(500),
            dsp: DspConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

/// Configuration for the full DSP pipeline.
#[derive(Debug, Clone, Default)]
pub struct DspConfig {
    pub skip_silence: SkipSilenceConfig,
    pub eq: EqConfig,
    pub crossfeed: CrossfeedConfig,
    pub replaygain: ReplayGainConfig,
    pub compressor: CompressorConfig,
    pub convolution: ConvolutionConfig,
    pub volume: VolumeConfig,
}

/// Skip-silence stage: trims silence below a threshold at track boundaries.
#[derive(Debug, Clone)]
pub struct SkipSilenceConfig {
    pub enabled: bool,
    /// Silence threshold in dBFS (negative). Samples below this level are considered silent.
    pub threshold_db: f64,
    /// Minimum consecutive silent frames before trimming begins.
    pub min_silence_samples: usize,
    /// Maximum consecutive silent frames to remove. Silence beyond this passes through
    /// unchanged to preserve intentional pauses.
    pub max_silence_samples: usize,
}

impl Default for SkipSilenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -60.0,
            min_silence_samples: 441,    // 10 ms at 44.1 kHz
            max_silence_samples: 132300, // 3 s at 44.1 kHz
        }
    }
}

/// Biquad filter type for a parametric EQ band.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FilterType {
    #[default]
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
    AllPass,
}

/// Parametric EQ stage.
#[derive(Debug, Clone, Default)]
pub struct EqConfig {
    pub enabled: bool,
    pub bands: Vec<EqBand>,
}

impl EqConfig {
    /// Returns a 10-band EQ at ISO standard center frequencies, all gains at 0 dB.
    pub fn iso_10_band_default() -> Self {
        const ISO_FREQS: [f64; 10] = [31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
        Self {
            enabled: false,
            bands: ISO_FREQS.iter().map(|&f| EqBand { frequency: f, gain_db: 0.0, q: 1.414, filter_type: FilterType::Peaking }).collect(),
        }
    }
}

/// A single parametric EQ band.
#[derive(Debug, Clone)]
pub struct EqBand {
    /// Center frequency in Hz.
    pub frequency: f64,
    /// Gain in dB (negative = cut, positive = boost).
    pub gain_db: f64,
    /// Q factor (bandwidth).
    pub q: f64,
    /// Filter topology.
    pub filter_type: FilterType,
}

/// Crossfeed stage: blends stereo channels for headphone listening.
#[derive(Debug, Clone)]
pub struct CrossfeedConfig {
    pub enabled: bool,
    /// Crossfeed strength 0.0–1.0.
    pub strength: f64,
}

impl Default for CrossfeedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: 0.3,
        }
    }
}

/// ReplayGain / EBU R128 normalization stage.
#[derive(Debug, Clone)]
pub struct ReplayGainConfig {
    pub enabled: bool,
    pub mode: ReplayGainMode,
    /// Pre-amplification gain in dB applied on top of the tag-based gain.
    pub preamp_db: f64,
    /// Fall back to track gain when album gain is unavailable.
    pub fallback_to_track: bool,
    /// Gain applied when no tags are found (default 0 dB = no change).
    pub fallback_gain_db: f64,
    /// Reduce gain if `peak * gain_linear > 1.0` to prevent clipping.
    pub prevent_clipping: bool,
    // Per-track metadata set by the engine when a new track starts.
    pub track_gain_db: Option<f64>,
    pub track_peak: Option<f64>,
    pub album_gain_db: Option<f64>,
    pub album_peak: Option<f64>,
    /// EBU R128 track loudness offset (used by Opus tracks).
    pub r128_track_gain: Option<f64>,
    pub r128_album_gain: Option<f64>,
}

impl Default for ReplayGainConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: true,
            track_gain_db: None,
            track_peak: None,
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: None,
            r128_album_gain: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplayGainMode {
    Track,
    Album,
    /// EBU R128 integrated loudness normalization.
    R128,
}

/// Dynamics compressor / limiter stage.
#[derive(Debug, Clone)]
pub struct CompressorConfig {
    pub enabled: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    /// Hard limiter ceiling in dBFS.
    pub limiter_ceiling_db: f64,
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -6.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 100.0,
            limiter_ceiling_db: -0.1,
        }
    }
}

/// Convolution reverb / room correction stage.
#[derive(Debug, Clone)]
pub struct ConvolutionConfig {
    pub enabled: bool,
    /// Path to the impulse response file. `None` = passthrough.
    pub ir_path: Option<std::path::PathBuf>,
    /// Output gain in dB applied after convolution.
    pub output_gain_db: f64,
}

impl Default for ConvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ir_path: None,
            output_gain_db: 0.0,
        }
    }
}

/// Volume control and TPDF dither stage.
#[derive(Debug, Clone)]
pub struct VolumeConfig {
    /// Master volume in dB (0 dB = unity, negative = attenuation).
    pub level_db: f64,
    /// Enable TPDF dither when quantizing to output bit depth.
    pub dither: bool,
}

impl Default for VolumeConfig {
    fn default() -> Self {
        Self {
            level_db: 0.0,
            dither: true,
        }
    }
}

/// Audio output configuration.
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Target output device name. `None` = system default.
    pub device_name: Option<String>,
    pub buffer_size: BufferSize,
    /// Request exclusive mode from the OS (WASAPI/CoreAudio).
    pub exclusive_mode: bool,
    /// Target bit depth for output quantization.
    pub bit_depth: u32,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            device_name: None,
            buffer_size: BufferSize::default(),
            exclusive_mode: false,
            bit_depth: 24,
        }
    }
}

/// Output buffer size hint passed to the audio backend.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum BufferSize {
    /// Let the backend choose the buffer size.
    #[default]
    Default,
    /// Fixed buffer size in frames.
    Fixed(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_defaults() {
        let cfg = EngineConfig::default();
        assert_eq!(cfg.ring_buffer_capacity, 65536);
        assert_eq!(cfg.prebuffer_threshold, Duration::from_millis(500));
        assert_eq!(cfg.output.bit_depth, 24);
    }

    #[test]
    fn dsp_config_stages_disabled_by_default() {
        let dsp = DspConfig::default();
        assert!(!dsp.skip_silence.enabled);
        assert!(!dsp.eq.enabled);
        assert!(!dsp.crossfeed.enabled);
        assert!(!dsp.replaygain.enabled);
        assert!(!dsp.compressor.enabled);
        assert!(!dsp.convolution.enabled);
    }

    #[test]
    fn volume_config_unity_gain_default() {
        let vol = VolumeConfig::default();
        assert_eq!(vol.level_db, 0.0);
        assert!(vol.dither);
    }

    #[test]
    fn compressor_config_reasonable_defaults() {
        let comp = CompressorConfig::default();
        assert!(comp.ratio > 1.0);
        assert!(comp.threshold_db < 0.0);
        assert!(comp.limiter_ceiling_db < 0.0);
    }

    #[test]
    fn skip_silence_threshold_is_negative() {
        let cfg = SkipSilenceConfig::default();
        assert!(cfg.threshold_db < 0.0);
        assert!(cfg.min_silence_samples > 0);
    }
}
