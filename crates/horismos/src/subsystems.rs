use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    pub db_path: PathBuf,
    pub read_pool_size: u32,
    pub write_pool_max: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("harmonia.db"),
            read_pool_size: 0, // 0 = auto-detect
            write_pool_max: 1,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExousiaConfig {
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_days: u64,
    pub jwt_secret: String,
}
impl std::fmt::Debug for ExousiaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExousiaConfig")
            .field("access_token_ttl_secs", &self.access_token_ttl_secs)
            .field("refresh_token_ttl_days", &self.refresh_token_ttl_days)
            .field("jwt_secret", &"[redacted]")
            .finish()
    }
}

impl Default for ExousiaConfig {
    fn default() -> Self {
        Self {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: String::new(), // intentionally invalid  -  validation rejects it
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ParocheConfig {
    pub listen_addr: String,
    pub port: u16,
    pub opds_page_size: usize,
    /// Shared secret renderers must present when registering over QUIC.
    /// Leaving it unset rejects every renderer registration (fail closed).
    #[serde(default)]
    pub renderer_api_key: Option<String>,
    /// Whether the KOSync endpoint accepts anonymous self-registration.
    /// KOReader clients self-register by protocol; operators exposing the
    /// server beyond a trusted network can close the abuse surface here.
    #[serde(default = "default_true")]
    pub kosync_registration_enabled: bool,
    /// Maximum concurrent renderer QUIC connections/tasks admitted before the
    /// pre-auth SessionInit handshake; excess connections are refused. Bounds
    /// server-side task/memory growth from an unauthenticated peer flood.
    pub renderer_max_connections: usize,
    /// Deadline (seconds) for a connecting renderer to complete the pre-auth
    /// SessionInit handshake (control-stream accept + read); a peer that
    /// stalls past this is dropped rather than held open indefinitely.
    pub renderer_session_init_timeout_secs: u64,
    /// UDP port the renderer QUIC endpoint binds on `listen_addr`. Changing
    /// it (or `listen_addr`) at runtime rebinds the endpoint live: new
    /// registrations move to the new address while established sessions
    /// drain on the old one.
    #[serde(default = "default_renderer_quic_port")]
    pub renderer_quic_port: u16,
}

fn default_true() -> bool {
    true
}

fn default_renderer_quic_port() -> u16 {
    4433
}

impl std::fmt::Debug for ParocheConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParocheConfig")
            .field("listen_addr", &self.listen_addr)
            .field("port", &self.port)
            .field("opds_page_size", &self.opds_page_size)
            .field(
                "renderer_api_key",
                &self.renderer_api_key.as_ref().map(|_| "[redacted]"),
            )
            .field(
                "kosync_registration_enabled",
                &self.kosync_registration_enabled,
            )
            .field("renderer_max_connections", &self.renderer_max_connections)
            .field(
                "renderer_session_init_timeout_secs",
                &self.renderer_session_init_timeout_secs,
            )
            .field("renderer_quic_port", &self.renderer_quic_port)
            .finish()
    }
}

impl Default for ParocheConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            port: 8096,
            opds_page_size: 50,
            renderer_api_key: None,
            kosync_registration_enabled: true,
            // WHY: mirrors syndesis::ServerConfig::max_sessions' default — the
            // sibling QUIC admission surface in this workspace.
            renderer_max_connections: 32,
            renderer_session_init_timeout_secs: 10,
            renderer_quic_port: default_renderer_quic_port(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WatcherMode {
    #[default]
    Auto,
    Inotify,
    Poll,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MediaType {
    #[default]
    Music,
    Video,
    Book,
    Audiobook,
    Comic,
    Podcast,
    Tv,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LibraryConfig {
    pub path: PathBuf,
    pub media_type: MediaType,
    pub watcher_mode: WatcherMode,
    pub poll_interval_seconds: u64,
    pub scan_interval_hours: u64,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            media_type: MediaType::default(),
            watcher_mode: WatcherMode::default(),
            poll_interval_seconds: 300,
            scan_interval_hours: 24,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct TaxisConfig {
    pub libraries: HashMap<String, LibraryConfig>,
    /// Filesystem watcher debounce window (milliseconds) before an event batch
    /// is flushed.
    pub watcher_debounce_ms: u64,
    /// Maximum concurrent library scan tasks across all libraries.
    pub scan_concurrency: usize,
}

impl Default for TaxisConfig {
    fn default() -> Self {
        Self {
            libraries: HashMap::new(),
            watcher_debounce_ms: 500,
            scan_concurrency: 4,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EpignosisConfig {
    pub cache_ttl_secs: u64,
    pub provider_timeout_secs: u64,
    /// Minimum AcoustID confidence score to accept a fingerprint match
    /// without ambiguity.
    pub fingerprint_accept_threshold: f64,
    /// Minimum AcoustID confidence score to consider a fingerprint match
    /// at all.
    pub fingerprint_ambiguous_threshold: f64,
    /// Maximum bytes buffered FROM a single metadata-provider response.
    #[serde(default = "default_provider_response_max_bytes")]
    pub provider_response_max_bytes: u64,
    /// AcoustID API key for fingerprint lookups. Absent = fingerprinting
    /// runs unauthenticated (degraded/rate-limited, not rejected).
    #[serde(default)]
    pub acoustid_key: Option<String>,
    /// TMDB API Read Access Token (v4); sent as an Authorization Bearer
    /// header, never as a URL query parameter. Absent = movie/secondary-TV
    /// metadata resolution is unavailable.
    #[serde(default)]
    pub tmdb_key: Option<String>,
    /// TheTVDB API key. Absent = TV metadata resolution is unavailable.
    #[serde(default)]
    pub tvdb_key: Option<String>,
    /// Comic Vine API key. Absent = comic metadata resolution is
    /// unavailable.
    #[serde(default)]
    pub comicvine_key: Option<String>,
    /// Google Books API key. Absent = the Google Books fallback (used when
    /// the canonical book provider returns nothing) runs unauthenticated
    /// (rate-limited, not rejected) — keyless book lookups still work via
    /// OpenLibrary.
    #[serde(default)]
    pub google_books_key: Option<String>,
}

fn default_provider_response_max_bytes() -> u64 {
    10 * 1024 * 1024
}

impl std::fmt::Debug for EpignosisConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpignosisConfig")
            .field("cache_ttl_secs", &self.cache_ttl_secs)
            .field("provider_timeout_secs", &self.provider_timeout_secs)
            .field(
                "fingerprint_accept_threshold",
                &self.fingerprint_accept_threshold,
            )
            .field(
                "fingerprint_ambiguous_threshold",
                &self.fingerprint_ambiguous_threshold,
            )
            .field(
                "provider_response_max_bytes",
                &self.provider_response_max_bytes,
            )
            .field(
                "acoustid_key",
                &self.acoustid_key.as_ref().map(|_| "[redacted]"),
            )
            .field("tmdb_key", &self.tmdb_key.as_ref().map(|_| "[redacted]"))
            .field("tvdb_key", &self.tvdb_key.as_ref().map(|_| "[redacted]"))
            .field(
                "comicvine_key",
                &self.comicvine_key.as_ref().map(|_| "[redacted]"),
            )
            .field(
                "google_books_key",
                &self.google_books_key.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

impl Default for EpignosisConfig {
    fn default() -> Self {
        Self {
            cache_ttl_secs: 86400,
            provider_timeout_secs: 10,
            fingerprint_accept_threshold: 0.8,
            fingerprint_ambiguous_threshold: 0.5,
            provider_response_max_bytes: default_provider_response_max_bytes(),
            acoustid_key: None,
            tmdb_key: None,
            tvdb_key: None,
            comicvine_key: None,
            google_books_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KritikeConfig {
    pub quality_check_concurrency: usize,
}

impl Default for KritikeConfig {
    fn default() -> Self {
        Self {
            quality_check_concurrency: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AggeliaConfig {
    pub buffer_size: usize,
}

impl Default for AggeliaConfig {
    fn default() -> Self {
        Self { buffer_size: 1024 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SearchSubsystemConfig {
    pub request_timeout_secs: u64,
    pub max_results_per_indexer: usize,
    /// Byte cap on any single indexer HTTP response body.
    ///
    /// WHY: indexer responses are third-party data — an unbounded read lets a
    /// hostile or broken indexer exhaust memory with one response.
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
    /// TTL (seconds) a completed search's results stay resolvable via `GET
    /// /api/v1/search/{query_id}/results` and enqueue-by-`release_id`.
    #[serde(default = "default_result_cache_ttl_seconds")]
    pub result_cache_ttl_seconds: u64,
    /// Maximum completed searches held in the in-memory results cache at
    /// once; the oldest is evicted (with its releases) once a new search
    /// would exceed this.
    #[serde(default = "default_result_cache_max_queries")]
    pub result_cache_max_queries: usize,
}

fn default_result_cache_ttl_seconds() -> u64 {
    1800
}

fn default_result_cache_max_queries() -> usize {
    32
}

impl Default for SearchSubsystemConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: 30,
            max_results_per_indexer: 100,
            max_response_body_bytes: 16 * 1024 * 1024,
            cloudflare_bypass_enabled: false,
            max_concurrent_searches: 10,
            per_indexer_rate_limit_requests: 5,
            per_indexer_rate_limit_window_seconds: 10,
            caps_refresh_hours: 24,
            search_timeout_seconds: 30,
            cardigann_definitions_dir: None,
            cf_proxy_url: None,
            cf_proxy_timeout_seconds: 60,
            cf_cookie_refresh_minutes: 30,
            result_cache_ttl_seconds: default_result_cache_ttl_seconds(),
            result_cache_max_queries: default_result_cache_max_queries(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ErgasiaConfig {
    pub download_dir: PathBuf,
    pub session_state_path: PathBuf,
    pub listen_port_range: [u16; 2],
    pub seed_ratio_threshold: f64,
    pub seed_time_threshold_hours: u64,
    pub peer_connect_timeout_seconds: u64,
    pub magnet_resolve_timeout_seconds: u64,
    pub max_extraction_depth: u8,
    pub max_decompression_ratio: f64,
}

impl Default for ErgasiaConfig {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from("/data/downloads"),
            session_state_path: PathBuf::from("/data/downloads/.librqbit-state"),
            listen_port_range: [6881, 6889],
            seed_ratio_threshold: 1.0,
            seed_time_threshold_hours: 72,
            peer_connect_timeout_seconds: 10,
            magnet_resolve_timeout_seconds: 120,
            max_extraction_depth: 3,
            max_decompression_ratio: 100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SyntaxisConfig {
    pub max_concurrent_downloads: usize,
    pub max_per_tracker: usize,
    pub retry_count: u32,
    pub retry_backoff_base_seconds: u64,
    pub stalled_download_timeout_hours: u64,
}

impl Default for SyntaxisConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 5,
            max_per_tracker: 3,
            retry_count: 3,
            retry_backoff_base_seconds: 30,
            stalled_download_timeout_hours: 24,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OpenSubtitlesConfig {
    pub api_key: String,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Maximum API requests per second.
    pub rate_limit_per_second: u32,
    /// Maximum bytes buffered FROM a single subtitle download.
    #[serde(default = "default_max_download_bytes")]
    pub max_download_bytes: u64,
}

fn default_max_download_bytes() -> u64 {
    10 * 1024 * 1024
}

impl std::fmt::Debug for OpenSubtitlesConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenSubtitlesConfig")
            .field("api_key", &"[redacted]")
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .field("rate_limit_per_second", &self.rate_limit_per_second)
            .field("max_download_bytes", &self.max_download_bytes)
            .finish()
    }
}

impl Default for OpenSubtitlesConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            username: None,
            password: None,
            rate_limit_per_second: 5,
            max_download_bytes: default_max_download_bytes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProsthekeConfig {
    /// BCP 47 language tags in preference ORDER, e.g. `["en", "fr"]`.
    pub languages: Vec<String>,
    pub include_hearing_impaired: bool,
    pub include_forced: bool,
    /// Minimum match quality score (0.0–1.0) to accept a subtitle.
    pub min_match_score: f64,
    pub opensubtitles: Option<OpenSubtitlesConfig>,
}

impl Default for ProsthekeConfig {
    fn default() -> Self {
        Self {
            languages: vec!["en".to_string()],
            include_hearing_impaired: false,
            include_forced: true,
            min_match_score: 0.7,
            opensubtitles: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KomideConfig {
    /// Poll interval for podcast feeds in minutes.
    pub podcast_poll_interval_minutes: u64,
    /// Poll interval for news feeds in minutes.
    pub news_poll_interval_minutes: u64,
    /// Directory WHERE podcast episode audio files are stored.
    pub podcast_dir: PathBuf,
    /// Keep articles published within this many days (0 = no LIMIT).
    pub news_retention_days: u64,
    /// Keep at most this many articles per news feed (0 = no LIMIT).
    pub news_retention_articles: u64,
    /// Auto-download the N most recent episodes when subscribing (0 = none).
    pub auto_download_latest_n: u64,
    /// Request timeout for feed fetches in seconds.
    pub fetch_timeout_secs: u64,
    /// Maximum feed response body size in bytes; larger responses are rejected.
    pub max_feed_bytes: u64,
    /// Maximum episode download size in bytes; larger downloads are rejected.
    pub max_episode_bytes: u64,
    /// Maximum exponential-backoff window (minutes) between feed polls after
    /// consecutive failures.
    pub max_backoff_minutes: u64,
    /// Jitter range applied to each poll interval (±% of base interval) to
    /// avoid thundering-herd fetches.
    pub jitter_percent: f64,
}

impl Default for KomideConfig {
    fn default() -> Self {
        Self {
            podcast_poll_interval_minutes: 30,
            news_poll_interval_minutes: 15,
            podcast_dir: PathBuf::from("/data/podcasts"),
            news_retention_days: 30,
            news_retention_articles: 500,
            auto_download_latest_n: 3,
            fetch_timeout_secs: 30,
            max_feed_bytes: 20 * 1024 * 1024,
            max_episode_bytes: 1024 * 1024 * 1024,
            max_backoff_minutes: 240,
            jitter_percent: 10.0,
        }
    }
}

// ── External API integration (Syndesmos) ──────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlexConfig {
    /// Base URL of the Plex Media Server, e.g. `http://localhost:32400`.
    pub url: String,
    /// X-Plex-Token for API authentication.
    pub token: String,
    /// Maps Harmonia media type to the Plex library section ID.
    pub library_sections: HashMap<themelion::MediaType, u32>,
}
impl std::fmt::Debug for PlexConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlexConfig")
            .field("url", &self.url)
            .field("token", &"[redacted]")
            .field("library_sections", &self.library_sections)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LastfmConfig {
    pub api_key: String,
    pub shared_secret: String,
    /// Populated after the user completes the Last.fm auth flow.
    pub session_key: Option<String>,
}
impl std::fmt::Debug for LastfmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LastfmConfig")
            .field("api_key", &"[redacted]")
            .field("shared_secret", &"[redacted]")
            .field("session_key", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TidalConfig {
    /// OAuth2 access token; refreshed automatically when expired.
    pub access_token: Option<String>,
}
impl std::fmt::Debug for TidalConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TidalConfig")
            .field("access_token", &"[redacted]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SyndesmosConfig {
    /// Plex integration  -  `None` disables Plex notify and collection management.
    pub plex: Option<PlexConfig>,
    /// Last.fm integration  -  `None` disables scrobbling and artist enrichment.
    pub lastfm: Option<LastfmConfig>,
    /// Tidal integration  -  `None` disables want-list sync.
    pub tidal: Option<TidalConfig>,
    /// Minutes a service's circuit breaker stays open after tripping.
    pub circuit_break_minutes: u64,
    /// Consecutive failures that trip a service's circuit breaker.
    pub circuit_break_failure_threshold: u32,
}

impl Default for SyndesmosConfig {
    fn default() -> Self {
        Self {
            plex: None,
            lastfm: None,
            tidal: None,
            circuit_break_minutes: 5,
            circuit_break_failure_threshold: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AitesisConfig {
    /// Maximum number of Submitted + Approved + Monitoring requests per user.
    pub max_pending_per_user: u32,
    /// Maximum requests a user may CREATE in a single calendar day (UTC).
    pub max_requests_per_day: u32,
    /// When true, Admin users' requests are approved and sent to monitoring immediately.
    pub auto_approve_admins: bool,
}

impl Default for AitesisConfig {
    fn default() -> Self {
        Self {
            max_pending_per_user: 25,
            max_requests_per_day: 10,
            auto_approve_admins: true,
        }
    }
}

// WHY: pure data — the `harmonia mcp` stdio surface and the `harmonia serve`
// acquisition bridge derive the SAME socket path FROM this config
// independently (#609): they are separate processes with no other shared
// state, so agreement has to come from a deterministic function of config,
// not coordination at runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct McpConfig {
    /// Unix-domain-socket path the acquisition bridge binds and the stdio
    /// surface connects to. `None` derives a default: `harmonia-mcp.sock`
    /// inside a dedicated `harmonia-mcp/` runtime subdirectory beside
    /// `database.db_path` (never `db_path`'s own directory, which the
    /// bridge does not own and must not chmod).
    #[serde(default)]
    pub socket_path: Option<PathBuf>,
    /// Deadline (seconds) for one `tools/call` round trip over the bridge
    /// socket. Should be at least `zetesis.search_timeout_seconds` — a
    /// shorter MCP deadline would cut off a legitimate full indexer fan-out.
    #[serde(default = "default_mcp_call_timeout_secs")]
    pub call_timeout_secs: u64,
}

fn default_mcp_call_timeout_secs() -> u64 {
    120
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
            call_timeout_secs: default_mcp_call_timeout_secs(),
        }
    }
}
