/// Zero-copy binary codec for wire frames.
use bytes::{Buf, BufMut, Bytes, BytesMut};
use snafu::ensure;

use super::frame::{
    AudioFrame, ClockSync, ClockSyncReply, Command, Frame, SessionAccept, SessionInit, StatusReport,
};
use super::{AudioCodec, CommandKind, DeviceState, FrameType};
use crate::error;

// WHY: Length-prefixed framing allows receivers to delimit frames on QUIC streams
// without relying on stream boundaries. DATAGRAMs are already delimited by QUIC.
const LENGTH_PREFIX_SIZE: usize = 4;

/// Encode a frame into a length-prefixed byte buffer.
#[must_use]
pub fn encode_frame(frame: &Frame) -> Bytes {
    let mut buf = BytesMut::new();
    // Reserve space for length prefix
    buf.put_u32(0);

    match frame {
        Frame::Audio(f) => encode_audio_frame(&mut buf, f),
        Frame::ClockSync(f) => encode_clock_sync(&mut buf, f),
        Frame::ClockSyncReply(f) => encode_clock_sync_reply(&mut buf, f),
        Frame::SessionInit(f) => encode_session_init(&mut buf, f),
        Frame::SessionAccept(f) => encode_session_accept(&mut buf, f),
        Frame::StatusReport(f) => encode_status_report(&mut buf, f),
        Frame::Command(f) => encode_command(&mut buf, f),
    }

    let frame_len = (buf.len() - LENGTH_PREFIX_SIZE) as u32;
    buf[..LENGTH_PREFIX_SIZE].copy_from_slice(&frame_len.to_be_bytes());

    buf.freeze()
}

/// Encode a frame without length prefix (for DATAGRAMs which are already delimited).
#[must_use]
pub fn encode_datagram(frame: &Frame) -> Bytes {
    let mut buf = BytesMut::new();
    match frame {
        Frame::ClockSync(f) => encode_clock_sync(&mut buf, f),
        Frame::ClockSyncReply(f) => encode_clock_sync_reply(&mut buf, f),
        Frame::StatusReport(f) => encode_status_report(&mut buf, f),
        Frame::Command(f) => encode_command(&mut buf, f),
        Frame::Audio(f) => encode_audio_frame(&mut buf, f),
        Frame::SessionInit(f) => encode_session_init(&mut buf, f),
        Frame::SessionAccept(f) => encode_session_accept(&mut buf, f),
    }
    buf.freeze()
}

/// Decode a length-prefixed frame from a buffer.
pub fn decode_frame(data: &mut Bytes) -> Result<Frame, error::SyndesisError> {
    ensure!(
        data.remaining() >= LENGTH_PREFIX_SIZE,
        error::ProtocolSnafu {
            reason: "buffer too short for length prefix"
        }
    );

    let frame_len = data.get_u32() as usize;
    ensure!(
        data.remaining() >= frame_len,
        error::ProtocolSnafu {
            reason: "buffer shorter than declared frame length"
        }
    );

    let mut frame_data = data.split_to(frame_len);
    decode_frame_body(&mut frame_data)
}

/// Decode a frame from raw bytes (no length prefix, for DATAGRAMs).
pub fn decode_datagram(data: &mut Bytes) -> Result<Frame, error::SyndesisError> {
    decode_frame_body(data)
}

fn decode_frame_body(buf: &mut Bytes) -> Result<Frame, error::SyndesisError> {
    ensure!(
        buf.has_remaining(),
        error::ProtocolSnafu {
            reason: "empty frame body"
        }
    );

    let type_byte = buf.get_u8();
    let frame_type = FrameType::from_u8(type_byte).ok_or_else(|| {
        error::ProtocolSnafu {
            reason: "unknown frame type",
        }
        .build()
    })?;

    match frame_type {
        FrameType::AudioFrame => decode_audio_frame(buf).map(Frame::Audio),
        FrameType::ClockSync => decode_clock_sync(buf).map(Frame::ClockSync),
        FrameType::ClockSyncReply => decode_clock_sync_reply(buf).map(Frame::ClockSyncReply),
        FrameType::SessionInit => decode_session_init(buf).map(Frame::SessionInit),
        FrameType::SessionAccept => decode_session_accept(buf).map(Frame::SessionAccept),
        FrameType::StatusReport => decode_status_report(buf).map(Frame::StatusReport),
        FrameType::Command => decode_command(buf).map(Frame::Command),
    }
}

