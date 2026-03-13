# Indexer Protocol — Zetesis Torznab/Newznab Implementation

> Zetesis implements Torznab and Newznab protocols directly — no Prowlarr dependency.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Zetesis ownership), [data/want-release.md](../data/want-release.md) (releases table), [download/cloudflare.md](cloudflare.md) (CF bypass for protected indexers).

---

## Protocol Overview

Torznab and Newznab are closely related XML-over-HTTP indexer protocols used by nearly all modern private and public torrent and Usenet indexers.

### Transport

- HTTP GET to `/api` endpoint (path may vary by indexer; configurable)
- Query parameters:
  - `t=` — function selector (caps, search, tvsearch, movie, music, book)
  - `q=` — free-text search term
  - `cat=` — comma-separated category IDs to restrict results
  - `apikey=` — authentication token
  - `limit=` — max results (default: 100)
  - `offset=` — pagination offset
  - Type-specific: `tvdbid=`, `imdbid=`, `tmdbid=`, `season=`, `ep=`, `artist=`, `album=`, `author=`, `title=`

### Protocol Variants

| Feature | Torznab | Newznab |
|---------|---------|---------|
| Response format | RSS XML + `<torznab:attr>` namespace | RSS XML + `<newznab:attr>` namespace |
| Release type | BitTorrent (.torrent files, magnet URIs) | Usenet (NZB files) |
| Seeders/leechers | Present (`torznab:attr name="seeders"`) | Absent (no peer concept) |
| Info hash | Present (`torznab:attr name="infohash"`) | Absent |
| Grabs count | Optional | Present (`newznab:attr name="grabs"`) |
| Download link | Torrent file URL or magnet | NZB file URL |

### Search Functions

| Function | `t=` value | Extra Parameters | Indexer Support |
|---------|------------|-----------------|-----------------|
| General search | `search` | `q=` only | Universal |
| TV search | `tvsearch` | `tvdbid=`, `imdbid=`, `season=`, `ep=` | Most TV-focused indexers |
| Movie search | `movie` | `imdbid=`, `tmdbid=` | Most movie-focused indexers |
| Music search | `music` | `artist=`, `album=`, `label=`, `year=` | Music indexers |
| Book search | `book` | `author=`, `title=` | Book indexers |
| Capabilities | `caps` | none | All Torznab/Newznab indexers |

### `t=caps` Negotiation

`t=caps` is a **mandatory first call** to every newly configured indexer. It returns the indexer's capabilities as XML: which search functions are supported, which category IDs are available, and any server limits. Zetesis caches the response in `indexers.caps_json`.

Caps must be refreshed when:
1. `caps_json` is `NULL` (first configuration or manual reset)
2. Indexer returns an unexpected category or unsupported function error
3. Configurable schedule — default 24 hours (`caps_refresh_hours`)

---

## XML Parsing

Zetesis uses `quick-xml` with `serde` deserialization for all Torznab/Newznab XML responses.

### Struct Hierarchy

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

### Attribute Extraction

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

### Key Attributes Extracted

| Attribute Name | Type | Description |
|---------------|------|-------------|
| `seeders` | `u32` | Active seeders (Torznab only) |
| `leechers` | `u32` | Active leechers (Torznab only) |
| `infohash` | `String` | Torrent info hash (Torznab only) — used for dedup |
| `size` | `u64` | Release size in bytes (also present as RSS `<size>` element) |
| `category` | `u32` | Primary category ID — matches `indexer_categories` |
| `downloadvolumefactor` | `f64` | Ratio credit modifier (freeleech = 0.0) |
| `uploadvolumefactor` | `f64` | Ratio credit modifier (double upload = 2.0) |
| `grabs` | `u32` | Download count (Newznab only) |
| `guid` | `String` | Unique release identifier from indexer |

### `t=caps` Response Parsing

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

## `IndexerClient` Trait

The abstraction boundary between Zetesis's search routing and specific protocol implementations:

```rust
pub trait IndexerClient: Send + Sync {
    async fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> Result<Vec<SearchResult>, ZetesisError>;

    async fn caps(
        &self,
        ct: CancellationToken,
    ) -> Result<IndexerCaps, ZetesisError>;

    async fn test(
        &self,
        ct: CancellationToken,
    ) -> Result<IndexerStatus, ZetesisError>;

    async fn download(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<DownloadResponse, ZetesisError>;
}
```

### Implementations

| Struct | Implements | Description |
|--------|-----------|-------------|
| `TorznabClient` | `IndexerClient` | Native HTTP + XML for Torznab indexers. Handles magnet URI extraction from enclosure elements. |
| `NewznabClient` | `IndexerClient` | Same protocol as Torznab, NZB-specific response handling. Parses NZB file URL from enclosure. |
| `CardigannClient` (future) | `IndexerClient` | YAML-driven scraping for indexers without native API. See Cardigann Compatibility section. |

