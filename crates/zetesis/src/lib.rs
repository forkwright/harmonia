pub mod cf_bypass;
pub mod client;
pub mod error;
pub mod rate_limit;
pub mod repo;
pub mod results_cache;
pub mod search;
pub(crate) mod test_support;
pub mod types;

pub use cf_bypass::CloudflareProxy;
pub use client::IndexerClient;
pub use client::cardigann::{CardigannClient, CardigannRegistry};
pub use error::SearchIndexerError;
pub use search::SearchIndexerService;
pub use types::{
    CataloguedResult, DownloadResponse, IndexerCaps, IndexerStatus, ReleaseProtocol,
    ResolvedRelease, SearchMediaType, SearchOutcome, SearchQuery, SearchResult,
};
