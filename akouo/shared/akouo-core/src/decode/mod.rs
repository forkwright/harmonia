pub mod metadata;
pub mod opus;
pub mod probe;
pub mod symphonia;
pub mod wavpack;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::error::DecodeError;

/// Identifies the codec of an audio stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Codec {
    Flac,
    Alac,
    Wav,
    Aiff,
    Mp3,
    Aac,
    Vorbis,
    Opus,
    /// A codec not explicitly enumerated  -  carries a human-readable label.
    Other(String),
}

/// Parameters describing a decoded audio stream.
#[derive(Debug, Clone)]
pub struct StreamParams {
    pub codec: Codec,
    pub sample_rate: u32,
    pub channels: u16,
    /// Bit depth of the source (e.g. 16, 24, 32). `None` for lossy codecs.
    pub bit_depth: Option<u32>,
    pub duration: Option<Duration>,
    /// Bitrate in kbps. `None` when unavailable.
    pub bitrate: Option<u32>,
}

/// Gapless playback metadata embedded in the source file.
#[derive(Debug, Clone)]
pub struct GaplessInfo {
    /// Encoder delay in samples to skip at the start.
    pub encoder_delay: u32,
    /// Encoder padding in samples to skip at the end.
    pub encoder_padding: u32,
    /// Total PCM sample count after applying delay and padding, if known.
    pub total_samples: Option<u64>,
}

/// A single decoded audio frame: interleaved f64 samples in [-1.0, 1.0].
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// Interleaved samples: for stereo, layout is [L, R, L, R, ...].
    pub samples: Box<[f64]>,
    pub channels: u16,
    pub sample_rate: u32,
    /// Sample OFFSET FROM the start of the stream (before gapless trimming).
    pub timestamp: u64,
}

/// An async audio decoder. Implementations drive symphonia, opus, or other backends.
///
/// Methods return `Pin<Box<dyn Future>>` so the trait is dyn-compatible and can be
/// used as `Box<dyn AudioDecoder>` without the async-trait crate. Decoder state is
/// mutably accessed only FROM the single decode task  -  no external locking needed.
///
/// The `Pin<Box<dyn Future>>` return types enable `Box<dyn AudioDecoder>`  -  necessary
/// for probe.rs to return an erased decoder without knowing the concrete type at compile
/// time. The `'_` lifetime binds the future's lifetime to the `&mut self` borrow.
pub trait AudioDecoder: Send {
    /// Returns the next decoded frame, or `None` at end of stream.
    fn next_frame(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DecodedFrame>, DecodeError>> + Send + '_>>;

    /// Seeks to the requested position. Returns the actual position reached.
    fn seek(
        &mut self,
        position: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Duration, DecodeError>> + Send + '_>>;

    /// Stream parameters discovered during open/probe.
    fn stream_params(&self) -> StreamParams;

    /// Gapless metadata if present in the source.
    fn gapless_info(&self) -> Option<GaplessInfo>;
}
