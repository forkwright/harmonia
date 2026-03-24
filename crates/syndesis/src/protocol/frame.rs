/// Wire frame types for the syndesis streaming protocol.
use bytes::Bytes;

use super::{AudioCodec, CommandKind, DeviceState};

#[derive(Debug, Clone, PartialEq)]
pub struct AudioFrame {
    pub sequence: u64,
    pub timestamp_us: u64,
    /// Target playout time for zone-synchronized playback. Zero if not zone-synced.
    pub playout_ts: u64,
    pub codec: AudioCodec,
    pub channels: u8,
    pub sample_rate: u32,
    pub payload: Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSync {
    pub originate_ts: u64,
    pub receive_ts: u64,
    pub transmit_ts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSyncReply {
    pub originate_ts: u64,
    pub receive_ts: u64,
    pub transmit_ts: u64,
    pub destination_ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInit {
    pub protocol_version: u8,
    pub supported_codecs: Vec<AudioCodec>,
    pub sample_rates: Vec<u32>,
    pub channel_configs: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAccept {
    pub codec: AudioCodec,
    pub sample_rate: u32,
    pub channels: u8,
    pub session_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReport {
    pub buffer_depth_ms: u16,
    pub latency_ms: u16,
    pub device_state: DeviceState,
    pub renderer_id: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    pub kind: CommandKind,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Audio(AudioFrame),
    ClockSync(ClockSync),
    ClockSyncReply(ClockSyncReply),
    SessionInit(SessionInit),
    SessionAccept(SessionAccept),
    StatusReport(StatusReport),
    Command(Command),
}
