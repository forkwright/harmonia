# Indexer protocol: Zetesis Torznab/Newznab implementation

> Zetesis implements Torznab and Newznab protocols directly; no Prowlarr dependency.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Zetesis ownership), [data/want-release.md](../data/want-release.md) (releases table), [download/cloudflare.md](cloudflare.md) (CF bypass for protected indexers).

---

## Protocol overview

Torznab and Newznab are closely related XML-over-HTTP indexer protocols used by nearly all modern private and public torrent and Usenet indexers.

### Transport

- HTTP GET to `/api` endpoint (path may vary by indexer; configurable)
- Query parameters:
  - `t=`: function selector (caps, search, tvsearch, movie, music, book)
  - `q=`: free-text search term
  - `cat=`: comma-separated category IDs to restrict results
  - `apikey=`: authentication token
  - `limit=`: max results (default: 100)
  - `offset=`: pagination offset
  - Type-specific: `tvdbid=`, `imdbid=`, `tmdbid=`, `season=`, `ep=`, `artist=`, `album=`, `author=`, `title=`

### Protocol variants

| Feature | Torznab | Newznab |
|---------|---------|---------|
| Response format | RSS XML + `<torznab:attr>` namespace | RSS XML + `<newznab:attr>` namespace |
| Release type | BitTorrent (.torrent files, magnet URIs) | Usenet (NZB files) |
| Seeders/leechers | Present (`torznab:attr name="seeders"`) | Absent (no peer concept) |
| Info hash | Present (`torznab:attr name="infohash"`) | Absent |
| Grabs count | Optional | Present (`newznab:attr name="grabs"`) |
| Download link | Torrent file URL or magnet | NZB file URL |

### Search functions

| Function | `t=` value | Extra Parameters | Indexer Support |
|---------|------------|-----------------|-----------------|
| General search | `search` | `q=` only | Universal |
| TV search | `tvsearch` | `tvdbid=`, `imdbid=`, `season=`, `ep=` | Most TV-focused indexers |
| Movie search | `movie` | `imdbid=`, `tmdbid=` | Most movie-focused indexers |
| Music search | `music` | `artist=`, `album=`, `label=`, `year=` | Music indexers |
| Book search | `book` | `author=`, `title=` | Book indexers |
| Capabilities | `caps` | none | All Torznab/Newznab indexers |

### `T=caps` negotiation

`t=caps` is a **mandatory first call** to every newly configured indexer. It returns the indexer's capabilities as XML: which search functions are supported, which category IDs are available, and any server limits. Zetesis caches the response in `indexers.caps_json`.

Caps must be refreshed when:
1. `caps_json` is `NULL` (first configuration or manual reset)
2. Indexer returns an unexpected category or unsupported function error
3. Configurable schedule: default 24 hours (`caps_refresh_hours`)

---

## XML parsing

Zetesis uses `quick-xml` with `serde` deserialization for all Torznab/Newznab XML responses.

### Struct hierarchy

```rust
// Feed root
#[derive(Debug, Deserialize)]
struct TorznabFeed {
    channel: TorznabChannel,
}

// Channel contains metadata and items
#[derive(Debug, Deserialize)]
struct TorznabChannel {
    title: Option<String>,
    #[serde(rename = "item", default)]
    items: Vec<TorznabItem>,
}

// Individual release result
#[derive(Debug, Deserialize)]
struct TorznabItem {
    title: String,
    guid: Option<String>,
    #[serde(rename = "pubDate")]
    pub_date: Option<String>,
    size: Option<u64>,
    link: Option<String>,          // download URL
    #[serde(rename = "attr", default)]
    attrs: Vec<TorznabAttr>,
}

// torznab:attr or newznab:attr elements
#[derive(Debug, Deserialize)]
struct TorznabAttr {
    name: String,
    value: String,
}
```

### Attribute extraction

```rust
fn get_attr<'a>(attrs: &'a [TorznabAttr], name: &str) -> Option<&'a str> {
    attrs.iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

fn get_attr_u64(attrs: &[TorznabAttr], name: &str) -> Option<u64> {
    get_attr(attrs, name)?.parse().ok()
}

fn get_attr_f64(attrs: &[TorznabAttr], name: &str) -> Option<f64> {
    get_attr(attrs, name)?.parse().ok()
}
```

