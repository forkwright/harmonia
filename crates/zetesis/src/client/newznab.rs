use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::cf_bypass::CloudflareProxy;
use crate::client::xml::{get_attr_f64, get_attr_u32, parse_caps_xml, parse_feed_xml};
use crate::client::{IndexerClient, IndexerConfig, build_caps_url, build_search_url};
use crate::error::{self, ZetesisError};
use crate::types::{
    DownloadResponse, IndexerCaps, IndexerStatus, ReleaseProtocol, SearchQuery, SearchResult,
};

pub struct NewznabClient {
    pub config: IndexerConfig,
    http: reqwest::Client,
    cf_proxy: Arc<dyn CloudflareProxy>,
    timeout: Duration,
}

impl NewznabClient {
    pub fn new(
        config: IndexerConfig,
        http: reqwest::Client,
        cf_proxy: Arc<dyn CloudflareProxy>,
        timeout: Duration,
    ) -> Self {
        Self {
            config,
            http,
            cf_proxy,
            timeout,
        }
    }

    async fn fetch_xml(&self, url: &str, ct: CancellationToken) -> Result<String, ZetesisError> {
        if self.config.cf_bypass {
            let response = self.cf_proxy.get(url, ct).await?;
            return Ok(response.body);
        }

        let fut = self.http.get(url).timeout(self.timeout).send();
        let response = tokio::select! {
            result = fut => result.context(error::HttpRequestSnafu { url })?,
            () = ct.cancelled() => {
                return Err(ZetesisError::ParseResponse {
                    url: url.to_string(),
                    error: "request cancelled".to_string(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ZetesisError::AuthFailed {
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
            return Err(ZetesisError::RateLimited {
                indexer_id: self.config.id,
                retry_after_seconds: retry_after,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }

        let body = response
            .text()
            .await
            .context(error::HttpRequestSnafu { url })?;
        Ok(body)
    }
}

impl IndexerClient for NewznabClient {
    #[instrument(skip(self, ct), fields(indexer_id = self.config.id, indexer_name = %self.config.name))]
    async fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> Result<Vec<SearchResult>, ZetesisError> {
        let url = build_search_url(&self.config, query);
        let xml = self.fetch_xml(&url, ct).await?;
        let feed = parse_feed_xml(&xml).map_err(|e| ZetesisError::ParseResponse {
            url: url.clone(),
            error: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

        let results = feed
            .channel
            .items
            .into_iter()
            .map(|item| {
                let download_url = item.link.unwrap_or_default();
                let category_id = get_attr_u32(&item.attrs, "category");
                let download_volume_factor =
                    get_attr_f64(&item.attrs, "downloadvolumefactor").unwrap_or(1.0);
                let upload_volume_factor =
                    get_attr_f64(&item.attrs, "uploadvolumefactor").unwrap_or(1.0);

                let mut custom_attrs = HashMap::new();
                for attr in &item.attrs {
                    match attr.name.as_str() {
                        "category" | "downloadvolumefactor" | "uploadvolumefactor" | "size" => {}
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
                    seeders: None,
                    leechers: None,
                    info_hash: None,
                    category_id,
                    publication_date: item.pub_date,
                    indexer_id: self.config.id,
                    protocol: ReleaseProtocol::Nzb,
                    download_volume_factor,
                    upload_volume_factor,
                    custom_attrs,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self, ct), fields(indexer_id = self.config.id))]
    async fn caps(&self, ct: CancellationToken) -> Result<IndexerCaps, ZetesisError> {
        let url = build_caps_url(&self.config);
        let xml = self.fetch_xml(&url, ct).await?;
        parse_caps_xml(&xml).map_err(|e| ZetesisError::ParseResponse {
            url,
            error: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })
    }

    #[instrument(skip(self, ct), fields(indexer_id = self.config.id))]
    async fn test(&self, ct: CancellationToken) -> Result<IndexerStatus, ZetesisError> {
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
    ) -> Result<DownloadResponse, ZetesisError> {
        let body = self.fetch_xml(url, ct).await?;
        Ok(DownloadResponse::NzbFile(Bytes::from(body)))
    }
}
