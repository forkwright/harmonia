// P1-03: OpusDecoder  -  FFI bridge wrapping libopus via the `opus` crate.
//
// Symphonia's OGG demuxer extracts raw Opus packets; `opus::Decoder` decodes them.
// When Symphonia's native Opus decoder reaches production readiness (PR #398), this
// bridge can be removed without API change  -  it implements the same AudioDecoder trait.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use symphonia::core::codecs::{CODEC_TYPE_OPUS, CodecParameters};
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
use symphonia::core::probe::ProbeResult;
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
    format_reader: Box<dyn FormatReader>,
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
    pub fn from_probed(probed: ProbeResult) -> Result<Box<dyn AudioDecoder>, DecodeError> {
        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec == CODEC_TYPE_OPUS)
            .ok_or_else(|| DecodeError::OpusDecode {
                message: "no Opus track found in OGG container".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

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

        let time_base = codec_params.time_base.unwrap_or(TimeBase {
            numer: 1,
            denom: OPUS_SAMPLE_RATE,
        });

        let duration = codec_params.n_frames.map(|n| {
            let t = time_base.calc_time(n);
            Duration::from_secs_f64(t.seconds as f64 + t.frac)
        });

        let params = StreamParams {
            codec: Codec::Opus,
            sample_rate: OPUS_SAMPLE_RATE,
            channels,
            bit_depth: None,
            duration,
            bitrate: codec_params.bits_per_coded_sample.map(|b| b / 1000),
        };

        let gapless = build_gapless_info(&codec_params);

        let buf_samples = OPUS_MAX_FRAME_SAMPLES * usize::try_from(channels).unwrap_or_default();
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
                Ok(p) => p,
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

            if packet.track_id() != self.track_id {
                continue;
            }

            let timestamp = packet.ts();

            // An empty slice triggers Opus Packet Loss Concealment (PLC).
            let n_samples_per_channel = self
                .decoder
                .decode_float(packet.buf(), &mut self.decode_buf, false)
                .map_err(|e| DecodeError::OpusDecode {
                    message: format!("decode_float failed: {e}"),
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?;

            let channels = usize::try_from(self.params.channels).unwrap_or_default();
            let total = n_samples_per_channel * channels;

            // Widen f32 → f64. Cast is lossless for audio-range VALUES.
            for (i, &s) in self.decode_buf[..total].iter().enumerate() {
                self.output_buf[i] = f64::try_from(s).unwrap_or_default();
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
        let time = Time {
            seconds: position.as_secs(),
            frac: position.subsec_nanos() as f64 / 1e9,
        };

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

        let actual_time = self.time_base.calc_time(seeked.actual_ts);
        Ok(Duration::from_secs_f64(
            actual_time.seconds as f64 + actual_time.frac,
        ))
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

fn build_gapless_info(params: &CodecParameters) -> Option<GaplessInfo> {
    let delay = params.delay?;
    let padding = params.padding.unwrap_or(0);
    let total_samples = params
        .n_frames
        .map(|n| n.saturating_sub(u64::from(delay) + u64::from(padding)));
    Some(GaplessInfo {
        encoder_delay: delay,
        encoder_padding: padding,
        total_samples,
    })
}

#[cfg(test)]
mod tests {
    use symphonia::core::codecs::CodecParameters;

    use super::*;

    #[test]
    fn f32_to_f64_conversion_is_lossless_for_audio_range() {
        let samples: &[f32] = &[1.0, -1.0, 0.5, -0.5, 0.0, 0.123_456_79];
        for &s in samples {
            let widened = f64::try_from(s).unwrap_or_default();
            // The round-trip back to f32 must be identical.
            assert_eq!(widened as f32, s, "cast must round-trip for {s}");
        }
    }

    #[test]
    fn build_gapless_info_extracts_pre_skip_and_padding() {
        let mut params = CodecParameters::new();
        params.delay = Some(3840);
        params.padding = Some(120);
        params.n_frames = Some(2_257_920);

        let info = build_gapless_info(&params).unwrap();
        assert_eq!(info.encoder_delay, 3840);
        assert_eq!(info.encoder_padding, 120);
        assert_eq!(info.total_samples, Some(2_257_920 - 3840 - 120));
    }

    #[test]
    fn build_gapless_info_returns_none_without_delay() {
        let params = CodecParameters::new();
        assert!(build_gapless_info(&params).is_none());
    }

    #[test]
    fn build_gapless_info_padding_defaults_to_zero() {
        let mut params = CodecParameters::new();
        params.delay = Some(312);
        // padding intentionally not SET

        let info = build_gapless_info(&params).unwrap();
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
