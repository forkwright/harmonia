use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::cf_bypass::cookies::CookieStore;
use crate::cf_bypass::{CloudflareProxy, Cookie, ProxyResponse};
use crate::error::{self, SearchIndexerError};

pub struct ByparrProxy {
    client: reqwest::Client,
    endpoint: String,
    timeout: Duration,
    max_body_bytes: u64,
    // WHY: a Cloudflare solve costs seconds — successful solves are cached
    // per target host and reused via a direct fetch until the clearance
    // cookies approach expiry (`zetesis.cf_cookie_refresh_minutes`) or the
    // origin challenges again.
    cookies: CookieStore,
    cookie_refresh: Duration,
}

/// Outcome of a direct (cookie-reuse) fetch: a usable response, or a
/// Cloudflare re-challenge that demands a fresh solve.
enum DirectOutcome {
    Response(ProxyResponse),
    Challenged,
}

#[derive(Debug, Serialize)]
struct ByparrRequest {
    cmd: &'static str,
    url: String,
    #[serde(rename = "maxTimeout")]
    max_timeout: u32,
}

#[derive(Debug, Deserialize)]
struct ByparrResponse {
    status: String,
    message: String,
    solution: Option<ByparrSolution>,
}

#[derive(Debug, Deserialize)]
struct ByparrSolution {
    #[expect(
        dead_code,
        reason = "schema-contract: byparr JSON response field, unused by current logic"
    )]
    url: String,
    status: u16,
    response: String,
    cookies: Vec<ByparrCookie>,
    #[serde(rename = "userAgent")]
    user_agent: String,
}

#[derive(Debug, Deserialize)]
struct ByparrCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    expires: f64,
    #[serde(rename = "httpOnly")]
    http_only: bool,
    secure: bool,
}

impl ByparrProxy {
    pub fn new(
        endpoint: String,
        timeout: Duration,
        max_body_bytes: u64,
        cookie_refresh: Duration,
    ) -> Self {
        // WHY unwrap_or_default: only catches a genuinely-invalid TLS
        // config (an Err). It does NOT catch reqwest's rustls-no-provider
        // "no crypto provider installed" failure — that's a panic!, not an
        // Err, and unwrap_or_default() cannot intercept a panic. Production
        // callers are covered by main.rs. Test callers currently always
        // spawn a mock server (which installs the provider) before calling
        // this — but that precondition is easy for a future test to skip
        // without noticing (exactly what happened in newznab.rs, torznab.rs,
        // and cardigann/tests.rs), so install it here too rather than trust
        // every future caller to remember.
        #[cfg(test)]
        crate::test_support::install_test_crypto_provider();
        let client = reqwest::Client::builder()
            .timeout(timeout + Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            endpoint,
            timeout,
            max_body_bytes,
            cookies: CookieStore::new(),
            cookie_refresh,
        }
    }
}

impl CloudflareProxy for ByparrProxy {
    fn get(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProxyResponse, SearchIndexerError>> + Send + '_>> {
        let url = url.to_string();
        Box::pin(async move { self.get_inner(&url, ct).await })
    }
}

impl ByparrProxy {
    // WHY: skip `url` — the indexer URL embeds `apikey=` and #[instrument]
    // would capture the raw value as a span field visible to any event
    // emitted in the span.
    #[instrument(skip(self, url, ct), fields(endpoint = %self.endpoint))]
    async fn get_inner(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<ProxyResponse, SearchIndexerError> {
        // WHY: errors embed the redacted form so the API key never reaches a
        // log line; the request itself still carries the real URL.
        let redacted = crate::client::redact_secrets(url);
        let host = reqwest::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string));

        if let Some(host) = &host
            && !self.cookies.needs_refresh(host, self.cookie_refresh)
            && let Some(cookie_header) = self.cookies.get_cookie_header(host)
            && let Some(user_agent) = self.cookies.get_user_agent(host)
        {
            match self
                .direct_get(url, &redacted, &cookie_header, &user_agent, &ct)
                .await?
            {
                DirectOutcome::Response(response) => return Ok(response),
                // WHY: the origin re-challenged despite fresh-looking
                // cookies — drop the jar and fall through to a new solve.
                DirectOutcome::Challenged => self.cookies.remove(host),
            }
        }

