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
use crate::decode::symphonia::{SymphoniaDecoder, map_codec};
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
            let mss = MediaSourceStream::new(
                Box::new(
                    std::fs::File::open(path).map_err(|e| DecodeError::SymphoniaRead {
                        message: format!("failed to open {}: {e}", path.display()),
                        location: snafu::Location::new(file!(), line!(), column!()),
                    })?,
                ),
                Default::default(),
            );
            let hint = hint_from_path(path);
            let dec = SymphoniaDecoder::new(mss, &hint)?;
            Ok(Box::new(dec) as Box<dyn AudioDecoder>)
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

    Ok(map_codec(codec_type))
}

/// Opens `path` and runs Symphonia's format probe. Shared by `open_decoder` and `probe_codec`.
fn probe_format(path: &Path) -> Result<symphonia::core::probe::ProbeResult, DecodeError> {
    let file = std::fs::File::open(path).map_err(|e| DecodeError::SymphoniaRead {
        message: format!("failed to open {}: {e}", path.display()),
        location: snafu::Location::new(file!(), line!(), column!()),
    })?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = hint_from_path(path);

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

fn hint_from_path(path: &Path) -> Hint {
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    hint
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    /// Builds a minimal valid WAV and writes it to a tempfile.
    fn wav_tempfile(channels: u16, sample_rate: u32, samples: &[i16]) -> NamedTempFile {
        let data_len = (samples.len() * 2) as u32;
        let byte_rate = sample_rate * u32::from(channels) * 2;
        let block_align = channels * 2;

        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&block_align.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for &s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }

        let mut f = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        f.write_all(&v).unwrap();
        f
    }

    #[tokio::test]
    async fn probe_wav_returns_wav_codec() {
        let f = wav_tempfile(2, 44100, &[0i16; 4]);
        let codec = probe_codec(f.path()).await.unwrap();
        assert!(matches!(codec, Codec::Wav), "expected Wav, got {codec:?}");
    }

    #[tokio::test]
    async fn open_decoder_wav_streams_frames() {
        let f = wav_tempfile(2, 44100, &[0i16; 4]);
        let mut dec = open_decoder(f.path()).await.unwrap();
        let frame = dec.next_frame().await.unwrap_or_default();
        assert!(
            frame.is_some(),
            "expected at least one frame FROM 4-sample WAV"
        );
    }

    #[tokio::test]
    async fn open_decoder_empty_wav_returns_none() {
        let f = wav_tempfile(2, 44100, &[]);
        let mut dec = open_decoder(f.path()).await.unwrap();
        let frame = dec.next_frame().await.unwrap_or_default();
        assert!(frame.is_none(), "expected Ok(None) for empty WAV");
    }

    #[tokio::test]
    async fn missing_file_returns_err() {
        let result = open_decoder(Path::new("/nonexistent/file.wav")).await;
        assert!(result.is_err());
    }
}
