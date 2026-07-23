use std::collections::{BTreeMap, HashMap};
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
use crate::client::cardigann::{CardigannRegistry, SessionStore};
use crate::client::newznab::NewznabClient;
use crate::client::torznab::TorznabClient;
use crate::client::{DynIndexerClient, IndexerConfig, SsrfGuardResolver};
use crate::error::{self, SearchIndexerError};
use crate::rate_limit::RateLimiter;
use crate::repo::{self, IndexerRow};
use crate::results_cache::ResultsCache;
use crate::types::{
    IndexerCaps, IndexerStatus, ResolvedRelease, SearchMediaType, SearchOutcome, SearchQuery,
    SearchResult,
};

/// Fallback back-off when a 429 carries no Retry-After header.
const DEFAULT_RETRY_AFTER_SECS: u64 = 60;
/// Upper bound on honored Retry-After — a hostile header must not park an
/// indexer indefinitely.
const MAX_RETRY_AFTER_SECS: u64 = 3600;

/// A custom DNS resolver re-validates every resolved address at connect
/// time, closing the DNS-rebinding TOCTOU that validate_fetch_url's
/// pre-check alone cannot — the client re-resolves after validation, so a
/// host that answers public then private would otherwise bypass the guard.
fn build_http_client(request_timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(request_timeout_secs))
        .dns_resolver(Arc::new(SsrfGuardResolver))
        .build()
        .unwrap_or_default() // WHY: reqwest::Client::default() is a valid fallback (build fails only with invalid TLS config); the validate_fetch_url pre-check still guards that path
}

pub struct SearchIndexerService {
    read_pool: SqlitePool,
    write_pool: SqlitePool,
    // WHY: swappable behind a std RwLock (never held across an .await —
    // every read site takes the lock, clones the Arc, and drops the guard
    // before doing anything async) so a `(cloudflare_bypass_enabled,
    // cf_proxy_url, cf_proxy_timeout_seconds)` change can swap the live proxy
    // without a service rebuild.
    cf_proxy: std::sync::RwLock<Arc<dyn CloudflareProxy>>,
    rate_limiter: RateLimiter,
    // WHY: (request_timeout_secs, client) cache — rebuilt only when the live
    // section's `request_timeout_secs` has actually changed, so most
    // operations reuse the client's connection pool instead of paying a
    // fresh-connect cost every call.
    http: std::sync::RwLock<(u64, reqwest::Client)>,
    config: horismos::Section<SearchSubsystemConfig>,
    // WHY: swappable behind a std RwLock, same discipline as `cf_proxy` — a
    // `cardigann_definitions_dir` change re-runs `CardigannRegistry::load`
    // and swaps the Arc; a load failure keeps the previous registry.
    cardigann: std::sync::RwLock<Arc<CardigannRegistry>>,
    // WHY: interactive-login sessions live on the service, not on the
    // per-search ephemeral CardigannClient — a session must outlive the
    // client that created it. In-memory only: a restart re-logs-in,
    // matching the CF-bypass cookie posture.
    cardigann_sessions: Arc<SessionStore>,
    // WHY: hand-rolled std Mutex, never held across an .await — see
    // `results_cache` module docs. TTL/cap are read live from `config` at
    // every insert/lookup call site, not cached on the service.
    results_cache: ResultsCache,
    event_tx: EventSender,
}

impl SearchIndexerService {
    pub fn new(
        read_pool: SqlitePool,
        write_pool: SqlitePool,
        cf_proxy: Arc<dyn CloudflareProxy>,
        config: horismos::Section<SearchSubsystemConfig>,
        event_tx: EventSender,
    ) -> Self {
        let cfg = config.get();
        let rate_limiter = RateLimiter::new(
            cfg.per_indexer_rate_limit_requests,
            Duration::from_secs(cfg.per_indexer_rate_limit_window_seconds),
        );

        let http = build_http_client(cfg.request_timeout_secs);

        // WHY: definitions are read once here at construction; a live
        // `cardigann_definitions_dir` change re-loads and swaps the registry
        // via `set_cardigann_registry` (archon's zetesis supervisor), never
        // re-reading this construction path.
        let cardigann = CardigannRegistry::load(Arc::new(cfg.clone()));

        Self {
            read_pool,
            write_pool,
            cf_proxy: std::sync::RwLock::new(cf_proxy),
            rate_limiter,
            http: std::sync::RwLock::new((cfg.request_timeout_secs, http)),
            config,
            cardigann: std::sync::RwLock::new(Arc::new(cardigann)),
            cardigann_sessions: Arc::new(SessionStore::new()),
            results_cache: ResultsCache::new(),
            event_tx,
        }
    }

