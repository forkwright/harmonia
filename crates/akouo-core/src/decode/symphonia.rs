use std::io::ErrorKind;
use std::pin::Pin;
use std::time::Duration;

use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymphErr;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::{Time, TimeBase};
use tracing::{instrument, warn};

use crate::decode::{AudioDecoder, Codec, DecodedFrame, GaplessInfo, StreamParams};
use crate::error::DecodeError;

pub struct SymphoniaDecoder {
    format: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    time_base: TimeBase,
    stream_params: StreamParams,
    gapless_info: Option<GaplessInfo>,
}

impl SymphoniaDecoder {
    /// Probes `mss` and creates a ready-to-decode instance.
    #[instrument(skip(mss))]
    pub fn new(mss: MediaSourceStream, hint: &Hint) -> Result<Self, DecodeError> {
        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let probed = symphonia::default::get_probe()
            .format(hint, mss, &format_opts, &MetadataOptions::default())
            .map_err(|e| DecodeError::SymphoniaRead {
                message: format!("probe failed: {e}"),
                location: snafu::location!(),
            })?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| DecodeError::SymphoniaRead {
                message: "no audio track found".to_string(),
                location: snafu::location!(),
            })?;

        let track_id = track.id;
        let codec = map_codec(track.codec_params.codec);

        let gapless_info = extract_gapless(track, &codec);

        let p = &track.codec_params;
        let sample_rate = p.sample_rate.unwrap_or(44100);
        let channels = p.channels.map(|c| c.count() as u16).unwrap_or(2);
        let duration = p
            .n_frames
            .map(|n| Duration::from_secs_f64(n as f64 / sample_rate as f64));

        let stream_params = StreamParams {
            codec,
            sample_rate,
            channels,
            bit_depth: p.bits_per_sample.or(p.bits_per_coded_sample),
            duration,
            bitrate: None,
        };

        let time_base = p.time_base.unwrap_or(TimeBase {
            numer: 1,
            denom: sample_rate,
        });

        let decoder = symphonia::default::get_codecs()
            .make(p, &DecoderOptions::default())
            .map_err(|e| DecodeError::SymphoniaRead {
                message: format!("decoder init failed: {e}"),
                location: snafu::location!(),
            })?;

        Ok(Self {
            format,
            decoder,
            track_id,
            time_base,
            stream_params,
            gapless_info,
        })
    }
}

impl AudioDecoder for SymphoniaDecoder {
    fn next_frame(
        &mut self,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Option<DecodedFrame>, DecodeError>> + Send + '_,
        >,
    > {
        Box::pin(async move { self.decode_next_frame() })
    }

    fn seek(
        &mut self,
        position: Duration,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Duration, DecodeError>> + Send + '_>> {
        Box::pin(async move { self.seek_to(position) })
    }

    fn stream_params(&self) -> StreamParams {
        self.stream_params.clone()
    }

    fn gapless_info(&self) -> Option<GaplessInfo> {
        self.gapless_info.clone()
    }
}

impl SymphoniaDecoder {
    fn decode_next_frame(&mut self) -> Result<Option<DecodedFrame>, DecodeError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphErr::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(SymphErr::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => {
                    return Err(DecodeError::SymphoniaRead {
                        message: e.to_string(),
                        location: snafu::location!(),
                    });
                }
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let buffer = match self.decoder.decode(&packet) {
                Ok(buf) => buf,
                Err(SymphErr::DecodeError(msg)) => {
                    warn!(message = %msg, "skipping corrupt frame");
                    continue;
                }
                Err(e) => {
                    return Err(DecodeError::SymphoniaDecode {
                        message: e.to_string(),
                        location: snafu::location!(),
                    });
                }
            };

            let timestamp = packet.ts;
            let channels = self.stream_params.channels;
            let sample_rate = self.stream_params.sample_rate;
            let samples = buffer_to_f64_interleaved(&buffer);

            return Ok(Some(DecodedFrame {
                samples: samples.into_boxed_slice(),
                channels,
                sample_rate,
                timestamp,
            }));
        }
    }

    fn seek_to(&mut self, position: Duration) -> Result<Duration, DecodeError> {
        let seek_to = SeekTo::Time {
            time: Time::from(position.as_secs_f64()),
            track_id: Some(self.track_id),
        };

        let seeked = self.format.seek(SeekMode::Coarse, seek_to).map_err(|e| {
            DecodeError::SymphoniaRead {
                message: format!("seek failed: {e}"),
                location: snafu::location!(),
            }
        })?;

        self.decoder.reset();

        let t = self.time_base.calc_time(seeked.actual_ts);
        Ok(Duration::from_secs_f64(t.seconds as f64 + t.frac))
    }
}

