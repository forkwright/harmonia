# Metadata Provider Strategy

> Epignosis's provider integration: canonical providers per type, rate limiting, caching, and API patterns.
> See [architecture/subsystems.md](../architecture/subsystems.md) for Epignosis ownership boundaries.
> See [architecture/communication.md](../architecture/communication.md) for the MetadataEnriched event.
> See [architecture/configuration.md](../architecture/configuration.md) for the [epignosis] config section.
> See [media/lifecycle.md](lifecycle.md) for the `enriched` state this document implements.

---

## Provider Strategy: Primary + Enrichment Merge

Each media type has one canonical provider (source of truth for identity resolution). Secondary providers enrich with additional metadata fields. The canonical provider is always queried first — enrichment providers only run after canonical succeeds.

If the canonical provider fails: the item stays in `imported` state and a retry is dispatched via Agoge with exponential backoff. Enrichment provider failures are non-fatal: the item proceeds to `organized` with partial metadata and a WARN log.

| Media Type | Canonical Provider | Enrichment Providers |
|------------|-------------------|---------------------|
| Music | MusicBrainz | Last.fm (tags, play counts, similar artists), AcoustID (fingerprint verification) |
| Movies | TMDB | — |
| TV | TVDB | TMDB (episode air dates, cast, overview) |
| Books | OpenLibrary | — |
| Audiobooks | Audnexus | OpenLibrary (book identity, ISBN, author) |
| Comics | ComicVine | — |
| Podcasts | iTunes Podcast API | — |

---

## Epignosis Architecture

Epignosis owns ALL external metadata API calls. No other subsystem makes provider requests directly.

### Public Trait

```rust
pub trait MetadataResolver: Send + Sync {
    /// Determine what a file IS — which album, movie, audiobook, etc.
    /// Uses filename parsing + provider search. Routes through ProviderQueue for rate limiting.
    async fn resolve_identity(
        &self,
        item: &UnidentifiedItem,
        ct: CancellationToken,
    ) -> Result<MediaIdentity, EpignosisError>;

    /// Fetch full metadata from canonical + enrichment providers.
    /// Canonical provider queried first; enrichment only after canonical succeeds.
    async fn enrich(
        &self,
        identity: &MediaIdentity,
        ct: CancellationToken,
    ) -> Result<EnrichedMetadata, EpignosisError>;

    /// Music only: compute AcoustID fingerprint and look up MBIDs.
    /// CPU-bound — runs in spawn_blocking internally.
    async fn fingerprint_audio(
        &self,
        file_path: &Path,
        ct: CancellationToken,
    ) -> Result<FingerprintResult, EpignosisError>;
}
```

Each method routes through the appropriate `ProviderQueue` for rate limiting before making any HTTP request.

### Subsystem Boundaries

Epignosis owns:
- All external metadata API credentials
- The in-process metadata cache (DashMap or equivalent)
- One `ProviderQueue` per provider (rate limiting)
- The TVDB JWT token and its refresh lifecycle

Epignosis does NOT own:
- Media file paths (Taxis owns those)
- Download state (Syntaxis owns that)
- Library organization (Taxis owns that)

---

## Per-Provider Rate Limiter

Token-bucket pattern using `tokio::time::interval`. One `ProviderQueue` per provider, created at Epignosis startup. Callers send a oneshot callback and receive a permit when their turn arrives. This serializes all requests to rate-limited providers.

```rust
struct ProviderQueue {
    tx: mpsc::Sender<oneshot::Sender<()>>,
}

impl ProviderQueue {
    fn new(requests_per_window: u32, window_seconds: u64) -> Self {
        let (tx, mut rx) = mpsc::channel(100);
        let interval_dur = Duration::from_millis(
            (window_seconds * 1000) / requests_per_window as u64
        );
        tokio::spawn(async move {
            let mut tick = interval(interval_dur);
            while let Some(caller_tx) = rx.recv().await {
                tick.tick().await;
                let _ = caller_tx.send(());
            }
        }.instrument(tracing::info_span!("provider_rate_limiter")));
        Self { tx }
    }

    async fn acquire(&self) -> ProviderPermit {
        let (cb_tx, cb_rx) = oneshot::channel();
        self.tx.send(cb_tx).await.ok();
        cb_rx.await.ok();
        ProviderPermit
    }
}
```