`TorznabClient` and `NewznabClient` share the XML parsing and HTTP transport layer — they differ primarily in how they interpret the download link and which `TorznabAttr` fields they extract.

### `DownloadResponse`

```rust
pub enum DownloadResponse {
    TorrentFile(Bytes),     // raw .torrent file bytes
    MagnetUri(String),      // magnet: URI extracted from response
    NzbFile(Bytes),         // raw .nzb file bytes
}
```

---

## SearchQuery and SearchResult Types

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

### From `SearchResult` to `releases` Table

Zetesis inserts a `releases` row for each `SearchResult` that passes initial filtering (category match, size limits):

| `SearchResult` field | `releases` column |
|---------------------|------------------|
| `title` | `title` |
| `download_url` | `download_url` |
| `size_bytes` | `size_bytes` |
| `info_hash` | `info_hash` |
| `indexer_id` | `indexer_id` |
| `protocol` | `protocol` ('torrent' or 'nzb') |
| Episkope quality evaluation | `quality_score` |
| Episkope custom format eval | `custom_format_score` |

`found_at` is set to the current UTC timestamp on insert.

---

## Indexer Registry Schema

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

### Column Definitions

| Column | Description |
|--------|-------------|
| `protocol` | `torznab` or `newznab` — determines which `IndexerClient` implementation to instantiate |
| `api_key` | Authentication token for `apikey=` parameter. Stored as plaintext in the database (which is itself protected by filesystem permissions) |
| `cf_bypass` | Whether this indexer is behind Cloudflare protection. When `TRUE`, requests are routed through the Byparr sidecar. See `cloudflare.md`. |
| `status` | `active` (healthy), `degraded` (CF bypass unavailable or intermittent errors), `failed` (unreachable or auth failure). Zetesis transitions status automatically based on request outcomes. |
| `last_tested` | Timestamp of the last `t=caps` or health check. Used with `caps_refresh_hours` to determine when to refresh caps. |
| `caps_json` | Serialized `IndexerCaps` JSON. `NULL` on first add — triggers immediate caps fetch. Populated after first successful `t=caps`. |
| `priority` | Search order. Lower number = searched first. Default 50. User-configurable per indexer. |

### `indexer_categories`

Populated from `t=caps` response — contains the indexer's supported category hierarchy. Used by search routing to filter which indexers to query for typed searches. `ON DELETE CASCADE` ensures categories are removed when the indexer is removed.

### `releases.indexer_id` Link

`releases.indexer_id` is an INTEGER FK pointing to `indexers.id`. As documented in `data/want-release.md`, there is no `REFERENCES` constraint on this column — it was left as a soft FK to avoid a forward dependency during Phase 4. The application layer (Zetesis insert path) enforces that `indexer_id` is always valid.

---

## Search Routing

Zetesis selects which indexers to query for a given `SearchQuery`:

### Step 1 — Filter Eligible Indexers

```sql
SELECT id, protocol, url, api_key, cf_bypass, caps_json, priority
FROM indexers
WHERE enabled = TRUE
  AND status != 'failed'
ORDER BY priority ASC
```

### Step 2 — Filter by Search Function Support

For typed searches (`Tv`, `Movie`, `Music`, `Book`), only include indexers whose `caps_json` includes the matching search function with `available = true`. Indexers without caps loaded yet (caps_json NULL) are included for `Any` searches but excluded for typed searches until caps are fetched.

### Step 3 — Parallel Fan-Out

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

### Step 4 — Deduplication

After merging results from all indexers:
- **Torrents**: deduplicate by `info_hash`. If two indexers return the same torrent, keep the one from the higher-priority indexer (lower `priority` value = earlier in sort).
- **NZBs**: deduplicate by `guid` (indexer-provided unique identifier).

### Step 5 — Return to Caller

Results are returned to Episkope as `Vec<SearchResult>`. Episkope evaluates each result against the want's quality profile (quality gate defined in `data/want-release.md`) and inserts accepted results as `releases` rows. Zetesis does not filter by quality — it returns all results that pass category and size constraints.

---

## Cardigann Compatibility — Future Extension

Prowlarr's Cardigann definitions provide 500+ indexer definitions for trackers that lack native Torznab/Newznab APIs. Full Cardigann support is out of scope for v1 — it requires a Go-style template engine, CSS/JSON/XML selectors, filter chains, and multi-step login flows (approximately 15K lines of implementation).

### v1 Scope: Interface Only

Phase 5 defines `CardigannClient` as a future `IndexerClient` implementation. The abstraction boundary is clear: any tracker that supports Torznab/Newznab natively uses `TorznabClient` or `NewznabClient`. Cardigann is only for trackers that require HTML scraping.