fn encode_audio_frame(buf: &mut BytesMut, f: &AudioFrame) {
    buf.put_u8(FrameType::AudioFrame as u8);
    buf.put_u64(f.sequence);
    buf.put_u64(f.timestamp_us);
    buf.put_u64(f.playout_ts);
    buf.put_u8(f.codec as u8);
    buf.put_u8(f.channels);
    buf.put_u32(f.sample_rate);
    buf.put_u32(f.payload.len() as u32);
    buf.put_slice(&f.payload);
}

fn decode_audio_frame(buf: &mut Bytes) -> Result<AudioFrame, error::SyndesisError> {
    ensure!(
        buf.remaining() >= 8 + 8 + 8 + 1 + 1 + 4 + 4,
        error::ProtocolSnafu {
            reason: "audio frame header too short"
        }
    );

    let sequence = buf.get_u64();
    let timestamp_us = buf.get_u64();
    let playout_ts = buf.get_u64();
    let codec_byte = buf.get_u8();
    let channels = buf.get_u8();
    let sample_rate = buf.get_u32();
    let payload_len = buf.get_u32() as usize;

    let codec = AudioCodec::from_u8(codec_byte).ok_or_else(|| {
        error::ProtocolSnafu {
            reason: "unknown audio codec",
        }
        .build()
    })?;

    ensure!(
        buf.remaining() >= payload_len,
        error::ProtocolSnafu {
            reason: "audio payload shorter than declared"
        }
    );

    let payload = buf.split_to(payload_len);

    Ok(AudioFrame {
        sequence,
        timestamp_us,
        playout_ts,
        codec,
        channels,
        sample_rate,
        payload,
    })
}

fn encode_clock_sync(buf: &mut BytesMut, f: &ClockSync) {
    buf.put_u8(FrameType::ClockSync as u8);
    buf.put_u64(f.originate_ts);
    buf.put_u64(f.receive_ts);
    buf.put_u64(f.transmit_ts);
}

fn decode_clock_sync(buf: &mut Bytes) -> Result<ClockSync, error::SyndesisError> {
    ensure!(
        buf.remaining() >= 24,
        error::ProtocolSnafu {
            reason: "clock sync too short"
        }
    );
    Ok(ClockSync {
        originate_ts: buf.get_u64(),
        receive_ts: buf.get_u64(),
        transmit_ts: buf.get_u64(),
    })
}

fn encode_clock_sync_reply(buf: &mut BytesMut, f: &ClockSyncReply) {
    buf.put_u8(FrameType::ClockSyncReply as u8);
    buf.put_u64(f.originate_ts);
    buf.put_u64(f.receive_ts);
    buf.put_u64(f.transmit_ts);
    buf.put_u64(f.destination_ts);
}

fn decode_clock_sync_reply(buf: &mut Bytes) -> Result<ClockSyncReply, error::SyndesisError> {
    ensure!(
        buf.remaining() >= 32,
        error::ProtocolSnafu {
            reason: "clock sync reply too short"
        }
    );
    Ok(ClockSyncReply {
        originate_ts: buf.get_u64(),
        receive_ts: buf.get_u64(),
        transmit_ts: buf.get_u64(),
        destination_ts: buf.get_u64(),
    })
}

fn encode_session_init(buf: &mut BytesMut, f: &SessionInit) {
    buf.put_u8(FrameType::SessionInit as u8);
    buf.put_u8(f.protocol_version);
    buf.put_u8(f.supported_codecs.len() as u8);
    for codec in &f.supported_codecs {
        buf.put_u8(*codec as u8);
    }
    buf.put_u8(f.sample_rates.len() as u8);
    for rate in &f.sample_rates {
        buf.put_u32(*rate);
    }
    buf.put_u8(f.channel_configs.len() as u8);
    for ch in &f.channel_configs {
        buf.put_u8(*ch);
    }
}