Per-provider rate limit budgets:

| Provider | Budget | Rationale |
|----------|--------|-----------|
| MusicBrainz | 1 req/s | Hard IP limit — HTTP 503 on violation, risk of IP ban |
| AcoustID | 3 req/s | Published limit |
| TMDB | 40 req/s | Conservative; no hard published limit |
| TVDB | 10 req/s | Conservative; no published hard limit |
| Audnexus | 5 req/s | Conservative; `retryAfterSeconds` field in error response |
| OpenLibrary | 10 req/s | Conservative; 100 req/5min for cover access endpoint |
| OpenSubtitles | 1 req/s | Token-bucket; `X-RateLimit-Remaining` header |
| ComicVine | 1 req/s | Published: 200 req/15min |

Constructed at startup:
```rust
// MusicBrainz: 1 request per 1 second
let mb_queue = ProviderQueue::new(1, 1);

// AcoustID: 3 requests per 1 second
let acoustid_queue = ProviderQueue::new(3, 1);

// TMDB: 40 requests per 1 second
let tmdb_queue = ProviderQueue::new(40, 1);
```

---

## Agoge Task Queue Integration

Metadata resolution is NOT synchronous with import. Taxis imports the file (creates `haves` row and per-type table row), then dispatches metadata resolution to Agoge as a background task. Import does not block waiting for metadata.

### Agoge Task Types

| Task | Description | Priority |
|------|-------------|----------|
| `ResolveIdentity { media_type, media_id }` | Determine what the item is | High (import-triggered) |
| `EnrichMetadata { media_type, media_id }` | Fetch full metadata from canonical + enrichment providers | Normal (import-triggered) |
| `FingerprintTrack { track_id }` | AcoustID fingerprinting for music (runs in spawn_blocking) | Normal |
| `ComputeLoudness { track_id }` | EBU R128 for music ReplayGain (runs in spawn_blocking) | Low (can run after available) |

### Priority Levels

1. **Interactive** (priority 4) — user-triggered metadata lookup. Bypasses queue for immediate execution.
2. **Import-triggered** (priority 3) — dispatched by Taxis on import. High priority to move item to `available` quickly.
3. **Scheduled refresh** (priority 1) — periodic background re-enrichment. Runs during idle periods.

### Retry Policy

Failed tasks are retried with exponential backoff:
- 3 retry attempts maximum per task
- Backoff: 60s → 120s → 240s (configurable via `[epignosis] identity_retry_backoff_seconds`)
- After 3 failures: task marked failed, item status transitions to `failed`, WARN log emitted

Agoge ensures per-provider rate limits are respected even under bulk import load by routing all provider calls through Epignosis's `ProviderQueue`.

---

## Caching Strategy

All provider responses are cached in Epignosis's in-process cache. Cache is populated on first access (lazy — not warmed at startup).

Cache key format: `{provider}:{entity_type}:{external_id}`

Examples:
- `musicbrainz:release:8a88b4b5-0571-3940-b0c3-afd91cf71c2d`
- `tmdb:movie:550`
- `acoustid:fingerprint:AQADtNRyxckiRBUFAFECAA`

Per-provider TTLs:

| Provider | TTL | Rationale |
|----------|-----|-----------|
| MusicBrainz | 7 days | Release data changes rarely; track data is effectively immutable |
| AcoustID | Permanent | Fingerprint-to-MBID mapping never changes for a given fingerprint |
| TMDB | 30 days | Movie metadata stable after release; TV data refreshed more aggressively |
| TVDB | 7 days | Ongoing series need more frequent refresh for upcoming episodes |
| Audnexus | 30 days | Narrator and chapter data stable |
| OpenLibrary | 14 days | Book metadata stable; covers change occasionally |
| OpenSubtitles | 24 hours | Subtitle availability changes frequently |

