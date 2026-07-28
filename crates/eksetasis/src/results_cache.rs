//! In-memory TTL cache mapping a completed search's `QueryId` to its
//! deduped results, and each result's minted `ReleaseId` back to the
//! ORIGINAL, credentialed `SearchResult` — the server-side join that lets
//! `POST /api/v1/downloads` enqueue-by-reference without ever handing the
//! indexer credential to a client (#608).
//!
//! WHY: hand-rolled `std::sync::Mutex`, never held across an `.await` —
//! matches this crate's lock discipline (`SearchIndexerService`'s
//! `cf_proxy`/`cardigann` swap fields in `search.rs`). Every method here is a
//! synchronous critical section; nothing awaits while the lock is held.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use themelion::{QueryId, ReleaseId};
use uuid::Uuid;

use crate::types::{CataloguedResult, ResolvedRelease, SearchOutcome, SearchResult};

struct CachedQuery {
    created: Instant,
    results: Vec<CataloguedResult>,
}

#[derive(Default)]
struct CacheInner {
    /// Insertion order, oldest first — also age order, since `QueryId` is
    /// minted immediately before insert and entries are appended in call
    /// order.
    order: VecDeque<QueryId>,
    by_query: HashMap<QueryId, CachedQuery>,
    by_release: HashMap<Uuid, (QueryId, usize)>,
}

impl CacheInner {
    fn is_stale(created: Instant, ttl: Duration, now: Instant) -> bool {
        now.duration_since(created) >= ttl
    }

    /// Drops every cached query whose age has passed `ttl`, oldest-first.
    /// `order` is age-ordered, so this stops at the first still-fresh entry
    /// — a single `ttl` value used within one call keeps staleness monotonic
    /// front-to-back.
    fn purge_expired(&mut self, ttl: Duration) {
        let now = Instant::now();
        while let Some(&query_id) = self.order.front() {
            let Some(cached) = self.by_query.get(&query_id) else {
                // INVARIANT: order and by_query stay in lockstep; a missing
                // entry here means a prior evict already dropped it.
                self.order.pop_front();
                continue;
            };
            if !Self::is_stale(cached.created, ttl, now) {
                break;
            }
            self.evict_front();
        }
    }

    /// Evicts the single oldest cached query: pops `order`, `by_query`, and
    /// every `by_release` entry it owns.
    fn evict_front(&mut self) {
        let Some(query_id) = self.order.pop_front() else {
            return;
        };
        if let Some(cached) = self.by_query.remove(&query_id) {
            for item in &cached.results {
                self.by_release.remove(item.release_id.as_uuid());
            }
        }
    }

    fn evict_to_cap(&mut self, max_queries: usize) {
        while self.order.len() > max_queries {
            self.evict_front();
        }
    }
}

/// Hand-rolled synchronous results cache — see module docs.
#[derive(Default)]
pub struct ResultsCache(std::sync::Mutex<CacheInner>);