/// Maps a symphonia `CodecType` to the crate's `Codec` enum.
pub(crate) fn map_codec(ct: symphonia::core::codecs::CodecType) -> Codec {
    use symphonia::core::codecs::*;
    match ct {
        CODEC_TYPE_FLAC => Codec::Flac,
        CODEC_TYPE_MP3 => Codec::Mp3,
        CODEC_TYPE_AAC => Codec::Aac,
        CODEC_TYPE_VORBIS => Codec::Vorbis,
        CODEC_TYPE_OPUS => Codec::Opus,
        CODEC_TYPE_ALAC => Codec::Alac,
        CODEC_TYPE_PCM_S16LE | CODEC_TYPE_PCM_S24LE | CODEC_TYPE_PCM_S32LE
        | CODEC_TYPE_PCM_F32LE | CODEC_TYPE_PCM_S16BE | CODEC_TYPE_PCM_S24BE
        | CODEC_TYPE_PCM_S32BE => Codec::Wav,
        _ => Codec::Other(format!("{ct:?}")),
    }
}

fn extract_gapless(track: &symphonia::core::formats::Track, codec: &Codec) -> Option<GaplessInfo> {
    // Symphonia issue #418: Vorbis pre-skip is not parsed  -  hardcode the standard value.
    if matches!(codec, Codec::Vorbis) {
        return Some(GaplessInfo {
            encoder_delay: 3456,
            encoder_padding: 0,
            total_samples: track.codec_params.n_frames,
        });
    }

    // Lossless codecs have no encoder delay.
    if matches!(codec, Codec::Flac | Codec::Wav | Codec::Aiff) {
        return None;
    }

    let p = &track.codec_params;
    if p.delay.is_some() || p.padding.is_some() {
        Some(GaplessInfo {
            encoder_delay: p.delay.unwrap_or(0),
            encoder_padding: p.padding.unwrap_or(0),
            total_samples: p.n_frames,
        })
    } else {
        None
    }
}