    /// Swaps the live Cloudflare-bypass proxy. Called by archon's zetesis
    /// supervisor on a `(cloudflare_bypass_enabled, cf_proxy_url,
    /// cf_proxy_timeout_seconds)` change.
    pub fn set_cf_proxy(&self, new: Arc<dyn CloudflareProxy>) {
        let mut guard = self.cf_proxy.write().unwrap_or_else(|e| e.into_inner());
        *guard = new;
    }

    /// Swaps the live Cardigann definitions registry. Called by archon's
    /// zetesis supervisor on a `cardigann_definitions_dir` change; a load
    /// failure is the caller's responsibility to detect — this always swaps
    /// to whatever it is given.
    pub fn set_cardigann_registry(&self, new: Arc<CardigannRegistry>) {
        let mut guard = self.cardigann.write().unwrap_or_else(|e| e.into_inner());
        *guard = new;
    }

    /// Live-reconfigures the per-indexer rate limiter. Called by archon's
    /// zetesis supervisor on a `(per_indexer_rate_limit_requests,
    /// per_indexer_rate_limit_window_seconds)` change; preserves every
    /// bucket's active embargo (see `RateLimiter::reconfigure`).
    pub async fn reconfigure_rate_limiter(&self, requests_per: u32, window: Duration) {
        self.rate_limiter.reconfigure(requests_per, window).await;
    }

