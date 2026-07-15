# Configuration architecture

> How Harmonia configuration is loaded, merged, validated, and distributed.
> Subsystem names used as config section keys match [subsystems.md](subsystems.md) and [lexicon.md](../lexicon.md).
> The Horismos crate that owns this logic is in [cargo.md](cargo.md).
> For what happens to a config change AFTER load — SIGHUP reload, the
> LIVE/RESTART/UNWIRED classification of every field, and rotation/drain
> semantics — see [config-reload.md](config-reload.md).

## Purpose

Horismos is the single source of truth for all system configuration. No other subsystem reads environment variables directly, parses files, or hardcodes thresholds. This document specifies how configuration is loaded (figment layered merge), how secrets are separated from committed config, how the typed `Config` struct is laid out, how subsystems receive their config slice, what Horismos validates at startup, and what patterns must never appear in subsystem code.

---

## Config file structure

`harmonia.toml`, path given via `--config` (default `harmonia.toml` in the working directory). TOML format. One `[subsystem]` section per subsystem that has configurable values. Committed to version control with safe defaults; no secrets, no credentials.

```toml
# harmonia.toml — committed, safe defaults only

[database]
db_path = "harmonia.db"
read_pool_size = 0                # 0 = auto-detect
write_pool_max = 1

[exousia]
# Auth — JWT TTL values only; the signing secret comes from secrets.toml or an env var
access_token_ttl_secs = 900       # 15 minutes
refresh_token_ttl_days = 30

[paroche]
# Media serving + renderer (KOSync/QUIC) registration
listen_addr = "0.0.0.0"
port = 8096
opds_page_size = 50
kosync_registration_enabled = true
renderer_max_connections = 32
renderer_session_init_timeout_secs = 10
renderer_quic_port = 4433
# renderer_api_key comes from secrets.toml — leaving it unset rejects every
# renderer registration (fail closed)

[taxis]
# Library scanning — libraries themselves are named sub-tables, one per library
watcher_debounce_ms = 500
scan_concurrency = 4

[taxis.libraries.music]
path = "/data/music"
media_type = "music"              # "music" | "video" | "book"
watcher_mode = "auto"              # "inotify" | "poll" | "auto"
poll_interval_seconds = 300
scan_interval_hours = 24

[epignosis]
# Metadata providers
cache_ttl_secs = 86400            # 24 hours
provider_timeout_secs = 10
fingerprint_accept_threshold = 0.8
fingerprint_ambiguous_threshold = 0.5
provider_response_max_bytes = 10485760  # cap on a buffered provider response
# acoustid_key / tmdb_key / tvdb_key / comicvine_key / google_books_key come
# from secrets.toml; an absent key degrades that provider (warning at
# startup), it does not fail validation — see Validation rules below

[kritike]
# Quality curation
quality_check_concurrency = 4

[aggelia]
# Internal broadcast event bus
buffer_size = 1024

[zetesis]
# Indexer search + Cloudflare bypass — see download/cloudflare.md
request_timeout_secs = 30
max_results_per_indexer = 100
max_response_body_bytes = 16777216   # 16 MiB cap on any single indexer response
cloudflare_bypass_enabled = false
max_concurrent_searches = 10
per_indexer_rate_limit_requests = 5
per_indexer_rate_limit_window_seconds = 10
caps_refresh_hours = 24
search_timeout_seconds = 30
# cf_proxy_url is REQUIRED when cloudflare_bypass_enabled = true (validation
# error otherwise); cardigann_definitions_dir is optional
cf_proxy_timeout_seconds = 60
cf_cookie_refresh_minutes = 30
result_cache_ttl_seconds = 1800      # how long a search's results stay enqueueable by release_id
result_cache_max_queries = 32        # oldest search evicted once exceeded

[ergasia]
# Torrent download execution (librqbit) — see download/torrent.md
download_dir = "/data/downloads"
session_state_path = "/data/downloads/.librqbit-state"
listen_port_range = [6881, 6889]
seed_ratio_threshold = 1.0
seed_time_threshold_hours = 72
peer_connect_timeout_seconds = 10
magnet_resolve_timeout_seconds = 120
max_extraction_depth = 3
max_decompression_ratio = 100.0

[syntaxis]
# Download queue and post-processing — see download/orchestration.md
max_concurrent_downloads = 5
max_per_tracker = 3
retry_count = 3
retry_backoff_base_seconds = 30
stalled_download_timeout_hours = 24

[prostheke]
# Subtitles
languages = ["en", "de"]
include_hearing_impaired = false
include_forced = true
min_match_score = 0.7
# prostheke.opensubtitles.api_key / username / password come from secrets.toml

[komide]
# Podcasts and news feeds
podcast_poll_interval_minutes = 30
news_poll_interval_minutes = 15
podcast_dir = "/data/podcasts"
news_retention_days = 30
news_retention_articles = 500
auto_download_latest_n = 3
fetch_timeout_secs = 30
max_feed_bytes = 20971520
max_episode_bytes = 1073741824
max_backoff_minutes = 240
jitter_percent = 10.0

[syndesmos]
# External integrations — Plex, Last.fm, Tidal are each optional sub-tables;
# omitting one disables that integration. Credentials live in secrets.toml.
circuit_break_minutes = 5
circuit_break_failure_threshold = 5

[aitesis]
# Request management
max_pending_per_user = 25
max_requests_per_day = 10
auto_approve_admins = true
```

