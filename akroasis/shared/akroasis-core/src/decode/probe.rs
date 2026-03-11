// Format detection and decoder selection.
//
// P1-02 owns: SymphoniaDecoder path (all non-Opus formats).
// P1-03 owns: Opus routing (OpusDecoder) + WavPack rejection.

use std::path::Path;

use symphonia::core::codecs::{CODEC_TYPE_NULL, CODEC_TYPE_OPUS, CODEC_TYPE_WAVPACK};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::decode::opus::OpusDecoder;
use crate::decode::{AudioDecoder, Codec};
use crate::error::DecodeError;

/// Probes `path` and returns a boxed decoder appropriate for the detected format.
///
/// Routing:
/// - OGG/Opus  → `OpusDecoder` (Symphonia OGG demux + libopus FFI)
/// - WavPack   → `UnsupportedCodec` error (implement via wavpack-sys when needed)
/// - All others (FLAC, WAV, ALAC, MP3, AAC, AIFF, Vorbis) → `SymphoniaDecoder` (P1-02)
pub async fn open_decoder(path: &Path) -> Result<Box<dyn AudioDecoder>, DecodeError> {
    let probed = probe_format(path)?;
    let codec = probed.format.default_track().map(|t| t.codec_params.codec);

    match codec {
        Some(CODEC_TYPE_OPUS) => OpusDecoder::from_probed(probed),

        Some(CODEC_TYPE_WAVPACK) => Err(DecodeError::UnsupportedCodec {
            codec: Codec::Other("WavPack".to_string()),
            location: snafu::Location::new(file!(), line!(), column!()),
        }),

        _ => {
            // P1-02: construct SymphoniaDecoder from the already-probed result to avoid
            // re-probing. SymphoniaDecoder needs a `from_probed(ProbeResult)` constructor.
            todo!("P1-02: SymphoniaDecoder::from_probed(probed)")
        }
    }
}

/// Returns the codec for a file without fully opening a decoder. Useful for UI display.
pub async fn probe_codec(path: &Path) -> Result<Codec, DecodeError> {
    let probed = probe_format(path)?;

    let codec_type = probed
        .format
        .default_track()
        .map(|t| t.codec_params.codec)
        .unwrap_or(CODEC_TYPE_NULL);

    if codec_type == CODEC_TYPE_OPUS {
        return Ok(Codec::Opus);
    }

    // P1-02: map remaining CodecTypes via symphonia_codec_id_to_codec.
    todo!("P1-02: map remaining symphonia CodecType → crate::decode::Codec")
}

/// Opens `path` and runs Symphonia's format probe. Shared by `open_decoder` and `probe_codec`.
fn probe_format(path: &Path) -> Result<symphonia::core::probe::ProbeResult, DecodeError> {
    let file = std::fs::File::open(path).map_err(|e| DecodeError::SymphoniaRead {
        message: format!("failed to open {}: {e}", path.display()),
        location: snafu::Location::new(file!(), line!(), column!()),
    })?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| DecodeError::SymphoniaRead {
            message: format!("format probe failed for {}: {e}", path.display()),
            location: snafu::Location::new(file!(), line!(), column!()),
        })
}
