// P1-03: OpusDecoder  -  FFI bridge wrapping libopus via the `opusic-c` crate.
//
// Symphonia's OGG demuxer extracts raw Opus packets; libopus decodes them  -
// `opusic_c::Decoder` for mono/stereo streams, `opusic_c::multistream::Decoder` for
// mapping-family-1 multichannel streams (RFC 7845 §5.1.1.2).
// When Symphonia's native Opus decoder reaches production readiness (PR #398), this
// bridge can be removed without API change  -  it implements the same SyncAudioDecoder
// trait.

use std::time::Duration;

use symphonia::core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};

use symphonia::core::units::{Time, TimeBase};

use crate::decode::{Codec, DecodedFrame, GaplessInfo, StreamParams, SyncAudioDecoder};
use crate::error::DecodeError;

/// Opus is always 48 kHz internally.
const OPUS_SAMPLE_RATE: u32 = 48_000;

/// Maximum Opus frame size: 120 ms at 48 kHz per channel.
const OPUS_MAX_FRAME_SAMPLES: usize = 5_760;

/// Maximum channel count for mapping family 1 (RFC 7845 §5.1.1.2, Vorbis channel order).
const OPUS_MAX_MAPPED_CHANNELS: usize = 8;

/// Opus FFI decoder. Demuxes OGG via Symphonia, decodes packets via libopus.
///
/// All I/O is synchronous (std file reads); the decoder implements `SyncAudioDecoder`
/// and is driven on a dedicated decode thread by `blocking::BlockingDecoder`.
pub struct OpusDecoder {
    decoder: OpusInner,
    format_reader: Box<dyn FormatReader + 'static>,
    track_id: u32,
    params: StreamParams,
    gapless: Option<GaplessInfo>,
    time_base: TimeBase,
    /// Pre-allocated f32 output FROM libopus float decode (interleaved channels).
    decode_buf: Box<[f32]>,
    /// Widened f64 copy for the internal pipeline.
    output_buf: Box<[f64]>,
}

/// Channel configuration resolved FROM the OpusHead identification header.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ChannelLayout {
    /// Mapping family 0: single elementary stream, mono or stereo.
    Single(opusic_c::Channels),
    /// Mapping family 1: multiple elementary streams with an explicit channel-mapping table.
    Multi {
        channels: u8,
        streams: u8,
        coupled_streams: u8,
        /// Channel-mapping table; only the first `channels` entries are meaningful.
        mapping: [u8; OPUS_MAX_MAPPED_CHANNELS],
    },
}

impl ChannelLayout {
    fn channel_count(&self) -> u16 {
        match self {
            Self::Single(opusic_c::Channels::Mono) => 1,
            Self::Single(opusic_c::Channels::Stereo) => 2,
            Self::Multi { channels, .. } => u16::from(*channels),
        }
    }
}

/// Newtype over `opusic_c::multistream::Decoder` carrying the `Send` impl upstream omits.
struct MultiDecoder(opusic_c::multistream::Decoder);

// SAFETY: `multistream::Decoder` uniquely owns its heap-allocated libopus state
// (`mem::Unique`), which has no thread affinity, and every method takes `&mut self`.
// opusic-c marks the structurally identical `Decoder`, `Encoder`, and
// `multistream::Encoder` as `Send`; the missing impl here is an upstream omission.
unsafe impl Send for MultiDecoder {}

/// libopus decoder behind a common decode/reset surface for both stream layouts.
enum OpusInner {
    Single(opusic_c::Decoder),
    Multi(MultiDecoder),
}

impl OpusInner {
    fn new(layout: &ChannelLayout) -> Result<Self, DecodeError> {
        match layout {
            ChannelLayout::Single(channels) => {
                opusic_c::Decoder::new(*channels, opusic_c::SampleRate::Hz48000)
                    .map(Self::Single)
                    .map_err(|e| DecodeError::OpusDecode {
                        message: format!("failed to initialise libopus decoder: {e:?}"),
                        location: snafu::Location::new(file!(), line!(), column!()),
                    })
            }
            ChannelLayout::Multi {
                channels,
                streams,
                coupled_streams,
                mapping,
            } => new_multistream(*channels, *streams, *coupled_streams, mapping)
                .map(|decoder| Self::Multi(MultiDecoder(decoder))),
        }
    }

