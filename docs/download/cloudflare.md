# Cloudflare bypass: pluggable proxy interface for cF-protected indexers

Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Zetesis ownership), [download/indexer-protocol.md](indexer-protocol.md) (indexers.cf_bypass column)

---

## The problem

Many private trackers and indexers deploy Cloudflare anti-bot protection. Standard HTTP requests from Harmonia are blocked by Cloudflare's challenge page (JS challenges, managed challenges, browser integrity checks). The challenge page returns a `403` or a redirect to a CAPTCHA that cannot be solved programmatically without a full browser runtime.

Browser automation is the only reliable bypass method. A CLI binary cannot embed a browser; this is the **one exception** to Harmonia's single-binary philosophy. The Byparr sidecar is the accepted approach: it runs as a separate process and exposes a simple HTTP API that Harmonia calls on demand.

Byparr is a drop-in replacement for FlareSolverr. It uses Camoufox (Firefox-based) instead of Chrome, which achieves better Cloudflare v2 challenge bypass. FlareSolverr itself is in maintenance mode; Byparr is the active fork.

---

## Pluggable proxy interface

Zetesis holds `Arc<dyn CloudflareProxy>`, injected at startup by `archon` based on configuration. Two implementations:

```rust
pub trait CloudflareProxy: Send + Sync {
    async fn get(&self, url: &str, ct: CancellationToken) -> Result<ProxyResponse, ZetesisError>;
}

pub struct ProxyResponse {
    pub status: u16,
    pub body: String,
    pub cookies: Vec<Cookie>,
    pub user_agent: String,
}
```

### ByparrProxy

Holds `reqwest::Client` and the configured endpoint URL. On `get()`:

1. POST to `{endpoint}/v1` with `ByparrRequest` body
2. Deserialize `ByparrResponse`
3. Map to `ProxyResponse`

```rust
pub struct ByparrProxy {
    client: reqwest::Client,
    endpoint: String,
}
```

### NoProxy

Returns `Err(ZetesisError::NoCfBypass { url })` immediately. Used when Byparr is not configured or health check failed at startup. Zetesis treats this as a known-absent capability, not an unexpected failure.

---

## Byparr / FlareSolverr API contract

The API is identical between Byparr and FlareSolverr v1.

### Request

POST to `http://{host}:{port}/v1`

```rust
#[derive(Debug, Serialize)]
pub struct ByparrRequest {
    pub cmd: &'static str,   // always "request.get"
    pub url: String,
    #[serde(rename = "maxTimeout")]
    pub max_timeout: u32,    // milliseconds; default 60000
}
```

JSON example:
```json
{
  "cmd": "request.get",
  "url": "https://some-indexer.example.com/api?t=search&q=test",
  "maxTimeout": 60000
}
```

### Response

```rust
#[derive(Debug, Deserialize)]
pub struct ByparrResponse {
    pub status: String,      // "ok" | "error"
    pub message: String,
    pub solution: Option<ByparrSolution>,
}

#[derive(Debug, Deserialize)]
pub struct ByparrSolution {
    pub url: String,
    pub status: u16,
    pub response: String,
    pub cookies: Vec<ByparrCookie>,
    #[serde(rename = "userAgent")]
    pub user_agent: String,
}

#[derive(Debug, Deserialize)]
pub struct ByparrCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,        // Unix timestamp; -1 means session cookie
    #[serde(rename = "httpOnly")]
    pub http_only: bool,
    pub secure: bool,
}
```

JSON example response:
```json
{
  "status": "ok",
  "message": "Challenge solved!",
  "solution": {
    "url": "https://some-indexer.example.com/api?t=search&q=test",
    "status": 200,
    "response": "<html>...</html>",
    "cookies": [
      {
        "name": "cf_clearance",
        "value": "abc123...",
        "domain": ".some-indexer.example.com",
        "path": "/",
        "expires": 1709500000.0,
        "httpOnly": false,
        "secure": true
      }
    ],
    "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0)..."
  }
}
```

Error response (status != "ok"):
```json
{
  "status": "error",
  "message": "Error: Unable to solve the challenge. Timeout.",
  "solution": null
}
```

---

## Cookie persistence

After a successful Cloudflare bypass, Harmonia avoids routing every subsequent request through Byparr:

