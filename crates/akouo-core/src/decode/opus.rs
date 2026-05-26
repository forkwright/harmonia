// P1-03: OpusDecoder  -  FFI bridge wrapping libopus via the `opus` crate.
//
// Symphonia's OGG demuxer extracts raw Opus packets; `opus::Decoder` decodes them.
// When Symphonia's native Opus decoder reaches production readiness (PR #398), this
// bridge can be removed without API change  -  it implements the same AudioDecoder trait.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use symphonia::core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};

use symphonia::core::units::{Time, TimeBase};

use crate::decode::{AudioDecoder, Codec, DecodedFrame, GaplessInfo, StreamParams};
use crate::error::DecodeError;

/// Opus is always 48 kHz internally.
const OPUS_SAMPLE_RATE: u32 = 48_000;

/// Maximum Opus frame size: 120 ms at 48 kHz per channel.
const OPUS_MAX_FRAME_SAMPLES: usize = 5_760;

/// Opus FFI decoder. Demuxes OGG via Symphonia, decodes packets via libopus.
///
/// All I/O is synchronous (std file reads); the `Pin<Box<dyn Future>>` wrappers
/// return `std::future::ready(result)` so the caller can await without blocking.
pub struct OpusDecoder {
    decoder: opus::Decoder,
    format_reader: Box<dyn FormatReader + 'static>,
    track_id: u32,
    params: StreamParams,
    gapless: Option<GaplessInfo>,
    time_base: TimeBase,
    /// Pre-allocated f32 output FROM `opus::Decoder::decode_float` (interleaved channels).
    decode_buf: Box<[f32]>,
    /// Widened f64 copy for the internal pipeline.
    output_buf: Box<[f64]>,
}