Cache eviction: TTL-based expiry only. No explicit eviction on data mutation — provider data is read-only from Harmonia's perspective. A background task runs every `cache_cleanup_interval_hours` (default: 1) to evict expired entries.

---

## Provider-Specific API Patterns

### MusicBrainz

**Authentication:** None required. User-Agent header is mandatory: `Harmonia/{version} (contact-url)`. Missing or generic User-Agent → HTTP 403.

**Rate limit:** 1 req/s hard limit per IP. Violation returns HTTP 503. Risk of IP ban on sustained violation.

**Endpoint flow for full album lookup:**
1. Search: `GET /ws/2/release?query={title}&fmt=json`
2. Release detail: `GET /ws/2/release/{mbid}?inc=recordings+artist-credits+labels&fmt=json`
3. Recording detail: `GET /ws/2/recording/{mbid}?inc=artist-credits&fmt=json`

**Schema alignment with existing data model:**

| MusicBrainz Entity | Harmonia Table |
|-------------------|----------------|
| `release_groups` | `music_release_groups` (mb_release_group_id) |
| `releases` | `music_releases` (mb_release_id) |
| `mediums` | `music_media` (format column) |
| `recordings` | `music_tracks` (mb_recording_id) |

**Batch lookups:** The recordings endpoint accepts comma-separated MBIDs:
`GET /ws/2/recording?mbid={id1}&mbid={id2}&fmt=json`

Use batch where an album's tracks are all being enriched simultaneously — reduces API calls from N to 1.

**Recommended schema additions** (not in Phase 4 schema, needed for full MusicBrainz depth):
- `music_releases.source_type TEXT CHECK(source_type IN ('cd', 'vinyl', 'digital', 'web', 'sacd', 'dvd_audio', 'unknown'))` — populated from MusicBrainz `release.packaging` field; default `'unknown'`
- `music_tracks.acoustid_fingerprint TEXT` — stored fingerprint prevents re-computation on re-import

---

### AcoustID

**Authentication:** Client API key required. Register application at acoustid.org.

**Rate limit:** 3 req/s.

**Lookup flow on music import:**

1. Taxis imports file, creates `music_tracks` row
2. Taxis emits `ImportCompleted { media_type: Music, ... }` via Aggelia
3. Epignosis subscriber dispatches `FingerprintTrack` task to Agoge
4. `spawn_blocking`: decode first 120 seconds of audio via Symphonia → extract PCM frames
5. Feed PCM frames to `rusty-chromaprint::Fingerprinter` → get fingerprint string + duration
6. `POST https://api.acoustid.org/v2/lookup` with `fingerprint`, `duration`, `client` params → recording MBIDs with confidence scores
7. If MBIDs returned: verify against existing `mb_recording_id`; update if different; emit `MetadataEnriched`
8. Store fingerprint on `music_tracks.acoustid_fingerprint` — skips re-computation on future re-imports

**Fingerprint backend:** Two implementations behind a `FingerprintBackend` trait:

```rust
pub trait FingerprintBackend: Send + Sync {
    fn compute(&self, pcm: &[f32], sample_rate: u32, channels: u32, duration_secs: f64)
        -> Result<String, FingerprintError>;
}

pub struct RustyChromaprintBackend;   // preferred: pure Rust, no system dependency
pub struct FpcalcSubprocessBackend;   // fallback: requires chromaprint system package
```

At implementation time: validate fingerprint output against AcoustID test fixtures. If `rusty-chromaprint` output diverges from the C library (required for AcoustID lookup compatibility), use `FpcalcSubprocessBackend`. Config selects the backend: `epignosis.fingerprint_backend = "rusty_chromaprint" | "fpcalc"`.

---

### TMDB

**Authentication:** API key (v3 key). Passed as `api_key` query parameter.

