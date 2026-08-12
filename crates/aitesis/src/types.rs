//! Domain types for the Aitesis request management subsystem.

use aggelmata::{MediaType, RequestId, UserId, WantId};
use serde::{Deserialize, Serialize};

/// Timestamp type used for request lifecycle fields.
pub type Timestamp = jiff::Timestamp;

/// A household media request from submission through fulfillment.
#[derive(Debug, Clone)]
pub struct MediaRequest {
    /// Stable request identifier.
    pub id: RequestId,
    /// User that submitted the request.
    pub user_id: UserId,
    /// Requested media category.
    pub media_type: MediaType,
    /// Human-readable requested title.
    pub title: String,
    /// IMDB, TVDB, MusicBrainz ID — used by Epignosis for identity resolution.
    pub external_id: Option<String>,
    /// Current request lifecycle status.
    pub status: RequestStatus,
    /// User that approved or denied the request.
    pub decided_by: Option<UserId>,
    /// Time when the request was approved or denied.
    pub decided_at: Option<Timestamp>,
    /// Human-readable denial reason.
    pub deny_reason: Option<String>,
    /// Links to the `wants` table after approval, once monitoring accepts the want.
    pub want_id: Option<WantId>,
    /// Time when the request was submitted.
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
    /// Handed to monitoring — actively searching.
    Monitoring,
    /// Download complete, media available.
    Fulfilled,
    /// Could not be fulfilled after reasonable attempts.
    Failed,
}

impl RequestStatus {
    /// Returns the database string representation for this status.
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

    /// Parses a database string representation into a status.
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
// WHY: pure data — user-submitted fields for a new media request.
#[derive(Debug, Clone)]
pub struct CreateRequestInput {
    /// Requested media category.
    pub media_type: MediaType,
    /// Human-readable requested title.
    pub title: String,
    /// Optional provider identifier used during identity validation.
    pub external_id: Option<String>,
}

/// Role of a user within the household — determines auto-approval and limit exemptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UserRole {
    /// Administrator with approval and limit-exemption privileges.
    Admin,
    /// Regular household member subject to request limits.
    Member,
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

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
