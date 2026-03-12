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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
