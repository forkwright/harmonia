pub mod cardigann;
pub mod newznab;
pub mod torznab;
pub mod xml;

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use futures::StreamExt;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use url::{Host, Url};

use crate::error::{self, SearchIndexerError};
use crate::types::{DownloadResponse, IndexerCaps, IndexerStatus, SearchQuery, SearchResult};

pub trait IndexerClient: Send + Sync {
    fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<Vec<SearchResult>, SearchIndexerError>> + Send;

    fn caps(
        &self,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<IndexerCaps, SearchIndexerError>> + Send;

    fn test(
        &self,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<IndexerStatus, SearchIndexerError>> + Send;

    fn download(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<DownloadResponse, SearchIndexerError>> + Send;
}

use std::future::Future;
use std::pin::Pin;

pub trait DynIndexerClient: Send + Sync {
    fn search_boxed<'a>(
        &'a self,
        query: &'a SearchQuery,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SearchResult>, SearchIndexerError>> + Send + 'a>>;

    fn caps_boxed(
        &self,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<IndexerCaps, SearchIndexerError>> + Send + '_>>;

    fn test_boxed(
        &self,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<IndexerStatus, SearchIndexerError>> + Send + '_>>;
}

impl<T: IndexerClient> DynIndexerClient for T {
    fn search_boxed<'a>(
        &'a self,
        query: &'a SearchQuery,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SearchResult>, SearchIndexerError>> + Send + 'a>>
    {
        Box::pin(self.search(query, ct))
    }

    fn caps_boxed(
        &self,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<IndexerCaps, SearchIndexerError>> + Send + '_>> {
        Box::pin(self.caps(ct))
    }

    fn test_boxed(
        &self,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<IndexerStatus, SearchIndexerError>> + Send + '_>> {
        Box::pin(self.test(ct))
    }
}

pub struct IndexerConfig {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub cf_bypass: bool,
    /// Per-indexer Cardigann `settings:` overrides (setting name -> value);
    /// empty for non-Cardigann protocols.
    pub settings: BTreeMap<String, String>,
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

/// Redacts the value of every secret-bearing query parameter in `url` (api key,
/// tracker passkey, rss key, torrent pass, auth token, session/cookie), leaving
/// the path and non-secret params intact for diagnostics.
///
/// WHY: indexer URLs — both the native Torznab/Newznab `apikey` and arbitrary
/// Cardigann-defined credentials like `passkey`/`rss_key`/`torrent_pass` — carry
/// credentials as query parameters; redacting at error-construction time keeps
/// them out of every downstream Display/log path. A single-parameter redactor
/// would leak any credential not literally named `apikey`.
pub(crate) fn redact_secrets(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return url.to_string();
    };
    let redacted = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((key, _)) if is_secret_param(key) => format!("{key}=[REDACTED]"),
            _ => pair.to_string(),
        })
        .collect::<Vec<_>>()
        .join("&");
    format!("{base}?{redacted}")
}

/// True when a query-parameter key names a credential. Substring-matched (case
/// insensitive) so variants like `torrent_passkey` or `rss_key` are covered.
fn is_secret_param(key: &str) -> bool {
    const SECRET_MARKERS: &[&str] = &[
        "apikey",
        "api_key",
        "passkey",
        "pass_key",
        "password",
        "rss_key",
        "rsskey",
        "torrent_pass",
        "authkey",
        "auth_key",
        "secret",
        "token",
        "session",
        "cookie",
    ];
    let key = key.to_ascii_lowercase();
    SECRET_MARKERS.iter().any(|marker| key.contains(marker))
}

/// Reads a response body into a UTF-8 string, enforcing `max_bytes`.
pub(crate) async fn read_body_bounded(
    response: reqwest::Response,
    url: &str,
    max_bytes: u64,
) -> Result<String, SearchIndexerError> {
    let body = read_body_bytes_bounded(response, url, max_bytes).await?;
    String::from_utf8(body).map_err(|e| SearchIndexerError::ParseResponse {
        url: redact_secrets(url),
        error: e.to_string(),
        location: std::panic::Location::caller(),
    })
}

/// Reads a response body into raw bytes, enforcing `max_bytes`.
///
/// Rejects on a declared `Content-Length` above the cap before reading any
/// body bytes, then enforces the cap again while streaming.
///
/// WHY: `Content-Length` is attacker-controlled and may be absent or wrong;
/// only a running counter over the actual stream bounds allocation.
pub(crate) async fn read_body_bytes_bounded(
    response: reqwest::Response,
    url: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, SearchIndexerError> {
    if let Some(declared) = response.content_length()
        && declared > max_bytes
    {
        return Err(SearchIndexerError::ResponseTooLarge {
            url: redact_secrets(url),
            size: declared,
            limit: max_bytes,
            location: std::panic::Location::caller(),
        });
    }

    let mut body: Vec<u8> = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context(error::HttpRequestSnafu {
            url: redact_secrets(url),
        })?;
        let received = body.len() as u64 + chunk.len() as u64;
        if received > max_bytes {
            return Err(SearchIndexerError::ResponseTooLarge {
                url: redact_secrets(url),
                size: received,
                limit: max_bytes,
                location: std::panic::Location::caller(),
            });
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

/// SSRF guard for attacker-supplied download URLs.
///
/// Accepts only http(s), requires a host, and rejects any target whose IP —
/// literal or DNS-resolved — is loopback, private, link-local, CGNAT,
/// unspecified, broadcast, or a ULA/mapped equivalent.
///
/// WHY: download URLs come from indexer response XML (third-party data), so a
/// public-looking hostname can point at internal infrastructure. Every
/// resolved address is checked; resolution failure rejects (fail-closed).
pub(crate) async fn validate_fetch_url(url: &str) -> Result<(), SearchIndexerError> {
    let reject = |reason: &str| SearchIndexerError::UnsafeUrl {
        url: redact_secrets(url),
        reason: reason.to_string(),
        location: std::panic::Location::caller(),
    };

    let parsed = Url::parse(url).map_err(|_| reject("not a valid URL"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err(reject("scheme must be http or https")),
    }
    let host = parsed
        .host()
        .ok_or_else(|| reject("URL must have a host"))?;

    match host {
        Host::Ipv4(ip) if ip_is_disallowed(IpAddr::V4(ip)) => {
            Err(reject("host is a private or local address"))
        }
        Host::Ipv6(ip) if ip_is_disallowed(IpAddr::V6(ip)) => {
            Err(reject("host is a private or local address"))
        }
        Host::Ipv4(_) | Host::Ipv6(_) => Ok(()),
        Host::Domain(domain) => {
            // WHY: port only satisfies lookup_host's addr format; http/https
            // always have a known default so the fallback is unreachable.
            let port = parsed.port_or_known_default().unwrap_or(443);
            let addrs = tokio::net::lookup_host((domain, port))
                .await
                .map_err(|_| reject("host did not resolve"))?;
            let mut resolved_any = false;
            for addr in addrs {
                resolved_any = true;
                if ip_is_disallowed(addr.ip()) {
                    return Err(reject("host resolves to a private or local address"));
                }
            }
            if !resolved_any {
                return Err(reject("host did not resolve"));
            }
            Ok(())
        }
    }
}

fn ip_is_disallowed(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                // NOTE: rejects RFC 6598 shared address space (CGNAT).
                // pii-allow: 100.64.0.0/10 is the range base and prefix RFC 6598 defines, not a fleet host.
                || (u32::from(v4) & 0xFFC0_0000) == 0x6440_0000
        }
        IpAddr::V6(v6) => {
            // WHY: an IPv4-mapped IPv6 literal (::ffff:127.0.0.1) must be
            // judged by its embedded IPv4 address or it bypasses every v4
            // range check.
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return ip_is_disallowed(IpAddr::V4(mapped));
            }
            // WHY: checked before the ::/96 embed below so :: (unspecified) and
            // ::1 (loopback) are judged as v6, not as the compatible 0.0.0.0 /
            // 0.0.0.1.
            if v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
            {
                return true;
            }
            // WHY: deprecated IPv4-compatible IPv6 (::a.b.c.d, ::/96) embeds a
            // v4 address in the low 32 bits — ::127.0.0.1 must be judged by
            // that v4 or it reaches loopback unchecked. to_ipv4_mapped only
            // matches ::ffff:*.
            let [s0, s1, s2, s3, s4, s5, s6, s7] = v6.segments();
            if [s0, s1, s2, s3, s4, s5] == [0, 0, 0, 0, 0, 0] {
                let embedded = Ipv4Addr::from((u32::from(s6) << 16) | u32::from(s7));
                return ip_is_disallowed(IpAddr::V4(embedded));
            }
            false
        }
    }
}

/// A reqwest DNS resolver that rejects the connection when ANY resolved
/// address is disallowed, then hands reqwest only the vetted addresses.
///
/// WHY: [`validate_fetch_url`] resolves and checks at validation time but then
/// discards the addresses; the shared client re-resolves at connect time, so a
/// host answering public at validation and private at connect (DNS rebinding)
/// would slip past the pre-check. Re-checking every address here — the moment
/// reqwest is about to connect — closes that TOCTOU window. Fail-closed:
/// resolution failure or any disallowed address rejects the whole connection.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SsrfGuardResolver;

impl Resolve for SsrfGuardResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            // WHY: port 0 — the address checks ignore the port and reqwest
            // overrides it with the target's real port after resolution.
            let resolved: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            for addr in &resolved {
                if ip_is_disallowed(addr.ip()) {
                    return Err(format!(
                        "refusing to connect to {host}: resolves to a private or local address"
                    )
                    .into());
                }
            }
            Ok(Box::new(resolved.into_iter()) as Addrs)
        })
    }
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
            settings: BTreeMap::new(),
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
            settings: BTreeMap::new(),
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
            settings: BTreeMap::new(),
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

