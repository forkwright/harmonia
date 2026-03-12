use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DspConfig {
    pub eq: EqConfig,
    pub crossfeed: CrossfeedConfig,
    pub replaygain: ReplayGainConfig,
    pub compressor: CompressorConfig,
    pub volume: VolumeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EqConfig {
    pub enabled: bool,
    pub preamp_db: f64,
    pub bands: Vec<EqBand>,
}

impl Default for EqConfig {
    fn default() -> Self {
        Self::iso_10_band()
    }
}

impl EqConfig {
    pub fn iso_10_band() -> Self {
        const ISO_FREQS: [f64; 10] = [
            31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
        ];
        Self {
            enabled: false,
            preamp_db: 0.0,
            bands: ISO_FREQS
                .iter()
                .map(|&f| EqBand {
                    frequency: f,
                    gain_db: 0.0,
                    q: 1.414,
                    filter_type: FilterType::Peaking,
                    enabled: true,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub frequency: f64,
    pub gain_db: f64,
    pub q: f64,
    pub filter_type: FilterType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
    AllPass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrossfeedConfig {
    pub enabled: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayGainMode {
    Track,
    Album,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayGainConfig {
    pub enabled: bool,
    pub mode: ReplayGainMode,
    pub preamp_db: f64,
    pub fallback_to_track: bool,
    pub prevent_clipping: bool,
}

impl Default for ReplayGainConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_to_track: true,
            prevent_clipping: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressorConfig {
    pub enabled: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VolumeConfig {
    pub level_db: f64,
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
