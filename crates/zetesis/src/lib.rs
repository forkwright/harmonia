pub mod cf_bypass;
pub mod client;
pub mod error;
pub mod rate_limit;
pub mod repo;
pub mod search;
pub(crate) mod test_support;
pub mod types;

use std::sync::Arc;

pub use cf_bypass::CloudflareProxy;
pub use client::IndexerClient;
pub use error::SearchIndexerError;
use horismos::SearchSubsystemConfig;
pub use search::SearchIndexerService;
pub use types::{
    DownloadResponse, IndexerCaps, IndexerStatus, ReleaseProtocol, SearchMediaType, SearchQuery,
    SearchResult,
};

pub struct CardigannClient {
    #[expect(dead_code)]
    config: Arc<SearchSubsystemConfig>,
    #[expect(dead_code)]
    http_client: reqwest::Client,
}
