use harmonia_common::MediaType;
use serde::{Deserialize, Serialize};

use crate::error::EpignosisError;

pub mod acoustid;
pub mod audnexus;
pub mod comicvine;
pub mod itunes;
pub mod musicbrainz;
pub mod openlibrary;
pub mod tmdb;
pub mod tvdb;

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub media_type: MediaType,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResult {
    pub provider_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub score: f64,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub provider_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<u32>,
    pub extra: serde_json::Value,
}

#[allow(async_fn_in_trait)]
pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, EpignosisError>;

    async fn get_metadata(&self, provider_id: &str) -> Result<ProviderMetadata, EpignosisError>;
}
