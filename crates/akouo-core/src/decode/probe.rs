// Format detection and decoder selection.
//
// P1-02 owns: SymphoniaDecoder path (all non-Opus formats).
// P1-03 owns: Opus routing (OpusDecoder) + WavPack rejection.

use std::path::Path;

use symphonia::core::codecs::audio::{
    CODEC_ID_NULL_AUDIO,
    well_known::{CODEC_ID_OPUS, CODEC_ID_WAVPACK},
};
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::decode::blocking::BlockingDecoder;
use crate::decode::opus::OpusDecoder;
use crate::decode::symphonia::{SymphoniaDecoder, map_codec};
use crate::decode::{AudioDecoder, Codec, SyncAudioDecoder};
use crate::error::DecodeError;

/// Probes `path` and returns a boxed decoder appropriate for the detected format.
///
/// Routing:
/// - OGG/Opus  → `OpusDecoder` (Symphonia OGG demux + libopus FFI)
/// - WavPack   → `UnsupportedCodec` error (implement via wavpack-sys when needed)
/// - All others (FLAC, WAV, ALAC, MP3, AAC, AIFF, Vorbis) → `SymphoniaDecoder` (P1-02)
///
/// The returned decoder is a `BlockingDecoder` wrapper: all subsequent decode and seek
/// I/O runs on a dedicated decode thread, never on the async executor.
pub async fn open_decoder(path: &Path) -> Result<Box<dyn AudioDecoder>, DecodeError> {
    let path = path.to_path_buf();
    // WHY: probe_format opens std::fs::File (blocking); spawn_blocking prevents stalling
    // the async executor thread during disk I/O on the open/probe hot path.
    let sync_decoder = tokio::task::spawn_blocking(move || {
        let probed = probe_format(&path)?;
        let codec = probed
            .default_track(symphonia::core::formats::TrackType::Audio)
            .and_then(|t| t.codec_params.as_ref())
            .and_then(|c| c.audio())
            .map(|a| a.codec);

        match codec {
            Some(CODEC_ID_OPUS) => OpusDecoder::from_probed(probed),

            Some(CODEC_ID_WAVPACK) => Err(DecodeError::UnsupportedCodec {
                codec: Codec::Other("WavPack".to_string()),
                location: snafu::Location::new(file!(), line!(), column!()),
            }),

            _ => {
                let mss = MediaSourceStream::new(
                    Box::new(std::fs::File::open(&path).map_err(|e| {
                        DecodeError::SymphoniaRead {
                            message: format!("failed to open {}: {e}", path.display()),
                            location: snafu::Location::new(file!(), line!(), column!()),
                        }
                    })?),
                    Default::default(),
                );
                let hint = hint_from_path(&path);
                let dec = SymphoniaDecoder::new(mss, &hint)?;
                Ok(Box::new(dec) as Box<dyn SyncAudioDecoder>)
            }
        }
    })
    .await
    .map_err(|e| DecodeError::TaskJoin {
        message: format!("decoder probe task failed to join: {e}"),
        location: snafu::location!(),
    })
    .and_then(|r| r)?;

    Ok(Box::new(BlockingDecoder::spawn(sync_decoder)?) as Box<dyn AudioDecoder>)
}

/// Returns the codec for a file without fully opening a decoder. Useful for UI display.
pub async fn probe_codec(path: &Path) -> Result<Codec, DecodeError> {
    let path = path.to_path_buf();
    // WHY(#403): probe_format opens std::fs::File (blocking); spawn_blocking keeps the
    // probe I/O off the async executor, mirroring open_decoder.
    tokio::task::spawn_blocking(move || {
        let probed = probe_format(&path)?;

        let codec_type = probed
            .default_track(symphonia::core::formats::TrackType::Audio)
            .and_then(|t| t.codec_params.as_ref())
            .and_then(|c| c.audio())
            .map(|a| a.codec)
            .unwrap_or(CODEC_ID_NULL_AUDIO);

        Ok(map_codec(codec_type))
    })
    .await
    .map_err(|e| DecodeError::TaskJoin {
        message: format!("codec probe task failed to join: {e}"),
        location: snafu::location!(),
    })
    .and_then(|r| r)
}

/// Opens `path` and runs Symphonia's format probe. Shared by `open_decoder` and `probe_codec`.
fn probe_format(
    path: &Path,
) -> Result<Box<dyn symphonia::core::formats::FormatReader + 'static>, DecodeError> {
    let file = std::fs::File::open(path).map_err(|e| DecodeError::SymphoniaRead {
        message: format!("failed to open {}: {e}", path.display()),
        location: snafu::Location::new(file!(), line!(), column!()),
    })?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = hint_from_path(path);

    symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
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

    // --- #403: blocking I/O must never run on the async executor ---

    /// On a current_thread runtime a task spawned before an await only runs if that
    /// await actually yields. The old probe_codec ran its blocking I/O inline in the
    /// async fn body (ready on first poll — no yield), starving other tasks.
    #[tokio::test]
    async fn probe_codec_yields_to_executor() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let f = wav_tempfile(2, 44100, &[0i16; 64]);
        let flag = Arc::new(AtomicBool::new(false));
        let flag_task = Arc::clone(&flag);
        tokio::spawn(async move {
            flag_task.store(true, Ordering::SeqCst);
        });

        let codec = probe_codec(f.path()).await.unwrap();
        assert!(matches!(codec, Codec::Wav));
        assert!(
            flag.load(Ordering::SeqCst),
            "executor was starved during probe I/O — blocking work ran inline"
        );
    }

    /// The old decoder next_frame wrappers returned already-ready futures whose blocking
    /// work ran inline, so a full decode loop never yielded to the scheduler. With the
    /// dedicated decode thread, every next_frame awaits a cross-thread reply and yields.
    #[tokio::test]
    async fn next_frame_yields_to_executor() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        // 5s stereo — several hundred packets, far more than the loop bound needs.
        let samples: Vec<i16> = vec![0i16; 44100 * 2 * 5];
        let f = wav_tempfile(2, 44100, &samples);
        let mut dec = open_decoder(f.path()).await.unwrap();

        let flag = Arc::new(AtomicBool::new(false));
        let flag_task = Arc::clone(&flag);
        tokio::spawn(async move {
            flag_task.store(true, Ordering::SeqCst);
        });

        let mut yielded = false;
        for _ in 0..1000 {
            let frame = dec.next_frame().await.unwrap();
            if flag.load(Ordering::SeqCst) {
                yielded = true;
                break;
            }
            if frame.is_none() {
                break;
            }
        }
        assert!(
            yielded,
            "decode loop never yielded to the executor — blocking work ran inline"
        );
    }
}
