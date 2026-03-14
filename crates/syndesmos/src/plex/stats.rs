//! Plex viewing statistics — Wrapperr replacement.
//!
//! Queries Plex watch history to power Harmonia's listening/viewing stats
//! without a separate Wrapperr process.

use crate::error::SyndesmodError;

/// Fetches viewing history from Plex.
///
/// Placeholder for v1: defines the trait surface; full implementation
/// queries `/status/sessions/history/all` and maps to Harmonia records.
#[expect(
    dead_code,
    reason = "placeholder trait surface defined by spec; implementation deferred to post-v1"
)]
pub(crate) trait StatsProvider: Send + Sync {
    /// Returns raw watch history entries for the given Plex user.
    fn fetch_watch_history(
        &self,
        plex_user_id: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<serde_json::Value>, SyndesmodError>>
                + Send
                + '_,
        >,
    >;
}
