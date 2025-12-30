// Audio decoder implementations

use crate::{AudioConfig, AudioError, Result};

pub trait AudioDecoder {
    fn decode(&mut self, input: &[u8]) -> Result<Vec<i16>>;
    fn config(&self) -> AudioConfig;
}

pub struct FlacDecoder {
    config: AudioConfig,
}

impl FlacDecoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: AudioConfig::default(),
        })
    }
}

impl AudioDecoder for FlacDecoder {
    fn decode(&mut self, _input: &[u8]) -> Result<Vec<i16>> {
        // TODO: Implement FLAC decoding with claxon
        Err(AudioError::DecodingError("Not implemented".into()))
    }

    fn config(&self) -> AudioConfig {
        AudioConfig {
            sample_rate: self.config.sample_rate,
            bit_depth: self.config.bit_depth,
            channels: self.config.channels,
        }
    }
}
