use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument, warn};

use themelion::{EventSender, HarmoniaEvent, QueryId};
use horismos::ZetesisConfig;

use crate::cf_bypass::CloudflareProxy;
use crate::client::newznab::NewznabClient;
use crate::client::torznab::TorznabClient;
use crate::client::{DynIndexerClient, IndexerConfig};
use crate::error::ZetesisError;
use crate::rate_limit::RateLimiter;
use crate::repo::{self, IndexerRow};
use crate::types::{IndexerCaps, SearchMediaType, SearchQuery, SearchResult};

pub struct ZetesisService {
    read_pool: SqlitePool,
    write_pool: SqlitePool,
    cf_proxy: Arc<dyn CloudflareProxy>,
    rate_limiter: RateLimiter,
    http: reqwest::Client,
    config: ZetesisConfig,
    event_tx: EventSender,
}

impl ZetesisService {
    pub fn new(
        read_pool: SqlitePool,
        write_pool: SqlitePool,
        cf_proxy: Arc<dyn CloudflareProxy>,
        config: ZetesisConfig,
        event_tx: EventSender,
    ) -> Self {
        let rate_limiter = RateLimiter::new(
            config.per_indexer_rate_limit_requests,
            Duration::from_secs(config.per_indexer_rate_limit_window_seconds),
        );

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            read_pool,
            write_pool,
            cf_proxy,
            rate_limiter,
            http,
            config,
            event_tx,
        }
    }

    #[instrument(skip(self, ct))]
    pub async fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> Result<Vec<SearchResult>, ZetesisError> {
        let query_id = QueryId::new();

        // Step 1: Filter eligible indexers
        let indexers = repo::get_eligible_indexers(&self.read_pool)
            .await
            .map_err(|e| ZetesisError::Database {
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        // Step 2: Filter by search function support
        let eligible = filter_by_capability(&indexers, query);

        info!(
            query_id = %query_id,
            eligible_count = eligible.len(),
            "starting search fan-out"
        );

        // Step 3: Parallel fan-out
        let cf_proxy = Arc::clone(&self.cf_proxy);
        let http = self.http.clone();
        let timeout = Duration::from_secs(self.config.search_timeout_seconds);
        let rate_limiter = &self.rate_limiter;

        let results: Vec<SearchResult> = stream::iter(eligible)
            .map(|indexer| {
                let cf = Arc::clone(&cf_proxy);
                let h = http.clone();
                let ct = ct.clone();
                let q = query;
                async move {
                    rate_limiter.acquire(indexer.id).await;
                    let client = make_client(indexer, h, cf, timeout);
                    match client.search_boxed(q, ct).await {
                        Ok(results) => results,
                        Err(e) => {
                            warn!(
                                indexer_id = indexer.id,
                                indexer_name = %indexer.name,
                                error = %e,
                                "search failed for indexer"
                            );
                            self.handle_search_error(indexer, &e).await;
                            Vec::new()
                        }
                    }
                }
            })
            .buffer_unordered(self.config.max_concurrent_searches)
            .flat_map(stream::iter)
            .collect()
            .await;

        // Step 4: Deduplication
        let deduped = deduplicate(results);

        // Step 5: Emit event
        let _ = self.event_tx.send(HarmoniaEvent::SearchCompleted {
            query_id,
            result_count: deduped.len(),
        });

        info!(
            query_id = %query_id,
            result_count = deduped.len(),
            "search completed"
        );

        Ok(deduped)
    }

    async fn handle_search_error(&self, indexer: &IndexerRow, error: &ZetesisError) {
        let new_status = match error {
            ZetesisError::AuthFailed { .. } => Some("failed"),
            ZetesisError::NoCfBypass { .. } => Some("degraded"),
            ZetesisError::CfProxyTimeout { .. } | ZetesisError::CfProxyError { .. } => {
                Some("degraded")
            }
            ZetesisError::ParseResponse { .. } => Some("degraded"),
            ZetesisError::HttpRequest { .. } => {
                if indexer.status == "degraded" {
                    Some("failed")
                } else {
                    Some("degraded")
                }
            }
            _ => None,
        };

        if let Some(status) = new_status
            && let Err(e) = repo::update_indexer_status(&self.write_pool, indexer.id, status).await
        {
            warn!(
                indexer_id = indexer.id,
                error = %e,
                "failed to UPDATE indexer status"
            );
        }
    }
}

fn filter_by_capability<'a>(
    indexers: &'a [IndexerRow],
    query: &SearchQuery,
) -> Vec<&'a IndexerRow> {
    let function_type = query.search_function();

    indexers
        .iter()
        .filter(|indexer| {
            if query.media_type == SearchMediaType::Any {
                return true;
            }

            let Some(ref caps_json) = indexer.caps_json else {
                return false;
            };

            let Ok(caps) = serde_json::from_str::<IndexerCaps>(caps_json) else {
                return false;
            };

            crate::types::supports_function(&caps, function_type)
        })
        .collect()
}

fn deduplicate(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();
    let mut seen_guids: HashMap<String, usize> = HashMap::new();
    let mut deduped: Vec<SearchResult> = Vec::with_capacity(results.len());

    for result in results {
        if let Some(ref hash) = result.info_hash {
            let hash_lower = hash.to_lowercase();
            if seen_hashes.contains_key(&hash_lower) {
                continue;
            }
            seen_hashes.insert(hash_lower, deduped.len());
        } else if let Some(ref guid) = result.guid {
            if seen_guids.contains_key(guid) {
                continue;
            }
            seen_guids.insert(guid.clone(), deduped.len());
        }

        deduped.push(result);
    }

    deduped
}