impl OpusDecoder {
    /// Constructs an `OpusDecoder` FROM an already-probed Symphonia `ProbeResult`.
    ///
    /// The caller (probe.rs) does the format detection; this constructor takes
    /// ownership and sets up the libopus decoder for the OGG/Opus track.
    pub fn from_probed(
        format: Box<dyn symphonia::core::formats::FormatReader + 'static>,
    ) -> Result<Box<dyn AudioDecoder>, DecodeError> {
        let track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params
                    .as_ref()
                    .and_then(|c| c.audio())
                    .map(|a| a.codec == CODEC_ID_OPUS)
                    .unwrap_or(false)
            })
            .ok_or_else(|| DecodeError::OpusDecode {
                message: "no Opus track found in OGG container".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        let track_id = track.id;
        let num_frames = track.num_frames;
        let track_time_base = track.time_base;
        let track_delay = track.delay;
        let track_padding = track.padding;
        let codec_params = track
            .codec_params
            .as_ref()
            .and_then(|c| c.audio())
            .cloned()
            .unwrap_or_default();

        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        let opus_channels = if channels == 1 {
            opus::Channels::Mono
        } else {
            opus::Channels::Stereo
        };

        let decoder = opus::Decoder::new(OPUS_SAMPLE_RATE, opus_channels).map_err(|e| {
            DecodeError::OpusDecode {
                message: format!("failed to initialise libopus decoder: {e}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            }
        })?;

        let time_base = track_time_base.unwrap_or_else(|| {
            TimeBase::try_from_recip(OPUS_SAMPLE_RATE).expect("OPUS_SAMPLE_RATE is non-zero")
        });

        let duration =
            num_frames.map(|n| Duration::from_secs_f64(n as f64 / OPUS_SAMPLE_RATE as f64));

        let params = StreamParams {
            codec: Codec::Opus,
            sample_rate: OPUS_SAMPLE_RATE,
            channels,
            bit_depth: None,
            duration,
            bitrate: codec_params.bits_per_coded_sample.map(|b| b / 1000),
        };

        let gapless = build_gapless_info(track_delay, track_padding, num_frames);

        let buf_samples = OPUS_MAX_FRAME_SAMPLES * usize::from(channels);
        let decode_buf = vec![0.0f32; buf_samples].into_boxed_slice();
        let output_buf = vec![0.0f64; buf_samples].into_boxed_slice();

        Ok(Box::new(Self {
            decoder,
            format_reader: format,
            track_id,
            params,
            gapless,
            time_base,
            decode_buf,
            output_buf,
        }))
    }

    fn decode_next_packet(&mut self) -> Result<Option<DecodedFrame>, DecodeError> {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return Ok(None),
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(e) => {
                    return Err(DecodeError::SymphoniaRead {
                        message: format!("OGG read error: {e}"),
                        location: snafu::Location::new(file!(), line!(), column!()),
                    });
                }
            };

            if packet.track_id != self.track_id {
                continue;
            }

            let timestamp = packet.pts.get().max(0) as u64;

            // An empty slice triggers Opus Packet Loss Concealment (PLC).
            let n_samples_per_channel = self
                .decoder
                .decode_float(packet.data.as_ref(), &mut self.decode_buf, false)
                .map_err(|e| DecodeError::OpusDecode {
                    message: format!("decode_float failed: {e}"),
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?;

            let channels = usize::from(self.params.channels);
            let total = n_samples_per_channel * channels;

            // Widen f32 → f64. Cast is lossless for audio-range VALUES.
            for (i, &s) in self.decode_buf[..total].iter().enumerate() {
                self.output_buf[i] = f64::from(s);
            }

            return Ok(Some(DecodedFrame {
                samples: self.output_buf[..total].to_vec().into_boxed_slice(),
                channels: self.params.channels,
                sample_rate: OPUS_SAMPLE_RATE,
                timestamp,
            }));
        }
    }

    fn do_seek(&mut self, position: Duration) -> Result<Duration, DecodeError> {
        let time = Time::try_from_secs_f64(position.as_secs_f64()).unwrap_or(Time::ZERO);

        let seeked = self
            .format_reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| DecodeError::SymphoniaRead {
                message: format!("seek failed: {e}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        // Recreate decoder to clear internal state after seek.
        let opus_channels = if self.params.channels == 1 {
            opus::Channels::Mono
        } else {
            opus::Channels::Stereo
        };
        self.decoder = opus::Decoder::new(OPUS_SAMPLE_RATE, opus_channels).map_err(|e| {
            DecodeError::OpusDecode {
                message: format!("decoder reset after seek failed: {e}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            }
        })?;

        let secs = self
            .time_base
            .calc_time(seeked.actual_ts)
            .map(|t| t.as_secs_f64())
            .unwrap_or(0.0);
        Ok(Duration::from_secs_f64(secs))
    }
}

impl AudioDecoder for OpusDecoder {
    fn next_frame(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DecodedFrame>, DecodeError>> + Send + '_>> {
        Box::pin(std::future::ready(self.decode_next_packet()))
    }

    fn seek(
        &mut self,
        position: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Duration, DecodeError>> + Send + '_>> {
        Box::pin(std::future::ready(self.do_seek(position)))
    }

    fn stream_params(&self) -> StreamParams {
        self.params.clone()
    }

    fn gapless_info(&self) -> Option<GaplessInfo> {
        self.gapless.clone()
    }
}

fn build_gapless_info(
    delay: Option<u32>,
    padding: Option<u32>,
    n_frames: Option<u64>,
) -> Option<GaplessInfo> {
    let delay = delay?;
    let padding = padding.unwrap_or(0);
    let total_samples = n_frames.map(|n| n.saturating_sub(u64::from(delay) + u64::from(padding)));
    Some(GaplessInfo {
        encoder_delay: delay,
        encoder_padding: padding,
        total_samples,
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn f32_to_f64_conversion_is_lossless_for_audio_range() {
        let samples: &[f32] = &[1.0, -1.0, 0.5, -0.5, 0.0, 0.123_456_79];
        for &s in samples {
            let widened = f64::from(s);
            // The round-trip back to f32 must be identical.
            assert_eq!(widened as f32, s, "cast must round-trip for {s}");
        }
    }

    #[test]
    fn build_gapless_info_extracts_pre_skip_and_padding() {
        let info = build_gapless_info(Some(3840), Some(120), Some(2_257_920)).unwrap();
        assert_eq!(info.encoder_delay, 3840);
        assert_eq!(info.encoder_padding, 120);
        assert_eq!(info.total_samples, Some(2_257_920 - 3840 - 120));
    }

    #[test]
    fn build_gapless_info_returns_none_without_delay() {
        assert!(build_gapless_info(None, None, None).is_none());
    }

    #[test]
    fn build_gapless_info_padding_defaults_to_zero() {
        let info = build_gapless_info(Some(312), None, None).unwrap();
        assert_eq!(info.encoder_padding, 0);
        assert_eq!(info.total_samples, None); // n_frames not SET
    }

    #[test]
    fn opus_plc_decode_produces_concealment_audio() {
        // Passing an empty slice to decode_float triggers Opus Packet Loss Concealment.
        let mut dec = opus::Decoder::new(48_000, opus::Channels::Stereo).unwrap();
        let mut buf = vec![0.0f32; OPUS_MAX_FRAME_SAMPLES * 2];
        let result = dec.decode_float(&[], &mut buf, false);
        assert!(result.is_ok(), "PLC decode must not error: {result:?}");
        assert!(result.unwrap() > 0, "PLC must produce samples");
    }
}
