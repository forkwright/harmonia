//! Retry logic and failure classification for download errors.

/// Classification of a download failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// Network error, tracker timeout, or other condition that may resolve on retry.
    Transient,
    /// No seeders after stalled timeout, corrupt torrent, or other unrecoverable state.
    Permanent,
}

/// Classifies a failure reason string into transient or permanent.
///
/// Permanent patterns are checked first; everything else is treated as transient.
pub(crate) fn classify_failure(reason: &str) -> FailureKind {
    let lower = reason.to_lowercase();
    let permanent_patterns = [
        "corrupt",
        "no seeders",
        "stalled",
        "invalid torrent",
        "magnet resolve failed",
        "bad data",
    ];
    if permanent_patterns.iter().any(|p| lower.contains(p)) {
        FailureKind::Permanent
    } else {
        FailureKind::Transient
    }
}

/// Returns the backoff duration in seconds for a given retry attempt (0-indexed).
///
/// Backoff schedule: `base`, `base * 4`, `base * 20`.
pub(crate) fn backoff_seconds(attempt: u32, base_seconds: u64) -> u64 {
    match attempt {
        0 => base_seconds,
        1 => base_seconds * 4,
        _ => base_seconds * 20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_torrent_is_permanent() {
        assert_eq!(
            classify_failure("corrupt torrent file"),
            FailureKind::Permanent
        );
    }

    #[test]
    fn no_seeders_is_permanent() {
        assert_eq!(
            classify_failure("no seeders available"),
            FailureKind::Permanent
        );
    }

    #[test]
    fn stalled_is_permanent() {
        assert_eq!(classify_failure("stalled download"), FailureKind::Permanent);
    }

    #[test]
    fn network_error_is_transient() {
        assert_eq!(
            classify_failure("network connection refused"),
            FailureKind::Transient
        );
    }

    #[test]
    fn tracker_timeout_is_transient() {
        assert_eq!(
            classify_failure("tracker announce timeout"),
            FailureKind::Transient
        );
    }

    #[test]
    fn backoff_first_attempt_is_base() {
        assert_eq!(backoff_seconds(0, 30), 30);
    }

    #[test]
    fn backoff_second_attempt_is_4x_base() {
        assert_eq!(backoff_seconds(1, 30), 120);
    }

    #[test]
    fn backoff_third_attempt_is_20x_base() {
        assert_eq!(backoff_seconds(2, 30), 600);
    }

    #[test]
    fn backoff_beyond_third_caps_at_20x() {
        assert_eq!(backoff_seconds(5, 30), 600);
    }
}