1. Extract `cookies` from `ProxyResponse`
2. Store in an in-memory per-indexer cookie jar (`DashMap<IndexerId, IndexerCookieJar>`)
3. On subsequent requests to the same indexer domain: include stored cookies as `Cookie` header in regular `reqwest` calls
4. Cookie TTL: respect `expires` timestamp from `ByparrCookie`. A value of `-1` means session cookie; treat as expired on restart.
5. When `cf_cookie_refresh_minutes` before expiry (default 30 min): proactively route through Byparr again to refresh cookies

This means the vast majority of indexer requests go through `reqwest` directly. Byparr is only invoked when:
- Cookies are absent (first visit, restart)
- Cookies are approaching expiry (proactive refresh)
- A request returns HTTP 403 (cookies were invalidated mid-session)

```rust
pub struct IndexerCookieJar {
    cookies: Vec<StoredCookie>,
    user_agent: String,
    last_refreshed: Instant,
}

pub struct StoredCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires_at: Option<SystemTime>,
}
```

---

## Degradation flow

Per the locked decision: CF-protected indexers become `degraded`, not `failed`, when bypass is unavailable.

### Startup

1. If `[zetesis] cf_proxy_url` is set:
   - POST health check to `{cf_proxy_url}/v1` with dummy URL (`http://localhost`)
   - If Byparr responds with any non-connection-error: inject `ByparrProxy`
   - If connection refused or timeout: log warning, inject `NoProxy`
2. If `cf_proxy_url` is not configured: inject `NoProxy`, log info (not warning, as this is a deliberate choice)

### Per-search degradation

When Zetesis searches a CF-protected indexer (`indexers.cf_bypass = TRUE`):

```
proxy.get(url, ct)
  |
  +-- ByparrProxy: success
  |     -> use response body as indexer response
  |
  +-- ByparrProxy: failure (timeout, challenge unsolved)
  |     -> log warning with indexer name and error
  |     -> UPDATE indexers SET status = 'degraded' WHERE id = ?
  |     -> return empty result set (not an error)
  |
  +-- NoProxy: NoCfBypass
        -> UPDATE indexers SET status = 'degraded' WHERE id = ?
        -> return empty result set
```

Episkope receives empty results from degraded indexers and proceeds with results from active indexers. The degraded indexer does not surface as an error to the user; it appears only as a status indicator in the indexer list UI.

### Periodic recovery

A background task runs every `cf_health_check_interval_minutes` (default: 5):

1. If current proxy is `ByparrProxy`: no action
2. If current proxy is `NoProxy` and `cf_proxy_url` is configured:
   - Attempt health check
   - If Byparr is now healthy:
     - Replace `NoProxy` with `ByparrProxy` (via `ArcSwap<dyn CloudflareProxy>`)
     - `UPDATE indexers SET status = 'active' WHERE status = 'degraded' AND cf_bypass = TRUE`
     - Log info: Byparr reconnected, X indexers restored

---

## Error handling

`ZetesisError` variants for Cloudflare bypass:

```rust
#[derive(Debug, Snafu)]
pub enum ZetesisError {
    #[snafu(display("CF bypass proxy not available for {url}"))]
    NoCfBypass {
        url: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Byparr did not respond within {timeout}s for {url}"))]
    CfProxyTimeout {
        url: String,
        timeout: u32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Byparr returned error for {url}: [{status}] {message}"))]
    CfProxyError {
        url: String,
        status: String,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("CF cookies expired for indexer {indexer_name}"))]
    CfCookieExpired {
        indexer_name: String,
        #[snafu(implicit)]
        location: Location,
    },
}
```

---

## Horismos configuration

`[zetesis]` additions in `harmonia.toml`:

```toml
[zetesis]
# CF bypass endpoint. If absent, CF bypass is disabled and CF-protected indexers become degraded.
cf_proxy_url = "http://localhost:8191"

# Byparr challenge timeout in seconds. Cloudflare challenges can be slow on first solve.
cf_proxy_timeout_seconds = 60

# How many minutes before cookie expiry to proactively refresh via Byparr.
cf_cookie_refresh_minutes = 30

# How often to re-check Byparr health when it was previously unavailable.
cf_health_check_interval_minutes = 5
```

Example Docker Compose entry for Byparr sidecar:

```yaml
services:
  byparr:
    image: ghcr.io/thephaseblog/byparr:latest
    ports:
      - "8191:8191"
    environment:
      - LOG_LEVEL=info
    restart: unless-stopped
```
