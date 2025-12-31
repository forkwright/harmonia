// Audio decoder implementations

use crate::{AudioConfig, AudioError, Result};
use std::io::Cursor;

pub trait AudioDecoder {
    fn decode_stream<R: std::io::Read>(&mut self, reader: R) -> Result<DecodedAudio>;
    fn config(&self) -> AudioConfig;
}

pub struct DecodedAudio {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
}

pub struct FlacDecoder {
    config: Option<AudioConfig>,
}

impl FlacDecoder {
    pub fn new() -> Result<Self> {
        Ok(Self { config: None })
    }

    pub fn decode_file(&mut self, data: &[u8]) -> Result<DecodedAudio> {
        let cursor = Cursor::new(data);
        self.decode_stream(cursor)
    }
}

impl Default for FlacDecoder {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl AudioDecoder for FlacDecoder {
    fn decode_stream<R: std::io::Read>(&mut self, reader: R) -> Result<DecodedAudio> {
        let mut decoder = claxon::FlacReader::new(reader)
            .map_err(|e| AudioError::DecodingError(format!("FLAC init failed: {}", e)))?;

        let info = decoder.streaminfo();
        let sample_rate = info.sample_rate;
        let channels = info.channels as u16;
        let bit_depth = info.bits_per_sample as u16;

        self.config = Some(AudioConfig {
            sample_rate,
            channels,
            bit_depth,
        });

        let mut samples = Vec::new();

        for sample_result in decoder.samples() {
            let sample = sample_result
                .map_err(|e| AudioError::DecodingError(format!("FLAC decode failed: {}", e)))?;

            let normalized = match bit_depth {
                16 => sample as i16,
                24 => (sample >> 8) as i16,
                32 => (sample >> 16) as i16,
                _ => return Err(AudioError::UnsupportedSampleRate(bit_depth as u32)),
            };

            samples.push(normalized);
        }

        Ok(DecodedAudio {
            samples,
            sample_rate,
            channels,
            bit_depth,
        })
    }

    fn config(&self) -> AudioConfig {
        self.config.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flac_decoder_creation() {
        let decoder = FlacDecoder::new();
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_decoder_config() {
        let decoder = FlacDecoder::new().unwrap();
        let config = decoder.config();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.bit_depth, 16);
        assert_eq!(config.channels, 2);
    }
}