fn decode_session_init(buf: &mut Bytes) -> Result<SessionInit, error::SyndesisError> {
    ensure!(
        buf.remaining() >= 2,
        error::ProtocolSnafu {
            reason: "session init too short"
        }
    );

    let protocol_version = buf.get_u8();
    let codec_count = buf.get_u8() as usize;
    ensure!(
        buf.remaining() >= codec_count,
        error::ProtocolSnafu {
            reason: "session init codecs truncated"
        }
    );
    let mut supported_codecs = Vec::with_capacity(codec_count);
    for _ in 0..codec_count {
        let c = AudioCodec::from_u8(buf.get_u8()).ok_or_else(|| {
            error::ProtocolSnafu {
                reason: "unknown codec in session init",
            }
            .build()
        })?;
        supported_codecs.push(c);
    }

    ensure!(
        buf.has_remaining(),
        error::ProtocolSnafu {
            reason: "session init sample rates missing"
        }
    );
    let rate_count = buf.get_u8() as usize;
    ensure!(
        buf.remaining() >= rate_count * 4,
        error::ProtocolSnafu {
            reason: "session init sample rates truncated"
        }
    );
    let mut sample_rates = Vec::with_capacity(rate_count);
    for _ in 0..rate_count {
        sample_rates.push(buf.get_u32());
    }

    ensure!(
        buf.has_remaining(),
        error::ProtocolSnafu {
            reason: "session init channel configs missing"
        }
    );
    let ch_count = buf.get_u8() as usize;
    ensure!(
        buf.remaining() >= ch_count,
        error::ProtocolSnafu {
            reason: "session init channel configs truncated"
        }
    );
    let mut channel_configs = Vec::with_capacity(ch_count);
    for _ in 0..ch_count {
        channel_configs.push(buf.get_u8());
    }

    Ok(SessionInit {
        protocol_version,
        supported_codecs,
        sample_rates,
        channel_configs,
    })
}

fn encode_session_accept(buf: &mut BytesMut, f: &SessionAccept) {
    buf.put_u8(FrameType::SessionAccept as u8);
    buf.put_u8(f.codec as u8);
    buf.put_u32(f.sample_rate);
    buf.put_u8(f.channels);
    buf.put_u64(f.session_id);
}

fn decode_session_accept(buf: &mut Bytes) -> Result<SessionAccept, error::SyndesisError> {
    ensure!(
        buf.remaining() >= 14,
        error::ProtocolSnafu {
            reason: "session accept too short"
        }
    );
    let codec_byte = buf.get_u8();
    let codec = AudioCodec::from_u8(codec_byte).ok_or_else(|| {
        error::ProtocolSnafu {
            reason: "unknown codec in session accept",
        }
        .build()
    })?;
    let sample_rate = buf.get_u32();
    let channels = buf.get_u8();
    let session_id = buf.get_u64();
    Ok(SessionAccept {
        codec,
        sample_rate,
        channels,
        session_id,
    })
}

fn encode_status_report(buf: &mut BytesMut, f: &StatusReport) {
    buf.put_u8(FrameType::StatusReport as u8);
    buf.put_u16(f.buffer_depth_ms);
    buf.put_u16(f.latency_ms);
    buf.put_u8(f.device_state as u8);
    buf.put_u16(f.renderer_id.len() as u16);
    buf.put_slice(&f.renderer_id);
}

fn decode_status_report(buf: &mut Bytes) -> Result<StatusReport, error::SyndesisError> {
    ensure!(
        buf.remaining() >= 7,
        error::ProtocolSnafu {
            reason: "status report too short"
        }
    );
    let buffer_depth_ms = buf.get_u16();
    let latency_ms = buf.get_u16();
    let state_byte = buf.get_u8();
    let device_state = DeviceState::from_u8(state_byte).ok_or_else(|| {
        error::ProtocolSnafu {
            reason: "unknown device state",
        }
        .build()
    })?;
    let id_len = buf.get_u16() as usize;
    ensure!(
        buf.remaining() >= id_len,
        error::ProtocolSnafu {
            reason: "status report renderer_id truncated"
        }
    );
    let renderer_id = buf.split_to(id_len).to_vec();
    Ok(StatusReport {
        buffer_depth_ms,
        latency_ms,
        device_state,
        renderer_id,
    })
}

fn encode_command(buf: &mut BytesMut, f: &Command) {
    buf.put_u8(FrameType::Command as u8);
    buf.put_u8(f.kind as u8);
    buf.put_i64(f.value);
}

