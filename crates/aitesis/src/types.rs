//! Domain types for the Aitesis request management subsystem.

use themelion::{MediaType, RequestId, UserId, WantId};
use serde::{Deserialize, Serialize};

pub type Timestamp = jiff::Timestamp;

/// A household media request from submission through fulfillment.
#[derive(Debug, Clone)]
pub struct MediaRequest {
    pub id: RequestId,
    pub user_id: UserId,
    pub media_type: MediaType,
    pub title: String,
    /// IMDB, TVDB, MusicBrainz ID — used by Epignosis for identity resolution.
    pub external_id: Option<String>,
    pub status: RequestStatus,
    pub decided_by: Option<UserId>,
    pub decided_at: Option<Timestamp>,
    pub deny_reason: Option<String>,
    /// Links to the `wants` table after approval — set when Episkope accepts the want.
    pub want_id: Option<WantId>,
    pub created_at: Timestamp,
}

/// Lifecycle state of a media request.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    /// Awaiting approval (Member) or auto-processing (Admin).
    Submitted,
    /// Approved, pending monitoring setup.
    Approved,
    /// Rejected by admin.
    Denied,
    /// Handed to Episkope — actively searching.
    Monitoring,
    /// Download complete, media available.
    Fulfilled,
    /// Could not be fulfilled after reasonable attempts.
    Failed,
}

impl RequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Monitoring => "monitoring",
            Self::Fulfilled => "fulfilled",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "submitted" => Some(Self::Submitted),
            "approved" => Some(Self::Approved),
            "denied" => Some(Self::Denied),
            "monitoring" => Some(Self::Monitoring),
            "fulfilled" => Some(Self::Fulfilled),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Input for creating a new media request.
#[derive(Debug, Clone)]
pub struct CreateRequestInput {
    pub media_type: MediaType,
    pub title: String,
    pub external_id: Option<String>,
}

/// Role of a user within the household — determines auto-approval and limit exemptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Admin,
    Member,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn request_status_serde_roundtrip() {
        let statuses = [
            RequestStatus::Submitted,
            RequestStatus::Approved,
            RequestStatus::Denied,
            RequestStatus::Monitoring,
            RequestStatus::Fulfilled,
            RequestStatus::Failed,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let recovered: RequestStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, recovered);
        }
    }

    #[test]
    fn request_status_as_str_and_parse_roundtrip() {
        let statuses = [
            RequestStatus::Submitted,
            RequestStatus::Approved,
            RequestStatus::Denied,
            RequestStatus::Monitoring,
            RequestStatus::Fulfilled,
            RequestStatus::Failed,
        ];
        for status in statuses {
            let s = status.as_str();
            let parsed = RequestStatus::parse(s).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn request_status_parse_unknown_returns_none() {
        assert!(RequestStatus::parse("unknown_status").is_none());
    }
}
