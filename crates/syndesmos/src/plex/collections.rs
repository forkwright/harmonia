//! Plex collection management — Kometa replacement.
//!
//! Maps Harmonia media tags to Plex collections, keeping library metadata
//! consistent without a separate Kometa/PMM process.

use crate::error::SyndesmodError;

/// Manages Plex collections derived from Harmonia metadata.
///
/// Placeholder for v1: defines the trait surface; full implementation
/// maps Harmonia tags → Plex collection create/update calls.
#[expect(
    dead_code,
    reason = "placeholder trait surface defined by spec; implementation deferred to post-v1"
)]
pub(crate) trait CollectionManager: Send + Sync {
    /// Synchronises a named collection, creating or updating it in Plex.
    fn sync_collection(
        &self,
        name: &str,
        media_ids: &[String],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SyndesmodError>> + Send + '_>>;
}
