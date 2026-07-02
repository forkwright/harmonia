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
use crate::client::cardigann::CardigannRegistry;
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
    cardigann: CardigannRegistry,
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

        // WHY: definitions are read once at startup — this is the read site
        // for `cardigann_definitions_dir`; per-search dispatch only resolves
        // against the loaded set.
        let cardigann = CardigannRegistry::load(Arc::new(config.clone()));

        Self {
            read_pool,
            write_pool,
            cf_proxy,
            rate_limiter,
            http,
            config,
            cardigann,
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
                    let client = match make_client(
                        &indexer,
                        h,
                        cf,
                        timeout,
                        max_body_bytes,
                        &self.cardigann,
                    ) {
                        Ok(client) => client,
                        Err(e) => {
                            warn!(
                                indexer_id = indexer.id,
                                indexer_name = %indexer.name,
                                error = %e,
                                "failed to construct indexer client"
                            );
                            self.handle_search_error(&indexer, &e).await;
                            return Vec::new();
                        }
                    };
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
            &self.cardigann,
        )?;
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
            &self.cardigann,
        )?;
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
            // WHY: a missing/unsupported/misconfigured Cardigann definition
            // fails every search identically until the operator intervenes.
            SearchIndexerError::DefinitionNotFound { .. }
            | SearchIndexerError::DefinitionLoad { .. }
            | SearchIndexerError::DefinitionInvalid { .. }
            | SearchIndexerError::DefinitionUnsupported { .. }
            | SearchIndexerError::LoginUnsupported { .. }
            | SearchIndexerError::CookieAuthRequired { .. } => Some("failed"),
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
    cardigann: &CardigannRegistry,
) -> Result<Box<dyn DynIndexerClient>, SearchIndexerError> {
    let config = IndexerConfig {
        id: indexer.id,
        name: indexer.name.clone(),
        url: indexer.url.clone(),
        api_key: indexer.api_key.clone(),
        cf_bypass: indexer.cf_bypass,
    };

    Ok(match indexer.protocol.as_str() {
        "newznab" => Box::new(NewznabClient::new(
            config,
            http,
            cf_proxy,
            timeout,
            max_body_bytes,
        )),
        "cardigann" => Box::new(cardigann.client_for(config, http, cf_proxy, timeout)?),
        _ => Box::new(TorznabClient::new(
            config,
            http,
            cf_proxy,
            timeout,
            max_body_bytes,
        )),
    })
}

#[cfg(test)]
mod tests;
