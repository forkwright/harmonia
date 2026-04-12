// Wire protocol for renderer ↔ server QUIC communication.

use serde::{Deserialize, Serialize};

use super::error::{ProtocolSnafu, RenderError};

pub const MSG_SESSION_INIT: u8 = 0x01;
pub const MSG_SESSION_ACCEPT: u8 = 0x02;
pub const MSG_AUDIO_FRAME: u8 = 0x03;
pub const MSG_STATUS_REPORT: u8 = 0x04;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInit {
    pub name: String,
    pub protocol_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAccept {
    pub session_id: String,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp: u64,
    pub samples: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub buffer_depth_ms: f64,
    pub latency_ms: f64,
    pub device_state: DeviceState,
    pub underrun_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceState {
    Opening,
    Playing,
    Stopped,
    Error(String),
}

impl std::fmt::Display for DeviceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Opening => write!(f, "opening"),
            Self::Playing => write!(f, "playing"),
            Self::Stopped => write!(f, "stopped"),
            Self::Error(msg) => write!(f, "error: {msg}"),
        }
    }
}

const AUDIO_FRAME_HEADER_LEN: usize = 4 + 2 + 8; // sample_rate + channels + timestamp

impl AudioFrame {
    pub fn encode_payload(&self) -> Vec<u8> {
        let sample_bytes = self.samples.len() * 8;
        let mut buf = Vec::with_capacity(AUDIO_FRAME_HEADER_LEN + sample_bytes);
        buf.extend_from_slice(&self.sample_rate.to_le_bytes());
        buf.extend_from_slice(&self.channels.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        for &s in &self.samples {
            buf.extend_from_slice(&s.to_le_bytes());
        }
        buf
    }

    pub fn decode_payload(payload: &[u8]) -> Result<Self, RenderError> {
        if payload.len() < AUDIO_FRAME_HEADER_LEN {
            return ProtocolSnafu {
                message: format!(
                    "audio frame too short: {} bytes, need at least {AUDIO_FRAME_HEADER_LEN}",
                    payload.len()
                ),
            }
            .fail();
        }
        let sample_rate = u32::from_le_bytes(payload[0..4].try_into().unwrap_or_default());
        let channels = u16::from_le_bytes(payload[4..6].try_into().unwrap_or_default());
        let timestamp = u64::from_le_bytes(payload[6..14].try_into().unwrap_or_default());
        let sample_data = &payload[14..];
        if !sample_data.len().is_multiple_of(8) {
            return ProtocolSnafu {
                message: format!(
                    "sample data length {} not a multiple of 8",
                    sample_data.len()
                ),
            }
            .fail();
        }
        let samples: Vec<f64> = sample_data
            .chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap_or_default()))
            .collect();
        Ok(Self {
            sample_rate,
            channels,
            timestamp,
            samples,
        })
    }
}

pub async fn send_message(
    stream: &mut quinn::SendStream,
    msg_type: u8,
    payload: &[u8],
) -> Result<(), RenderError> {
    let header = [msg_type];
    let len = (payload.len() as u32).to_le_bytes();
    stream.write_all(&header).await?;
    stream.write_all(&len).await?;
    stream.write_all(payload).await?;
    Ok(())
}

pub async fn recv_message(stream: &mut quinn::RecvStream) -> Result<(u8, Vec<u8>), RenderError> {
    let mut header = [0u8];
    stream.read_exact(&mut header).await?;
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    // NOTE: cap at 16 MB to prevent allocation bombs FROM malformed frames
    if len > 16 * 1024 * 1024 {
        return ProtocolSnafu {
            message: format!("message payload too large: {len} bytes"),
        }
        .fail();
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await?;
    Ok((header.get(0).copied().unwrap_or_default(), payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_frame_roundtrip() {
        let frame = AudioFrame {
            sample_rate: 44100,
            channels: 2,
            timestamp: 12345,
            samples: vec![0.5, -0.25, 0.125, -0.0625],
        };
        let encoded = frame.encode_payload();
        let decoded = AudioFrame::decode_payload(&encoded).unwrap();
        assert_eq!(decoded.sample_rate, 44100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.timestamp, 12345);
        assert_eq!(decoded.samples.len(), 4);
        assert!((decoded.samples.get(0).copied().unwrap_or_default() - 0.5).abs() < f64::EPSILON);
        assert!((decoded.samples.get(1).copied().unwrap_or_default() - (-0.25)).abs() < f64::EPSILON);
    }

    #[test]
    fn audio_frame_decode_rejects_truncated_payload() {
        let result = AudioFrame::decode_payload(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn audio_frame_decode_rejects_misaligned_samples() {
        let mut buf = vec![0u8; AUDIO_FRAME_HEADER_LEN + 5];
        buf[0..4].copy_from_slice(&44100u32.to_le_bytes());
        buf[4..6].copy_from_slice(&2u16.to_le_bytes());
        let result = AudioFrame::decode_payload(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn session_init_serializes_to_json() {
        let init = SessionInit {
            name: "living-room".into(),
            protocol_version: PROTOCOL_VERSION,
        };
        let json = serde_json::to_string(&init).unwrap_or_default();
        assert!(json.contains("living-room"));
    }

    #[test]
    fn status_report_roundtrip() {
        let report = StatusReport {
            buffer_depth_ms: 95.0,
            latency_ms: 42.5,
            device_state: DeviceState::Playing,
            underrun_count: 3,
        };
        let json = serde_json::to_vec(&report).unwrap_or_default();
        let decoded: StatusReport = serde_json::from_slice(&json).unwrap();
        assert!((decoded.buffer_depth_ms - 95.0).abs() < f64::EPSILON);
        assert_eq!(decoded.underrun_count, 3);
    }
}
