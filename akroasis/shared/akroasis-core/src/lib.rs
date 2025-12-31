// High-fidelity audio playback core

pub mod decoder;
pub mod buffer;
pub mod replaygain;
pub mod error;

#[cfg(feature = "android")]
pub mod jni;

pub use error::{AudioError, Result};
pub use decoder::{AudioDecoder, DecodedAudio, FlacDecoder};

#[derive(Debug, Clone, PartialEq)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub channels: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            bit_depth: 16,
            channels: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.bit_depth, 16);
        assert_eq!(config.channels, 2);
    }
}
