use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::cf_bypass::CloudflareProxy;
use crate::client::xml::{get_attr, get_attr_f64, get_attr_u32, parse_caps_xml, parse_feed_xml};
use crate::client::{
    IndexerClient, IndexerConfig, build_caps_url, build_search_url, read_body_bounded,
    redact_api_key, validate_fetch_url,
};
use crate::error::{self, SearchIndexerError};
use crate::types::{
    DownloadResponse, IndexerCaps, IndexerStatus, ReleaseProtocol, SearchQuery, SearchResult,
};

pub struct TorznabClient {
    pub config: IndexerConfig,
    http: reqwest::Client,
    cf_proxy: Arc<dyn CloudflareProxy>,
    timeout: Duration,
    max_body_bytes: u64,
}

impl TorznabClient {
    pub fn new(
        config: IndexerConfig,
        http: reqwest::Client,
        cf_proxy: Arc<dyn CloudflareProxy>,
        timeout: Duration,
        max_body_bytes: u64,
    ) -> Self {
        Self {
            config,
            http,
            cf_proxy,
            timeout,
            max_body_bytes,
        }
    }

    async fn fetch_xml(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<String, SearchIndexerError> {
        if self.config.cf_bypass {
            let response = self.cf_proxy.get(url, ct).await?;
            return Ok(response.body);
        }

        let fut = self.http.get(url).timeout(self.timeout).send();
        let response = tokio::select! {
            result = fut => result.context(error::HttpRequestSnafu { url: redact_api_key(url) })?,
            () = ct.cancelled() => {
                return Err(SearchIndexerError::Cancelled {
                    url: redact_api_key(url),
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(SearchIndexerError::AuthFailed {
                indexer_id: self.config.id,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            return Err(SearchIndexerError::RateLimited {
                indexer_id: self.config.id,
                retry_after_seconds: retry_after,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }

        read_body_bounded(response, url, self.max_body_bytes).await
    }
}

impl IndexerClient for TorznabClient {
    #[instrument(skip(self, ct), fields(indexer_id = self.config.id, indexer_name = %self.config.name))]
    async fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> Result<Vec<SearchResult>, SearchIndexerError> {
        let url = build_search_url(&self.config, query);
        let xml = self.fetch_xml(&url, ct).await?;
        let feed = parse_feed_xml(&xml).map_err(|e| SearchIndexerError::ParseResponse {
            url: redact_api_key(&url),
            error: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

        let results = feed
            .channel
            .items
            .into_iter()
            .map(|item| {
                let download_url = item.link.unwrap_or_default();
                let info_hash = get_attr(&item.attrs, "infohash").map(str::to_string);
                let seeders = get_attr_u32(&item.attrs, "seeders");
                let leechers = get_attr_u32(&item.attrs, "leechers");
                let category_id = get_attr_u32(&item.attrs, "category");
                let download_volume_factor =
                    get_attr_f64(&item.attrs, "downloadvolumefactor").unwrap_or(1.0);
                let upload_volume_factor =
                    get_attr_f64(&item.attrs, "uploadvolumefactor").unwrap_or(1.0);

                let mut custom_attrs = HashMap::new();
                for attr in &item.attrs {
                    match attr.name.as_str() {
                        "seeders"
                        | "leechers"
                        | "infohash"
                        | "category"
                        | "downloadvolumefactor"
                        | "uploadvolumefactor"
                        | "size" => {}
                        _ => {
                            custom_attrs.insert(attr.name.clone(), attr.value.clone());
                        }
                    }
                }

                SearchResult {
                    title: item.title,
                    guid: item.guid,
                    download_url,
                    size_bytes: item.size,
                    seeders,
                    leechers,
                    info_hash,
                    category_id,
                    publication_date: item.pub_date,
                    indexer_id: self.config.id,
                    protocol: ReleaseProtocol::Torrent,
                    download_volume_factor,
                    upload_volume_factor,
                    custom_attrs,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self, ct), fields(indexer_id = self.config.id))]
    async fn caps(&self, ct: CancellationToken) -> Result<IndexerCaps, SearchIndexerError> {
        let url = build_caps_url(&self.config);
        let xml = self.fetch_xml(&url, ct).await?;
        parse_caps_xml(&xml).map_err(|e| SearchIndexerError::ParseResponse {
            url: redact_api_key(&url),
            error: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })
    }

    #[instrument(skip(self, ct), fields(indexer_id = self.config.id))]
    async fn test(&self, ct: CancellationToken) -> Result<IndexerStatus, SearchIndexerError> {
        match self.caps(ct).await {
            Ok(caps) => Ok(IndexerStatus {
                healthy: true,
                caps: Some(caps),
                error: None,
            }),
            Err(e) => Ok(IndexerStatus {
                healthy: false,
                caps: None,
                error: Some(e.to_string()),
            }),
        }
    }

    #[instrument(skip(self, ct), fields(indexer_id = self.config.id))]
    async fn download(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<DownloadResponse, SearchIndexerError> {
        if url.starts_with("magnet:") {
            return Ok(DownloadResponse::MagnetUri(url.to_string()));
        }

        // SAFETY: download URLs originate in indexer response XML (third-party
        // data) — validate scheme + resolved addresses before any fetch.
        validate_fetch_url(url).await?;
        let body = self.fetch_xml(url, ct).await?;
        Ok(DownloadResponse::TorrentFile(Bytes::from(body)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cf_bypass::noop::NoProxy;
    use crate::test_support::{spawn_hang_http, spawn_one_shot_http, spawn_raw_http};

    const FEED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Test Indexer</title>
    <item>
      <title>Test.Release.2024.FLAC</title>
      <guid>abc123</guid>
      <size>734003200</size>
      <link>https://example.com/download/abc123</link>
      <torznab:attr name="seeders" value="42"/>
      <torznab:attr name="infohash" value="deadbeef"/>
    </item>
  </channel>
</rss>"#;

    fn client(url: String, api_key: Option<&str>, max_body_bytes: u64) -> TorznabClient {
        TorznabClient::new(
            IndexerConfig {
                id: 1,
                name: "Test".to_string(),
                url,
                api_key: api_key.map(str::to_string),
                cf_bypass: false,
            },
            reqwest::Client::new(),
            Arc::new(NoProxy),
            Duration::from_secs(5),
            max_body_bytes,
        )
    }

    fn query() -> SearchQuery {
        SearchQuery {
            query_text: Some("test".to_string()),
            limit: 100,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn fetch_200_returns_parsed_results() {
        let (url, _server) = spawn_one_shot_http(200, "OK", &[], FEED_XML).await;
        let results = client(url, None, 1 << 20)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test.Release.2024.FLAC");
        assert_eq!(results[0].seeders, Some(42));
        assert_eq!(results[0].protocol, ReleaseProtocol::Torrent);
    }

    #[tokio::test]
    async fn fetch_401_maps_to_auth_failed() {
        let (url, _server) = spawn_one_shot_http(401, "Unauthorized", &[], "").await;
        let err = client(url, None, 1 << 20)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            SearchIndexerError::AuthFailed { indexer_id: 1, .. }
        ));
    }

    #[tokio::test]
    async fn fetch_403_maps_to_auth_failed() {
        let (url, _server) = spawn_one_shot_http(403, "Forbidden", &[], "").await;
        let err = client(url, None, 1 << 20)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(err, SearchIndexerError::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn fetch_429_with_retry_after_maps_to_rate_limited() {
        let (url, _server) =
            spawn_one_shot_http(429, "Too Many Requests", &[("retry-after", "120")], "").await;
        let err = client(url, None, 1 << 20)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            SearchIndexerError::RateLimited {
                retry_after_seconds: Some(120),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn fetch_429_without_retry_after_maps_to_rate_limited_none() {
        let (url, _server) = spawn_one_shot_http(429, "Too Many Requests", &[], "").await;
        let err = client(url, None, 1 << 20)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            SearchIndexerError::RateLimited {
                retry_after_seconds: None,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn fetch_500_falls_through_to_parse_error() {
        // NOTE: documents current behavior — a 5xx body is read and fails XML
        // parsing rather than mapping to a dedicated status error.
        let (url, _server) =
            spawn_one_shot_http(500, "Internal Server Error", &[], "not xml").await;
        let err = client(url, None, 1 << 20)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(err, SearchIndexerError::ParseResponse { .. }));
    }

    #[tokio::test]
    async fn fetch_cancelled_returns_cancelled_variant() {
        let (url, server) = spawn_hang_http().await;
        let ct = CancellationToken::new();
        ct.cancel();
        let err = client(url, None, 1 << 20)
            .search(&query(), ct)
            .await
            .unwrap_err();
        assert!(
            matches!(err, SearchIndexerError::Cancelled { .. }),
            "expected Cancelled, got {err:?}"
        );
        server.abort();
    }

    #[tokio::test]
    async fn fetch_rejects_oversized_content_length_before_body() {
        let raw =
            b"HTTP/1.1 200 OK\r\ncontent-length: 10000000\r\nconnection: close\r\n\r\n".to_vec();
        let (url, _server) = spawn_raw_http(raw).await;
        let err = client(url, None, 64)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                SearchIndexerError::ResponseTooLarge {
                    size: 10_000_000,
                    limit: 64,
                    ..
                }
            ),
            "expected ResponseTooLarge, got {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_rejects_oversized_stream_without_content_length() {
        // WHY: no content-length header — the streamed running counter is the
        // only enforcement (declared length can be absent or forged).
        let mut raw = b"HTTP/1.1 200 OK\r\nconnection: close\r\n\r\n".to_vec();
        raw.extend_from_slice(&[b'x'; 4096]);
        let (url, _server) = spawn_raw_http(raw).await;
        let err = client(url, None, 64)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(
            matches!(err, SearchIndexerError::ResponseTooLarge { limit: 64, .. }),
            "expected ResponseTooLarge, got {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_allows_body_under_limit() {
        let (url, _server) = spawn_one_shot_http(200, "OK", &[], FEED_XML).await;
        let results = client(url, None, FEED_XML.len() as u64 + 1)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn error_display_does_not_leak_api_key() {
        // WHY: port 9 (discard) on loopback refuses connections — forces an
        // HttpRequest error whose url field must carry the redacted form.
        let err = client(
            "http://127.0.0.1:9/api".to_string(),
            Some("supersecret"),
            1 << 20,
        )
        .search(&query(), CancellationToken::new())
        .await
        .unwrap_err();
        let display = err.to_string();
        assert!(!display.contains("supersecret"), "leaked: {display}");
        assert!(
            matches!(err, SearchIndexerError::HttpRequest { ref url, .. } if url.contains("[REDACTED]"))
        );
    }

    #[tokio::test]
    async fn parse_error_display_does_not_leak_api_key() {
        let (url, _server) = spawn_one_shot_http(200, "OK", &[], "not xml").await;
        let err = client(url, Some("supersecret"), 1 << 20)
            .search(&query(), CancellationToken::new())
            .await
            .unwrap_err();
        let display = err.to_string();
        assert!(!display.contains("supersecret"), "leaked: {display}");
    }

    #[tokio::test]
    async fn download_magnet_uri_short_circuits_without_network() {
        // WHY: config URL points at a refusing port — any network attempt
        // would error, so Ok(MagnetUri) proves no fetch happened.
        let magnet = "magnet:?xt=urn:btih:abc123";
        let response = client("http://127.0.0.1:9/api".to_string(), None, 1 << 20)
            .download(magnet, CancellationToken::new())
            .await
            .unwrap();
        assert!(matches!(response, DownloadResponse::MagnetUri(uri) if uri == magnet));
    }

    #[tokio::test]
    async fn download_rejects_ssrf_url() {
        for target in [
            "http://127.0.0.1:8080/steal",
            "http://169.254.169.254/latest/meta-data/",
            "http://192.168.1.1/admin",
            "file:///etc/passwd",
        ] {
            let err = client("http://127.0.0.1:9/api".to_string(), None, 1 << 20)
                .download(target, CancellationToken::new())
                .await
                .unwrap_err();
            assert!(
                matches!(err, SearchIndexerError::UnsafeUrl { .. }),
                "expected UnsafeUrl for {target}, got {err:?}"
            );
        }
    }
}