**Rate limit:** Conservative 40 req/s budget. No hard published limit; respect 429 responses.

**Endpoints:**
- Movie search: `GET /3/search/movie?query={title}&api_key={key}`
- Movie detail: `GET /3/movie/{id}?api_key={key}`
- TV series: `GET /3/tv/{id}?api_key={key}`
- TV season: `GET /3/tv/{id}/season/{n}?api_key={key}`
- Image base URL: fetched from `GET /3/configuration?api_key={key}` at startup and cached

**Used for:** Movie metadata (canonical). TV enrichment (episode air dates, cast, overview) supplementing TVDB.

---

### TVDB v4

**Authentication:** JWT bearer token. Flow:
1. `POST /v4/login` with `{"apikey": "...", "pin": "..."}` → JWT token
2. Include `Authorization: Bearer {token}` on all subsequent requests
3. Detect `exp` claim; refresh before expiry

**Token refresh lock** — prevents concurrent refresh race:

```rust
// In Epignosis state
struct TvdbTokenState {
    token: Option<TvdbToken>,
    // Mutex is held across .await during refresh — use tokio::sync::Mutex
}
static TVDB_TOKEN: tokio::sync::Mutex<TvdbTokenState> = ...;

async fn get_tvdb_token(config: &TvdbConfig) -> Result<String, EpignosisError> {
    let mut state = TVDB_TOKEN.lock().await;
    if let Some(token) = &state.token {
        if !token.is_expired() {
            return Ok(token.value.clone());
        }
    }
    // First waiter refreshes; subsequent waiters block on the mutex and reuse
    let new_token = fetch_tvdb_token(config).await?;
    state.token = Some(new_token.clone());
    Ok(new_token.value)
}
```

**Rate limit:** Conservative 10 req/s. No published hard limit.

**Endpoints:**
- Series: `GET /v4/series/{id}`
- Episodes: `GET /v4/series/{id}/episodes/official?season={n}`

---

### OpenLibrary

**Authentication:** None required. User-Agent header required with application name and contact info.

**Rate limit:** Conservative 10 req/s. Respect 503 responses.

**Endpoints:**
- Search: `GET /search.json?q={query}` or `GET /search.json?isbn={isbn}`
- Work: `GET /works/{olid}.json` — abstract book concept
- Edition: `GET /editions/{olid}.json` — specific edition metadata

**Limitations:** Most complete free book metadata source; gaps in newer titles (post-2020). No narrator metadata — Audnexus is the authoritative source for audiobook narrator data.

---

### Audnexus

**Authentication:** None required. Community API at `https://api.audnex.us`. No SLA — design for graceful degradation.

**Rate limit:** Conservative 5 req/s. `retryAfterSeconds` field in error response when rate limited.

**Endpoints:**
- Audiobook metadata: `GET /books/{asin}` — narrator, series position, chapter data
- Chapter list: `GET /books/{asin}/chapters` — chapter titles and timestamps
- Author/narrator: `GET /authors/{asin}`

**ASIN resolution chain** — Audnexus requires an ASIN (Amazon Standard Identification Number):

1. If `audiobooks.asin` is already populated: use directly
2. Search Audnexus by title + author: `GET /books?title={t}&author={a}` — verify this endpoint at implementation time
3. Fall back to OpenLibrary for book identity (author, title, publication date). Chapter data only from Audnexus or embedded M4B markers.

For ambiguous matches (multiple candidates): mark item as `IdentityAmbiguous`, log at WARN. Item proceeds to `organized` without full Audnexus enrichment. User can resolve manually.

---

### ComicVine

**Authentication:** API key required. Passed as `api_key` query parameter.

**Rate limit:** 1 req/s conservative (published: 200 req/15min).

**Endpoints:**
- Search: `GET /api/search/?api_key={key}&query={title}&resources=issue&format=json`
- Issue detail: `GET /api/issue/4000-{id}/?api_key={key}&format=json`

---

