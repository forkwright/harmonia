// Renderer configuration loaded FROM TOML via figment.

use std::path::Path;

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use super::error::{ConfigSnafu, RenderError};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RendererConfig {
    pub output: OutputSettings,
    pub dsp: DspSettings,
    pub buffer: BufferSettings,
    pub reconnect: ReconnectSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSettings {
    pub device: String,
    pub exclusive_mode: bool,
    pub bit_depth: u32,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            device: "default".to_string(),
            exclusive_mode: false,
            bit_depth: 24,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DspSettings {
    pub volume: VolumeSettings,
    pub eq: EqSettings,
    pub crossfeed: CrossfeedSettings,
    pub replaygain: ReplayGainSettings,
    pub compressor: CompressorSettings,
    pub convolution: ConvolutionSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VolumeSettings {
    pub level_db: f64,
    pub dither: bool,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            level_db: 0.0,
            dither: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EqSettings {
    pub enabled: bool,
    pub bands: Vec<EqBandSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBandSettings {
    pub frequency: f64,
    pub gain_db: f64,
    #[serde(default = "default_q")]
    pub q: f64,
}

fn default_q() -> f64 {
    1.414
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrossfeedSettings {
    pub enabled: bool,
    pub strength: f64,
}

impl Default for CrossfeedSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayGainSettings {
    pub enabled: bool,
    pub mode: ReplayGainModeConfig,
    pub preamp_db: f64,
}

impl Default for ReplayGainSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ReplayGainModeConfig::Track,
            preamp_db: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayGainModeConfig {
    Track,
    Album,
    R128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressorSettings {
    pub enabled: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -6.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 100.0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConvolutionSettings {
    pub enabled: bool,
    pub ir_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BufferSettings {
    /// Default jitter-buffer depth (milliseconds) for the renderer.
    pub depth_ms: u64,
    /// Playout pipeline: lower bound on the adaptive buffer target.
    pub playout_min_depth_ms: u64,
    /// Playout pipeline: upper bound on the adaptive buffer target.
    pub playout_max_depth_ms: u64,
    /// Playout pipeline: initial buffer target before adaptation starts.
    pub playout_initial_depth_ms: u64,
    /// Playout pipeline: size of the rolling late/on-time stats window used
    /// to drive adaptive buffer sizing.
    pub playout_stats_window: usize,
}

impl Default for BufferSettings {
    fn default() -> Self {
        Self {
            depth_ms: 100,
            playout_min_depth_ms: 20,
            playout_max_depth_ms: 200,
            playout_initial_depth_ms: 80,
            playout_stats_window: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReconnectSettings {
    pub enabled: bool,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for ReconnectSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30_000,
        }
    }
}

impl RendererConfig {
    pub fn dsp_config(&self) -> akouo_core::DspConfig {
        akouo_core::DspConfig {
            skip_silence: akouo_core::SkipSilenceConfig::default(),
            eq: akouo_core::EqConfig {
                enabled: self.dsp.eq.enabled,
                bands: self
                    .dsp
                    .eq
                    .bands
                    .iter()
                    .map(|b| akouo_core::EqBand {
                        frequency: b.frequency,
                        gain_db: b.gain_db,
                        q: b.q,
                        filter_type: akouo_core::config::FilterType::Peaking,
                    })
                    .collect(),
            },
            crossfeed: akouo_core::CrossfeedConfig {
                enabled: self.dsp.crossfeed.enabled,
                strength: self.dsp.crossfeed.strength,
            },
            replaygain: akouo_core::ReplayGainConfig {
                enabled: self.dsp.replaygain.enabled,
                mode: match self.dsp.replaygain.mode {
                    ReplayGainModeConfig::Track => akouo_core::ReplayGainMode::Track,
                    ReplayGainModeConfig::Album => akouo_core::ReplayGainMode::Album,
                    ReplayGainModeConfig::R128 => akouo_core::ReplayGainMode::R128,
                },
                preamp_db: self.dsp.replaygain.preamp_db,
                ..akouo_core::ReplayGainConfig::default()
            },
            compressor: akouo_core::CompressorConfig {
                enabled: self.dsp.compressor.enabled,
                threshold_db: self.dsp.compressor.threshold_db,
                ratio: self.dsp.compressor.ratio,
                attack_ms: self.dsp.compressor.attack_ms,
                release_ms: self.dsp.compressor.release_ms,
                ..akouo_core::CompressorConfig::default()
            },
            convolution: akouo_core::ConvolutionConfig {
                enabled: self.dsp.convolution.enabled,
                ir_path: self.dsp.convolution.ir_path.as_ref().map(Into::into),
                ..akouo_core::ConvolutionConfig::default()
            },
            volume: akouo_core::VolumeConfig {
                level_db: self.dsp.volume.level_db,
                dither: self.dsp.volume.dither,
            },
        }
    }

    pub fn ring_buffer_capacity(&self) -> usize {
        let samples_per_ms = 48; // 48kHz
        let target = usize::try_from(self.buffer.depth_ms).unwrap_or_default() * samples_per_ms * 2;
        target.next_power_of_two().max(8192)
    }
}

pub fn load_renderer_config(path: Option<&Path>) -> Result<RendererConfig, RenderError> {
    let mut figment = Figment::from(Serialized::defaults(RendererConfig::default()));
    if let Some(p) = path {
        figment = figment.merge(Toml::file(p));
    }
    figment = figment.merge(Env::prefixed("HARMONIA_RENDER_").split("__"));
    figment.extract().map_err(Box::new).context(ConfigSnafu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_loads_without_file() {
        let config = load_renderer_config(None).unwrap_or_default();
        assert_eq!(config.output.device, "default");
        assert_eq!(config.output.bit_depth, 24);
        assert!(!config.dsp.eq.enabled);
        assert!(config.reconnect.enabled);
    }

    #[test]
    fn dsp_config_conversion_preserves_values() {
        let mut config = RendererConfig::default();
        config.dsp.volume.level_db = -6.0;
        config.dsp.crossfeed.enabled = true;
        config.dsp.crossfeed.strength = 0.5;

        let dsp = config.dsp_config();
        assert_eq!(dsp.volume.level_db, -6.0);
        assert!(dsp.crossfeed.enabled);
        assert_eq!(dsp.crossfeed.strength, 0.5);
    }

    #[test]
    fn ring_buffer_capacity_is_power_of_two() {
        let config = RendererConfig::default();
        let cap = config.ring_buffer_capacity();
        assert!(cap.is_power_of_two());
        assert!(cap >= 8192);
    }

    #[test]
    fn config_loads_from_toml_string() {
        let toml = r#"
[output]
device = "hw:1"
bit_depth = 32

[dsp.volume]
level_db = -3.0

[dsp.crossfeed]
enabled = true
strength = 0.6

[buffer]
depth_ms = 200

[reconnect]
max_backoff_ms = 60000
"#;
        let tmp = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
        std::fs::write(tmp.path(), toml).unwrap();

        let config = load_renderer_config(Some(tmp.path())).unwrap_or_default();
        assert_eq!(config.output.device, "hw:1");
        assert_eq!(config.output.bit_depth, 32);
        assert_eq!(config.dsp.volume.level_db, -3.0);
        assert!(config.dsp.crossfeed.enabled);
        assert_eq!(config.buffer.depth_ms, 200);
        assert_eq!(config.reconnect.max_backoff_ms, 60000);
    }
}
