//! Request state machine — validates status transitions.

use crate::error::{AitesisError, InvalidTransitionSnafu};
use crate::types::RequestStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValidTransition {
    from: RequestStatus,
    to: RequestStatus,
}

impl ValidTransition {
    #[cfg(test)]
    pub(crate) fn from(self) -> RequestStatus {
        self.from
    }

    pub(crate) fn to(self) -> RequestStatus {
        self.to
    }
}

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
) -> Result<ValidTransition, AitesisError> {
    let allowed = matches!(
        (from, to),
        (RequestStatus::Submitted, RequestStatus::Approved)
            | (RequestStatus::Submitted, RequestStatus::Denied)
            | (RequestStatus::Approved, RequestStatus::Monitoring)
            | (RequestStatus::Monitoring, RequestStatus::Fulfilled)
            | (RequestStatus::Monitoring, RequestStatus::Failed)
    );

    if allowed {
        Ok(ValidTransition { from, to })
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
        assert!(matches!(
            validate_transition(RequestStatus::Submitted, RequestStatus::Approved),
            Ok(valid)
                if valid.from() == RequestStatus::Submitted
                    && valid.to() == RequestStatus::Approved
        ));
    }

    #[test]
    fn submitted_to_denied_is_valid() {
        assert!(matches!(
            validate_transition(RequestStatus::Submitted, RequestStatus::Denied),
            Ok(valid)
                if valid.from() == RequestStatus::Submitted
                    && valid.to() == RequestStatus::Denied
        ));
    }

    #[test]
    fn approved_to_monitoring_is_valid() {
        assert!(matches!(
            validate_transition(RequestStatus::Approved, RequestStatus::Monitoring),
            Ok(valid)
                if valid.from() == RequestStatus::Approved
                    && valid.to() == RequestStatus::Monitoring
        ));
    }

    #[test]
    fn monitoring_to_fulfilled_is_valid() {
        assert!(matches!(
            validate_transition(RequestStatus::Monitoring, RequestStatus::Fulfilled),
            Ok(valid)
                if valid.from() == RequestStatus::Monitoring
                    && valid.to() == RequestStatus::Fulfilled
        ));
    }

    #[test]
    fn monitoring_to_failed_is_valid() {
        assert!(matches!(
            validate_transition(RequestStatus::Monitoring, RequestStatus::Failed),
            Ok(valid)
                if valid.from() == RequestStatus::Monitoring
                    && valid.to() == RequestStatus::Failed
        ));
    }

    #[test]
    fn denied_to_approved_is_invalid() {
        assert!(matches!(
            validate_transition(RequestStatus::Denied, RequestStatus::Approved),
            Err(AitesisError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn fulfilled_to_monitoring_is_invalid() {
        assert!(matches!(
            validate_transition(RequestStatus::Fulfilled, RequestStatus::Monitoring),
            Err(AitesisError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn submitted_to_monitoring_is_invalid() {
        assert!(matches!(
            validate_transition(RequestStatus::Submitted, RequestStatus::Monitoring),
            Err(AitesisError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn full_lifecycle_transitions_are_valid() {
        assert!(matches!(
            validate_transition(RequestStatus::Submitted, RequestStatus::Approved),
            Ok(valid)
                if valid.from() == RequestStatus::Submitted
                    && valid.to() == RequestStatus::Approved
        ));
        assert!(matches!(
            validate_transition(RequestStatus::Approved, RequestStatus::Monitoring),
            Ok(valid)
                if valid.from() == RequestStatus::Approved
                    && valid.to() == RequestStatus::Monitoring
        ));
        assert!(matches!(
            validate_transition(RequestStatus::Monitoring, RequestStatus::Fulfilled),
            Ok(valid)
                if valid.from() == RequestStatus::Monitoring
                    && valid.to() == RequestStatus::Fulfilled
        ));
    }
}