    fn cf_proxy_snapshot(&self) -> Arc<dyn CloudflareProxy> {
        let guard = self.cf_proxy.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard)
    }

    fn cardigann_snapshot(&self) -> Arc<CardigannRegistry> {
        let guard = self.cardigann.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard)
    }

    /// Returns a client built for `request_timeout_secs`, rebuilding (and
    /// caching) only when the live value differs from the last build — most
    /// operations reuse the cached client's connection pool while
    /// `zetesis.request_timeout_secs` still goes live on the next change.
    fn http_client(&self, request_timeout_secs: u64) -> reqwest::Client {
        {
            let guard = self.http.read().unwrap_or_else(|e| e.into_inner());
            if guard.0 == request_timeout_secs {
                return guard.1.clone();
            }
        }
        let client = build_http_client(request_timeout_secs);
        let mut guard = self.http.write().unwrap_or_else(|e| e.into_inner());
        *guard = (request_timeout_secs, client.clone());
        client
    }

    #[instrument(skip(self, ct))]
    pub async fn search(
        &self,
        query: SearchQuery,
        ct: CancellationToken,
    ) -> Result<SearchOutcome, SearchIndexerError> {
        let query_id = QueryId::new();
        // WHY: one snapshot for the whole operation — a mid-search reload
        // cannot mix an old bound from before the change with a new one FROM
        // after it (torn-config guard).
        let cfg = self.config.get();

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
        let cf_proxy = self.cf_proxy_snapshot();
        let cardigann = self.cardigann_snapshot();
        let cardigann_sessions = Arc::clone(&self.cardigann_sessions);
        let http = self.http_client(cfg.request_timeout_secs);
        let timeout = Duration::from_secs(cfg.search_timeout_seconds);
        let max_body_bytes = cfg.max_response_body_bytes;
        let max_results_per_indexer = cfg.max_results_per_indexer;
        let rate_limiter = &self.rate_limiter;

        let results: Vec<SearchResult> = stream::iter(eligible)
            .map(|indexer| {
                let cf = Arc::clone(&cf_proxy);
                let cardigann = Arc::clone(&cardigann);
                let sessions = Arc::clone(&cardigann_sessions);
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
                        &cardigann,
                        sessions,
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
                        Ok(mut results) => {
                            // WHY: cap each indexer's contribution BEFORE the
                            // merged collect — one oversized response must not
                            // drown out every other indexer in the fan-out.
                            results.truncate(max_results_per_indexer);
                            results
                        }
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
            .buffer_unordered(cfg.max_concurrent_searches)
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

        // WHY: catalogs the whole deduped set under query_id, minting one
        // ReleaseId per result — the server-side join enqueue-by-reference
        // resolves at enqueue time (#608). TTL/cap are read from the same
        // `cfg` snapshot this call already took, so a mid-search reload
        // cannot mix an old bound with a new one.
        let outcome = self.results_cache.insert(
            query_id,
            deduped,
            Duration::from_secs(cfg.result_cache_ttl_seconds),
            cfg.result_cache_max_queries,
        );

        Ok(outcome)
    }

    /// Retrieves a prior search's cached results by `query_id`. Idempotent —
    /// a repeat call before the TTL elapses returns the same outcome.
    /// `None` on an unknown or expired query id.
    pub fn cached_results(&self, query_id: QueryId) -> Option<SearchOutcome> {
        let ttl = Duration::from_secs(self.config.get().result_cache_ttl_seconds);
        self.results_cache.cached_results(query_id, ttl)
    }

    /// Resolves a cached release's REAL, unredacted download URL — the
    /// server-side join `paroche::routes::download::enqueue_download` uses
    /// so a credentialed indexer URL never crosses the HTTP boundary to a
    /// client. Idempotent and non-consuming: a retry after a failed enqueue
    /// still resolves. `None` when the release id is unknown or its parent
    /// query has expired.
    pub fn resolve_release(&self, release_id: uuid::Uuid) -> Option<ResolvedRelease> {
        let ttl = Duration::from_secs(self.config.get().result_cache_ttl_seconds);
        self.results_cache.resolve_release(release_id, ttl)
    }

    pub async fn test_indexer(
        &self,
        indexer_id: i64,
        ct: CancellationToken,
    ) -> Result<IndexerStatus, SearchIndexerError> {
        let cfg = self.config.get();
        let indexer = self.load_indexer(indexer_id).await?;
        let client = make_client(
            &indexer,
            self.http_client(cfg.request_timeout_secs),
            self.cf_proxy_snapshot(),
            Duration::from_secs(cfg.request_timeout_secs),
            cfg.max_response_body_bytes,
            &self.cardigann_snapshot(),
            Arc::clone(&self.cardigann_sessions),
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
        let cfg = self.config.get();
        let indexer = self.load_indexer(indexer_id).await?;
        let client = make_client(
            &indexer,
            self.http_client(cfg.request_timeout_secs),
            self.cf_proxy_snapshot(),
            Duration::from_secs(cfg.request_timeout_secs),
            cfg.max_response_body_bytes,
            &self.cardigann_snapshot(),
            Arc::clone(&self.cardigann_sessions),
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

    /// Refreshes caps for every eligible indexer whose `last_tested` is
    /// missing, unparseable, or older than `max_age`. Called by archon's
    /// zetesis supervisor on its scheduled caps-refresh tick
    /// (`zetesis.caps_refresh_hours`); the manual `POST /{id}/caps` route
    /// remains the on-demand path. Returns the ids actually refreshed; a
    /// per-indexer failure is logged and skipped — one unreachable indexer
    /// must not starve the rest of the sweep, and it stays stale for the
    /// next tick to retry.
    pub async fn refresh_stale_caps(
        &self,
        max_age: Duration,
        ct: CancellationToken,
    ) -> Result<Vec<i64>, SearchIndexerError> {
        let indexers = repo::get_eligible_indexers(&self.read_pool)
            .await
            .map_err(|e| SearchIndexerError::Database {
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
        let now = jiff::Timestamp::now();
        let mut refreshed = Vec::new();
        for indexer in indexers {
            if ct.is_cancelled() {
                break;
            }
            if !caps_stale(indexer.last_tested.as_deref(), max_age, now) {
                continue;
            }
            match self.refresh_caps(indexer.id, ct.clone()).await {
                Ok(_) => refreshed.push(indexer.id),
                Err(e) => warn!(
                    indexer_id = indexer.id,
                    indexer_name = %indexer.name,
                    error = %e,
                    "scheduled caps refresh failed for indexer"
                ),
            }
        }
        Ok(refreshed)
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
            // WHY: a rejected login (bad credentials, off-host submit, failed
            // login-test) needs operator intervention, same as a bad API key.
            SearchIndexerError::AuthFailed { .. } | SearchIndexerError::LoginFailed { .. } => {
                Some("failed")
            }
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
            // (or a corrupt/invalid settings override) fails every search
            // identically until the operator intervenes.
            SearchIndexerError::DefinitionNotFound { .. }
            | SearchIndexerError::DefinitionLoad { .. }
            | SearchIndexerError::DefinitionInvalid { .. }
            | SearchIndexerError::DefinitionUnsupported { .. }
            | SearchIndexerError::LoginUnsupported { .. }
            | SearchIndexerError::CookieAuthRequired { .. }
            | SearchIndexerError::SettingsJsonInvalid { .. }
            | SearchIndexerError::SettingsInvalid { .. } => Some("failed"),
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

/// True when a `last_tested` timestamp is absent, unparseable, or older than
/// `max_age` — the scheduled caps-refresh eligibility check.
fn caps_stale(last_tested: Option<&str>, max_age: Duration, now: jiff::Timestamp) -> bool {
    let Some(raw) = last_tested else {
        return true;
    };
    match raw.parse::<jiff::Timestamp>() {
        // WHY: a future timestamp (clock skew) yields a negative age, which
        // u64::try_from rejects — treated as fresh rather than wrapping.
        Ok(tested) => u64::try_from(now.as_second() - tested.as_second())
            .is_ok_and(|age_secs| age_secs > max_age.as_secs()),
        // WHY: an unparseable timestamp counts as stale — a corrupt value
        // must not permanently exempt an indexer from refresh.
        Err(_) => true,
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

// WHY: a protocol-dispatch factory that threads every per-request dependency
// (transport, proxy, timeouts, definition registry, session store) into the
// right client — a params struct would just relocate the same wiring.
#[expect(
    clippy::too_many_arguments,
    reason = "dependency-injection factory; each arg is a distinct live dependency"
)]
fn make_client(
    indexer: &IndexerRow,
    http: reqwest::Client,
    cf_proxy: Arc<dyn CloudflareProxy>,
    timeout: Duration,
    max_body_bytes: u64,
    cardigann: &CardigannRegistry,
    cardigann_sessions: Arc<SessionStore>,
) -> Result<Box<dyn DynIndexerClient>, SearchIndexerError> {
    // WHY: a corrupt settings_json blob must fail loud — silently falling
    // back to defaults would mask the row that just lost its overrides.
    let settings: BTreeMap<String, String> = match indexer.settings_json.as_deref() {
        Some(json) => {
            serde_json::from_str(json).map_err(|e| SearchIndexerError::SettingsJsonInvalid {
                indexer_id: indexer.id,
                reason: e.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?
        }
        None => BTreeMap::new(),
    };

    let config = IndexerConfig {
        id: indexer.id,
        name: indexer.name.clone(),
        url: indexer.url.clone(),
        api_key: indexer.api_key.clone(),
        cf_bypass: indexer.cf_bypass,
        settings,
    };

    Ok(match indexer.protocol.as_str() {
        "newznab" => Box::new(NewznabClient::new(
            config,
            http,
            cf_proxy,
            timeout,
            max_body_bytes,
        )),
        "cardigann" => {
            Box::new(cardigann.client_for(config, http, cf_proxy, timeout, cardigann_sessions)?)
        }
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