---

## Secrets separation

`secrets.toml`, resolved as a sibling of the config file by default (same directory, filename `secrets.toml`) — or, if the `HARMONIA_SECRETS_PATH` environment variable is set, read from that path instead (`crates/horismos/src/secrets.rs`). The env-var form exists for deployments where the config file has no writable sibling directory, e.g. the NixOS module (`nix/module.nix`), which places `harmonia.toml` in the read-only Nix store and delivers the secrets file via systemd `LoadCredential` at a runtime-only path. Gitignored; never committed. Contains: the JWT signing secret, the renderer registration key, metadata-provider API keys, subtitle-provider credentials, and the Plex/Last.fm/Tidal integration credentials. Same TOML structure as `harmonia.toml`; figment merges them transparently with `secrets.toml` taking precedence over `harmonia.toml`.

```toml
# secrets.toml — gitignored, never committed

[exousia]
jwt_secret = "..."                 # Required: min 32 bytes of entropy

[paroche]
renderer_api_key = "..."           # Unset = every renderer registration is rejected (fail closed)

[epignosis]
acoustid_key = "..."               # Optional — absent runs fingerprinting unauthenticated
tmdb_key = "..."                   # Optional — absent disables movie/secondary-TV resolution
tvdb_key = "..."                   # Optional — absent disables TV metadata resolution
comicvine_key = "..."              # Optional — absent disables comic metadata resolution
google_books_key = "..."           # Optional — absent runs the Google Books fallback unauthenticated

[prostheke.opensubtitles]
api_key = "..."
username = "..."                   # Optional — enables the authenticated download quota
password = "..."

[syndesmos.plex]
url = "http://localhost:32400"
token = "..."

[syndesmos.lastfm]
api_key = "..."
shared_secret = "..."

[syndesmos.tidal]
access_token = "..."               # OAuth2 access token; refreshed automatically when expired
```

**CRITICAL: JWT secret validation.** The JWT secret must never come from `harmonia.toml` (committed). Horismos validates at startup that `exousia.jwt_secret` is not empty and not a placeholder value (`"changeme"` or `"default"`), and is at least 32 bytes. A failing check is a startup error — the process exits before serving any requests.

