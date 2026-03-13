pub mod cache;
pub mod error;
pub mod identity;
pub mod providers;
pub mod rate_limit;
pub mod resolver;

pub use error::EpignosisError;
pub use identity::{
    EnrichedMetadata, FingerprintResult, MediaIdentity, ParsedFilename, ProviderEnrichment,
    UnidentifiedItem, parse_filename,
};
pub use resolver::EpignosisService;

use std::path::Path;

use tokio_util::sync::CancellationToken;

#[expect(
    async_fn_in_trait,
    reason = "async fn in trait is stable since Rust 1.75; suppressed until Send bound concern is resolved"
)]
pub trait MetadataResolver: Send + Sync {
    async fn resolve_identity(
        &self,
        item: &UnidentifiedItem,
        ct: CancellationToken,
    ) -> Result<MediaIdentity, EpignosisError>;

    async fn enrich(
        &self,
        identity: &MediaIdentity,
        ct: CancellationToken,
    ) -> Result<EnrichedMetadata, EpignosisError>;

    async fn fingerprint_audio(
        &self,
        file_path: &Path,
        ct: CancellationToken,
    ) -> Result<FingerprintResult, EpignosisError>;
}
