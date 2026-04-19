use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::instrument;

struct CacheEntry<V> {
    value: V,
    expires_at: Option<Instant>,
}

impl<V> CacheEntry<V> {
    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|t| Instant::now() > t)
    }
}

pub struct MetadataCache<K, V> {
    store: Arc<DashMap<K, CacheEntry<V>>>,
    default_ttl: Duration,
}

impl<K, V> MetadataCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub(crate) fn new(default_ttl: Duration) -> Self {
        Self {
            store: Arc::new(DashMap::new()),
            default_ttl,
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<V> {
        let entry = self.store.get(key)?;
        if entry.is_expired() {
            drop(entry);
            self.store.remove(key);
            return None;
        }
        Some(entry.value.clone())
    }

    pub(crate) fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, Some(self.default_ttl));
    }

    pub fn insert_permanent(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, None);
    }

    pub(crate) fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        let expires_at = ttl.map(|d| Instant::now() + d);
        self.store.insert(key, CacheEntry { value, expires_at });
    }

    /// Remove all expired entries. Called by background cleanup task.
    #[instrument(skip(self), name = "cache_evict_expired")]
    pub fn evict_expired(&self) {
        self.store.retain(|_, v| !v.is_expired());
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn insert_and_get() {
        let cache = MetadataCache::new(Duration::from_secs(60));
        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    }

    #[test]
    fn missing_key_returns_none() {
        let cache: MetadataCache<String, String> = MetadataCache::new(Duration::from_secs(60));
        assert_eq!(cache.get(&"absent".to_string()), None);
    }

    #[test]
    fn ttl_expiry() {
        let cache = MetadataCache::new(Duration::from_millis(1));
        cache.insert("key".to_string(), "value".to_string());
        // Spin-wait for expiry — avoids sleep in tests but guarantees 1ms has elapsed.
        let deadline = Instant::now() + Duration::from_millis(10);
        while Instant::now() < deadline {
            std::hint::spin_loop();
        }
        assert_eq!(cache.get(&"key".to_string()), None);
    }

    #[test]
    fn permanent_entry_does_not_expire() {
        let cache = MetadataCache::new(Duration::from_millis(1));
        cache.insert_permanent("key".to_string(), "perm".to_string());
        let deadline = Instant::now() + Duration::from_millis(10);
        while Instant::now() < deadline {
            std::hint::spin_loop();
        }
        assert_eq!(cache.get(&"key".to_string()), Some("perm".to_string()));
    }

    #[test]
    fn evict_expired_removes_stale_entries() {
        let cache = MetadataCache::new(Duration::from_millis(1));
        cache.insert("stale".to_string(), "old".to_string());
        cache.insert_permanent("fresh".to_string(), "new".to_string());

        let deadline = Instant::now() + Duration::from_millis(10);
        while Instant::now() < deadline {
            std::hint::spin_loop();
        }

        cache.evict_expired();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"fresh".to_string()), Some("new".to_string()));
    }

    #[test]
    fn len_tracks_entries() {
        let cache = MetadataCache::new(Duration::from_secs(60));
        assert_eq!(cache.len(), 0);
        cache.insert("a".to_string(), 1u32);
        cache.insert("b".to_string(), 2u32);
        assert_eq!(cache.len(), 2);
    }
}
