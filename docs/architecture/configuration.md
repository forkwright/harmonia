# Configuration architecture

> How Harmonia configuration is loaded, merged, validated, and distributed.
> Subsystem names used as config section keys match [subsystems.md](subsystems.md) and [lexicon.md](../lexicon.md).
> The Horismos crate that owns this logic is in [cargo.md](cargo.md).

## Purpose

Horismos is the single source of truth for all system configuration. No other subsystem reads environment variables directly, parses files, or hardcodes thresholds. This document specifies how configuration is loaded (figment layered merge), how secrets are separated from committed config, how the typed `Config` struct is laid out, how subsystems receive their config slice, what Horismos validates at startup, and what patterns must never appear in subsystem code.

---

## Config file structure

`harmonia.toml` at the workspace root (`harmonia/`). TOML format. One `[subsystem]` section per subsystem that has configurable values. Committed to version control with safe defaults; no secrets, no credentials.

```toml
# harmonia.toml — committed, safe defaults only

[zetesis]
# Indexer search
request_timeout_secs = 30
max_results_per_indexer = 100
cloudflare_bypass_enabled = false

[exousia]
# Auth — JWT TTL values only; secrets come from secrets.toml or env vars
access_token_ttl_secs = 900       # 15 minutes
refresh_token_ttl_days = 30

[paroche]
# Media serving
stream_buffer_kb = 256
transcode_concurrency = 2
opds_page_size = 50

[ergasia]
# Download execution
download_dir = "/data/downloads"
max_concurrent_downloads = 3
seeding_ratio_limit = 2.0
seeding_time_limit_hours = 168    # 7 days

[aggelia]
# Internal event bus
buffer_size = 1024
download_queue_size = 512

[zetesis.indexers]
# Indexer credentials live in secrets.toml, not here
# This section contains protocol-level config only
torznab_timeout_secs = 15

[epignosis]
# Metadata providers
cache_ttl_secs = 86400            # 24 hours
max_provider_retries = 3
provider_timeout_secs = 10

[kritike]
# Quality curation
scan_interval_hours = 24
quality_check_concurrency = 4

[prostheke]
# Subtitles
languages = ["en", "de"]
hearing_impaired = false
provider_timeout_secs = 15

[taxis]
# Library import and organization
library_music_root = "/data/music"
library_video_root = "/data/video"
library_books_root = "/data/books"
file_naming_dry_run = false

[syntaxis]
# Queue management
max_queue_size = 1000
priority_strategy = "fifo"

[episkope]
# Monitoring
check_interval_mins = 60
search_concurrency = 2

[aitesis]
# Request management
max_requests_per_user = 10
request_auto_approve = false
```

---

## Secrets separation

`secrets.toml` at the workspace root (`harmonia/`). Gitignored; never committed. Contains: JWT signing secret, API keys for external services (Plex, Last.fm, Tidal), indexer credentials. Same TOML structure as `harmonia.toml`; figment merges them transparently with `secrets.toml` taking precedence over `harmonia.toml`.

```toml
# secrets.toml — gitignored, never committed

[exousia]
jwt_secret = "..."                # Required: min 32 bytes of entropy

[syndesmos]
plex_token = "..."                # Required when plex feature is enabled
lastfm_api_key = "..."            # Required when lastfm feature is enabled
lastfm_api_secret = "..."
tidal_client_id = "..."           # Required when tidal feature is enabled
tidal_client_secret = "..."

[zetesis.indexers.prowlarr]
api_key = "..."

[zetesis.indexers.jackett]
api_key = "..."
```

**CRITICAL: JWT secret validation.** The JWT secret must never come from `harmonia.toml` (committed). Horismos validates at startup that `exousia.jwt_secret` is not the compiled-in default value (`""` empty string or any placeholder like `"changeme"`). If the secret is the default, Horismos returns an error and the process exits before serving any requests. This prevents accidentally running with a known signing key.