fn decode_command(buf: &mut Bytes) -> Result<Command, error::SyndesisError> {
    ensure!(
        buf.remaining() >= 9,
        error::ProtocolSnafu {
            reason: "command too short"
        }
    );
    let kind_byte = buf.get_u8();
    let kind = CommandKind::from_u8(kind_byte).ok_or_else(|| {
        error::ProtocolSnafu {
            reason: "unknown command kind",
        }
        .build()
    })?;
    let value = buf.get_i64();
    Ok(Command { kind, value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{AudioCodec, CommandKind, DeviceState, PROTOCOL_VERSION};

    #[test]
    fn audio_frame_round_trip() {
        let original = Frame::Audio(AudioFrame {
            sequence: 42,
            timestamp_us: 1_000_000,
            playout_ts: 1_010_000,
            codec: AudioCodec::Flac,
            channels: 2,
            sample_rate: 44100,
            payload: Bytes::from_static(b"fake-flac-data"),
        });
        let encoded = encode_frame(&original);
        let mut buf = encoded;
        let decoded = decode_frame(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn clock_sync_round_trip() {
        let original = Frame::ClockSync(ClockSync {
            originate_ts: 100,
            receive_ts: 200,
            transmit_ts: 300,
        });
        let encoded = encode_datagram(&original);
        let mut buf = encoded;
        let decoded = decode_datagram(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn clock_sync_reply_round_trip() {
        let original = Frame::ClockSyncReply(ClockSyncReply {
            originate_ts: 100,
            receive_ts: 200,
            transmit_ts: 300,
            destination_ts: 400,
        });
        let encoded = encode_datagram(&original);
        let mut buf = encoded;
        let decoded = decode_datagram(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn session_init_round_trip() {
        let original = Frame::SessionInit(SessionInit {
            protocol_version: PROTOCOL_VERSION,
            supported_codecs: vec![AudioCodec::Flac, AudioCodec::Pcm],
            sample_rates: vec![44100, 48000, 96000],
            channel_configs: vec![2, 6],
        });
        let encoded = encode_frame(&original);
        let mut buf = encoded;
        let decoded = decode_frame(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn session_accept_round_trip() {
        let original = Frame::SessionAccept(SessionAccept {
            codec: AudioCodec::Flac,
            sample_rate: 48000,
            channels: 2,
            session_id: 0xDEAD_BEEF,
        });
        let encoded = encode_frame(&original);
        let mut buf = encoded;
        let decoded = decode_frame(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn status_report_round_trip() {
        let original = Frame::StatusReport(StatusReport {
            buffer_depth_ms: 100,
            latency_ms: 15,
            device_state: DeviceState::Active,
            renderer_id: b"renderer-01".to_vec(),
        });
        let encoded = encode_datagram(&original);
        let mut buf = encoded;
        let decoded = decode_datagram(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn command_round_trip() {
        let original = Frame::Command(Command {
            kind: CommandKind::VolumeAdjust,
            value: -5,
        });
        let encoded = encode_frame(&original);
        let mut buf = encoded;
        let decoded = decode_frame(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn empty_payload_audio_frame() {
        let original = Frame::Audio(AudioFrame {
            sequence: 0,
            timestamp_us: 0,
            playout_ts: 0,
            codec: AudioCodec::Pcm,
            channels: 1,
            sample_rate: 16000,
            payload: Bytes::new(),
        });
        let encoded = encode_frame(&original);
        let mut buf = encoded;
        let decoded = decode_frame(&mut buf).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn truncated_frame_returns_error() {
        let mut buf = Bytes::from_static(&[0x00, 0x00, 0x00, 0x10, 0x01]);
        assert!(decode_frame(&mut buf).is_err());
    }

    #[test]
    fn unknown_frame_type_returns_error() {
        let mut buf = Bytes::from_static(&[0xFF]);
        assert!(decode_datagram(&mut buf).is_err());
    }

    #[test]
    fn all_command_kinds_round_trip() {
        for (kind, val) in [
            (CommandKind::Pause, 0),
            (CommandKind::Resume, 0),
            (CommandKind::VolumeAdjust, 75),
            (CommandKind::Seek, 120_000_000),
        ] {
            let original = Frame::Command(Command { kind, value: val });
            let encoded = encode_frame(&original);
            let mut buf = encoded;
            let decoded = decode_frame(&mut buf).expect("decode should succeed");
            assert_eq!(original, decoded);
        }
    }
}
