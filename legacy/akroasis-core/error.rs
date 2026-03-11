// Error types for audio processing

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("Invalid audio format")]
    InvalidFormat,

    #[error("Unsupported sample rate: {0}")]
    UnsupportedSampleRate(u32),

    #[error("Buffer underrun")]
    BufferUnderrun,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AudioError>;