```rust
// In Horismos validation — called during Config::load()
fn validate_jwt_secret(config: &Config) -> Result<(), HorisomosError> {
    let secret = &config.exousia.jwt_secret;
    if secret.is_empty() || secret == "changeme" || secret == "default" {
        return Err(HorisomosError::InsecureDefault {
            field: "exousia.jwt_secret".to_string(),
            hint: "set via secrets.toml or HARMONIA__EXOUSIA__JWT_SECRET env var".to_string(),
        });
    }
    if secret.len() < 32 {
        return Err(HorisomosError::SecretTooShort {
            field: "exousia.jwt_secret".to_string(),
            minimum_bytes: 32,
            actual_bytes: secret.len(),
        });
    }
    Ok(())
}
```

---

## Figment layer order

figment merges providers in order, with **later providers taking precedence**. The complete merge sequence:

| Order | Provider | Source | Notes |
|-------|----------|--------|-------|
| 1 (lowest) | `Serialized::defaults(Config::default())` | Compiled-in Rust defaults | Safe baseline; system must work with defaults alone (except secrets) |
| 2 | `Toml::file("harmonia.toml")` | `harmonia/harmonia.toml` | User config; committed, no secrets |
| 3 | `Toml::file("secrets.toml")` | `harmonia/secrets.toml` | Secret overrides; gitignored, optional file |
| 4 | `Env::prefixed("HARMONIA__").split("__")` | Environment variables | Container/CI deployment overrides |
| 5 (highest) | `Serialized` of CLI args | `archon` startup args | Explicit runtime overrides |

```rust
// In crates/horismos/src/lib.rs
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};

impl Config {
    pub fn load() -> Result<Self, HorisomosError> {
        let config: Config = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file("harmonia.toml"))
            .merge(Toml::file("secrets.toml"))  // optional — figment silently ignores missing files
            .merge(Env::prefixed("HARMONIA__").split("__"))
            .extract()
            .context(ConfigExtractSnafu)?;

        config.validate()?;
        Ok(config)
    }
}
```

### Double-underscore separator: critical detail

figment's `Env::split("__")` uses double underscore as the nesting level separator. This is not optional; single underscore `split("_")` has a known ambiguity with snake_case field names (figment issue #12).

**How it works:** figment replaces the split string with `.` to form a dotted key path. `HARMONIA__ZETESIS__TIMEOUT` becomes `zetesis.timeout`, which maps to `Config.zetesis.timeout`. The prefix `HARMONIA__` is stripped first, then each `__` becomes a nesting level.

| Env Var | Maps To | Why |
|---------|---------|-----|
| `HARMONIA__ZETESIS__TIMEOUT` | `[zetesis] timeout` | `__` splits nesting levels: `zetesis` → `timeout` |
| `HARMONIA__EXOUSIA__SECRET_KEY` | `[exousia] secret_key` | single `_` in `secret_key` is preserved as part of the field name |
| `HARMONIA__ERGASIA__MAX_CONCURRENT_DOWNLOADS` | `[ergasia] max_concurrent_downloads` | underscores within a key name are preserved |
| `HARMONIA__PAROCHE__STREAM_BUFFER_KB` | `[paroche] stream_buffer_kb` | same; field name contains underscores, preserved |
| `HARMONIA__EXOUSIA__JWT_SECRET` | `[exousia] jwt_secret` | correct way to set JWT secret in container environments |

**WRONG: why `split("_")` fails:**

`Env::prefixed("HARMONIA_").split("_")` would replace every `_` with `.`:
- `HARMONIA_SECRET_KEY` → `secret.key`: is this `[secret] key` (nested) or `secret_key` (flat field)? Ambiguous.
- `HARMONIA_MAX_CONCURRENT_DOWNLOADS` → `max.concurrent.downloads`: three nesting levels instead of one field name.

Always use `Env::prefixed("HARMONIA__").split("__")`, double underscore for both prefix and separator.

