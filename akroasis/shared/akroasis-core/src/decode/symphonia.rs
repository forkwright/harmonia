// P1-02: SymphoniaDecoder — multi-codec decode via symphonia.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::decode::{AudioDecoder, Codec, DecodedFrame, GaplessInfo, StreamParams};
use crate::error::DecodeError;

/// Symphonia-backed decoder supporting FLAC, WAV, ALAC, AIFF, MP3, AAC, Vorbis, and OGG.
pub struct SymphoniaDecoder {
    _private: (),
}

impl SymphoniaDecoder {
    /// Opens and probes `reader` to create a ready-to-decode instance.
    pub async fn open<R>(_reader: R) -> Result<Self, DecodeError>
    where
        R: std::io::Read + std::io::Seek + Send + 'static,
    {
        todo!("P1-02: open symphonia format reader, select track, build decoder pipeline")
    }
}

impl AudioDecoder for SymphoniaDecoder {
    fn next_frame(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DecodedFrame>, DecodeError>> + Send + '_>> {
        todo!("P1-02: decode next packet via symphonia, convert samples to f64")
    }

    fn seek(
        &mut self,
        _position: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Duration, DecodeError>> + Send + '_>> {
        todo!("P1-02: symphonia seek_to_time, return actual seeked timestamp")
    }

    fn stream_params(&self) -> StreamParams {
        todo!("P1-02: return StreamParams from probed format/track info")
    }

    fn gapless_info(&self) -> Option<GaplessInfo> {
        todo!("P1-02: extract iTunSMPB / LAME header gapless metadata via lofty")
    }
}

// Codec detection helper used by probe.rs (P1-02).
#[expect(dead_code)]
pub(crate) fn symphonia_codec_id_to_codec(
    _codec: symphonia::core::codecs::CodecType,
) -> Option<Codec> {
    todo!("P1-02: map symphonia CodecType to crate::decode::Codec")
}