### iTunes Podcast API

**Authentication:** None required.

**Rate limit:** No published limit; conservative 5 req/s.

**Search:** `GET https://itunes.apple.com/search?media=podcast&term={query}`

Used for podcast subscription metadata (title, artwork, description). Episode data comes from the RSS feed directly.

---

## Error Handling

`EpignosisError` enum using `snafu`:

```rust
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum EpignosisError {
    #[snafu(display("rate limit exceeded for {provider}, retry after {retry_after:?}"))]
    ProviderRateLimited {
        provider: String,
        retry_after: Option<Duration>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("request to {provider} timed out: {url}"))]
    ProviderTimeout {
        provider: String,
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("authentication failed for {provider}"))]
    ProviderAuthFailed {
        provider: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("no match found in {provider} for query: {query}"))]
    ProviderNotFound {
        provider: String,
        query: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to parse response from {provider}"))]
    ProviderParseError {
        provider: String,
        source: serde_json::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("audio fingerprint computation failed for {path:?}"))]
    FingerprintFailed {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("identity ambiguous in {provider}: {candidates} candidates found"))]
    IdentityAmbiguous {
        provider: String,
        candidates: usize,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
```

**Non-fatal errors** — item proceeds with partial metadata, WARN log:
- `FingerprintFailed` — AcoustID fingerprint failed. Item proceeds to `enriched` without fingerprint.
- Enrichment provider failure — canonical succeeded; enrichment failure is non-fatal.
- `ProviderNotFound` from enrichment provider (not canonical).

**Fatal errors** — item transitions to `failed`, Agoge retries:
- `ProviderNotFound` from canonical provider — identity cannot be resolved.
- `ProviderAuthFailed` — API key invalid or expired. Requires operator intervention.
- `ProviderRateLimited` from canonical — retry after backoff.

---

## Horismos Configuration — [epignosis] Section

```toml
[epignosis]
# Rate limit budgets (requests per window_seconds for each provider)
musicbrainz_rate_limit_requests = 1
musicbrainz_rate_limit_window_seconds = 1
acoustid_rate_limit_requests = 3
acoustid_rate_limit_window_seconds = 1
tmdb_rate_limit_requests = 40
tmdb_rate_limit_window_seconds = 1
tvdb_rate_limit_requests = 10
tvdb_rate_limit_window_seconds = 1
audnexus_rate_limit_requests = 5
audnexus_rate_limit_window_seconds = 1
openlibrary_rate_limit_requests = 10
openlibrary_rate_limit_window_seconds = 1
opensubtitles_rate_limit_requests = 1
opensubtitles_rate_limit_window_seconds = 1
comicvine_rate_limit_requests = 1
comicvine_rate_limit_window_seconds = 1

# Required: identifies Harmonia to MusicBrainz (format: AppName/Version contact)
musicbrainz_user_agent = "Harmonia/1.0 (https://github.com/user/harmonia)"

# Cache
cache_cleanup_interval_hours = 1    # evict expired entries periodically
cache_acoustid_permanent = true     # AcoustID fingerprint mappings never expire

# Retry
identity_retry_max = 3
identity_retry_backoff_seconds = 60

# Fingerprinting
fingerprint_backend = "rusty_chromaprint"  # "rusty_chromaprint" | "fpcalc"

# Base URLs (override for development/testing)
audnexus_base_url = "https://api.audnex.us"
```

Secrets in `secrets.toml` (gitignored):

```toml
[epignosis]
tmdb_api_key = "..."              # required for movies and TV enrichment
tvdb_api_key = "..."              # required for TV
tvdb_subscriber_pin = "..."       # required for TV (TVDB v4 auth)
acoustid_api_key = "..."          # required for music fingerprinting
comicvine_api_key = "..."         # required for comics
opensubtitles_api_key = "..."     # required for subtitles
```

Horismos validates at startup that required API keys are present when their corresponding media types are enabled. Missing `tmdb_api_key` with a movie library configured is a startup error.
