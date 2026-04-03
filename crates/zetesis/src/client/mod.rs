pub mod newznab;
pub mod torznab;
pub mod xml;

use tokio_util::sync::CancellationToken;

use crate::error::ZetesisError;
use crate::types::{DownloadResponse, IndexerCaps, IndexerStatus, SearchQuery, SearchResult};

pub trait IndexerClient: Send + Sync {
    fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<Vec<SearchResult>, ZetesisError>> + Send;

    fn caps(
        &self,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<IndexerCaps, ZetesisError>> + Send;

    fn test(
        &self,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<IndexerStatus, ZetesisError>> + Send;

    fn download(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<DownloadResponse, ZetesisError>> + Send;
}

use std::future::Future;
use std::pin::Pin;

pub trait DynIndexerClient: Send + Sync {
    fn search_boxed<'a>(
        &'a self,
        query: &'a SearchQuery,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SearchResult>, ZetesisError>> + Send + 'a>>;
}

impl<T: IndexerClient> DynIndexerClient for T {
    fn search_boxed<'a>(
        &'a self,
        query: &'a SearchQuery,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SearchResult>, ZetesisError>> + Send + 'a>> {
        Box::pin(self.search(query, ct))
    }
}

pub struct IndexerConfig {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub cf_bypass: bool,
}

pub(crate) fn build_search_url(config: &IndexerConfig, query: &SearchQuery) -> String {
    let mut url = format!(
        "{}?t={}",
        config.url.trim_end_matches('/'),
        query.search_function()
    );

    if let Some(ref q) = query.query_text {
        url.push_str(&format!("&q={}", urlencoding(q)));
    }

    if !query.category_ids.is_empty() {
        let cats: Vec<String> = query.category_ids.iter().map(|c| c.to_string()).collect();
        url.push_str(&format!("&cat={}", cats.join(",")));
    }

    if let Some(ref imdb) = query.imdb_id {
        url.push_str(&format!("&imdbid={imdb}"));
    }
    if let Some(tvdb) = query.tvdb_id {
        url.push_str(&format!("&tvdbid={tvdb}"));
    }
    if let Some(tmdb) = query.tmdb_id {
        url.push_str(&format!("&tmdbid={tmdb}"));
    }
    if let Some(ref artist) = query.artist {
        url.push_str(&format!("&artist={}", urlencoding(artist)));
    }
    if let Some(ref album) = query.album {
        url.push_str(&format!("&album={}", urlencoding(album)));
    }
    if let Some(ref author) = query.author {
        url.push_str(&format!("&author={}", urlencoding(author)));
    }
    if let Some(season) = query.season {
        url.push_str(&format!("&season={season}"));
    }
    if let Some(episode) = query.episode {
        url.push_str(&format!("&ep={episode}"));
    }

    url.push_str(&format!("&LIMIT={}", query.limit));
    if query.offset > 0 {
        url.push_str(&format!("&OFFSET={}", query.offset));
    }

    if let Some(ref key) = config.api_key {
        url.push_str(&format!("&apikey={key}"));
    }

    url
}

pub(crate) fn build_caps_url(config: &IndexerConfig) -> String {
    let mut url = format!("{}?t=caps", config.url.trim_end_matches('/'));
    if let Some(ref key) = config.api_key {
        url.push_str(&format!("&apikey={key}"));
    }
    url
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{b:02X}"));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SearchMediaType;

    #[test]
    fn build_search_url_basic() {
        let config = IndexerConfig {
            id: 1,
            name: "Test".to_string(),
            url: "https://example.com/api".to_string(),
            api_key: Some("abc123".to_string()),
            cf_bypass: false,
        };
        let query = SearchQuery {
            query_text: Some("test query".to_string()),
            media_type: SearchMediaType::Any,
            limit: 100,
            ..Default::default()
        };

        let url = build_search_url(&config, &query);
        assert!(url.starts_with("https://example.com/api?t=search"));
        assert!(url.contains("q=test+query"));
        assert!(url.contains("apikey=abc123"));
        assert!(url.contains("LIMIT=100"));
    }

    #[test]
    fn build_search_url_tv() {
        let config = IndexerConfig {
            id: 1,
            name: "Test".to_string(),
            url: "https://example.com/api/".to_string(),
            api_key: None,
            cf_bypass: false,
        };
        let query = SearchQuery {
            media_type: SearchMediaType::Tv,
            tvdb_id: Some(12345),
            season: Some(3),
            episode: Some(5),
            limit: 50,
            ..Default::default()
        };

        let url = build_search_url(&config, &query);
        assert!(url.starts_with("https://example.com/api?t=tvsearch"));
        assert!(url.contains("tvdbid=12345"));
        assert!(url.contains("season=3"));
        assert!(url.contains("ep=5"));
        assert!(!url.contains("apikey="));
    }

    #[test]
    fn build_caps_url_with_key() {
        let config = IndexerConfig {
            id: 1,
            name: "Test".to_string(),
            url: "https://example.com/api".to_string(),
            api_key: Some("key123".to_string()),
            cf_bypass: false,
        };

        let url = build_caps_url(&config);
        assert_eq!(url, "https://example.com/api?t=caps&apikey=key123");
    }

    #[test]
    fn urlencoding_special_chars() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("test&value"), "test%26value");
        assert_eq!(urlencoding("normal"), "normal");
    }
}