### Key attributes extracted

| Attribute Name | Type | Description |
|---------------|------|-------------|
| `seeders` | `u32` | Active seeders (Torznab only) |
| `leechers` | `u32` | Active leechers (Torznab only) |
| `infohash` | `String` | Torrent info hash (Torznab only), used for dedup |
| `size` | `u64` | Release size in bytes (also present as RSS `<size>` element) |
| `category` | `u32` | Primary category ID, matches `indexer_categories` |
| `downloadvolumefactor` | `f64` | Ratio credit modifier (freeleech = 0.0) |
| `uploadvolumefactor` | `f64` | Ratio credit modifier (double upload = 2.0) |
| `grabs` | `u32` | Download count (Newznab only) |
| `guid` | `String` | Unique release identifier from indexer |

### `T=caps` response parsing

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct IndexerCaps {
    pub server: ServerInfo,
    pub limits: SearchLimits,
    pub search_functions: Vec<SearchFunction>,
    pub categories: Vec<IndexerCategory>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchLimits {
    pub default: u32,    // default result count
    pub max: u32,        // maximum allowed
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchFunction {
    pub function_type: String,   // "search", "tvsearch", "movie", "music", "book"
    pub available: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndexerCategory {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub subcategories: Vec<IndexerCategory>,
}
```

Caps are stored as serialized JSON in `indexers.caps_json`. On next startup, Zetesis deserializes from this column rather than re-fetching from the indexer, unless the refresh schedule requires it.

---

## `IndexerClient` trait

The abstraction boundary between Zetesis's search routing and specific protocol implementations:

```rust
pub trait IndexerClient: Send + Sync {
    async fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> Result<Vec<SearchResult>, SearchIndexerError>;

    async fn caps(
        &self,
        ct: CancellationToken,
    ) -> Result<IndexerCaps, SearchIndexerError>;

    async fn test(
        &self,
        ct: CancellationToken,
    ) -> Result<IndexerStatus, SearchIndexerError>;

    async fn download(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<DownloadResponse, SearchIndexerError>;
}
```

### Implementations

| Struct | Implements | Description |
|--------|-----------|-------------|
| `TorznabClient` | `IndexerClient` | Native HTTP + XML for Torznab indexers. Handles magnet URI extraction from enclosure elements. |
| `NewznabClient` | `IndexerClient` | Same protocol as Torznab, NZB-specific response handling. Parses NZB file URL from enclosure. |
| `CardigannClient` | `IndexerClient` | YAML-definition-driven HTML scraping for indexers without native API. See Cardigann Compatibility section. |

`TorznabClient` and `NewznabClient` share the XML parsing and HTTP transport layer; they differ primarily in how they interpret the download link and which `TorznabAttr` fields they extract.

### `DownloadResponse`

```rust
pub enum DownloadResponse {
    TorrentFile(Bytes),     // raw .torrent file bytes
    MagnetUri(String),      // magnet: URI extracted from response
    NzbFile(Bytes),         // raw .nzb file bytes
}
```

---

## SearchQuery and SearchResult types

### `SearchQuery`

```rust
pub struct SearchQuery {
    pub query_text: Option<String>,
    pub media_type: SearchMediaType,
    pub category_ids: Vec<u32>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u32>,
    pub tmdb_id: Option<u32>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub author: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub limit: u32,           // default: 100
    pub offset: u32,          // default: 0 — pagination
}

pub enum SearchMediaType {
    Any,
    Tv,
    Movie,
    Music,
    Book,
}
```

`SearchMediaType` maps to the `t=` parameter:
- `Any` → `t=search`
- `Tv` → `t=tvsearch` (if indexer caps support it; falls back to `t=search` + `cat=5000`)
- `Movie` → `t=movie` (similar fallback to `t=search` + `cat=2000`)
- `Music` → `t=music` (fallback to `t=search` + `cat=3000`)
- `Book` → `t=book` (fallback to `t=search` + `cat=7000`)

### `SearchResult`

```rust
pub struct SearchResult {
    pub title: String,
    pub download_url: String,
    pub size_bytes: Option<u64>,
    pub seeders: Option<u32>,         // None for Newznab results
    pub leechers: Option<u32>,        // None for Newznab results
    pub info_hash: Option<String>,    // None for Newznab results
    pub category_id: Option<u32>,
    pub publication_date: Option<DateTime<Utc>>,
    pub indexer_id: i64,              // FK to indexers.id
    pub protocol: ReleaseProtocol,
    pub download_volume_factor: f64,  // freeleech multiplier (1.0 = normal)
    pub upload_volume_factor: f64,    // ratio credit multiplier (1.0 = normal)
    pub custom_attrs: HashMap<String, String>,
}

pub enum ReleaseProtocol {
    Torrent,
    Nzb,
}
```

### From `SearchResult` to `releases` table

Zetesis inserts a `releases` row for each `SearchResult` that passes initial filtering (category match, size limits):

| `SearchResult` field | `releases` column |
|---------------------|------------------|
| `title` | `title` |
| `download_url` | `download_url` |
| `size_bytes` | `size_bytes` |
| `info_hash` | `info_hash` |
| `indexer_id` | `indexer_id` |
| `protocol` | `protocol` ('torrent' or 'nzb') |
| Monitoring quality evaluation | `quality_score` |
| Monitoring custom format eval | `custom_format_score` |

`found_at` is set to the current UTC timestamp on insert.

---

## Indexer registry schema

Owned by Zetesis. Stored in the main SQLite database alongside all other tables.

```sql
CREATE TABLE indexers (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    protocol    TEXT NOT NULL CHECK (protocol IN ('torznab', 'newznab')),
    api_key     TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    cf_bypass   BOOLEAN NOT NULL DEFAULT FALSE,
    status      TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'degraded', 'failed')),
    last_tested DATETIME,
    caps_json   TEXT,
    priority    INTEGER NOT NULL DEFAULT 50,
    added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE indexer_categories (
    indexer_id  INTEGER NOT NULL REFERENCES indexers(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL,
    name        TEXT NOT NULL,
    PRIMARY KEY (indexer_id, category_id)
);
```

### Column definitions

| Column | Description |
|--------|-------------|
| `protocol` | `torznab` or `newznab`; determines which `IndexerClient` implementation to instantiate |
| `api_key` | Authentication token for `apikey=` parameter. Stored as plaintext in the database (which is itself protected by filesystem permissions) |
| `cf_bypass` | Whether this indexer is behind Cloudflare protection. When `TRUE`, requests are routed through the Byparr sidecar. See `cloudflare.md`. |
| `status` | `active` (healthy), `degraded` (CF bypass unavailable or intermittent errors), `failed` (unreachable or auth failure). Zetesis transitions status automatically based on request outcomes. |
| `last_tested` | Timestamp of the last `t=caps` or health check. Used with `caps_refresh_hours` to determine when to refresh caps. |
| `caps_json` | Serialized `IndexerCaps` JSON. `NULL` on first add, which triggers immediate caps fetch. Populated after first successful `t=caps`. |
| `priority` | Search order. Lower number = searched first. Default 50. User-configurable per indexer. |

### `Indexer_categories`

Populated from `t=caps` response; contains the indexer's supported category hierarchy. Used by search routing to filter which indexers to query for typed searches. `ON DELETE CASCADE` ensures categories are removed when the indexer is removed.

### `Releases.indexer_id` link

`releases.indexer_id` is an INTEGER FK pointing to `indexers.id`. As documented in `data/want-release.md`, there is no `REFERENCES` constraint on this column; it was left as a soft FK to avoid a forward dependency during Phase 4. The application layer (Zetesis insert path) enforces that `indexer_id` is always valid.

---

## Search routing

Zetesis selects which indexers to query for a given `SearchQuery`:

### Step 1: filter eligible indexers

```sql
SELECT id, protocol, url, api_key, cf_bypass, caps_json, priority
FROM indexers
WHERE enabled = TRUE
  AND status != 'failed'
ORDER BY priority ASC
```

### Step 2: filter by search function support

For typed searches (`Tv`, `Movie`, `Music`, `Book`), only include indexers whose `caps_json` includes the matching search function with `available = true`. Indexers without caps loaded yet (caps_json NULL) are included for `Any` searches but excluded for typed searches until caps are fetched.

### Step 3: parallel fan-out

All eligible indexers are queried concurrently, up to `max_concurrent_searches` total parallel requests. Results from all indexers are merged into a single collection.

```rust
let results: Vec<SearchResult> = futures::stream::iter(eligible_indexers)
    .map(|indexer| {
        let client = self.client_for(&indexer);
        async move { client.search(query, ct.clone()).await }
    })
    .buffer_unordered(config.max_concurrent_searches)
    .filter_map(|r| async move { r.ok() })
    .flatten()
    .collect()
    .await;
```

### Step 4: deduplication

After merging results from all indexers:
- **Torrents**: deduplicate by `info_hash`. If two indexers return the same torrent, keep the one from the higher-priority indexer (lower `priority` value = earlier in sort).
- **NZBs**: deduplicate by `guid` (indexer-provided unique identifier).

### Step 5: return to caller

Results are returned to the monitoring layer as `Vec<SearchResult>`. Monitoring evaluates each result against the want's quality profile (quality gate defined in `data/want-release.md`) and inserts accepted results as `releases` rows. Zetesis does not filter by quality; it returns all results that pass category and size constraints.

---

## Cardigann compatibility

Prowlarr's Cardigann definitions provide 500+ indexer definitions for trackers that lack native Torznab/Newznab APIs. `CardigannClient` (`crates/zetesis/src/client/cardigann/`) executes the core Cardigann subset: templated GET search paths, CSS row/field selectors, a filter pipeline, and category mapping. The abstraction boundary is unchanged: any tracker that supports Torznab/Newznab natively uses `TorznabClient` or `NewznabClient`; Cardigann is only for trackers that require HTML scraping.

Indexer rows select the client with `protocol = 'cardigann'`; the row's `url` column carries either a definition id or one of the definition's site links. For `login.method: cookie`, the row's `api_key` column carries the session cookie.

### Supported Cardigann YAML subset

| YAML Section | Support | Notes |
|-------------|-----------|-------|
| `id`, `name`, `description`, `links` | Yes | Identity fields; templated links rejected |
| `caps.categorymappings` (+ legacy `caps.categories`) | Yes | Category ID mapping via the standard Torznab table |
| `caps.modes` | Yes | Supported search functions |
| `search.paths` (+ legacy `search.path`) | GET + POST | POST sends inputs as an `application/x-www-form-urlencoded` body (`$raw` + POST rejected at load; POST + `cf_bypass` rejected at construction). `response.type: json` parses flat and nested result arrays — `rows.selector` and field `selector`s are dotted/`[n]`/`['key']` JSON paths, arrays comma-join, JSON `null` → empty; nested rows drill into each parent via `rows.attribute`/`multiple` (+ `missingAttributeEqualsNoResults`), with a leading-`..` field reading the parent object. `:has()/:not()/:contains()` pseudo-filter selectors and `$..`/mid-path recursive descent are rejected at load; `xml` rejected at load |
| `search.inputs` / `keywordsfilters` | Yes | Template subset: `.Keywords`, `.Categories`, `.Config.<key>`, `.Query.<field>`, `join`; `$raw` input supported. `if`/`range` rejected at load |
| `search.rows` | `selector` + `remove` | `filters` (andmatch), `after`, `dateheaders` warned and ignored |
| `search.fields` | Yes | `selector`, `attribute`, `text`, `optional`, `case`, `remove`, filters |
| Filter chains | Common set | `regexp`, `re_replace`, `replace`, `split`, `trim`, `prepend`, `append`, `tolower`, `toupper`, `querystring`, `dateparse`, `timeago` (best-effort); unknown filters rejected at load |
| `download` | `selector`/`attribute`/filters | `before` pre-requests and `infohash` fallback warned and ignored |
| `login` | `none`, `cookie`, `form`, `post`, `get` | Interactive methods log in against the site and cache a per-indexer session cookie. `login.error` blocks surface the tracker's failure message; `login.test` (path + selector) is a content assertion. `login.selectorinputs` / `login.captcha` and `oneurl` / unknown methods are rejected. See **Interactive login** below |
| `settings` | Yes | Per-indexer overrides stored on the indexer row (`settings_json`), validated at client construction against the declared `settings:` fields, and overlaid onto `.Config.<key>` (definition `default:` → user override → injected `cookie`). `checkbox` values render as the literal strings `"true"`/`"false"`; `{{ if .Config.x }}` conditionals stay rejected at load (this engine's template subset has no `if`) |
| `ratio` | No | Deferred |

**Definition source**: Harmonia reads Prowlarr-compatible YAML definitions from `config.zetesis.cardigann_definitions_dir`, one indexer per `.yml`/`.yaml` file, at startup (`SearchIndexerService::new` → `CardigannRegistry::load`). Definitions that fail validation are skipped with a warning naming the unsupported construct. Third-party definitions are not vendored into the repo.

**Authentication-required trackers**: `login.method: none`, `cookie`, `form`, `post`, and `get` are executed directly. For `cookie`, the indexer row's `api_key` column carries the session cookie. For the interactive methods (`form`/`post`/`get`), credentials come from the indexer's `settings` (e.g. `username`/`password` referenced as `{{ .Config.username }}` in `login.inputs`) and Harmonia performs the login itself, caching the resulting session per indexer. Trackers that need CAPTCHA solving, selector-driven inputs (`login.selectorinputs`), or the `oneurl` flow are rejected at load; those still route through a Prowlarr sidecar exposed as a Torznab feed.

### Interactive login (`form` / `post` / `get`)

`CardigannClient` establishes a session before the first search/test/download and reuses it across requests:

- **`form`**: GET `login.path` with a dedicated no-redirect client, harvest every `Set-Cookie` and every `input[name]` (hidden CSRF tokens included) inside the `login.form` selector (default `form`), overlay the rendered `login.inputs`, then POST `application/x-www-form-urlencoded` to `login.submitpath` (rendered) — else the form's `action`, else the login-page URL.
- **`post`**: skip the page fetch; POST the rendered `login.inputs` straight to `login.path`.
- **`get`**: same as `post` with an HTTP GET, inputs carried as query pairs.

Shared discipline:

- **Same-host gate (credential-exfiltration chokepoint)**: the resolved submit target and every redirect hop must share the indexer's host, or the login fails without a request leaving — a hostile definition or a compromised login page cannot POST credentials off-origin. Redirects are followed manually (capped at 5) so each hop's `Set-Cookie` is captured.
- **Sessions** are keyed by indexer-instance id (two rows on one host with different accounts never mix), held in memory only (a restart re-logs-in), and reused until they go stale.
- **Staleness**: a search whose final URL lands back on the login path, or a 401/403, invalidates the session and re-logs-in once before retrying the fetch once; a second failure is an auth failure.
- **`login.error`** blocks turn the tracker's own message into a `LoginFailed` error (the submitted body/credentials are never echoed). **`login.test`** (path + selector) is a content assertion — a reachable login page is not a healthy session.
- **`cf_bypass`** is incompatible with every authenticated login method (the bypass proxy carries no request headers) and is rejected at client construction.
- Rendered login bodies and `get`-method URLs (whose query carries credentials) are never logged.

---

## Error handling

`SearchIndexerError` (`crates/zetesis/src/error.rs`) is one `#[non_exhaustive]` snafu enum shared by every indexer client (Torznab, Newznab, Cardigann) and the search-dispatch service, per `standards/RUST.md`. Every variant carries a `#[snafu(implicit)] location`; read the enum directly for the current, full variant list rather than a doc snapshot — it grows as new client/definition/settings failure modes are added. Broad categories: transport (`HttpRequest`, `Cancelled`, `ResponseTooLarge`, `UnsafeUrl`), indexer-reported (`AuthFailed`, `RateLimited`), Cloudflare-bypass (`NoCfBypass`, `CfProxyTimeout`, `CfProxyError`, `CfCookieExpired`), persistence (`Database`, `IndexerNotFound`), response parsing (`ParseResponse`, `CapsUnavailable`), and Cardigann configuration (`DefinitionLoad`, `DefinitionInvalid`, `DefinitionUnsupported`, `DefinitionNotFound`, `LoginUnsupported`, `LoginFailed`, `CookieAuthRequired`, `SettingsJsonInvalid`, `SettingsInvalid`).

### Error → status transitions

`SearchIndexerService::handle_search_error` (`crates/zetesis/src/search.rs`) maps a subset of variants to an indexer status change; every other variant leaves status untouched.

| Error | Indexer Status Transition | Notes |
|-------|--------------------------|-------|
| `AuthFailed` | → `failed` | Bad API key requires user intervention |
| `HttpRequest` (repeated) | → `degraded` then `failed` | Second consecutive failure while already `degraded` → `failed` |
| `RateLimited` | No status change | Back off per `Retry-After` header; resume normally |
| `Cancelled` | No status change | Caller intent, not indexer failure |
| `NoCfBypass`, `CfProxyTimeout`, `CfProxyError` | → `degraded` | Recoverable when Byparr becomes available |
| `CapsUnavailable` | → `degraded` | Can still serve cached caps; retry on schedule |
| `ParseResponse`, `ResponseTooLarge` | → `degraded` | Malformed/oversized response; may recover on next request |
| `DefinitionNotFound`, `DefinitionLoad`, `DefinitionInvalid`, `DefinitionUnsupported`, `LoginUnsupported`, `CookieAuthRequired`, `SettingsJsonInvalid`, `SettingsInvalid` | → `failed` | Misconfiguration (definition, login, or settings override) fails every search identically until the operator intervenes |
| `LoginFailed` | → `failed` | Interactive login rejected (bad credentials, off-host submit, redirect cap, failed login-test) |

### Rate limiting

Per-indexer rate limiter using token bucket algorithm. Default limits:

```toml
[zetesis]
per_indexer_rate_limit_requests = 5
per_indexer_rate_limit_window_seconds = 10
```

Rate limits are applied per `indexer.id`, independent of whether requests come from search, caps refresh, or health checks. When rate limited by the indexer (HTTP 429), Zetesis respects the `Retry-After` header if present, otherwise backs off for `per_indexer_rate_limit_window_seconds`.

---

## Horismos configuration: `[zetesis]` section

```toml
[zetesis]
# Maximum parallel indexer requests across all searches
max_concurrent_searches = 10

# Per-indexer rate limiting
per_indexer_rate_limit_requests = 5
per_indexer_rate_limit_window_seconds = 10

# How often to refresh caps from each indexer (hours)
caps_refresh_hours = 24

# Timeout for individual indexer search requests (seconds)
search_timeout_seconds = 30

# Optional: directory containing Prowlarr-compatible Cardigann YAML definitions
# cardigann_definitions_dir = "/data/config/indexer-definitions"

# Existing fields (from Phase 3 configuration.md):
request_timeout_secs = 30
max_results_per_indexer = 100
cloudflare_bypass_enabled = false
```

`SearchSubsystemConfig` in `crates/horismos/src/subsystems.rs` (not `ZetesisConfig` — that name never shipped):

```rust
pub struct SearchSubsystemConfig {
    pub request_timeout_secs: u64,
    pub max_results_per_indexer: usize,
    pub max_response_body_bytes: u64,
    pub cloudflare_bypass_enabled: bool,
    pub max_concurrent_searches: usize,
    pub per_indexer_rate_limit_requests: u32,
    pub per_indexer_rate_limit_window_seconds: u64,
    pub caps_refresh_hours: u64,
    pub search_timeout_seconds: u64,
    pub cardigann_definitions_dir: Option<PathBuf>,
    pub cf_proxy_url: Option<String>,
    pub cf_proxy_timeout_seconds: u64,
    pub cf_cookie_refresh_minutes: u64,
}
```

Field defaults live in `SearchSubsystemConfig`'s `Default` impl alongside the struct — read it directly rather than a doc snapshot.
