/// QUIC streaming wire protocol definitions.
pub mod codec;
pub mod frame;

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    AudioFrame = 0x01,
    ClockSync = 0x02,
    ClockSyncReply = 0x03,
    SessionInit = 0x04,
    SessionAccept = 0x05,
    StatusReport = 0x06,
    Command = 0x07,
}

impl FrameType {
    pub(crate) fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::AudioFrame),
            0x02 => Some(Self::ClockSync),
            0x03 => Some(Self::ClockSyncReply),
            0x04 => Some(Self::SessionInit),
            0x05 => Some(Self::SessionAccept),
            0x06 => Some(Self::StatusReport),
            0x07 => Some(Self::Command),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum AudioCodec {
    Flac = 0x01,
    Pcm = 0x02,
}

impl AudioCodec {
    pub(crate) fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Flac),
            0x02 => Some(Self::Pcm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum DeviceState {
    Active = 0x01,
    Idle = 0x02,
    Error = 0x03,
    Buffering = 0x04,
}

impl DeviceState {
    pub(crate) fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Active),
            0x02 => Some(Self::Idle),
            0x03 => Some(Self::Error),
            0x04 => Some(Self::Buffering),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandKind {
    Pause = 0x01,
    Resume = 0x02,
    VolumeAdjust = 0x03,
    Seek = 0x04,
}

impl CommandKind {
    pub(crate) fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Pause),
            0x02 => Some(Self::Resume),
            0x03 => Some(Self::VolumeAdjust),
            0x04 => Some(Self::Seek),
            _ => None,
        }
    }
}