        let response = self.solve(url, &redacted, &ct).await?;
        // WHY: an empty cookie set has nothing to reuse — storing it would
        // only add a jar that the reuse guard skips anyway.
        if let Some(host) = &host
            && !response.cookies.is_empty()
        {
            self.cookies
                .store(host, response.cookies.clone(), response.user_agent.clone());
        }
        Ok(response)
    }

    /// Direct fetch of `url` reusing solved clearance cookies. A 403/503
    /// answer is Cloudflare challenging again — reported as `Challenged`
    /// instead of handing the challenge page to the indexer parser.
    async fn direct_get(
        &self,
        url: &str,
        redacted: &str,
        cookie_header: &str,
        user_agent: &str,
        ct: &CancellationToken,
    ) -> Result<DirectOutcome, SearchIndexerError> {
        let request = self
            .client
            .get(url)
            .header(reqwest::header::COOKIE, cookie_header)
            .header(reqwest::header::USER_AGENT, user_agent);
        let response = tokio::select! {
            result = request.send() => {
                result.context(error::HttpRequestSnafu { url: redacted.to_string() })?
            }
            () = ct.cancelled() => {
                return Err(SearchIndexerError::Cancelled {
                    url: redacted.to_string(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        };

        let status = response.status().as_u16();
        if status == 403 || status == 503 {
            return Ok(DirectOutcome::Challenged);
        }

        // WHY: same body-size cap as the solve path — a direct fetch returns
        // third-party bytes and must not buffer unbounded.
        let raw =
            crate::client::read_body_bytes_bounded(response, redacted, self.max_body_bytes).await?;
        Ok(DirectOutcome::Response(ProxyResponse {
            status,
            body: String::from_utf8_lossy(&raw).into_owned(),
            cookies: Vec::new(),
            user_agent: user_agent.to_string(),
        }))
    }

    /// Full Cloudflare solve through the byparr service.
    async fn solve(
        &self,
        url: &str,
        redacted: &str,
        ct: &CancellationToken,
    ) -> Result<ProxyResponse, SearchIndexerError> {
        let req = ByparrRequest {
            cmd: "request.get",
            url: url.to_string(),
            max_timeout: self.timeout.as_millis() as u32,
        };

        let endpoint_url = format!("{}/v1", self.endpoint.trim_end_matches('/'));

        let response = tokio::select! {
            result = self.client.post(&endpoint_url).json(&req).send() => {
                result.context(error::HttpRequestSnafu { url: redacted.to_string() })?
            }
            () = ct.cancelled() => {
                return Err(SearchIndexerError::Cancelled {
                    url: redacted.to_string(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        };

        // WHY: enforce the body-size cap HERE — the cf_bypass fetch paths
        // return this ProxyResponse.body directly, skipping read_body_bounded,
        // so without a cap on the byparr envelope `max_response_body_bytes` is
        // a no-op for cf_bypass indexers. The cap bounds the byparr JSON
        // (Content-Length precheck + streamed running counter) before it is
        // buffered; the wrapped indexer body is a subset, so the effective
        // ceiling is conservative by the envelope overhead.
        let raw =
            crate::client::read_body_bytes_bounded(response, redacted, self.max_body_bytes).await?;
        let byparr_resp: ByparrResponse =
            serde_json::from_slice(&raw).map_err(|e| SearchIndexerError::ParseResponse {
                url: redacted.to_string(),
                error: e.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        if byparr_resp.status != "ok" {
            return Err(SearchIndexerError::CfProxyError {
                url: redacted.to_string(),
                status: byparr_resp.status,
                message: byparr_resp.message,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }

        let solution = byparr_resp
            .solution
            .ok_or_else(|| SearchIndexerError::CfProxyError {
                url: redacted.to_string(),
                status: "ok".to_string(),
                message: "no solution in response".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        let cookies = solution
            .cookies
            .into_iter()
            .map(|c| Cookie {
                name: c.name,
                value: c.value,
                domain: c.domain,
                path: c.path,
                expires: c.expires,
                http_only: c.http_only,
                secure: c.secure,
            })
            .collect();

        Ok(ProxyResponse {
            status: solution.status,
            body: solution.response,
            cookies,
            user_agent: solution.user_agent,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    use super::*;
    use crate::test_support::{install_test_crypto_provider, spawn_one_shot_http};

    /// Refresh window for tests where reuse-vs-resolve is not under test.
    const ANY_REFRESH: Duration = Duration::from_secs(60);

    /// Spawns a TCP server that answers EVERY request with `status` + `body`,
    /// counting hits and recording each raw request head.
    async fn spawn_counting_http(
        status: u16,
        body: String,
        hits: Arc<AtomicUsize>,
        heads: Arc<Mutex<Vec<String>>>,
    ) -> (String, JoinHandle<()>) {
        // WHY: ByparrProxy::new below eagerly builds a TLS connector and
        // PANICS (not a recoverable Err — .unwrap_or_default() only catches
        // Err) with no provider installed; see
        // test_support::install_test_crypto_provider's WHY note. This is a
        // file-local spawn helper, separate from test_support's, so it needs
        // its own install call rather than inheriting one transitively.
        install_test_crypto_provider();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                hits.fetch_add(1, Ordering::SeqCst);
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = stream.read(&mut chunk).await.unwrap();
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                heads
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&buf).into_owned());
                let response = format!(
                    "HTTP/1.1 {status} X\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.ok();
                stream.shutdown().await.ok();
            }
        });
        (url, handle)
    }

    fn counting() -> (Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
        (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(Mutex::new(Vec::new())),
        )
    }

    fn solution_json(cookie_expires: f64, response_body: &str) -> String {
        format!(
            r#"{{"status":"ok","message":"solved","solution":{{"url":"https://e.example","status":200,"response":"{response_body}","cookies":[{{"name":"cf_clearance","value":"clearance-token","domain":".example.com","path":"/","expires":{cookie_expires},"httpOnly":false,"secure":true}}],"userAgent":"UA-solved"}}}}"#
        )
    }

    fn unix_now_plus(secs: f64) -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + secs
    }

    #[tokio::test]
    async fn get_enforces_body_size_cap() {
        // WHY: the cf_bypass fetch paths return ByparrProxy's body directly,
        // so the cap must live here or `max_response_body_bytes` is a no-op
        // for cf_bypass indexers. A byparr envelope declaring a size above the
        // cap must be rejected before it is buffered.
        let big = "x".repeat(10_000);
        let json = format!(
            r#"{{"status":"ok","message":"","solution":{{"url":"https://e.example","status":200,"response":"{big}","cookies":[],"userAgent":"UA"}}}}"#
        );
        let (endpoint, _server) = spawn_one_shot_http(200, "OK", &[], &json).await;
        let proxy = ByparrProxy::new(endpoint, Duration::from_secs(5), 64, ANY_REFRESH);
        let err = proxy
            .get(
                "https://indexer.example/api?apikey=secret",
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, SearchIndexerError::ResponseTooLarge { limit: 64, .. }),
            "expected ResponseTooLarge, got {err:?}"
        );
    }

    #[tokio::test]
    async fn fresh_cookies_reuse_direct_fetch_with_one_solve() {
        let (byparr_hits, byparr_heads) = counting();
        let (endpoint, _byparr) = spawn_counting_http(
            200,
            solution_json(unix_now_plus(3600.0), "<solved/>"),
            Arc::clone(&byparr_hits),
            byparr_heads,
        )
        .await;
        let (indexer_hits, indexer_heads) = counting();
        let (indexer_url, _indexer) = spawn_counting_http(
            200,
            "<direct/>".to_string(),
            Arc::clone(&indexer_hits),
            Arc::clone(&indexer_heads),
        )
        .await;

        let proxy = ByparrProxy::new(
            endpoint,
            Duration::from_secs(5),
            1024 * 1024,
            Duration::from_secs(60),
        );

        let first = proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(first.body, "<solved/>");
        assert_eq!(byparr_hits.load(Ordering::SeqCst), 1);
        assert_eq!(indexer_hits.load(Ordering::SeqCst), 0);

        let second = proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(second.body, "<direct/>");
        assert_eq!(
            byparr_hits.load(Ordering::SeqCst),
            1,
            "a fresh cookie must not trigger a second solve"
        );
        assert_eq!(indexer_hits.load(Ordering::SeqCst), 1);

        let head = indexer_heads.lock().unwrap().first().cloned().unwrap();
        assert!(
            head.contains("cf_clearance=clearance-token"),
            "direct fetch must send the stored cookie: {head}"
        );
        assert!(
            head.contains("UA-solved"),
            "direct fetch must reuse the solve's user agent: {head}"
        );
    }

    #[tokio::test]
    async fn near_expiry_cookie_forces_resolve() {
        let (byparr_hits, byparr_heads) = counting();
        // WHY: the cookie's remaining life (30s) is inside the refresh window
        // (300s) — the second request must re-solve, never reuse.
        let (endpoint, _byparr) = spawn_counting_http(
            200,
            solution_json(unix_now_plus(30.0), "<solved/>"),
            Arc::clone(&byparr_hits),
            byparr_heads,
        )
        .await;
        let (indexer_hits, indexer_heads) = counting();
        let (indexer_url, _indexer) = spawn_counting_http(
            200,
            "<direct/>".to_string(),
            Arc::clone(&indexer_hits),
            indexer_heads,
        )
        .await;

        let proxy = ByparrProxy::new(
            endpoint,
            Duration::from_secs(5),
            1024 * 1024,
            Duration::from_secs(300),
        );

        proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap();
        let second = proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(second.body, "<solved/>");
        assert_eq!(
            byparr_hits.load(Ordering::SeqCst),
            2,
            "a near-expiry cookie must force a re-solve"
        );
        assert_eq!(indexer_hits.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn challenged_direct_fetch_falls_back_to_resolve() {
        let (byparr_hits, byparr_heads) = counting();
        let (endpoint, _byparr) = spawn_counting_http(
            200,
            solution_json(unix_now_plus(3600.0), "<solved/>"),
            Arc::clone(&byparr_hits),
            byparr_heads,
        )
        .await;
        let (indexer_hits, indexer_heads) = counting();
        let (indexer_url, _indexer) = spawn_counting_http(
            403,
            "challenge page".to_string(),
            Arc::clone(&indexer_hits),
            indexer_heads,
        )
        .await;

        let proxy = ByparrProxy::new(
            endpoint,
            Duration::from_secs(5),
            1024 * 1024,
            Duration::from_secs(60),
        );

        let first = proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(first.body, "<solved/>");

        let second = proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            second.body, "<solved/>",
            "a challenged direct fetch must fall back to a fresh solve"
        );
        assert_eq!(
            indexer_hits.load(Ordering::SeqCst),
            1,
            "the direct attempt reached the origin exactly once"
        );
        assert_eq!(
            byparr_hits.load(Ordering::SeqCst),
            2,
            "the challenge must force a re-solve"
        );
    }

    #[tokio::test]
    async fn direct_fetch_enforces_body_size_cap() {
        let (byparr_hits, byparr_heads) = counting();
        let (endpoint, _byparr) = spawn_counting_http(
            200,
            solution_json(unix_now_plus(3600.0), "ok"),
            byparr_hits,
            byparr_heads,
        )
        .await;
        let (indexer_hits, indexer_heads) = counting();
        let (indexer_url, _indexer) =
            spawn_counting_http(200, "x".repeat(10_000), indexer_hits, indexer_heads).await;

        // WHY: 2048 admits the byparr envelope but not the 10_000-byte direct
        // body — the cap must bind on the reuse path too.
        let proxy = ByparrProxy::new(endpoint, Duration::from_secs(5), 2048, ANY_REFRESH);

        proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap();
        let err = proxy
            .get(&indexer_url, CancellationToken::new())
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                SearchIndexerError::ResponseTooLarge { limit: 2048, .. }
            ),
            "expected ResponseTooLarge on the direct path, got {err:?}"
        );
    }

    #[test]
    fn byparr_request_serialization() {
        let req = ByparrRequest {
            cmd: "request.get",
            url: "https://example.com".to_string(),
            max_timeout: 60000,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["cmd"], "request.get");
        assert_eq!(json["maxTimeout"], 60000);
    }

    #[test]
    fn byparr_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "message": "Challenge solved!",
            "solution": {
                "url": "https://example.com",
                "status": 200,
                "response": "<html>test</html>",
                "cookies": [
                    {
                        "name": "cf_clearance",
                        "value": "abc123",
                        "domain": ".example.com",
                        "path": "/",
                        "expires": 1709500000.0,
                        "httpOnly": false,
                        "secure": true
                    }
                ],
                "userAgent": "Mozilla/5.0 Test"
            }
        }"#;

        let resp: ByparrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "ok");
        let solution = resp.solution.unwrap();
        assert_eq!(solution.status, 200);
        assert_eq!(solution.cookies.len(), 1);
        assert_eq!(solution.cookies[0].name, "cf_clearance");
        assert_eq!(solution.user_agent, "Mozilla/5.0 Test");
    }

    #[test]
    fn byparr_error_response_deserialization() {
        let json = r#"{
            "status": "error",
            "message": "Error: Unable to solve the challenge. Timeout.",
            "solution": null
        }"#;

        let resp: ByparrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "error");
        assert!(resp.solution.is_none());
    }
}