```rust
// crates/horismos/src/validation.rs
fn validate_jwt_secret(config: &Config) -> Result<(), HorismosError> {
    let secret = &config.exousia.jwt_secret;
    if secret.is_empty() || secret == "changeme" || secret == "default" {
        return ValidationSnafu {
            message: "exousia.jwt_secret must not be empty or a placeholder value — \
                       set via secrets.toml or HARMONIA__EXOUSIA__JWT_SECRET".to_string(),
        }
        .fail();
    }
    if secret.len() < 32 {
        return ValidationSnafu {
            message: format!(
                "exousia.jwt_secret is too short ({} bytes); minimum is 32 bytes",
                secret.len()
            ),
        }
        .fail();
    }
    Ok(())
}
```

Every validation failure — JWT secret included — returns the same `HorismosError::Validation { message }` variant; there is no per-check error type. See **Validation rules** below for the full check list.

---

## Figment layer order

figment merges providers in order, with **later providers taking precedence**:

| Order | Provider | Source | Notes |
|-------|----------|--------|-------|
| 1 (lowest) | `Serialized::defaults(Config::default())` | Compiled-in Rust defaults | Safe baseline; system must work with defaults alone (except secrets) |
| 2 | `Toml::file(config_path)` | The path given to `--config` (default `harmonia.toml`) | User config; committed, no secrets |
| 3 (highest) | `Toml::file(secrets_path(config_path))` | `HARMONIA_SECRETS_PATH` if set, else `secrets.toml` sibling of the config file | Secret overrides; gitignored, optional file |
| 4 | `Env::prefixed("HARMONIA__").split("__")` | Environment variables | Container/CI deployment overrides |

```rust
// crates/horismos/src/lib.rs
pub fn load_config(
    config_path: Option<&Path>,
) -> Result<(Config, Vec<ValidationWarning>), HorismosError> {
    let config_path = config_path.unwrap_or_else(|| Path::new("harmonia.toml"));

    let figment = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file(config_path))
        .merge(Toml::file(secrets_path(config_path)))
        .merge(Env::prefixed("HARMONIA__").split("__"));

    let config: Config = figment.extract().context(ConfigParseSnafu)?;
    let warnings = validate_config(&config)?;
    Ok((config, warnings))
}
```

No figment layer carries CLI arguments. `archon`'s `--listen`/`--port` flags are the one CLI-driven override: `run_serve` calls `load_config()`, then mutates `config.paroche.listen_addr`/`.port` directly from the parsed args (not through figment), and records the same two values in a `ConfigOverrides` struct so a later `SIGHUP` reload re-applies them — otherwise a reload would silently drop a `--port` override back to the on-disk value.

### Double-underscore separator: critical detail

figment's `Env::split("__")` uses double underscore as the nesting level separator. This is not optional; single underscore `split("_")` has a known ambiguity with snake_case field names.

**How it works:** figment replaces the split string with `.` to form a dotted key path. `HARMONIA__ZETESIS__REQUEST_TIMEOUT_SECS` becomes `zetesis.request_timeout_secs`, which maps to `Config.zetesis.request_timeout_secs`. The prefix `HARMONIA__` is stripped first, then each `__` becomes a nesting level.

| Env Var | Maps To | Why |
|---------|---------|-----|
| `HARMONIA__ZETESIS__REQUEST_TIMEOUT_SECS` | `[zetesis] request_timeout_secs` | `__` splits nesting levels: `zetesis` → `request_timeout_secs` |
| `HARMONIA__EXOUSIA__JWT_SECRET` | `[exousia] jwt_secret` | correct way to set the JWT secret in container environments |
| `HARMONIA__SYNTAXIS__MAX_CONCURRENT_DOWNLOADS` | `[syntaxis] max_concurrent_downloads` | underscores within a field name are preserved |
| `HARMONIA__PAROCHE__RENDERER_MAX_CONNECTIONS` | `[paroche] renderer_max_connections` | same; field name contains underscores, preserved |

**WRONG: why `split("_")` fails:**

`Env::prefixed("HARMONIA_").split("_")` would replace every `_` with `.`:
- `HARMONIA_JWT_SECRET` → `jwt.secret`: is this `[jwt] secret` (nested) or `jwt_secret` (flat field)? Ambiguous.
- `HARMONIA_MAX_CONCURRENT_DOWNLOADS` → `max.concurrent.downloads`: three nesting levels instead of one field name.

