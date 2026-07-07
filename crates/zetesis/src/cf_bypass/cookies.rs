use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use tokio::time::Instant;

use crate::cf_bypass::Cookie;

/// Per-host cache of solved Cloudflare clearance cookies. Keyed by the
/// target host so every indexer behind the same origin shares one solve.
pub struct CookieStore {
    jars: DashMap<String, HostCookieJar>,
}

// WHY: pure data — cookie jar for one Cloudflare-protected host.
pub struct HostCookieJar {
    pub cookies: Vec<StoredCookie>,
    pub user_agent: String,
    pub last_refreshed: Instant,
}

// WHY: pure data — persisted cookie entry.
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires_at: Option<SystemTime>,
}

impl CookieStore {
    pub fn new() -> Self {
        Self {
            jars: DashMap::new(),
        }
    }

    pub fn store(&self, host: &str, cookies: Vec<Cookie>, user_agent: String) {
        let stored: Vec<StoredCookie> = cookies
            .into_iter()
            .map(|c| {
                let expires_at = if c.expires < 0.0 {
                    None
                } else {
                    Some(UNIX_EPOCH + Duration::from_secs_f64(c.expires))
                };
                StoredCookie {
                    name: c.name,
                    value: c.value,
                    domain: c.domain,
                    path: c.path,
                    expires_at,
                }
            })
            .collect();

        self.jars.insert(
            host.to_string(),
            HostCookieJar {
                cookies: stored,
                user_agent,
                last_refreshed: Instant::now(),
            },
        );
    }

    pub fn get_cookie_header(&self, host: &str) -> Option<String> {
        let jar = self.jars.get(host)?;
        let now = SystemTime::now();

        let valid_cookies: Vec<String> = jar
            .cookies
            .iter()
            .filter(|c| c.expires_at.map(|exp| exp > now).unwrap_or(true))
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();

        if valid_cookies.is_empty() {
            return None;
        }
        Some(valid_cookies.join("; "))
    }

    pub fn get_user_agent(&self, host: &str) -> Option<String> {
        self.jars.get(host).map(|j| j.user_agent.clone())
    }

    pub fn needs_refresh(&self, host: &str, refresh_before_expiry: Duration) -> bool {
        let Some(jar) = self.jars.get(host) else {
            return true;
        };

        let now = SystemTime::now();
        jar.cookies.iter().any(|c| {
            c.expires_at
                .map(|exp| {
                    let threshold = exp.checked_sub(refresh_before_expiry).unwrap_or(UNIX_EPOCH);
                    now >= threshold
                })
                .unwrap_or(false)
        })
    }

    pub fn remove(&self, host: &str) {
        self.jars.remove(host);
    }

    pub fn has_cookies(&self, host: &str) -> bool {
        self.get_cookie_header(host).is_some()
    }
}

impl Default for CookieStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cookie(name: &str, value: &str, expires: f64) -> Cookie {
        Cookie {
            name: name.to_string(),
            value: value.to_string(),
            domain: ".example.com".to_string(),
            path: "/".to_string(),
            expires,
            http_only: false,
            secure: true,
        }
    }

    #[test]
    fn store_and_retrieve_cookies() {
        let store = CookieStore::new();
        let future_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 3600.0;

        store.store(
            "indexer.example.com",
            vec![make_cookie("cf_clearance", "abc123", future_ts)],
            "Mozilla/5.0".to_string(),
        );

        let header = store.get_cookie_header("indexer.example.com");
        assert_eq!(header, Some("cf_clearance=abc123".to_string()));
        assert_eq!(
            store.get_user_agent("indexer.example.com"),
            Some("Mozilla/5.0".to_string())
        );
    }

    #[test]
    fn expired_cookies_filtered() {
        let store = CookieStore::new();
        let past_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            - 3600.0;

        store.store(
            "indexer.example.com",
            vec![make_cookie("cf_clearance", "expired", past_ts)],
            "Mozilla/5.0".to_string(),
        );

        assert!(store.get_cookie_header("indexer.example.com").is_none());
    }

    #[test]
    fn session_cookies_always_valid() {
        let store = CookieStore::new();
        store.store(
            "indexer.example.com",
            vec![make_cookie("session", "val", -1.0)],
            "Agent".to_string(),
        );

        assert!(store.get_cookie_header("indexer.example.com").is_some());
    }

    #[test]
    fn needs_refresh_when_absent() {
        let store = CookieStore::new();
        assert!(store.needs_refresh("indexer.example.com", Duration::from_secs(1800)));
    }

    #[test]
    fn needs_refresh_before_expiry() {
        let store = CookieStore::new();
        let near_expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 900.0; // 15 minutes from now

        store.store(
            "indexer.example.com",
            vec![make_cookie("cf_clearance", "val", near_expiry)],
            "Agent".to_string(),
        );

        assert!(store.needs_refresh("indexer.example.com", Duration::from_secs(1800)));
    }

    #[test]
    fn fresh_cookie_does_not_need_refresh() {
        let store = CookieStore::new();
        let far_expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 3600.0;

        store.store(
            "indexer.example.com",
            vec![make_cookie("cf_clearance", "val", far_expiry)],
            "Agent".to_string(),
        );

        assert!(!store.needs_refresh("indexer.example.com", Duration::from_secs(1800)));
    }

    #[test]
    fn hosts_are_isolated() {
        let store = CookieStore::new();
        let future_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 3600.0;

        store.store(
            "a.example.com",
            vec![make_cookie("cf_clearance", "for-a", future_ts)],
            "Agent".to_string(),
        );

        assert!(store.has_cookies("a.example.com"));
        assert!(!store.has_cookies("b.example.com"));
        assert!(store.needs_refresh("b.example.com", Duration::from_secs(1800)));
    }

    #[test]
    fn remove_cookies() {
        let store = CookieStore::new();
        let future_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 3600.0;

        store.store(
            "indexer.example.com",
            vec![make_cookie("cf_clearance", "val", future_ts)],
            "Agent".to_string(),
        );
        assert!(store.has_cookies("indexer.example.com"));

        store.remove("indexer.example.com");
        assert!(!store.has_cookies("indexer.example.com"));
    }

    #[test]
    fn multiple_cookies_joined() {
        let store = CookieStore::new();
        let future_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 3600.0;

        store.store(
            "indexer.example.com",
            vec![
                make_cookie("cf_clearance", "abc", future_ts),
                make_cookie("session", "xyz", future_ts),
            ],
            "Agent".to_string(),
        );

        let header = store.get_cookie_header("indexer.example.com").unwrap();
        assert!(header.contains("cf_clearance=abc"));
        assert!(header.contains("session=xyz"));
        assert!(header.contains("; "));
    }
}
