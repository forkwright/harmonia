# Config reload (#529)

> How a running Harmonia process picks up a config change without a restart.
> Companion to [configuration.md](configuration.md) (load/merge/validate) and
> [subsystems.md](subsystems.md) (which crate owns which section).
> The Horismos crate that owns this logic is in [cargo.md](cargo.md).

## Purpose

`SIGHUP` re-reads `harmonia.toml` (+ `secrets.toml` + `HARMONIA__` env) and
applies the delta to the running process. Every config leaf falls into
exactly one of three classes:

- **LIVE** — takes effect on the next operation, with no restart. Some LIVE
  fields are read per-op straight from the live config (`Section::get()`);
  others are LIVE via a swap or a teardown+rebuild supervisor watching the
  section for changes.
- **RESTART-required** — held back: the running process keeps the OLD
  effective value, and the field is reported in `restart_pending` until the
  process restarts or the on-disk value reverts. Nothing is silently dropped.
- **UNWIRED** — parsed and validated, but no code reads it to change
  behavior. Tracked as tech debt ([#575](https://github.com/forkwright/harmonia/issues/575)),
  not part of the reactive-config contract.

`crates/horismos/src/diff.rs` is the single source of truth for this
classification (`LIVE`, `RESTART_REQUIRED`, `UNWIRED` — the last two are
crate-private/pub respectively; `LIVE` and `UNWIRED` are re-exported from
`horismos`). A test in that file (`every_leaf_is_classified_exactly_once`)
walks every leaf of a fully-populated `Config` and fails the build if a leaf
is in zero or more-than-one of the three lists — adding a new config field
without classifying it is a test failure, not a silent gap.

---

## Mechanism

| Type | Where | Role |
|---|---|---|
| `ConfigManager` | `handle.rs` | Owner side (archon holds this). `reload()` re-reads from disk; `replace()` publishes a config programmatically (the test lever). Both funnel through `publish()`. |
| `ConfigHandle` | `handle.rs` | Subscriber side, cheaply `Clone`. `.current()` returns an owned `Arc<Config>` snapshot; `.section(fn)` returns a typed `Section<T>`; `.watch_section(fn)` returns a `SectionWatcher<T>` for supervisor loops; `.restart_pending()` returns the held-back dotted paths. |
| `Section<T>` | `handle.rs` | Typed live sub-view. `.get()` returns an OWNED clone — the idiom is ONE `get()` per operation (a mid-operation second read would risk a torn config: e.g. minting a JWT with the new secret but the old TTL). |
| `SectionWatcher<T>` | `handle.rs` | `.changed().await` yields only when its projected section differs from the last-seen value; returns `None` once the `ConfigManager` is dropped (a supervisor's exit signal). |
| `ReloadOutcome` | `handle.rs` | `{ warnings, applied, restart_pending }` — dotted leaf paths, not values (values are never logged for secrets like `jwt_secret`). |

`publish()` (`handle.rs`) is the only place a reload actually happens:

1. Diff the current effective config against the newly-parsed one
   (`diff_config`, `crates/horismos/src/diff.rs`) — produces dotted leaf
   paths (`taxis.libraries.<key>.path`, arrays are atomic leaves).
2. Every changed leaf whose path starts with a `RESTART_REQUIRED` prefix goes
   to `restart_pending`; everything else goes to `applied`.
3. If anything is `applied`, `held_back_merge` builds the new EFFECTIVE
   config — restart-class leaves keep their OLD value, everything else takes
   the new value — and broadcasts it over the `watch` channel.
4. `restart_pending` is derived fresh on every publish (not accumulated), so
   reverting the on-disk value clears it automatically.

Archon's SIGHUP handler (`crates/archon/src/serve.rs`) calls
`reload_config()` (extracted so tests can drive a reload without a real
signal) and logs `ReloadOutcome` honestly: applied paths at `info!`,
restart-pending paths at `warn!`. The admin diagnostic endpoint
`GET /api/system/config` (`crates/paroche/src/routes/system.rs`) echoes
`restart_pending` so an operator can see a held-back change without reading
logs.

---

## Field classification

Grouped by config section. "Mechanism" names how a LIVE field actually takes
effect; REBUILD means a supervisor tears the subsystem down and rebuilds it
from the new section on change (`SectionWatcher::changed()`).

### exousia — LIVE (per-op `Section`, step 3)

| Field | Mechanism |
|---|---|
| `access_token_ttl_secs` | per-op read at mint; mint-forward only (already-issued expirations honored) |
| `refresh_token_ttl_days` | per-op read at mint |
| `jwt_secret` | per-op read at verify — see **JWT rotation** below |

### paroche — LIVE (steps 2, 4, 5)

| Field | Class | Mechanism |
|---|---|---|
| `listen_addr` | LIVE | HTTP dual-listener rebind (step 5) |
| `port` | LIVE | HTTP dual-listener rebind (step 5) |
| `opds_page_size` | LIVE | per-op read off `ConfigHandle` |
| `renderer_api_key` | LIVE | per-connection section read at SessionInit; rotation affects new registrations only |
| `kosync_registration_enabled` | LIVE | per-request read |
| `renderer_max_connections` | LIVE | `themelion::LiveGate` admission cap |
| `renderer_session_init_timeout_secs` | LIVE | per-connection section read |
| `renderer_quic_port` | LIVE | QUIC dual-endpoint rebind (step 5) |

### taxis — LIVE (scanner REBUILD, step 6)

| Field | Class |
|---|---|
| `libraries.<key>.path` / `.media_type` / `.watcher_mode` / `.poll_interval_seconds` / `.scan_interval_hours` | LIVE |
| `watcher_debounce_ms` | LIVE |
| `scan_concurrency` | LIVE |

### epignosis — LIVE (resolver REBUILD, step 8)

| Field | Class | Mechanism |
|---|---|---|
| `cache_ttl_secs` / `provider_timeout_secs` / `provider_response_max_bytes` | LIVE | metadata cache RESETS on rebuild (logged) |
| `acoustid_key` / `tmdb_key` / `tvdb_key` / `comicvine_key` / `google_books_key` | LIVE | #578 — resolver rebuild re-derives `ProviderCredentials::from(&new_cfg)`; key rotation is live for free |
| `fingerprint_accept_threshold` / `fingerprint_ambiguous_threshold` | LIVE | #575 — `merge_lookup_matches` classifies every fingerprint lookup against both thresholds (accepted / ambiguous-held / dropped) |

### kritike

| Field | Class | Mechanism |
|---|---|---|
| `quality_check_concurrency` | LIVE | `themelion::LiveGate` (step 8) |

### aggelia

| Field | Class |
|---|---|
| `buffer_size` | RESTART — broadcast channel capacity fixed at creation |

### zetesis — LIVE (steps 7) except three UNWIRED

| Field | Class | Mechanism |
|---|---|---|
| `request_timeout_secs` / `max_response_body_bytes` / `max_concurrent_searches` / `search_timeout_seconds` | LIVE | per-op reads |
| `cloudflare_bypass_enabled` / `cf_proxy_url` / `cf_proxy_timeout_seconds` | LIVE | cf-proxy rebuild + swap |
| `per_indexer_rate_limit_requests` / `per_indexer_rate_limit_window_seconds` | LIVE | `RateLimiter::reconfigure` — preserves in-flight embargoes |
| `cardigann_definitions_dir` | LIVE | registry reload + swap |
| `max_results_per_indexer` / `caps_refresh_hours` / `cf_cookie_refresh_minutes` | UNWIRED | #575 |

### ergasia — mostly RESTART/UNWIRED; two LIVE

| Field | Class | Why |
|---|---|---|
| `download_dir` / `session_state_path` / `listen_port_range` / `peer_connect_timeout_seconds` | RESTART | frozen into the librqbit session at build |
| `seed_ratio_threshold` / `seed_time_threshold_hours` | RESTART | frozen into librqbit's `SeedingPolicy`, no reconfigure API (reclassified in step 7 — previously a silent no-op) |
| `max_extraction_depth` / `max_decompression_ratio` | LIVE | `SessionEngine` rebuilds `ExtractionLimits` per extract call |
| `magnet_resolve_timeout_seconds` | UNWIRED | #575 |

### syntaxis — LIVE (step 7) except one UNWIRED

| Field | Class |
|---|---|
| `max_concurrent_downloads` / `max_per_tracker` / `retry_count` / `retry_backoff_base_seconds` | LIVE — `DownloadQueue::update_config` + `SlotAllocator::set_limits` |
| `stalled_download_timeout_hours` | UNWIRED — #575 |

### prostheke — LIVE (step 8) except two UNWIRED

| Field | Class | Mechanism |
|---|---|---|
| `languages` / `include_hearing_impaired` / `include_forced` / `min_match_score` | LIVE | per-op `Section` read |
| `opensubtitles.api_key` / `.rate_limit_per_second` / `.max_download_bytes` | LIVE | provider rebuild + `set_providers` — rate limiter state RESETS with the provider (logged) |
| `opensubtitles.username` / `.password` | UNWIRED | #575 — no login flow exists |

### komide — LIVE (feed scheduler REBUILD, step 6)

| Field | Class |
|---|---|
| `podcast_poll_interval_minutes`, `news_poll_interval_minutes`, `news_retention_days`, `news_retention_articles`, `auto_download_latest_n`, `fetch_timeout_secs`, `max_feed_bytes`, `max_backoff_minutes`, `jitter_percent` | LIVE |
| `podcast_dir` / `max_episode_bytes` | LIVE — episode auto-download (after subscribe/refresh) and the `POST /api/podcasts/episodes/{id}/download` route read them FROM the service's config, which the supervisor rebuilds on any `komide.*` change |

The feed supervisor has a second, non-config rebuild trigger: a
`FeedSetChanged` bus event (runtime subscribe/unsubscribe/activation flip)
re-enumerates the poll tasks after a short coalescing window, REUSING the
existing `FeedSchedulerService` — the etag/last-modified cache and reqwest
client survive, and no config reload is involved (#577).

### syndesmos — LIVE (client REBUILD, step 8)

| Field | Class | Notes |
|---|---|---|
| `plex.url` / `.token` / `.library_sections` | LIVE | rebuild + swap |
| `lastfm.api_key` / `.shared_secret` / `.session_key` | LIVE | rebuild + swap |
| `tidal.access_token` | LIVE | the only Tidal field a real client reads |
| `circuit_break_minutes` | LIVE | feeds `CircuitBreaker` cooldown |
| `circuit_break_failure_threshold` | LIVE | reaches the rebuild supervisor like every other `syndesmos.*` leaf, but `ScrobbleClientBuilder::build()` currently hardcodes the breaker threshold to 5 — a within-crate wiring gap tracked separately, NOT part of #575's dead-config list |

Rebuilding the client on ANY `syndesmos.*` change costs two honest things,
both logged: circuit breakers reset (fresh breakers, no carried trip state),
and events published between handler-cancel and re-subscribe are lost (a
bounded scrobble-loss window — broadcast receivers only see
post-subscription events).

### aitesis — LIVE (step 8)

| Field | Class |
|---|---|
| `max_pending_per_user` / `max_requests_per_day` / `auto_approve_admins` | LIVE — per-op `Section` read |

### database — RESTART (all three fields)

| Field | Why |
|---|---|
| `db_path` / `read_pool_size` / `write_pool_max` | sqlx pool sizing is fixed at connect (no resize API); every service holds a frozen `SqlitePool` clone |

---

## Special semantics

### JWT immediate rotation (exousia)

Rotating `exousia.jwt_secret` (edit + `SIGHUP`) takes effect at the next
`validate_bearer` call: every in-flight access token signed with the OLD
secret fails signature verification **immediately** → HTTP 401
(`UNAUTHORIZED`), not `TOKEN_EXPIRED`. This is deliberate — rotating a
compromised secret must kill outstanding bearers at once; there is no
dual-secret grace window.

Refresh tokens are opaque, sha256-hashed, DB-stored values — NOT signed with
`jwt_secret` — so a session survives rotation: a client that refreshes on any
401 (not just `TOKEN_EXPIRED`) recovers a token pair verifiable under the NEW
secret, without re-login. TTL changes are mint-forward only: an
`access_token_ttl_secs`/`refresh_token_ttl_days` change affects tokens minted
AFTER the reload; already-issued expirations are honored as-is.

An invalid rotation (secret too short or a placeholder value) is rejected
atomically at validation — the OLD secret stays in force, and the process
never observes the bad value.

### QUIC / HTTP dual-endpoint unbounded drain (paroche listener rebind, step 5)

A change to `(listen_addr, port)` or `(listen_addr, renderer_quic_port)`
triggers **make-before-break**: bind the NEW endpoint/listener first, then
retire the OLD generation's ACCEPT loop only (new connections stop landing
there; already-connected sessions are untouched). The retiring generation
drains via `wait_idle()` (QUIC) or axum graceful shutdown (HTTP) with **no
bound** — a deliberate operator-locked posture: a wedged renderer or immortal
WebSocket keeps an old generation alive rather than being force-closed
mid-session. A `DRAIN_HEARTBEAT` (`render/server.rs`) logs `info!` every 30s
for a still-draining generation so a stuck drain stays visible instead of
silent. If the new bind fails (e.g. an address conflict), the supervisor
falls back to break-before-make with bounded retries and a rollback attempt
to the previous address — every stage logged at `error!`; the server keeps
running regardless (a failed rebind never crashes the process).

### Rebuild-class resets (steps 6-8)

A REBUILD-class subsystem (scanner, feed scheduler, epignosis resolver,
syndesmos client, prostheke's OpenSubtitles provider) is torn down and
reconstructed from the new config on any change to its section. Costs are
honest and logged, never silent:

- **epignosis**: the metadata cache resets (`"resolver rebuilt, metadata
  cache reset"`) — a rebuild is stateless-cheap except for this cache.
- **prostheke**: the OpenSubtitles rate limiter resets with the provider
  (politeness limiter, not an embargo — acceptable).
- **syndesmos**: circuit breakers reset (fresh breakers) and there is a
  bounded event-loss window between handler cancel and re-subscribe.
- **zetesis** is the one exception: `RateLimiter::reconfigure` explicitly
  PRESERVES in-flight embargo/accrual state across a reload — an embargoed
  indexer must never un-embargo just because a reload happened.

---

## Completeness test

`crates/horismos/src/diff.rs::every_leaf_is_classified_exactly_once` walks
every leaf of an exemplar `Config` (every `Option<Struct>` subsystem
populated with `Some(..)` so mixed LIVE/UNWIRED subtrees like
`prostheke.opensubtitles` expose per-field granularity instead of collapsing
to one leaf; `taxis.libraries` gets one synthetic entry canonicalized to a
`taxis.libraries.*.<field>` path so the leaf set is deterministic regardless
of the real library key) and asserts:

1. Every leaf is in exactly one of `LIVE`, `RESTART_REQUIRED`, `UNWIRED`.
2. `LIVE ∪ RESTART_REQUIRED ∪ UNWIRED` exactly equals the full leaf set (no
   ghost entries in either direction).

Adding a new config field without adding it to one of the three lists fails
this test — "silently unobserved config" is a build failure, not a
production incident waiting to be discovered.