fn make_client(
    indexer: &IndexerRow,
    http: reqwest::Client,
    cf_proxy: Arc<dyn CloudflareProxy>,
    timeout: Duration,
) -> Box<dyn DynIndexerClient> {
    let config = IndexerConfig {
        id: indexer.id,
        name: indexer.name.clone(),
        url: indexer.url.clone(),
        api_key: indexer.api_key.clone(),
        cf_bypass: indexer.cf_bypass,
    };

    match indexer.protocol.as_str() {
        "newznab" => Box::new(NewznabClient::new(config, http, cf_proxy, timeout)),
        _ => Box::new(TorznabClient::new(config, http, cf_proxy, timeout)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ReleaseProtocol;

    fn make_result(
        title: &str,
        info_hash: Option<&str>,
        guid: Option<&str>,
        indexer_id: i64,
    ) -> SearchResult {
        SearchResult {
            title: title.to_string(),
            guid: guid.map(str::to_string),
            download_url: format!("https://example.com/{title}"),
            size_bytes: Some(1_000_000),
            seeders: Some(10),
            leechers: Some(2),
            info_hash: info_hash.map(str::to_string),
            category_id: Some(2000),
            publication_date: None,
            indexer_id,
            protocol: ReleaseProtocol::Torrent,
            download_volume_factor: 1.0,
            upload_volume_factor: 1.0,
            custom_attrs: HashMap::new(),
        }
    }

    #[test]
    fn dedup_by_info_hash() {
        let results = vec![
            make_result("Release.A", Some("abc123"), None, 1),
            make_result("Release.A.dupe", Some("abc123"), None, 2),
            make_result("Release.B", Some("def456"), None, 1),
        ];

        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].title, "Release.A");
        assert_eq!(deduped[1].title, "Release.B");
    }

    #[test]
    fn dedup_by_guid() {
        let results = vec![
            make_result("NZB.A", None, Some("guid-1"), 1),
            make_result("NZB.A.dupe", None, Some("guid-1"), 2),
            make_result("NZB.B", None, Some("guid-2"), 1),
        ];

        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].title, "NZB.A");
        assert_eq!(deduped[1].title, "NZB.B");
    }

    #[test]
    fn dedup_case_insensitive_hash() {
        let results = vec![
            make_result("Release.A", Some("ABC123"), None, 1),
            make_result("Release.A.dupe", Some("abc123"), None, 2),
        ];

        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn dedup_keeps_higher_priority() {
        let results = vec![
            make_result("Release.Priority1", Some("hash1"), None, 1),
            make_result("Release.Priority2", Some("hash1"), None, 2),
        ];

        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].indexer_id, 1);
    }

    #[test]
    fn dedup_no_hash_no_guid_keeps_all() {
        let results = vec![
            make_result("Release.A", None, None, 1),
            make_result("Release.B", None, None, 2),
        ];

        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn filter_capability_any_includes_all() {
        let indexers = vec![IndexerRow {
            id: 1,
            name: "Test1".to_string(),
            url: "https://example.com/api".to_string(),
            protocol: "torznab".to_string(),
            api_key: None,
            enabled: true,
            cf_bypass: false,
            status: "active".to_string(),
            last_tested: None,
            caps_json: None,
            priority: 50,
            added_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let query = SearchQuery {
            media_type: SearchMediaType::Any,
            ..Default::default()
        };

        let eligible = filter_by_capability(&indexers, &query);
        assert_eq!(eligible.len(), 1);
    }

    #[test]
    fn filter_capability_typed_excludes_no_caps() {
        let indexers = vec![IndexerRow {
            id: 1,
            name: "NoCaps".to_string(),
            url: "https://example.com/api".to_string(),
            protocol: "torznab".to_string(),
            api_key: None,
            enabled: true,
            cf_bypass: false,
            status: "active".to_string(),
            last_tested: None,
            caps_json: None,
            priority: 50,
            added_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let query = SearchQuery {
            media_type: SearchMediaType::Tv,
            ..Default::default()
        };

        let eligible = filter_by_capability(&indexers, &query);
        assert!(eligible.is_empty());
    }

    #[test]
    fn filter_capability_typed_includes_supported() {
        let caps = IndexerCaps {
            server: crate::types::ServerInfo {
                title: None,
                version: None,
            },
            limits: crate::types::SearchLimits::default(),
            search_functions: vec![crate::types::SearchFunction {
                function_type: "tvsearch".to_string(),
                available: true,
            }],
            categories: vec![],
        };

        let indexers = vec![IndexerRow {
            id: 1,
            name: "TVIndexer".to_string(),
            url: "https://example.com/api".to_string(),
            protocol: "torznab".to_string(),
            api_key: None,
            enabled: true,
            cf_bypass: false,
            status: "active".to_string(),
            last_tested: None,
            caps_json: Some(serde_json::to_string(&caps).unwrap()),
            priority: 50,
            added_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let query = SearchQuery {
            media_type: SearchMediaType::Tv,
            ..Default::default()
        };

        let eligible = filter_by_capability(&indexers, &query);
        assert_eq!(eligible.len(), 1);
    }
}
