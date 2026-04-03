// Wire-format frame types for the renderer-server session protocol
use serde::{Deserialize, Serialize};

/// Sent by the renderer to initiate a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionInit {
    /// Human-readable renderer name.
    pub renderer_name: String,
    /// Stable renderer identity (UUID v7 as string).
    pub renderer_id: String,
    /// API key FROM a prior pairing, base64url-encoded (no padding).
    /// Present on authenticated reconnects; absent on first connection.
    pub api_key: Option<String>,
    /// Set to true when the renderer has no stored credentials.
    pub is_new: bool,
}

/// Sent by the server during the pairing flow to let the renderer
/// record the server's identity for TOFU verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairingChallenge {
    /// Human-readable server name.
    pub server_name: String,
    /// Hex-encoded SHA-256 fingerprint of the server's TLS certificate.
    pub cert_fingerprint: String,
}

/// Sent by the server after successful pairing, carrying the provisioned API key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairingComplete {
    /// Base64url-encoded (no padding) API key for future authentication.
    pub api_key: SecretString,
}

/// Sent by the server when a session is successfully established.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionAccepted {
    /// The canonical renderer ID as stored in the server registry.
    pub renderer_id: String,
}

/// Sent by the server when a session is rejected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionRejected {
    pub reason: String,
}

/// Top-level protocol frame envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    SessionInit(SessionInit),
    PairingChallenge(PairingChallenge),
    PairingComplete(PairingComplete),
    SessionAccepted(SessionAccepted),
    SessionRejected(SessionRejected),
}

impl Frame {
    /// Encode this frame as a newline-terminated JSON string.
    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut bytes = serde_json::to_vec(self)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Decode a frame FROM a JSON byte slice (trailing newline is ignored).
    pub fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        let trimmed = bytes.trim_ascii_end();
        serde_json::from_slice(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_init_round_trip() {
        let frame = Frame::SessionInit(SessionInit {
            renderer_name: "Living Room".to_string(),
            renderer_id: "01HN1234567890ABCDEFGHIJKL".to_string(),
            api_key: None,
            is_new: true,
        });
        let encoded = frame.encode().unwrap();
        let decoded = Frame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }

    #[test]
    fn pairing_challenge_round_trip() {
        let frame = Frame::PairingChallenge(PairingChallenge {
            server_name: "Harmonia".to_string(),
            cert_fingerprint: "a".repeat(64),
        });
        let encoded = frame.encode().unwrap();
        let decoded = Frame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }

    #[test]
    fn pairing_complete_round_trip() {
        let frame = Frame::PairingComplete(PairingComplete {
            api_key: "abc123def456".to_string(),
        });
        let encoded = frame.encode().unwrap();
        let decoded = Frame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }

    #[test]
    fn session_init_with_api_key_round_trip() {
        let frame = Frame::SessionInit(SessionInit {
            renderer_name: "Kitchen".to_string(),
            renderer_id: "01HN1234567890ABCDEFGHIJKL".to_string(),
            api_key: Some("some-api-key".to_string()),
            is_new: false,
        });
        let encoded = frame.encode().unwrap();
        let decoded = Frame::decode(&encoded).unwrap();
        assert_eq!(frame, decoded);
    }
}