Always use `Env::prefixed("HARMONIA__").split("__")`, double underscore for both prefix and separator.

---

## Config struct layout

The top-level `Config` struct in `crates/horismos/src/config.rs`. One field per subsystem that has configurable values:

```rust
// crates/horismos/src/config.rs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)] pub database: DatabaseConfig,
    #[serde(default)] pub exousia: ExousiaConfig,
    #[serde(default)] pub paroche: ParocheConfig,
    #[serde(default)] pub taxis: TaxisConfig,
    #[serde(default)] pub epignosis: EpignosisConfig,
    #[serde(default)] pub kritike: KritikeConfig,
    #[serde(default)] pub aggelia: AggeliaConfig,
    #[serde(default)] pub zetesis: SearchSubsystemConfig,
    #[serde(default)] pub ergasia: ErgasiaConfig,
    #[serde(default)] pub syntaxis: SyntaxisConfig,
    #[serde(default)] pub prostheke: ProsthekeConfig,
    #[serde(default)] pub komide: KomideConfig,
    #[serde(default)] pub syndesmos: SyndesmosConfig,
    #[serde(default)] pub aitesis: AitesisConfig,
}
```

Note the TOML section key (`zetesis`) does not always match the Rust struct name (`SearchSubsystemConfig`) — the struct name reflects what it configures, the field name is the stable section key.

**Config structs are plain data.** No methods beyond `Default` (and a redacting `Debug` impl for any struct holding a secret). No logic. No external dependencies. They are deserialization targets; figment fills them; subsystems read them. `ExousiaConfig` is the smallest complete example of the pattern:

```rust
// crates/horismos/src/subsystems.rs
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExousiaConfig {
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_days: u64,
    pub jwt_secret: String,
}

impl Default for ExousiaConfig {
    fn default() -> Self {
        Self {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: String::new(), // intentionally invalid — validation rejects it
        }
    }
}
```

This document does not duplicate the full per-subsystem field list — that drifts. `crates/horismos/src/subsystems.rs` is the source of truth for every field, its type, and its default; `config-reload.md` is the source of truth for which fields are LIVE, RESTART-required, or UNWIRED.

Every subsystem config carries `#[serde(deny_unknown_fields)]`: a config file setting a field that does not exist (removed, renamed, or never real) fails to parse at startup rather than being silently ignored.

---

## Config distribution

archon calls `horismos::load_config()` once at startup, splits the resulting `Config` into per-subsystem slices, and constructs each subsystem. Fields classified LIVE (see [config-reload.md](config-reload.md)) are handed a `Section<T>`/`SectionWatcher<T>` off the shared `ConfigHandle` instead of a frozen clone, so a later `SIGHUP` reaches them without a restart; RESTART-class fields are read once at construction, matching the "held-back" contract.

```rust
// crates/archon/src/serve.rs (shape, not verbatim)
let (config, warnings) = horismos::load_config(Some(args.config.as_path()))?;
// ... apply --listen/--port CLI overrides directly on `config` ...
let (config_manager, config_handle) = ConfigManager::new(config.clone(), config_path, overrides);

let auth = ExousiaServiceImpl::new(db.clone(), config_handle.section(|c| &c.exousia));
```

