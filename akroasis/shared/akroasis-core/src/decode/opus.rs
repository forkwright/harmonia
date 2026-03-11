// P1-03: OpusDecoder — pure Rust Opus decoding via the `opus` crate.

use std::time::Duration;

use crate::decode::{AudioDecoder, DecodedFrame, GaplessInfo, StreamParams};
use crate::error::DecodeError;

/// Opus decoder for raw Opus frames (not wrapped in OGG — use SymphoniaDecoder for OGG Opus).
pub struct OpusDecoder {
    _private: (),
}

impl OpusDecoder {
    pub async fn open<R>(_reader: R) -> Result<Self, DecodeError>
    where
        R: std::io::Read + Send + 'static,
    {
        todo!("P1-03: initialise opus::Decoder at stream sample rate")
    }
}

impl AudioDecoder for OpusDecoder {
    async fn next_frame(&mut self) -> Result<Option<DecodedFrame>, DecodeError> {
        todo!("P1-03: read Opus packet, decode to f32, convert to f64")
    }

    async fn seek(&mut self, _position: Duration) -> Result<Duration, DecodeError> {
        todo!("P1-03: Opus seek (OGG page granule position)")
    }

    fn stream_params(&self) -> StreamParams {
        todo!("P1-03: return StreamParams — Opus is always 48 kHz internally")
    }

    fn gapless_info(&self) -> Option<GaplessInfo> {
        todo!("P1-03: read R128_TRACK_GAIN and pre-skip from OpusHead")
    }
}