    // ── redact_secrets ────────────────────────────────────────────────────────

    #[test]
    fn redact_secrets_strips_value() {
        let redacted = redact_secrets("https://x/api?t=caps&apikey=secret123");
        assert!(!redacted.contains("secret123"), "leaked: {redacted}");
        assert_eq!(redacted, "https://x/api?t=caps&apikey=[REDACTED]");
    }

    #[test]
    fn redact_secrets_preserves_trailing_params() {
        let redacted = redact_secrets("https://x/api?apikey=secret123&t=search&q=abc");
        assert!(!redacted.contains("secret123"), "leaked: {redacted}");
        assert_eq!(redacted, "https://x/api?apikey=[REDACTED]&t=search&q=abc");
    }

    #[test]
    fn redact_secrets_handles_multiple_occurrences() {
        let redacted = redact_secrets("https://x/?apikey=one&t=search&apikey=two");
        assert!(!redacted.contains("one"), "leaked: {redacted}");
        assert!(!redacted.contains("two"), "leaked: {redacted}");
        assert_eq!(
            redacted,
            "https://x/?apikey=[REDACTED]&t=search&apikey=[REDACTED]"
        );
    }

    #[test]
    fn redact_secrets_no_key_present_noop() {
        assert_eq!(
            redact_secrets("https://x/api?t=caps"),
            "https://x/api?t=caps"
        );
        assert_eq!(redact_secrets(""), "");
    }

