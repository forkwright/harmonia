//! Addic7ed subtitle provider — placeholder pending full implementation.

use harmonia_common::{MediaId, MediaType};

use crate::error::ProsthekeError;
use crate::providers::SubtitleProvider;
use crate::types::SubtitleMatch;

/// Addic7ed provider stub.
///
/// Addic7ed is a secondary subtitle source specialising in TV episode
/// subtitles. Full implementation is deferred; this placeholder returns
/// empty results so the search orchestrator can be wired without blocking.
pub struct Addic7edProvider;

impl Addic7edProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Addic7edProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SubtitleProvider for Addic7edProvider {
    fn name(&self) -> &str {
        "addic7ed"
    }

    async fn search(
        &self,
        _media_id: &MediaId,
        _media_type: MediaType,
        _title: &str,
        _year: Option<u16>,
        _season: Option<u32>,
        _episode: Option<u32>,
        _languages: &[String],
        _file_hash: Option<&str>,
    ) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
        Ok(vec![])
    }

    async fn download(&self, _subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn placeholder_search_returns_empty() {
        let provider = Addic7edProvider::new();
        let media_id = harmonia_common::MediaId::new();
        let result = provider
            .search(
                &media_id,
                MediaType::Tv,
                "Breaking Bad",
                Some(2008),
                Some(1),
                Some(1),
                &["en".to_string()],
                None,
            )
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
