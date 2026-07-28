use std::collections::HashMap;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub query_text: Option<String>,
    pub media_type: SearchMediaType,
    pub category_ids: Vec<u32>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u32>,
    pub tmdb_id: Option<u32>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub author: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub limit: u32,
    pub offset: u32,
}

impl SearchQuery {
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }

    pub fn search_function(&self) -> &'static str {
        match self.media_type {
            SearchMediaType::Any => "search",
            SearchMediaType::Tv => "tvsearch",
            SearchMediaType::Movie => "movie",
            SearchMediaType::Music => "music",
            SearchMediaType::Book => "book",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMediaType {
    #[default]
    Any,
    Tv,
    Movie,
    Music,
    Book,
}

impl SearchMediaType {
    pub fn fallback_category(&self) -> Option<u32> {
        match self {
            Self::Any => None,
            Self::Tv => Some(5000),
            Self::Movie => Some(2000),
            Self::Music => Some(3000),
            Self::Book => Some(7000),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub guid: Option<String>,
    pub download_url: String,
    pub size_bytes: Option<u64>,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub info_hash: Option<String>,
    pub category_id: Option<u32>,
    pub publication_date: Option<String>,
    pub indexer_id: i64,
    pub protocol: ReleaseProtocol,
    pub download_volume_factor: f64,
    pub upload_volume_factor: f64,
    pub custom_attrs: HashMap<String, String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseProtocol {
    Torrent,
    Nzb,
}

/// A search result joined with its server-side release identity — the
/// catalog entry the results cache hands back. `release_id` is the stable
/// key `POST /api/v1/downloads` resolves server-side at enqueue time; the
/// paroche HTTP boundary redacts `download_url` on every RESPONSE path
/// (this type carries the raw value — redaction happens at the route layer,
/// not here).
#[derive(Debug, Clone, Serialize)]
pub struct CataloguedResult {
    pub release_id: themelion::ReleaseId,
    #[serde(flatten)]
    pub result: SearchResult,
}

// WHY: ergonomic field/method access to the wrapped SearchResult
// (`catalogued.title`, `catalogued.download_url`) without repeating
// `.result` at every call site — the release_id is the only genuinely new
// field this type adds.
impl std::ops::Deref for CataloguedResult {
    type Target = SearchResult;
    fn deref(&self) -> &SearchResult {
        &self.result
    }
}

/// A completed search: its `QueryId` (the key `GET
/// /api/v1/search/{query_id}/results` looks up) and every catalogued
/// result — what the results cache stores and `SearchIndexerService::search`
/// returns.
#[derive(Debug, Clone, Serialize)]
pub struct SearchOutcome {
    pub query_id: themelion::QueryId,
    pub results: Vec<CataloguedResult>,
}

// WHY: ergonomic Vec-like access (`outcome.len()`, `outcome.is_empty()`,
// `outcome[0]`) — SearchOutcome's results ARE the search's payload; query_id
// is a genuine field, accessed directly, not through this deref.
impl std::ops::Deref for SearchOutcome {
    type Target = Vec<CataloguedResult>;
    fn deref(&self) -> &Vec<CataloguedResult> {
        &self.results
    }
}

/// Server-side-only resolution of a cached release's REAL download URL —
/// the credentialed value a Torznab/Newznab indexer embeds. Deliberately NOT
/// `Serialize`: a compile error is the guard against ever placing this on an
/// HTTP response path. The only legitimate consumer is
/// `SearchIndexerService::resolve_release`'s caller inside
/// `paroche::routes::download::enqueue_download`.
#[derive(Clone)]
pub struct ResolvedRelease {
    pub download_url: String,
    pub protocol: ReleaseProtocol,
    pub info_hash: Option<String>,
    pub indexer_id: i64,
    /// The catalogued result's title — carried through so a caller can
    /// persist a durable `releases` row before enqueueing (#651); this
    /// struct is otherwise download-URL-resolution-only.
    pub title: String,
    pub size_bytes: Option<u64>,
}

impl std::fmt::Debug for ResolvedRelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedRelease")
            .field("download_url", &"[redacted]")
            .field("protocol", &self.protocol)
            .field("info_hash", &self.info_hash)
            .field("indexer_id", &self.indexer_id)
            .field("title", &self.title)
            .field("size_bytes", &self.size_bytes)
            .finish()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum DownloadResponse {
    TorrentFile(Bytes),
    MagnetUri(String),
    NzbFile(Bytes),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerCaps {
    pub server: ServerInfo,
    pub limits: SearchLimits,
    pub search_functions: Vec<SearchFunction>,
    pub categories: Vec<IndexerCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub title: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchLimits {
    pub default: u32,
    pub max: u32,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            default: 100,
            max: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFunction {
    pub function_type: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerCategory {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub subcategories: Vec<IndexerCategory>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexerStatus {
    pub healthy: bool,
    pub caps: Option<IndexerCaps>,
    pub error: Option<String>,
}

pub fn supports_function(caps: &IndexerCaps, function_type: &str) -> bool {
    caps.search_functions
        .iter()
        .any(|f| f.function_type == function_type && f.available)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_release_debug_never_prints_the_credentialed_url() {
        let resolved = ResolvedRelease {
            download_url: "https://indexer.example/dl/42?apikey=SECRET".to_string(),
            protocol: ReleaseProtocol::Torrent,
            info_hash: Some("abc123".to_string()),
            indexer_id: 7,
            title: "Some.Release.Title".to_string(),
            size_bytes: Some(1_000_000),
        };
        let debug = format!("{resolved:?}");
        assert!(
            !debug.contains("SECRET"),
            "ResolvedRelease Debug must never print the credentialed URL: {debug}"
        );
        assert!(!debug.contains("indexer.example"));
    }
}