---

## Config struct layout

The top-level `Config` struct in `crates/horismos/`. One field per subsystem that has configurable values.

```rust
// crates/horismos/src/config.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub zetesis: ZetesisConfig,
    pub exousia: ExousiaConfig,
    pub paroche: ParocheConfig,
    pub ergasia: ErgasiaConfig,
    pub syntaxis: SyntaxisConfig,
    pub taxis: TaxisConfig,
    pub kritike: KritikeConfig,
    pub prostheke: ProsthekeConfig,
    pub epignosis: EpignosisConfig,
    pub episkope: EpiskopConfig,
    pub aitesis: AitesisConfig,
    pub syndesmos: SyndesmosConfig,
    pub aggelia: AggeliaConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            zetesis: ZetesisConfig::default(),
            exousia: ExousiaConfig::default(),
            paroche: ParocheConfig::default(),
            ergasia: ErgasiaConfig::default(),
            syntaxis: SyntaxisConfig::default(),
            taxis: TaxisConfig::default(),
            kritike: KritikeConfig::default(),
            prostheke: ProsthekeConfig::default(),
            epignosis: EpignosisConfig::default(),
            episkope: EpiskopConfig::default(),
            aitesis: AitesisConfig::default(),
            syndesmos: SyndesmosConfig::default(),
            aggelia: AggeliaConfig::default(),
        }
    }
}

// Each SubsystemConfig is a plain data struct — no logic, no external deps
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExousiaConfig {
    pub access_token_ttl_secs: u64,    // default: 900 (15 min)
    pub refresh_token_ttl_days: u64,   // default: 30
    pub jwt_secret: String,            // MUST NOT be empty or default at runtime
}

impl Default for ExousiaConfig {
    fn default() -> Self {
        Self {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: String::new(),  // intentionally invalid default — validation rejects it
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZetesisConfig {
    pub request_timeout_secs: u64,         // default: 30
    pub max_results_per_indexer: usize,    // default: 100
    pub cloudflare_bypass_enabled: bool,   // default: false
}

impl Default for ZetesisConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: 30,
            max_results_per_indexer: 100,
            cloudflare_bypass_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErgasiaConfig {
    pub download_dir: PathBuf,
    pub max_concurrent_downloads: usize,
    pub seeding_ratio_limit: f64,
    pub seeding_time_limit_hours: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaxisConfig {
    pub library_music_root: PathBuf,
    pub library_video_root: PathBuf,
    pub library_books_root: PathBuf,
    pub file_naming_dry_run: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AggeliaConfig {
    pub buffer_size: usize,          // default: 1024 — broadcast channel buffer
    pub download_queue_size: usize,  // default: 512 — mpsc channel for download queue
}

impl Default for AggeliaConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1024,
            download_queue_size: 512,
        }
    }
}

// ... one struct per subsystem following the same pattern
```

**Config structs are plain data.** No methods beyond `Default`. No logic. No external dependencies. They are deserialization targets; figment fills them; subsystems read them.

---

## Config distribution

archon calls `Config::load()` once at startup. The resulting `Config` is split into per-subsystem `Arc<SubsystemConfig>` references and passed to each subsystem's constructor.

```rust
// In archon main()
let config = Config::load().expect("configuration load failed");

// Each subsystem receives only its own config slice — not the full Config.
// Arc<_> avoids copying the config data; subsystems store the Arc and clone it as needed.
let exousia = Exousia::new(Arc::new(config.exousia.clone()), /* other deps */);
let zetesis  = Zetesis::new(Arc::new(config.zetesis.clone()), /* other deps */);
let kathodos = Kathodos::new(Arc::new(config.taxis.clone()),  /* other deps */);
// ...
```

**Subsystem storage pattern:**

