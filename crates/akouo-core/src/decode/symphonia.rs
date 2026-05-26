use std::io::ErrorKind;
use std::pin::Pin;
use std::time::Duration;

use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::{AudioCodecId, AudioDecoderOptions, CODEC_ID_NULL_AUDIO};
use symphonia::core::errors::Error as SymphErr;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Time, TimeBase};
use tracing::{instrument, warn};

use crate::decode::{AudioDecoder, Codec, DecodedFrame, GaplessInfo, StreamParams};
use crate::error::DecodeError;

pub struct SymphoniaDecoder {
    format: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::audio::AudioDecoder>,
    track_id: u32,
    time_base: TimeBase,
    stream_params: StreamParams,
    gapless_info: Option<GaplessInfo>,
}

impl SymphoniaDecoder {
    /// Probes `mss` and creates a ready-to-decode instance.
    #[instrument(skip(mss))]
    pub fn new(mss: MediaSourceStream<'static>, hint: &Hint) -> Result<Self, DecodeError> {
        let format = symphonia::default::get_probe()
            .probe(
                hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| DecodeError::SymphoniaRead {
                message: format!("probe failed: {e}"),
                location: snafu::location!(),
            })?;

        let track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params
                    .as_ref()
                    .and_then(|c| c.audio())
                    .map(|a| a.codec != CODEC_ID_NULL_AUDIO)
                    .unwrap_or(false)
            })
            .ok_or_else(|| DecodeError::SymphoniaRead {
                message: "no audio track found".to_string(),
                location: snafu::location!(),
            })?;

        let track_id = track.id;
        let p = track
            .codec_params
            .as_ref()
            .and_then(|c| c.audio())
            .ok_or_else(|| DecodeError::SymphoniaRead {
                message: "track has no audio codec parameters".to_string(),
                location: snafu::location!(),
            })?;

        let codec = map_codec(p.codec);
        let gapless_info = extract_gapless(track, &codec);

        let sample_rate = p.sample_rate.unwrap_or(44100);
        let channels = p.channels.as_ref().map(|c| c.count() as u16).unwrap_or(2);
        let duration = track
            .num_frames
            .map(|n| Duration::from_secs_f64(n as f64 / sample_rate as f64));

        let stream_params = StreamParams {
            codec,
            sample_rate,
            channels,
            bit_depth: p.bits_per_sample.or(p.bits_per_coded_sample),
            duration,
            bitrate: None,
        };

        let time_base = track
            .time_base
            .or_else(|| TimeBase::try_from_recip(sample_rate))
            .or_else(|| TimeBase::try_new(1, 44100))
            .ok_or_else(|| DecodeError::SymphoniaRead {
                message: "failed to derive time base for audio stream".to_string(),
                location: snafu::location!(),
            })?;

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(p, &AudioDecoderOptions::default())
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
                Ok(Some(p)) => p,
                Ok(None) => return Ok(None),
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

            if packet.track_id != self.track_id {
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

            let timestamp = packet.pts.get().max(0) as u64;
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
        let time = Time::try_from_secs_f64(position.as_secs_f64()).unwrap_or(Time::ZERO);
        let seek_to = SeekTo::Time {
            time,
            track_id: Some(self.track_id),
        };

        let seeked = self.format.seek(SeekMode::Coarse, seek_to).map_err(|e| {
            DecodeError::SymphoniaRead {
                message: format!("seek failed: {e}"),
                location: snafu::location!(),
            }
        })?;

        self.decoder.reset();

        let t = self
            .time_base
            .calc_time(seeked.actual_ts)
            .unwrap_or(Time::ZERO);
        Ok(Duration::from_secs_f64(t.as_secs_f64()))
    }
}

/// Maps a symphonia `AudioCodecId` to the crate's `Codec` enum.
pub(crate) fn map_codec(ct: AudioCodecId) -> Codec {
    use symphonia::core::codecs::audio::well_known::*;
    match ct {
        CODEC_ID_FLAC => Codec::Flac,
        CODEC_ID_MP3 => Codec::Mp3,
        CODEC_ID_AAC => Codec::Aac,
        CODEC_ID_VORBIS => Codec::Vorbis,
        CODEC_ID_OPUS => Codec::Opus,
        CODEC_ID_ALAC => Codec::Alac,
        c if [
            CODEC_ID_PCM_S16LE,
            CODEC_ID_PCM_S24LE,
            CODEC_ID_PCM_S32LE,
            CODEC_ID_PCM_F32LE,
            CODEC_ID_PCM_S16BE,
            CODEC_ID_PCM_S24BE,
            CODEC_ID_PCM_S32BE,
        ]
        .contains(&c) =>
        {
            Codec::Wav
        }
        _ => Codec::Other(format!("{ct:?}")),
    }
}

fn extract_gapless(track: &symphonia::core::formats::Track, codec: &Codec) -> Option<GaplessInfo> {
    // WHY: Symphonia issue #418 — Vorbis pre-skip not parsed; hardcode standard value.
    if matches!(codec, Codec::Vorbis) {
        return Some(GaplessInfo {
            encoder_delay: 3456,
            encoder_padding: 0,
            total_samples: track.num_frames,
        });
    }

    // Lossless codecs have no encoder delay.
    if matches!(codec, Codec::Flac | Codec::Wav | Codec::Aiff) {
        return None;
    }

    if track.delay.is_some() || track.padding.is_some() {
        Some(GaplessInfo {
            encoder_delay: track.delay.unwrap_or(0),
            encoder_padding: track.padding.unwrap_or(0),
            total_samples: track.num_frames,
        })
    } else {
        None
    }
}

fn buffer_to_f64_interleaved(buf: &GenericAudioBufferRef<'_>) -> Vec<f64> {
    let mut out = Vec::with_capacity(buf.frames() * buf.num_planes());
    buf.copy_to_vec_interleaved(&mut out);
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
        assert!(l < -0.999, "LEFT should be approx -1.0, got {l}");
        assert!(r > 0.999, "RIGHT should be approx +1.0, got {r}");
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
