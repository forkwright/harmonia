// P1-02: Format detection and decoder selection.

use std::path::Path;

use crate::decode::{AudioDecoder, Codec};
use crate::error::DecodeError;

/// Probes `path` and returns a boxed decoder appropriate for the detected format.
///
/// Selection order:
/// 1. OGG Opus → SymphoniaDecoder (handles container + Opus packets)
/// 2. Raw Opus → OpusDecoder
/// 3. All other formats (FLAC, WAV, ALAC, MP3, AAC, AIFF, Vorbis) → SymphoniaDecoder
pub async fn open_decoder(
    _path: &Path,
) -> Result<Box<dyn AudioDecoder>, DecodeError> {
    todo!("P1-02: probe format via symphonia MediaSourceStream, select decoder")
}

/// Returns the codec for a file without fully opening a decoder. Useful for UI display.
pub async fn probe_codec(_path: &Path) -> Result<Codec, DecodeError> {
    todo!("P1-02: lightweight format probe — open, read header, close")
}