    fn decode_float_to_slice(
        &mut self,
        input: &[u8],
        output: &mut [f32],
        decode_fec: bool,
    ) -> Result<usize, opusic_c::ErrorCode> {
        match self {
            Self::Single(decoder) => decoder.decode_float_to_slice(input, output, decode_fec),
            Self::Multi(decoder) => decoder.0.decode_float_to_slice(input, output, decode_fec),
        }
    }

    /// Resets decoder state to initial (libopus `OPUS_RESET_STATE`).
    fn reset(&mut self) -> Result<(), opusic_c::ErrorCode> {
        match self {
            Self::Single(decoder) => decoder.reset(),
            Self::Multi(decoder) => decoder.0.reset(),
        }
    }
}

/// Constructs a multistream decoder for `channels` I/O channels.
fn new_multistream(
    channels: u8,
    streams: u8,
    coupled_streams: u8,
    mapping: &[u8; OPUS_MAX_MAPPED_CHANNELS],
) -> Result<opusic_c::multistream::Decoder, DecodeError> {
    fn build<const CH: usize>(
        streams: u8,
        coupled_streams: u8,
        mapping: &[u8; OPUS_MAX_MAPPED_CHANNELS],
    ) -> Result<opusic_c::multistream::Decoder, DecodeError> {
        let mut table = [0u8; CH];
        table.copy_from_slice(&mapping[..CH]);
        let config = opusic_c::multistream::Config::<CH>::try_new(streams, coupled_streams, table)
            .ok_or_else(|| DecodeError::OpusDecode {
                message: format!(
                    "invalid Opus channel mapping: {streams} streams, {coupled_streams} coupled, {CH} channels"
                ),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        opusic_c::multistream::Decoder::new(config, opusic_c::SampleRate::Hz48000).map_err(|e| {
            DecodeError::OpusDecode {
                message: format!("failed to initialise libopus multistream decoder: {e:?}"),
                location: snafu::Location::new(file!(), line!(), column!()),
            }
        })
    }

    // WHY: `multistream::Config` is const-generic over the channel count, so each
    // family-1 count (1..=8) needs its own monomorphisation.
    match channels {
        1 => build::<1>(streams, coupled_streams, mapping),
        2 => build::<2>(streams, coupled_streams, mapping),
        3 => build::<3>(streams, coupled_streams, mapping),
        4 => build::<4>(streams, coupled_streams, mapping),
        5 => build::<5>(streams, coupled_streams, mapping),
        6 => build::<6>(streams, coupled_streams, mapping),
        7 => build::<7>(streams, coupled_streams, mapping),
        8 => build::<8>(streams, coupled_streams, mapping),
        // INVARIANT: resolve_channel_layout only produces Multi for 1..=8 channels.
        _ => Err(unsupported_layout(u16::from(channels), Some(1))),
    }
}

/// Fields of the OpusHead identification header needed for decoder construction.
///
/// All containers Symphonia routes here (OGG, Matroska/WebM, MP4 dOps) normalise
/// `extra_data` to this magic-prefixed layout. Only single-byte fields are read, so
/// the OGG (little-endian) vs MP4 (big-endian) multi-byte field difference is moot.
struct OpusHead {
    channels: u8,
    mapping_family: u8,
    /// Present only when `mapping_family != 0` and the table fits family 1's bounds.
    table: Option<MappingTable>,
}

struct MappingTable {
    streams: u8,
    coupled_streams: u8,
    mapping: [u8; OPUS_MAX_MAPPED_CHANNELS],
}

fn parse_opus_head(extra_data: &[u8]) -> Option<OpusHead> {
    const MAGIC: &[u8; 8] = b"OpusHead";
    const CHANNELS_OFFSET: usize = 9;
    const FAMILY_OFFSET: usize = 18;
    const STREAMS_OFFSET: usize = 19;
    const COUPLED_OFFSET: usize = 20;
    const TABLE_OFFSET: usize = 21;

    if extra_data.len() <= FAMILY_OFFSET || !extra_data.starts_with(MAGIC) {
        return None;
    }

    let channels = extra_data[CHANNELS_OFFSET];
    let mapping_family = extra_data[FAMILY_OFFSET];

    let table_len = usize::from(channels);
    let table = if mapping_family == 0
        || table_len > OPUS_MAX_MAPPED_CHANNELS
        || extra_data.len() < TABLE_OFFSET + table_len
    {
        None
    } else {
        let mut mapping = [0u8; OPUS_MAX_MAPPED_CHANNELS];
        mapping[..table_len].copy_from_slice(&extra_data[TABLE_OFFSET..TABLE_OFFSET + table_len]);
        Some(MappingTable {
            streams: extra_data[STREAMS_OFFSET],
            coupled_streams: extra_data[COUPLED_OFFSET],
            mapping,
        })
    };

    Some(OpusHead {
        channels,
        mapping_family,
        table,
    })
}

/// Resolves the decoder channel layout FROM the identification header.
///
/// WHY(#544): the layout must reflect the stream's TRUE channel count. Collapsing >2
/// channels to stereo configured libopus for 2 channels while `total` used the real
/// count  -  fabricating extra-channel audio FROM stale buffer bytes and desyncing
/// every downstream per-channel DSP stage. Unsupported layouts fail loudly instead.
fn resolve_channel_layout(
    fallback_channels: u16,
    extra_data: Option<&[u8]>,
) -> Result<ChannelLayout, DecodeError> {
    match extra_data.and_then(parse_opus_head) {
        Some(head) => match (head.mapping_family, head.channels) {
            (0, 1) => Ok(ChannelLayout::Single(opusic_c::Channels::Mono)),
            (0, 2) => Ok(ChannelLayout::Single(opusic_c::Channels::Stereo)),
            (1, channels @ 1..=8) => match head.table {
                Some(table) => Ok(ChannelLayout::Multi {
                    channels,
                    streams: table.streams,
                    coupled_streams: table.coupled_streams,
                    mapping: table.mapping,
                }),
                None => Err(unsupported_layout(u16::from(channels), Some(1))),
            },
            (family, channels) => Err(unsupported_layout(u16::from(channels), Some(family))),
        },
        // NOTE: without an OpusHead there is no channel-mapping table, so only the
        // implicit mono/stereo layouts of mapping family 0 are decodable.
        None => match fallback_channels {
            1 => Ok(ChannelLayout::Single(opusic_c::Channels::Mono)),
            2 => Ok(ChannelLayout::Single(opusic_c::Channels::Stereo)),
            channels => Err(unsupported_layout(channels, None)),
        },
    }
}

fn unsupported_layout(channels: u16, mapping_family: Option<u8>) -> DecodeError {
    let label = match mapping_family {
        Some(family) => format!("Opus ({channels} channels, mapping family {family})"),
        None => format!("Opus ({channels} channels, no identification header)"),
    };
    DecodeError::UnsupportedCodec {
        codec: Codec::Other(label),
        location: snafu::Location::new(file!(), line!(), column!()),
    }
}

impl OpusDecoder {
    /// Constructs an `OpusDecoder` FROM an already-probed Symphonia `ProbeResult`.
    ///
    /// The caller (probe.rs) does the format detection; this constructor takes
    /// ownership and sets up the libopus decoder for the OGG/Opus track.
    pub fn from_probed(
        format: Box<dyn symphonia::core::formats::FormatReader + 'static>,
    ) -> Result<Box<dyn SyncAudioDecoder>, DecodeError> {
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

        let demuxed_channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        let layout = resolve_channel_layout(demuxed_channels, codec_params.extra_data.as_deref())?;
        // INVARIANT: `channels` matches the libopus decoder configuration  -  the
        // interleaved sample math in decode_next_packet depends on this.
        let channels = layout.channel_count();
        let decoder = OpusInner::new(&layout)?;

        let time_base = track_time_base
            .or_else(|| TimeBase::try_from_recip(OPUS_SAMPLE_RATE))
            .ok_or_else(|| DecodeError::OpusDecode {
                message: "failed to derive time base for Opus stream".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

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
                .decode_float_to_slice(packet.data.as_ref(), &mut self.decode_buf, false)
                .map_err(|e| DecodeError::OpusDecode {
                    message: format!("decode_float failed: {e:?}"),
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

        // WHY(#544): reset in place (`OPUS_RESET_STATE`) clears post-seek decoder state
        // without re-deriving the channel layout  -  the old rebuild duplicated layout
        // logic here and silently collapsed >2-channel streams to stereo.
        self.decoder.reset().map_err(|e| DecodeError::OpusDecode {
            message: format!("decoder reset after seek failed: {e:?}"),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

        let secs = self
            .time_base
            .calc_time(seeked.actual_ts)
            .map(|t| t.as_secs_f64())
            .unwrap_or(0.0);
        Ok(Duration::from_secs_f64(secs))
    }
}

// WHY(#403): OpusDecoder performs blocking OGG file reads, so it implements only the
// synchronous trait; `blocking::BlockingDecoder` hosts it on a dedicated decode thread.
impl SyncAudioDecoder for OpusDecoder {
    fn next_frame(&mut self) -> Result<Option<DecodedFrame>, DecodeError> {
        self.decode_next_packet()
    }

    fn seek(&mut self, position: Duration) -> Result<Duration, DecodeError> {
        self.do_seek(position)
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
        // Passing an empty slice to decode_float_to_slice triggers Opus Packet Loss Concealment.
        let mut dec =
            opusic_c::Decoder::new(opusic_c::Channels::Stereo, opusic_c::SampleRate::Hz48000)
                .unwrap();
        let mut buf = vec![0.0f32; OPUS_MAX_FRAME_SAMPLES * 2];
        let result = dec.decode_float_to_slice(&[], &mut buf, false);
        assert!(result.is_ok(), "PLC decode must not error: {result:?}");
        assert!(result.unwrap() > 0, "PLC must produce samples");
    }

    // --- #544: >2-channel streams must never be collapsed to a stereo decoder ---

    /// RFC 7845 5.1 mapping: 4 streams, 2 coupled, Vorbis order [FL FC FR RL RR LFE].
    const SURROUND_5_1_TABLE: [u8; 8] = [4, 2, 0, 4, 1, 2, 3, 5];

    /// Builds a synthetic OpusHead identification header. `table` carries
    /// `[streams, coupled, mapping...]` and must be empty for mapping family 0.
    fn opus_head_bytes(channels: u8, family: u8, table: &[u8]) -> Vec<u8> {
        let mut head = Vec::new();
        head.extend_from_slice(b"OpusHead");
        head.push(1); // version
        head.push(channels);
        head.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
        head.extend_from_slice(&48_000u32.to_le_bytes()); // input sample rate
        head.extend_from_slice(&0u16.to_le_bytes()); // output gain
        head.push(family);
        head.extend_from_slice(table);
        head
    }

    #[test]
    fn parse_opus_head_reads_family1_mapping_table() {
        let head = parse_opus_head(&opus_head_bytes(6, 1, &SURROUND_5_1_TABLE[..8])).unwrap();
        assert_eq!(head.channels, 6);
        assert_eq!(head.mapping_family, 1);
        let table = head.table.unwrap();
        assert_eq!(table.streams, 4);
        assert_eq!(table.coupled_streams, 2);
        assert_eq!(&table.mapping[..6], &[0, 4, 1, 2, 3, 5]);
    }

    #[test]
    fn parse_opus_head_rejects_bad_magic_and_truncation() {
        let mut bad_magic = opus_head_bytes(2, 0, &[]);
        bad_magic[0] = b'X';
        assert!(parse_opus_head(&bad_magic).is_none());
        assert!(parse_opus_head(&opus_head_bytes(2, 0, &[])[..12]).is_none());
    }

    #[test]
    fn resolve_5_1_family1_yields_multistream_layout() {
        let head = opus_head_bytes(6, 1, &SURROUND_5_1_TABLE[..8]);
        let layout = resolve_channel_layout(6, Some(&head)).unwrap();
        assert_eq!(layout.channel_count(), 6, "true channel count must survive");
        assert!(
            matches!(
                layout,
                ChannelLayout::Multi {
                    channels: 6,
                    streams: 4,
                    coupled_streams: 2,
                    ..
                }
            ),
            "5.1 must resolve to a multistream layout, got {layout:?}"
        );
    }

    #[test]
    fn resolve_family0_mono_and_stereo_use_single_decoder() {
        let mono = resolve_channel_layout(1, Some(&opus_head_bytes(1, 0, &[]))).unwrap();
        assert_eq!(mono, ChannelLayout::Single(opusic_c::Channels::Mono));
        let stereo = resolve_channel_layout(2, Some(&opus_head_bytes(2, 0, &[]))).unwrap();
        assert_eq!(stereo, ChannelLayout::Single(opusic_c::Channels::Stereo));
        // No identification header: the demuxed channel count alone drives the layout.
        let fallback = resolve_channel_layout(2, None).unwrap();
        assert_eq!(fallback, ChannelLayout::Single(opusic_c::Channels::Stereo));
    }

    #[test]
    fn resolve_rejects_multichannel_without_mapping_table() {
        // 6 channels but no OpusHead: no mapping table exists, so constructing a
        // decoder must fail loudly instead of collapsing to stereo.
        let err = resolve_channel_layout(6, None).unwrap_err();
        assert!(
            matches!(err, DecodeError::UnsupportedCodec { .. }),
            "expected UnsupportedCodec, got {err:?}"
        );
    }

    #[test]
    fn resolve_rejects_family0_multichannel() {
        // Mapping family 0 is defined only for 1-2 channels (RFC 7845 §5.1.1.1).
        let err = resolve_channel_layout(6, Some(&opus_head_bytes(6, 0, &[]))).unwrap_err();
        assert!(matches!(err, DecodeError::UnsupportedCodec { .. }));
    }

    #[test]
    fn resolve_rejects_reserved_mapping_family() {
        let head = opus_head_bytes(6, 255, &SURROUND_5_1_TABLE[..8]);
        let err = resolve_channel_layout(6, Some(&head)).unwrap_err();
        assert!(matches!(err, DecodeError::UnsupportedCodec { .. }));
    }

    #[test]
    fn multistream_5_1_round_trip_decodes_six_channels() {
        const CHANNELS: usize = 6;
        const FRAME_SAMPLES: usize = 960; // 20 ms at 48 kHz

        // Encode one genuine 6-channel frame with the libopus multistream encoder.
        let mapping = [0u8, 4, 1, 2, 3, 5];
        let config = opusic_c::multistream::Config::<CHANNELS>::try_new(4, 2, mapping).unwrap();
        let mut encoder = opusic_c::multistream::Encoder::new(
            config,
            opusic_c::SampleRate::Hz48000,
            opusic_c::Application::Audio,
        )
        .unwrap();
        let mut pcm = vec![0.0f32; FRAME_SAMPLES * CHANNELS];
        for (i, sample) in pcm.iter_mut().enumerate() {
            *sample = (i % 480) as f32 / 4_800.0 - 0.05;
        }
        let mut packet = vec![0u8; 4_000];
        let packet_len = encoder.encode_float_to_slice(&pcm, &mut packet).unwrap();

        // The decoder constructed FROM a 5.1 OpusHead must decode the full frame.
        let head = opus_head_bytes(6, 1, &SURROUND_5_1_TABLE[..8]);
        let layout = resolve_channel_layout(6, Some(&head)).unwrap();
        let mut decoder = OpusInner::new(&layout).unwrap();
        let mut out = vec![0.0f32; OPUS_MAX_FRAME_SAMPLES * CHANNELS];
        let decoded = decoder
            .decode_float_to_slice(&packet[..packet_len], &mut out, false)
            .unwrap();
        assert_eq!(
            decoded, FRAME_SAMPLES,
            "6-channel packet must decode a full frame per channel"
        );
    }
}