```rust
// Future implementation placeholder — not implemented in v1
pub struct CardigannClient {
    config: Arc<ZetesisConfig>,
    definition: CardigannDefinition,
    http_client: reqwest::Client,
}

// CardigannClient will implement IndexerClient when built
// impl IndexerClient for CardigannClient { ... }
```

### Cardigann YAML Subset for v1 Implementation

When Cardigann support is built, the first iteration should handle this YAML subset from Prowlarr-compatible definitions:

| YAML Section | v1 Support | Notes |
|-------------|-----------|-------|
| `id`, `name`, `description` | Yes | Identity fields |
| `caps.categorymappings` | Yes | Category ID mapping |
| `caps.modes` | Yes | Supported search functions |
| `search.paths` | Yes | URL path and parameter templates |
| `search.rows` | Yes | CSS selector for result rows |
| `search.fields` | Yes | Field extractors (CSS, JSON, regex) |
| `download` | Yes | Download link construction |
| `login` | **No** | Multi-step login adds major complexity — deferred |
| `ratio` | No | Ratio parsing — deferred |
| Filter chains (`re_replace`, `split`, etc.) | Partial | Common filters only |

**Definition source**: Harmonia reads Prowlarr-compatible YAML definitions from `config.zetesis.cardigann_definitions_dir`. Prowlarr's definition repository is the reference. Definitions are read at startup (or on directory watch trigger, future).

**Authentication-required trackers**: Trackers that require login (cookie-based sessions, form submission) are excluded from v1 Cardigann support. The `login` section is the primary source of Cardigann complexity. Users who need these trackers should use a Prowlarr sidecar and expose it as a Torznab feed to Harmonia — this is the practical escape hatch for the most complex trackers.

---

## Error Handling

`ZetesisError` uses snafu per `standards/RUST.md`:

```rust
#[derive(Debug, Snafu)]
pub enum ZetesisError {
    #[snafu(display("HTTP request to indexer {url} failed"))]
    HttpRequest {
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to parse Torznab/Newznab XML response from {url}"))]
    ParseResponse {
        url: String,
        source: quick_xml::DeError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} returned auth failure (bad API key)"))]
    AuthFailed {
        indexer_id: i64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} rate limited — retry after {retry_after_seconds}s"))]
    RateLimited {
        indexer_id: i64,
        retry_after_seconds: Option<u64>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} requires Cloudflare bypass but Byparr sidecar is unavailable"))]
    NoCfBypass {
        indexer_id: i64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("caps negotiation failed for indexer {indexer_id}"))]
    CapsUnavailable {
        indexer_id: i64,
        source: Box<ZetesisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
```

### Error → Status Transitions

| Error | Indexer Status Transition | Notes |
|-------|--------------------------|-------|
| `AuthFailed` | → `failed` | Bad API key requires user intervention |
| `HttpRequest` (repeated) | → `degraded` then `failed` | 3 consecutive failures → failed |
| `RateLimited` | No status change | Back off per `Retry-After` header; resume normally |
| `NoCfBypass` | → `degraded` | Degraded (not failed) — recoverable when Byparr starts |
| `CapsUnavailable` | → `degraded` | Can still serve cached caps; retry on schedule |
| `ParseResponse` | → `degraded` | Malformed response — may recover on next request |

### Rate Limiting

Per-indexer rate limiter using token bucket algorithm. Default limits:

```toml
[zetesis]
per_indexer_rate_limit_requests = 5
per_indexer_rate_limit_window_seconds = 10
```

Rate limits are applied per `indexer.id` — independent of whether requests come from search, caps refresh, or health checks. When rate limited by the indexer (HTTP 429), Zetesis respects the `Retry-After` header if present, otherwise backs off for `per_indexer_rate_limit_window_seconds`.

---

## Horismos Configuration — `[zetesis]` Section

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

`ZetesisConfig` struct additions in `crates/horismos/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZetesisConfig {
    // Existing fields:
    pub request_timeout_secs: u64,
    pub max_results_per_indexer: usize,
    pub cloudflare_bypass_enabled: bool,

    // New fields from this design:
    pub max_concurrent_searches: usize,                  // default: 10
    pub per_indexer_rate_limit_requests: u32,            // default: 5
    pub per_indexer_rate_limit_window_seconds: u64,      // default: 10
    pub caps_refresh_hours: u64,                         // default: 24
    pub search_timeout_seconds: u64,                     // default: 30
    pub cardigann_definitions_dir: Option<PathBuf>,      // default: None
}

impl Default for ZetesisConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: 30,
            max_results_per_indexer: 100,
            cloudflare_bypass_enabled: false,
            max_concurrent_searches: 10,
            per_indexer_rate_limit_requests: 5,
            per_indexer_rate_limit_window_seconds: 10,
            caps_refresh_hours: 24,
            search_timeout_seconds: 30,
            cardigann_definitions_dir: None,
        }
    }
}
```
