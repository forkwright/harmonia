//! Request state machine — validates status transitions.

use crate::error::{AitesisError, InvalidTransitionSnafu};
use crate::types::RequestStatus;

/// Valid transitions:
/// ```text
/// Submitted → Approved
/// Submitted → Denied
/// Approved  → Monitoring
/// Monitoring → Fulfilled
/// Monitoring → Failed
/// ```
pub(crate) fn validate_transition(
    from: RequestStatus,
    to: RequestStatus,
) -> Result<(), AitesisError> {
    let allowed = matches!(
        (from, to),
        (RequestStatus::Submitted, RequestStatus::Approved)
            | (RequestStatus::Submitted, RequestStatus::Denied)
            | (RequestStatus::Approved, RequestStatus::Monitoring)
            | (RequestStatus::Monitoring, RequestStatus::Fulfilled)
            | (RequestStatus::Monitoring, RequestStatus::Failed)
    );

    if allowed {
        Ok(())
    } else {
        InvalidTransitionSnafu {
            from: from.as_str().to_string(),
            to: to.as_str().to_string(),
        }
        .fail()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RequestStatus;

    #[test]
    fn submitted_to_approved_is_valid() {
        assert!(validate_transition(RequestStatus::Submitted, RequestStatus::Approved).is_ok());
    }

    #[test]
    fn submitted_to_denied_is_valid() {
        assert!(validate_transition(RequestStatus::Submitted, RequestStatus::Denied).is_ok());
    }

    #[test]
    fn approved_to_monitoring_is_valid() {
        assert!(validate_transition(RequestStatus::Approved, RequestStatus::Monitoring).is_ok());
    }

    #[test]
    fn monitoring_to_fulfilled_is_valid() {
        assert!(validate_transition(RequestStatus::Monitoring, RequestStatus::Fulfilled).is_ok());
    }

    #[test]
    fn monitoring_to_failed_is_valid() {
        assert!(validate_transition(RequestStatus::Monitoring, RequestStatus::Failed).is_ok());
    }

    #[test]
    fn denied_to_approved_is_invalid() {
        let err = validate_transition(RequestStatus::Denied, RequestStatus::Approved).unwrap_err();
        assert!(matches!(err, AitesisError::InvalidTransition { .. }));
    }

    #[test]
    fn fulfilled_to_monitoring_is_invalid() {
        let err =
            validate_transition(RequestStatus::Fulfilled, RequestStatus::Monitoring).unwrap_err();
        assert!(matches!(err, AitesisError::InvalidTransition { .. }));
    }

    #[test]
    fn submitted_to_monitoring_is_invalid() {
        let err =
            validate_transition(RequestStatus::Submitted, RequestStatus::Monitoring).unwrap_err();
        assert!(matches!(err, AitesisError::InvalidTransition { .. }));
    }

    #[test]
    fn full_lifecycle_transitions_are_valid() {
        validate_transition(RequestStatus::Submitted, RequestStatus::Approved).unwrap();
        validate_transition(RequestStatus::Approved, RequestStatus::Monitoring).unwrap();
        validate_transition(RequestStatus::Monitoring, RequestStatus::Fulfilled).unwrap();
    }
}