    #[test]
    fn redact_secrets_value_at_end_without_ampersand() {
        let redacted = redact_secrets("https://x/api?apikey=secret123");
        assert!(!redacted.contains("secret123"), "leaked: {redacted}");
        assert_eq!(redacted, "https://x/api?apikey=[REDACTED]");
    }

    #[test]
    fn redact_secrets_covers_non_apikey_credentials() {
        for (url, secret) in [
            ("https://t/dl?passkey=abc123def", "abc123def"),
            ("https://t/rss?rss_key=zzz999", "zzz999"),
            ("https://t/get?torrent_pass=pw77", "pw77"),
            ("https://t/api?authkey=ak55&t=search", "ak55"),
        ] {
            let redacted = redact_secrets(url);
            assert!(!redacted.contains(secret), "leaked in {redacted}");
            assert!(redacted.contains("[REDACTED]"), "not redacted: {redacted}");
        }
        // WHY: non-secret params survive for diagnostics; only the credential
        // value is scrubbed.
        let redacted = redact_secrets("https://t/api?passkey=SECRET&t=search&cat=2000");
        assert_eq!(
            redacted,
            "https://t/api?passkey=[REDACTED]&t=search&cat=2000"
        );
    }

    // ── validate_fetch_url ────────────────────────────────────────────────────

    #[tokio::test]
    async fn validate_fetch_url_rejects_unparseable() {
        assert!(validate_fetch_url("not a url").await.is_err());
        assert!(validate_fetch_url("").await.is_err());
    }

    #[tokio::test]
    async fn validate_fetch_url_rejects_non_http_scheme() {
        for url in [
            "file:///etc/passwd",
            "ftp://example.com/file.torrent",
            "gopher://example.com/",
            "javascript:alert(1)",
        ] {
            let err = validate_fetch_url(url).await.expect_err(url);
            assert!(matches!(err, SearchIndexerError::UnsafeUrl { .. }));
        }
    }