fn buffer_to_f64_interleaved(buf: &AudioBufferRef<'_>) -> Vec<f64> {
    let n_channels = buf.spec().channels.count();
    let n_frames = buf.frames();
    let mut out = vec![0.0f64; n_frames * n_channels];

    macro_rules! interleave {
        ($b:expr, $convert:expr) => {{
            for (ch, plane) in $b.planes().planes().iter().enumerate() {
                for (frame, &s) in plane.iter().enumerate() {
                    out[frame * n_channels + ch] = $convert(s);
                }
            }
        }};
    }

    match buf {
        AudioBufferRef::U8(b) => {
            interleave!(b, |s: u8| (f64::from(s) - 128.0) / 128.0)
        }
        AudioBufferRef::U16(b) => {
            interleave!(b, |s: u16| (f64::from(s) - 32768.0) / 32768.0)
        }
        AudioBufferRef::U24(b) => {
            interleave!(b, |s: symphonia::core::sample::u24| {
                (s.inner() as f64 - 8_388_608.0) / 8_388_608.0
            })
        }
        AudioBufferRef::U32(b) => {
            interleave!(b, |s: u32| (f64::from(s) - 2_147_483_648.0) / 2_147_483_648.0)
        }
        AudioBufferRef::S8(b) => {
            interleave!(b, |s: i8| f64::from(s) / 128.0)
        }
        AudioBufferRef::S16(b) => {
            interleave!(b, |s: i16| f64::from(s) / 32_768.0)
        }
        AudioBufferRef::S24(b) => {
            interleave!(b, |s: symphonia::core::sample::i24| {
                s.inner() as f64 / 8_388_608.0
            })
        }
        AudioBufferRef::S32(b) => {
            interleave!(b, |s: i32| f64::from(s) / 2_147_483_648.0)
        }
        AudioBufferRef::F32(b) => interleave!(b, |s: f32| f64::from(s)),
        AudioBufferRef::F64(b) => interleave!(b, |s: f64| s),
    }

    out
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use symphonia::core::io::ReadOnlySource;

    use super::*;

    /// Builds a minimal valid WAV in memory.
    fn wav_bytes(channels: u16, sample_rate: u32, samples_i16: &[i16]) -> Vec<u8> {
        let data_len = (samples_i16.len() * 2) as u32;
        let byte_rate = sample_rate * u32::from(channels) * 2;
        let block_align = channels * 2;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&block_align.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for &s in samples_i16 {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    fn decoder_from_wav(wav: Vec<u8>) -> SymphoniaDecoder {
        let cursor = Cursor::new(wav);
        let source = ReadOnlySource::new(cursor);
        let mss = MediaSourceStream::new(Box::new(source), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("wav");
        SymphoniaDecoder::new(mss, &hint).unwrap()
    }

    // --- Normalization unit tests (no I/O needed) ---

    #[test]
    fn normalize_i16_zero() {
        assert_eq!(0i16 as f64 / 32_768.0, 0.0);
    }

    #[test]
    fn normalize_i16_min_is_neg_one() {
        let v = i16::MIN as f64 / 32_768.0;
        assert_eq!(v, -1.0);
    }

    #[test]
    fn normalize_i16_max_near_one() {
        let v = i16::MAX as f64 / 32_768.0;
        assert!(v > 0.999 && v <= 1.0, "i16 max normalized = {v}");
    }

    #[test]
    fn normalize_i32_min_is_neg_one() {
        let v = i32::MIN as f64 / 2_147_483_648.0;
        assert_eq!(v, -1.0);
    }

    #[test]
    fn normalize_i32_max_near_one() {
        let v = i32::MAX as f64 / 2_147_483_648.0;
        assert!(v > 0.999 && v <= 1.0, "i32 max normalized = {v}");
    }

    #[test]
    fn normalize_i32_quarter_scale() {
        // 2^29 / 2^31 = 0.25
        let v = (1i32 << 29) as f64 / 2_147_483_648.0;
        assert!((v - 0.25).abs() < 1e-9);
    }

    #[test]
    fn normalize_f32_passthrough() {
        let v: f64 = f64::from(0.5f32);
        assert!((v - 0.5).abs() < f64::EPSILON);
    }

    // --- Decode loop tests ---

    #[tokio::test]
    async fn empty_wav_returns_ok_none() {
        let wav = wav_bytes(2, 44100, &[]);
        let mut dec = decoder_from_wav(wav);
        let result = dec.next_frame().await.unwrap_or_default();
        assert!(result.is_none(), "expected Ok(None) for empty stream");
    }

    #[tokio::test]
    async fn wav_stream_params_populated() {
        let wav = wav_bytes(2, 44100, &[0i16; 4]);
        let dec = decoder_from_wav(wav);
        let p = dec.stream_params();
        assert_eq!(p.sample_rate, 44100);
        assert_eq!(p.channels, 2);
        assert_eq!(p.bit_depth, Some(16));
        assert!(matches!(p.codec, Codec::Wav));
    }

    #[tokio::test]
    async fn wav_decodes_first_frame() {
        // 4 interleaved stereo samples: [0x7FFF, 0x7FFF, 0, 0]
        let samples: &[i16] = &[i16::MAX, i16::MAX, 0, 0];
        let wav = wav_bytes(2, 44100, samples);
        let mut dec = decoder_from_wav(wav);
        let frame = dec.next_frame().await.unwrap().unwrap();
        assert_eq!(frame.channels, 2);
        assert_eq!(frame.sample_rate, 44100);
        assert!(!frame.samples.is_empty());
        // First sample pair should be near +1.0
        let l = frame.samples.first().copied().unwrap_or_default();
        let r = frame.samples.get(1).copied().unwrap_or_default();
        assert!(l > 0.999, "LEFT channel = {l}");
        assert!(r > 0.999, "RIGHT channel = {r}");
    }

    #[tokio::test]
    async fn interleave_ordering_l_then_r() {
        // Left = i16::MIN (-1.0), Right = i16::MAX (~1.0)
        let samples: &[i16] = &[i16::MIN, i16::MAX];
        let wav = wav_bytes(2, 44100, samples);
        let mut dec = decoder_from_wav(wav);
        let frame = dec.next_frame().await.unwrap().unwrap();
        let l = frame.samples.first().copied().unwrap_or_default();
        let r = frame.samples.get(1).copied().unwrap_or_default();
        assert!(l < -0.999, "LEFT should be ≈ -1.0, got {l}");
        assert!(r > 0.999, "RIGHT should be ≈ +1.0, got {r}");
    }

    #[tokio::test]
    async fn seek_returns_duration() {
        let samples: Vec<i16> = vec![0i16; 44100 * 2 * 2]; // 1s stereo
        let wav = wav_bytes(2, 44100, &samples);
        let mut dec = decoder_from_wav(wav);
        let target = Duration::from_millis(500);
        let actual = dec.seek(target).await.unwrap_or_default();
        // Coarse seek  -  should be within 500ms of requested
        assert!(actual.as_millis() < 600, "seek overshot: {actual:?}");
    }

    #[tokio::test]
    async fn gapless_none_for_wav() {
        let wav = wav_bytes(2, 44100, &[]);
        let dec = decoder_from_wav(wav);
        assert!(dec.gapless_info().is_none());
    }
}
