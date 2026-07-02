use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use apotheke::error::TransactionSnafu;
use futures::stream::{self, StreamExt};
use horismos::SearchSubsystemConfig;
use snafu::ResultExt;
use sqlx::SqlitePool;
use themelion::{EventSender, HarmoniaEvent, QueryId};
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument, warn};

use crate::cf_bypass::CloudflareProxy;
use crate::client::newznab::NewznabClient;
use crate::client::torznab::TorznabClient;
use crate::client::{DynIndexerClient, IndexerConfig};
use crate::error::{self, SearchIndexerError};
use crate::rate_limit::RateLimiter;
use crate::repo::{self, IndexerRow};
use crate::types::{IndexerCaps, IndexerStatus, SearchMediaType, SearchQuery, SearchResult};

/// Fallback back-off when a 429 carries no Retry-After header.
const DEFAULT_RETRY_AFTER_SECS: u64 = 60;
/// Upper bound on honored Retry-After — a hostile header must not park an
/// indexer indefinitely.
const MAX_RETRY_AFTER_SECS: u64 = 3600;

pub struct SearchIndexerService {
    read_pool: SqlitePool,
    write_pool: SqlitePool,
    cf_proxy: Arc<dyn CloudflareProxy>,
    rate_limiter: RateLimiter,
    http: reqwest::Client,
    config: SearchSubsystemConfig,
    event_tx: EventSender,
}