**Runtime config reload exists (#529).** `SIGHUP` re-reads and applies config changes without a restart, for every field classified LIVE. A field classified RESTART-required is held back (the running process keeps its old effective value) and reported in `restart_pending` until the process restarts or the on-disk value reverts — nothing is silently dropped either way. See [config-reload.md](config-reload.md) for the mechanism and the full field classification.

---

## Validation rules

`validate_config(&Config) -> Result<Vec<ValidationWarning>, HorismosError>` (`crates/horismos/src/validation.rs`) runs after all providers are merged, against the final resolved values. It either returns `Err` (a hard startup failure) or `Ok(warnings)` — non-fatal degraded-capability notices logged and, for `restart_pending`-adjacent surfacing, echoed by the admin diagnostic endpoint.

**Hard errors (reject startup):**

| Check | Rule |
|---|---|
| `exousia.jwt_secret` | Not empty, not `"changeme"`/`"default"`, at least 32 bytes |
| `exousia.access_token_ttl_secs` | Greater than 0 |
| `exousia.refresh_token_ttl_days` | Between 1 and 36,500 (100 years — catches a typo'd unit, not a policy choice) |
| `paroche.port` / `paroche.renderer_quic_port` | Each ≥ 1024 (Harmonia must not run as root); the two must differ |
| `zetesis.request_timeout_secs` | Greater than 0 |
| `zetesis.max_response_body_bytes` | Greater than 0 |
| `zetesis.cf_proxy_url` | Required when `zetesis.cloudflare_bypass_enabled = true` |
| `epignosis.provider_timeout_secs` | Greater than 0 |
| `prostheke.min_match_score` | Between 0.0 and 1.0 |
| `taxis.scan_concurrency` | Greater than 0 (sizes a semaphore; 0 permits blocks the first scan forever) |
| `kritike.quality_check_concurrency` | Greater than 0 |
| `database.write_pool_max` | Greater than 0 |

**Warnings (logged, never fatal):**

| Check | Behavior |
|---|---|
| `epignosis.{acoustid,tmdb,tvdb,comicvine,google_books}_key` | Absent → warning naming the degraded capability (keyless operation is a legitimate posture). An explicitly-set placeholder (empty string, `"changeme"`, `"default"`) is still a hard error — that can only come from a botched edit, never from omitting the field. |
| `taxis.libraries.<name>.path` | Not accessible at startup → warning per library. Not a startup error — a library can be created or remounted after Harmonia starts. |

No credential-presence check gates `syndesmos.plex`/`.lastfm`/`.tidal`: each is an `Option<...>`, and simply being absent (`None`) disables that integration. No separate "feature flag" layer gates them.

---

## Anti-patterns

**No subsystem reads environment variables directly.** Only Horismos calls figment. If a subsystem needs a value from an env var, that env var must be declared in the figment `Env` provider (with the `HARMONIA__` prefix), reflected in the TOML schema, and surfaced via the `SubsystemConfig` struct. Bypassing Horismos to call `std::env::var()` directly creates an undocumented, untested configuration path.

**No hardcoded values that should be configurable.** File paths, timeouts, buffer sizes, API endpoints, retry limits; if the value might need to differ between development, staging, and production, it belongs in the config struct. The test for "should this be configurable" is: "would a user ever need to change this?" Timeouts, limits, paths, and URLs always qualify.

**No direct config mutation.** Subsystems never hold `&mut SubsystemConfig` or write to a config struct in place. A LIVE field changes only by publishing a whole new validated `Config` through `ConfigManager` (`SIGHUP` or `.replace()`) — see [config-reload.md](config-reload.md). Dynamic values that change at runtime (download progress, queue depth, session state) belong in subsystem-internal state, not in config.

**No full Config passed to subsystems.** Each subsystem receives only its own config slice (`Section<T>` for LIVE fields, a frozen clone for RESTART-class ones). Passing the full `Config` to subsystems couples every subsystem's constructor to the full config shape; adding a field to any subsystem's config would recompile all of them. The slice boundary also prevents a subsystem from accidentally reading another subsystem's credentials.

**No secrets in `harmonia.toml`.** The file is committed. Any value that provides access to external services (API keys, JWT signing secrets, database credentials) must live in `secrets.toml` (gitignored) or in a `HARMONIA__{SUBSYSTEM}__{KEY}` environment variable. The `harmonia.toml` file should be safe to publish; containing it in a public repository must not compromise the running system.

**Config structs are never hand-copied into other docs.** A doc that needs to reference a field cites `crates/horismos/src/subsystems.rs` (schema) and [config-reload.md](config-reload.md) (LIVE/RESTART/UNWIRED classification) rather than re-listing the struct — a copy drifts the moment the struct changes.