impl ResultsCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a completed search's deduped results under `query_id`, minting
    /// one fresh `ReleaseId` per result. Purges expired queries and evicts
    /// down to `max_queries` (oldest-first) AFTER the insert — with
    /// `ttl == Duration::ZERO` the entry just inserted is immediately
    /// purged, so the cache retains nothing (an immediate miss on every
    /// subsequent lookup), which is the correct behavior for a
    /// zero-configured TTL.
    pub fn insert(
        &self,
        query_id: QueryId,
        results: Vec<SearchResult>,
        ttl: Duration,
        max_queries: usize,
    ) -> SearchOutcome {
        let catalogued: Vec<CataloguedResult> = results
            .into_iter()
            .map(|result| CataloguedResult {
                release_id: ReleaseId::new(),
                result,
            })
            .collect();

        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());

        for (idx, item) in catalogued.iter().enumerate() {
            inner
                .by_release
                .insert(*item.release_id.as_uuid(), (query_id, idx));
        }
        inner.by_query.insert(
            query_id,
            CachedQuery {
                created: Instant::now(),
                results: catalogued.clone(),
            },
        );
        inner.order.push_back(query_id);

        inner.purge_expired(ttl);
        inner.evict_to_cap(max_queries);

        SearchOutcome {
            query_id,
            results: catalogued,
        }
    }

    /// Looks up a prior search's cached results. Lazy TTL expiry: a stale
    /// entry reads as absent WITHOUT being physically purged here (the next
    /// `insert` sweeps it) — idempotent, side-effect-free.
    pub fn cached_results(&self, query_id: QueryId, ttl: Duration) -> Option<SearchOutcome> {
        let inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let cached = inner.by_query.get(&query_id)?;
        if CacheInner::is_stale(cached.created, ttl, Instant::now()) {
            return None;
        }
        Some(SearchOutcome {
            query_id,
            results: cached.results.clone(),
        })
    }

    /// Resolves a single release's UNREDACTED download URL. Idempotent and
    /// non-consuming — a retry after a failed enqueue must still resolve
    /// (see `paroche::routes::download::enqueue_download`).
    pub fn resolve_release(&self, release_id: Uuid, ttl: Duration) -> Option<ResolvedRelease> {
        let inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let &(query_id, idx) = inner.by_release.get(&release_id)?;
        let cached = inner.by_query.get(&query_id)?;
        if CacheInner::is_stale(cached.created, ttl, Instant::now()) {
            return None;
        }
        let item = cached.results.get(idx)?;
        Some(ResolvedRelease {
            download_url: item.result.download_url.clone(),
            protocol: item.result.protocol,
            info_hash: item.result.info_hash.clone(),
            indexer_id: item.result.indexer_id,
            title: item.result.title.clone(),
            size_bytes: item.result.size_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap as StdHashMap;

    use super::*;
    use crate::types::ReleaseProtocol;

    fn result(title: &str, indexer_id: i64) -> SearchResult {
        SearchResult {
            title: title.to_string(),
            guid: None,
            download_url: format!("https://indexer.example/dl/{title}?apikey=SECRET"),
            size_bytes: Some(1_000_000),
            seeders: Some(5),
            leechers: Some(1),
            info_hash: Some(format!("hash-{title}")),
            category_id: Some(2000),
            publication_date: None,
            indexer_id,
            protocol: ReleaseProtocol::Torrent,
            download_volume_factor: 1.0,
            upload_volume_factor: 1.0,
            custom_attrs: StdHashMap::new(),
        }
    }

    #[test]
    fn insert_then_resolve_returns_the_exact_credentialed_url() {
        let cache = ResultsCache::new();
        let query_id = QueryId::new();
        let outcome = cache.insert(
            query_id,
            vec![result("Release.One", 1)],
            Duration::from_secs(60),
            32,
        );
        let release_id = *outcome.results[0].release_id.as_uuid();

        let resolved = cache
            .resolve_release(release_id, Duration::from_secs(60))
            .expect("just-inserted release must resolve");
        assert_eq!(
            resolved.download_url,
            "https://indexer.example/dl/Release.One?apikey=SECRET"
        );
        assert_eq!(resolved.indexer_id, 1);
    }

    #[test]
    fn cached_results_returns_the_same_outcome() {
        let cache = ResultsCache::new();
        let query_id = QueryId::new();
        cache.insert(
            query_id,
            vec![result("Release.One", 1)],
            Duration::from_secs(60),
            32,
        );

        let fetched = cache
            .cached_results(query_id, Duration::from_secs(60))
            .expect("cached query must be retrievable");
        assert_eq!(fetched.results.len(), 1);
        assert_eq!(fetched.results[0].result.title, "Release.One");
    }

    #[test]
    fn ttl_zero_is_an_immediate_miss() {
        let cache = ResultsCache::new();
        let query_id = QueryId::new();
        let outcome = cache.insert(query_id, vec![result("Release.One", 1)], Duration::ZERO, 32);
        let release_id = *outcome.results[0].release_id.as_uuid();

        assert!(cache.cached_results(query_id, Duration::ZERO).is_none());
        assert!(cache.resolve_release(release_id, Duration::ZERO).is_none());
    }

    #[test]
    fn unknown_query_and_release_miss_cleanly() {
        let cache = ResultsCache::new();
        assert!(
            cache
                .cached_results(QueryId::new(), Duration::from_secs(60))
                .is_none()
        );
        assert!(
            cache
                .resolve_release(Uuid::now_v7(), Duration::from_secs(60))
                .is_none()
        );
    }

    #[test]
    fn cap_eviction_drops_the_oldest_query_and_its_releases() {
        let cache = ResultsCache::new();
        let q1 = QueryId::new();
        let o1 = cache.insert(q1, vec![result("Q1", 1)], Duration::from_secs(60), 2);
        let r1 = *o1.results[0].release_id.as_uuid();

        let q2 = QueryId::new();
        cache.insert(q2, vec![result("Q2", 1)], Duration::from_secs(60), 2);

        // Third search — over the cap of 2 — must evict q1 (oldest), taking
        // its by_release entry with it.
        let q3 = QueryId::new();
        cache.insert(q3, vec![result("Q3", 1)], Duration::from_secs(60), 2);

        assert!(
            cache.cached_results(q1, Duration::from_secs(60)).is_none(),
            "the oldest query must be evicted once the cap is exceeded"
        );
        assert!(
            cache.resolve_release(r1, Duration::from_secs(60)).is_none(),
            "evicting a query must drop its by_release entries too"
        );
        assert!(cache.cached_results(q2, Duration::from_secs(60)).is_some());
        assert!(cache.cached_results(q3, Duration::from_secs(60)).is_some());
    }
}