```rust
// In crates/zetesis/src/lib.rs
pub struct ZetesisService {
    config: Arc<ZetesisConfig>,
    // ... other fields
}

impl ZetesisService {
    pub fn new(config: Arc<ZetesisConfig>, /* ... */) -> Self {
        Self { config, /* ... */ }
    }
}
```

**No runtime config reload in v1.** Configuration is read once at startup. To apply config changes, restart the process. This is a deliberate simplification; runtime config reload requires distributed consensus across subsystems and adds substantial complexity for minimal benefit in a single-binary system. Document restart as the intended config update path for v1.

---

## Validation rules

Horismos validates the merged config before returning it. Validation runs after all providers are merged, against the final resolved values.

```rust
impl Config {
    fn validate(&self) -> Result<(), HorisomosError> {
        // 1. JWT secret must not be the default value
        self.validate_jwt_secret()?;

        // 2. Library root paths must exist and be writable
        self.validate_library_paths()?;

        // 3. Port numbers must be in valid range
        self.validate_ports()?;

        // 4. Required API keys must be present when their feature flag is enabled
        self.validate_feature_keys()?;

        Ok(())
    }
}
```

**Rule 1, JWT secret:** `exousia.jwt_secret` must not be empty, `"changeme"`, or any compiled-in placeholder. Must be at least 32 bytes. Enforced regardless of feature flags; JWT auth is always active.

**Rule 2, library paths:** `taxis.library_music_root`, `taxis.library_video_root`, `taxis.library_books_root` must exist as directories and be writable by the Harmonia process. A missing or read-only library root is a startup error; the system cannot import media without it. Ergasia's `download_dir` is validated the same way.

**Rule 3, port ranges:** Any configured port number must be in the range `1024..=65535`. Ports below 1024 require elevated privileges; Harmonia must not run as root. If a port falls below 1024, Horismos returns a config error that names the field.

**Rule 4, feature key presence:** When a feature flag is enabled, its required credentials must be present:
- `plex` feature → `syndesmos.plex_token` must not be empty
- `lastfm` feature → `syndesmos.lastfm_api_key` and `syndesmos.lastfm_api_secret` must not be empty
- `tidal` feature → `syndesmos.tidal_client_id` and `syndesmos.tidal_client_secret` must not be empty

Feature flags that are not enabled skip their credential validation; a system running without Tidal need not supply Tidal credentials.

---

## Anti-patterns

**No subsystem reads environment variables directly.** Only Horismos calls figment. If a subsystem needs a value from an env var, that env var must be declared in the figment Env provider (with the `HARMONIA__` prefix), reflected in the TOML schema, and surfaced via the `SubsystemConfig` struct. Bypassing Horismos to call `std::env::var()` directly creates an undocumented, untested configuration path.

**No hardcoded values that should be configurable.** File paths, timeouts, buffer sizes, API endpoints, retry limits; if the value might need to differ between development, staging, and production, it belongs in the config struct. The test for "should this be configurable" is: "would a user ever need to change this?" Timeouts, limits, paths, and URLs always qualify.

**No config mutation after startup.** Config structs are read-only after `Config::load()` returns. Subsystems must not hold `&mut ZetesisConfig` or any other mutable config reference. Dynamic values that change at runtime (download progress, queue depth, session state) belong in subsystem-internal state, not in config.

**No full Config passed to subsystems.** Each subsystem receives only its own `Arc<SubsystemConfig>` slice. Passing the full `Config` to subsystems couples every subsystem's constructor to the full config shape; adding a field to any subsystem's config would recompile all of them. The slice boundary also prevents a subsystem from accidentally reading another subsystem's credentials.

**No secrets in `harmonia.toml`.** The file is committed. Any value that provides access to external services (API keys, JWT signing secrets, database credentials) must live in `secrets.toml` (gitignored) or in a `HARMONIA__{SUBSYSTEM}__{KEY}` environment variable. The `harmonia.toml` file should be safe to publish; containing it in a public repository must not compromise the running system.