    #[tokio::test]
    async fn validate_fetch_url_rejects_loopback() {
        for url in ["http://127.0.0.1/x", "http://127.8.9.10:8080/x"] {
            let err = validate_fetch_url(url).await.expect_err(url);
            assert!(matches!(err, SearchIndexerError::UnsafeUrl { .. }));
        }
    }

    #[tokio::test]
    async fn validate_fetch_url_rejects_localhost_hostname() {
        // WHY: exercises the DNS-resolution branch — "localhost" resolves via
        // the system resolver to a loopback address, which must be rejected.
        let err = validate_fetch_url("http://localhost:8080/x")
            .await
            .expect_err("localhost must be rejected");
        assert!(matches!(err, SearchIndexerError::UnsafeUrl { .. }));
    }

    #[tokio::test]
    async fn validate_fetch_url_rejects_link_local() {
        let err = validate_fetch_url("http://169.254.169.254/latest/meta-data/")
            .await
            .expect_err("metadata endpoint must be rejected");
        assert!(matches!(err, SearchIndexerError::UnsafeUrl { .. }));
    }

    #[tokio::test]
    async fn validate_fetch_url_rejects_private_ranges() {
        for url in [
            "http://10.0.0.1/x",
            "http://172.16.5.5/x",
            "http://192.168.1.1/x",
            "http://100.64.0.1/x",
            "http://0.0.0.0/x",
        ] {
            let err = validate_fetch_url(url).await.expect_err(url);
            assert!(matches!(err, SearchIndexerError::UnsafeUrl { .. }));
        }
    }

    #[tokio::test]
    async fn validate_fetch_url_rejects_ipv6_local_and_mapped() {
        for url in [
            "http://[::1]/x",
            "http://[fc00::1]/x",
            "http://[fe80::1]/x",
            "http://[::ffff:127.0.0.1]/x",
            "http://[::ffff:192.168.1.1]/x",
            // WHY: deprecated IPv4-compatible IPv6 (::/96) embed — must be
            // judged by the embedded v4 or [::127.0.0.1] reaches loopback.
            "http://[::127.0.0.1]/x",
            "http://[::169.254.169.254]/x",
        ] {
            let err = validate_fetch_url(url).await.expect_err(url);
            assert!(matches!(err, SearchIndexerError::UnsafeUrl { .. }));
        }
    }

    #[tokio::test]
    async fn validate_fetch_url_allows_public_ip_literal() {
        // NOTE: TEST-NET-3 documentation range — public per the enforced
        // ranges, never actually fetched by this validation.
        assert!(
            validate_fetch_url("http://203.0.113.10/file.torrent")
                .await
                .is_ok()
        );
        assert!(
            validate_fetch_url("https://203.0.113.10:8443/file.torrent")
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn validate_fetch_url_error_redacts_api_key() {
        let err = validate_fetch_url("http://127.0.0.1/get?apikey=supersecret")
            .await
            .expect_err("loopback must be rejected");
        let display = err.to_string();
        assert!(!display.contains("supersecret"), "leaked: {display}");
    }

    #[test]
    fn ip_range_classification() {
        use std::net::{Ipv4Addr, Ipv6Addr};
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(172, 31, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(100, 127, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        // NOTE: deprecated IPv4-compatible IPv6 (::/96) — judged by embedded v4.
        assert!(ip_is_disallowed("::127.0.0.1".parse().unwrap()));
        assert!(ip_is_disallowed("::10.0.0.1".parse().unwrap()));
        assert!(ip_is_disallowed("::169.254.169.254".parse().unwrap()));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))));
        assert!(!ip_is_disallowed("::203.0.113.1".parse().unwrap()));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(100, 63, 0, 1))));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
    }

    #[tokio::test]
    async fn ssrf_resolver_rejects_loopback_resolution() {
        // WHY: "localhost" resolves to a loopback address via the system
        // resolver — the connect-time guard must reject it, proving the
        // rebinding TOCTOU is closed even after validate_fetch_url passed.
        let resolver = SsrfGuardResolver;
        let name: Name = "localhost".parse().expect("localhost is a valid DNS name");
        let result = resolver.resolve(name).await;
        assert!(
            result.is_err(),
            "resolver must reject a host that resolves to a loopback address"
        );
    }
}