impl SearchIndexerService {
    pub fn new(
        read_pool: SqlitePool,
        write_pool: SqlitePool,
        cf_proxy: Arc<dyn CloudflareProxy>,
        config: SearchSubsystemConfig,
        event_tx: EventSender,
    ) -> Self {
        let rate_limiter = RateLimiter::new(
            config.per_indexer_rate_limit_requests,
            Duration::from_secs(config.per_indexer_rate_limit_window_seconds),
        );

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .build()
            .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config

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
        query: SearchQuery,
        ct: CancellationToken,
    ) -> Result<Vec<SearchResult>, SearchIndexerError> {
        let query_id = QueryId::new();

        // Step 1: Filter eligible indexers
        let indexers = repo::get_eligible_indexers(&self.read_pool)
            .await
            .map_err(|e| SearchIndexerError::Database {
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        // Step 2: Filter by search function support
        let eligible = filter_by_capability(&indexers, &query);

        info!(
            query_id = %query_id,
            eligible_count = eligible.len(),
            "starting search fan-out"
        );

        // Step 3: Parallel fan-out
        let cf_proxy = Arc::clone(&self.cf_proxy);
        let http = self.http.clone();
        let timeout = Duration::from_secs(self.config.search_timeout_seconds);
        let max_body_bytes = self.config.max_response_body_bytes;
        let rate_limiter = &self.rate_limiter;

        let results: Vec<SearchResult> = stream::iter(eligible)
            .map(|indexer| {
                let cf = Arc::clone(&cf_proxy);
                let h = http.clone();
                let ct = ct.clone();
                let q = query.clone();
                async move {
                    if !rate_limiter.acquire(indexer.id, &ct).await {
                        // WHY: cancellation during rate-limit back-off — the
                        // search is abandoned, skip the fetch entirely.
                        info!(
                            indexer_id = indexer.id,
                            indexer_name = %indexer.name,
                            "search cancelled while awaiting rate limit"
                        );
                        return Vec::new();
                    }
                    let client = make_client(&indexer, h, cf, timeout, max_body_bytes);
                    match client.search_boxed(&q, ct).await {
                        Ok(results) => results,
                        Err(e @ SearchIndexerError::Cancelled { .. }) => {
                            // WHY: cancellation is caller intent, not indexer
                            // failure — no warn, no status change.
                            info!(
                                indexer_id = indexer.id,
                                indexer_name = %indexer.name,
                                error = %e,
                                "search cancelled for indexer"
                            );
                            Vec::new()
                        }
                        Err(e) => {
                            warn!(
                                indexer_id = indexer.id,
                                indexer_name = %indexer.name,
                                error = %e,
                                "search failed for indexer"
                            );
                            self.handle_search_error(&indexer, &e).await;
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
        self.event_tx
            .send(HarmoniaEvent::SearchCompleted {
                query_id,
                result_count: deduped.len(),
            })
            .ok();

        info!(
            query_id = %query_id,
            result_count = deduped.len(),
            "search completed"
        );

        Ok(deduped)
    }

    pub async fn test_indexer(
        &self,
        indexer_id: i64,
        ct: CancellationToken,
    ) -> Result<IndexerStatus, SearchIndexerError> {
        let indexer = self.load_indexer(indexer_id).await?;
        let client = make_client(
            &indexer,
            self.http.clone(),
            Arc::clone(&self.cf_proxy),
            Duration::from_secs(self.config.request_timeout_secs),
            self.config.max_response_body_bytes,
        );
        let status = client.test_boxed(ct).await?;
        let db_status = if status.healthy { "active" } else { "degraded" };
        repo::update_indexer_status(&self.write_pool, indexer_id, db_status)
            .await
            .map_err(|e| SearchIndexerError::Database {
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        Ok(status)
    }

    pub async fn refresh_caps(
        &self,
        indexer_id: i64,
        ct: CancellationToken,
    ) -> Result<IndexerCaps, SearchIndexerError> {
        let indexer = self.load_indexer(indexer_id).await?;
        let client = make_client(
            &indexer,
            self.http.clone(),
            Arc::clone(&self.cf_proxy),
            Duration::from_secs(self.config.request_timeout_secs),
            self.config.max_response_body_bytes,
        );
        let caps =
            client
                .caps_boxed(ct)
                .await
                .map_err(|source| SearchIndexerError::CapsUnavailable {
                    indexer_id,
                    source: Box::new(source),
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?;
        let caps_json =
            serde_json::to_string(&caps).map_err(|error| SearchIndexerError::ParseResponse {
                url: indexer.url.clone(),
                error: error.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        let now = jiff::Timestamp::now().to_string();

        // WHY: caps, categories, and status describe one observation of the
        // indexer — a single transaction keeps them consistent under failure.
        let mut tx = self
            .write_pool
            .begin()
            .await
            .context(TransactionSnafu)
            .context(error::DatabaseSnafu)?;
        repo::update_indexer_caps(&mut *tx, indexer_id, &caps_json, &now)
            .await
            .context(error::DatabaseSnafu)?;
        repo::upsert_indexer_categories(&mut tx, indexer_id, &caps)
            .await
            .context(error::DatabaseSnafu)?;
        repo::update_indexer_status(&mut *tx, indexer_id, "active")
            .await
            .context(error::DatabaseSnafu)?;
        tx.commit()
            .await
            .context(TransactionSnafu)
            .context(error::DatabaseSnafu)?;
        Ok(caps)
    }

    async fn load_indexer(&self, indexer_id: i64) -> Result<IndexerRow, SearchIndexerError> {
        repo::get_indexer(&self.read_pool, indexer_id)
            .await
            .map_err(|e| SearchIndexerError::Database {
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })?
            .ok_or_else(|| SearchIndexerError::IndexerNotFound {
                indexer_id,
                location: snafu::Location::new(file!(), line!(), column!()),
            })
    }

    async fn handle_search_error(&self, indexer: &IndexerRow, error: &SearchIndexerError) {
        if let SearchIndexerError::RateLimited {
            retry_after_seconds,
            ..
        } = error
        {
            let secs = retry_after_seconds
                .unwrap_or(DEFAULT_RETRY_AFTER_SECS)
                .min(MAX_RETRY_AFTER_SECS);
            self.rate_limiter
                .set_retry_after(indexer.id, Duration::from_secs(secs))
                .await;
        }

        let new_status = match error {
            SearchIndexerError::AuthFailed { .. } => Some("failed"),
            SearchIndexerError::NoCfBypass { .. } => Some("degraded"),
            SearchIndexerError::CfProxyTimeout { .. } | SearchIndexerError::CfProxyError { .. } => {
                Some("degraded")
            }
            SearchIndexerError::ParseResponse { .. }
            | SearchIndexerError::ResponseTooLarge { .. } => Some("degraded"),
            SearchIndexerError::HttpRequest { .. } => {
                if indexer.status == "degraded" {
                    Some("failed")
                } else {
                    Some("degraded")
                }
            }
            // WHY: cancellation is caller intent and a 429 is back-pressure —
            // neither says the indexer itself is unhealthy.
            SearchIndexerError::Cancelled { .. } | SearchIndexerError::RateLimited { .. } => None,
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

fn filter_by_capability(indexers: &[IndexerRow], query: &SearchQuery) -> Vec<IndexerRow> {
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

            let caps = match serde_json::from_str::<IndexerCaps>(caps_json) {
                Ok(caps) => caps,
                Err(e) => {
                    // WHY: this is the decision site excluding the indexer from a
                    // typed search — stale/corrupt caps must be visible, not a
                    // silent disappearance from results.
                    warn!(
                        indexer_id = indexer.id,
                        error = %e,
                        "invalid caps_json, excluding indexer from typed search"
                    );
                    return false;
                }
            };

            crate::types::supports_function(&caps, function_type)
        })
        .cloned()
        .collect()
}

fn deduplicate(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();
    let mut seen_guids: HashMap<String, usize> = HashMap::new();
    let mut deduped: Vec<SearchResult> = Vec::with_capacity(results.len());

    for result in results {
        // WHY: hash and guid are checked independently (not else-if) — a result
        // carrying both must register both keys, or a later copy sharing only
        // the guid slips past dedup.
        let hash_lower = result.info_hash.as_ref().map(|h| h.to_lowercase());
        let is_dupe = hash_lower
            .as_ref()
            .is_some_and(|h| seen_hashes.contains_key(h))
            || result
                .guid
                .as_ref()
                .is_some_and(|g| seen_guids.contains_key(g));
        if is_dupe {
            continue;
        }
        if let Some(hash) = hash_lower {
            seen_hashes.insert(hash, deduped.len());
        }
        if let Some(ref guid) = result.guid {
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
    max_body_bytes: u64,
) -> Box<dyn DynIndexerClient> {
    let config = IndexerConfig {
        id: indexer.id,
        name: indexer.name.clone(),
        url: indexer.url.clone(),
        api_key: indexer.api_key.clone(),
        cf_bypass: indexer.cf_bypass,
    };

    match indexer.protocol.as_str() {
        "newznab" => Box::new(NewznabClient::new(
            config,
            http,
            cf_proxy,
            timeout,
            max_body_bytes,
        )),
        _ => Box::new(TorznabClient::new(
            config,
            http,
            cf_proxy,
            timeout,
            max_body_bytes,
        )),
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
    fn dedup_registers_guid_of_hash_bearing_result() {
        // WHY: a result carrying both keys must register its guid too — a later
        // copy sharing only the guid must not slip past dedup.
        let results = vec![
            make_result("Release.A", Some("abc123"), Some("guid-1"), 1),
            make_result("Release.A.guid-dupe", None, Some("guid-1"), 2),
        ];

        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].title, "Release.A");
    }

    #[test]
    fn dedup_same_hash_different_guid_still_dedupes() {
        let results = vec![
            make_result("Release.A", Some("abc123"), Some("guid-1"), 1),
            make_result("Release.A.hash-dupe", Some("abc123"), Some("guid-2"), 2),
        ];

        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].title, "Release.A");
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

    // ── handle_search_error / refresh_caps (live service over in-memory db) ──

    use apotheke::migrate::MIGRATOR;
    use themelion::create_event_bus;

    use crate::cf_bypass::noop::NoProxy;
    use crate::repo::InsertIndexerParams;
    use crate::test_support::spawn_one_shot_http;

    async fn make_service() -> (SearchIndexerService, SqlitePool) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let (event_tx, _) = create_event_bus(16);
        let service = SearchIndexerService::new(
            pool.clone(),
            pool.clone(),
            Arc::new(NoProxy),
            SearchSubsystemConfig::default(),
            event_tx,
        );
        (service, pool)
    }

    async fn seed_indexer(pool: &SqlitePool, url: &str) -> IndexerRow {
        let id = repo::insert_indexer(
            pool,
            InsertIndexerParams {
                name: "Seeded",
                url,
                protocol: "torznab",
                api_key: Some("key"),
                cf_bypass: false,
                priority: 50,
            },
        )
        .await
        .unwrap();
        repo::get_indexer(pool, id).await.unwrap().unwrap()
    }

    fn cancelled_error() -> SearchIndexerError {
        SearchIndexerError::Cancelled {
            url: "https://example.com/api".to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        }
    }

    fn rate_limited_error(retry_after_seconds: Option<u64>) -> SearchIndexerError {
        SearchIndexerError::RateLimited {
            indexer_id: 1,
            retry_after_seconds,
            location: snafu::Location::new(file!(), line!(), column!()),
        }
    }

    #[tokio::test]
    async fn handle_search_error_cancelled_leaves_status_unchanged() {
        let (service, pool) = make_service().await;
        let indexer = seed_indexer(&pool, "https://example.com/api").await;
        assert_eq!(indexer.status, "active");

        service
            .handle_search_error(&indexer, &cancelled_error())
            .await;

        let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
        assert_eq!(row.status, "active");
    }

    #[tokio::test]
    async fn handle_search_error_auth_failed_marks_failed() {
        let (service, pool) = make_service().await;
        let indexer = seed_indexer(&pool, "https://example.com/api").await;

        let error = SearchIndexerError::AuthFailed {
            indexer_id: indexer.id,
            location: snafu::Location::new(file!(), line!(), column!()),
        };
        service.handle_search_error(&indexer, &error).await;

        let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
        assert_eq!(row.status, "failed");
    }

    #[tokio::test]
    async fn handle_search_error_parse_response_marks_degraded() {
        let (service, pool) = make_service().await;
        let indexer = seed_indexer(&pool, "https://example.com/api").await;

        let error = SearchIndexerError::ParseResponse {
            url: "https://example.com/api".to_string(),
            error: "bad xml".to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        };
        service.handle_search_error(&indexer, &error).await;

        let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
        assert_eq!(row.status, "degraded");
    }

    #[tokio::test]
    async fn handle_search_error_http_request_active_marks_degraded() {
        let (service, pool) = make_service().await;
        let indexer = seed_indexer(&pool, "https://example.com/api").await;
        assert_eq!(indexer.status, "active");

        let error = SearchIndexerError::HttpRequest {
            url: "https://example.com/api".to_string(),
            source: reqwest::Client::new()
                .get("http://127.0.0.1:9/")
                .send()
                .await
                .unwrap_err(),
            location: snafu::Location::new(file!(), line!(), column!()),
        };
        service.handle_search_error(&indexer, &error).await;

        let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
        assert_eq!(row.status, "degraded");
    }

    #[tokio::test]
    async fn handle_search_error_http_request_degraded_escalates_to_failed() {
        let (service, pool) = make_service().await;
        let mut indexer = seed_indexer(&pool, "https://example.com/api").await;
        repo::update_indexer_status(&pool, indexer.id, "degraded")
            .await
            .unwrap();
        indexer.status = "degraded".to_string();

        let error = SearchIndexerError::HttpRequest {
            url: "https://example.com/api".to_string(),
            source: reqwest::Client::new()
                .get("http://127.0.0.1:9/")
                .send()
                .await
                .unwrap_err(),
            location: snafu::Location::new(file!(), line!(), column!()),
        };
        service.handle_search_error(&indexer, &error).await;

        let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
        assert_eq!(row.status, "failed");
    }

    // WHY: the clock is paused only around the limiter interaction — sqlx's
    // sqlite worker runs on a real thread, and a paused clock during pool
    // setup auto-advances straight into PoolTimedOut.
    #[tokio::test]
    async fn handle_search_error_rate_limited_engages_retry_after() {
        let (service, pool) = make_service().await;
        let indexer = seed_indexer(&pool, "https://example.com/api").await;

        tokio::time::pause();
        service
            .handle_search_error(&indexer, &rate_limited_error(Some(120)))
            .await;

        let before = tokio::time::Instant::now();
        assert!(
            service
                .rate_limiter
                .acquire(indexer.id, &CancellationToken::new())
                .await
        );
        let elapsed = before.elapsed();
        tokio::time::resume();
        assert!(
            elapsed >= Duration::from_secs(120),
            "expected >=120s back-off, got {elapsed:?}"
        );

        // 429 is back-pressure, not indexer failure — status must not change.
        let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
        assert_eq!(row.status, "active");
    }

    #[tokio::test]
    async fn handle_search_error_rate_limited_without_header_uses_default() {
        let (service, pool) = make_service().await;
        let indexer = seed_indexer(&pool, "https://example.com/api").await;

        tokio::time::pause();
        service
            .handle_search_error(&indexer, &rate_limited_error(None))
            .await;

        let before = tokio::time::Instant::now();
        assert!(
            service
                .rate_limiter
                .acquire(indexer.id, &CancellationToken::new())
                .await
        );
        let elapsed = before.elapsed();
        tokio::time::resume();
        assert!(
            elapsed >= Duration::from_secs(DEFAULT_RETRY_AFTER_SECS),
            "expected >={DEFAULT_RETRY_AFTER_SECS}s default back-off, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn handle_search_error_rate_limited_clamps_hostile_retry_after() {
        let (service, pool) = make_service().await;
        let indexer = seed_indexer(&pool, "https://example.com/api").await;

        tokio::time::pause();
        service
            .handle_search_error(&indexer, &rate_limited_error(Some(999_999)))
            .await;

        let before = tokio::time::Instant::now();
        assert!(
            service
                .rate_limiter
                .acquire(indexer.id, &CancellationToken::new())
                .await
        );
        let elapsed = before.elapsed();
        tokio::time::resume();
        assert!(
            elapsed >= Duration::from_secs(MAX_RETRY_AFTER_SECS),
            "expected the cap to engage, got {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(MAX_RETRY_AFTER_SECS + 60),
            "expected clamp at {MAX_RETRY_AFTER_SECS}s, got {elapsed:?}"
        );
    }

    const CAPS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<caps>
  <server title="Test Indexer" version="1.0"/>
  <limits default="100" max="500"/>
  <searching>
    <search available="yes"/>
  </searching>
  <categories>
    <category id="2000" name="Movies">
      <subcat id="2010" name="Movies/Foreign"/>
    </category>
  </categories>
</caps>"#;

    #[tokio::test]
    async fn refresh_caps_commits_caps_categories_and_status_together() {
        let (service, pool) = make_service().await;
        let (url, _server) = spawn_one_shot_http(200, "OK", &[], CAPS_XML).await;
        let indexer = seed_indexer(&pool, &url).await;
        repo::update_indexer_status(&pool, indexer.id, "degraded")
            .await
            .unwrap();

        let caps = service
            .refresh_caps(indexer.id, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(caps.categories.len(), 1);

        let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
        assert_eq!(row.status, "active");
        assert!(row.caps_json.is_some());
        assert!(row.last_tested.is_some());

        let categories = sqlx::query_as::<_, (i64, String)>(
            "SELECT category_id, name FROM indexer_categories
             WHERE indexer_id = ? ORDER BY category_id",
        )
        .bind(indexer.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            categories,
            vec![
                (2000, "Movies".to_string()),
                (2010, "Movies/Foreign".to_string())
            ]
        );
    }
}
