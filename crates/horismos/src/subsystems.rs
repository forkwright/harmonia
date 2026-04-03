use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_path: PathBuf,
    pub read_pool_size: u32,
    pub write_pool_max: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::FROM("harmonia.db"),
            read_pool_size: 0, // 0 = auto-detect
            write_pool_max: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExousiaConfig {
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_days: u64,
    pub jwt_secret: SecretString,
}

impl Default for ExousiaConfig {
    fn default() -> Self {
        Self {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: SecretString::new(), // intentionally invalid  -  validation rejects it
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParocheConfig {
    pub listen_addr: String,
    pub port: u16,
    pub stream_buffer_kb: usize,
    pub transcode_concurrency: usize,
    pub opds_page_size: usize,
}

impl Default for ParocheConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            port: 8096,
            stream_buffer_kb: 256,
            transcode_concurrency: 2,
            opds_page_size: 50,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatcherMode {
    #[default]
    Auto,
    Inotify,
    Poll,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    #[default]
    Music,
    Video,
    Book,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub path: PathBuf,
    pub media_type: MediaType,
    pub watcher_mode: WatcherMode,
    pub poll_interval_seconds: u64,
    pub auto_import: bool,
    pub scan_interval_hours: u64,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            media_type: MediaType::default(),
            watcher_mode: WatcherMode::default(),
            poll_interval_seconds: 300,
            auto_import: true,
            scan_interval_hours: 24,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaxisConfig {
    pub libraries: HashMap<String, LibraryConfig>,
    pub file_naming_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpignosisConfig {
    pub cache_ttl_secs: u64,
    pub max_retries: u32,
    pub provider_timeout_secs: u64,
}

impl Default for EpignosisConfig {
    fn default() -> Self {
        Self {
            cache_ttl_secs: 86400,
            max_retries: 3,
            provider_timeout_secs: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KritikeConfig {
    pub scan_interval_hours: u64,
    pub quality_check_concurrency: usize,
}

impl Default for KritikeConfig {
    fn default() -> Self {
        Self {
            scan_interval_hours: 24,
            quality_check_concurrency: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggeliaConfig {
    pub buffer_size: usize,
    pub download_queue_size: usize,
}

impl Default for AggeliaConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1024,
            download_queue_size: 512,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZetesisConfig {
    pub request_timeout_secs: u64,
    pub max_results_per_indexer: usize,
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
    pub cf_health_check_interval_minutes: u64,
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
            cf_proxy_url: None,
            cf_proxy_timeout_seconds: 60,
            cf_cookie_refresh_minutes: 30,
            cf_health_check_interval_minutes: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerSeedPolicy {
    pub ratio_threshold: f64,
    pub time_threshold_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErgasiaConfig {
    pub download_dir: PathBuf,
    pub session_state_path: PathBuf,
    pub listen_port_range: [u16; 2],
    pub max_concurrent_downloads: usize,
    pub seed_ratio_threshold: f64,
    pub seed_time_threshold_hours: u64,
    pub tracker_seed_policies: HashMap<String, TrackerSeedPolicy>,
    pub progress_throttle_seconds: u64,
    pub extraction_temp_dir: PathBuf,
    pub peer_connect_timeout_seconds: u64,
    pub max_connections_per_torrent: u32,
    pub magnet_resolve_timeout_seconds: u64,
    pub max_extraction_depth: u8,
    pub extraction_cleanup_hours: u64,
}

impl Default for ErgasiaConfig {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::FROM("/data/downloads"),
            session_state_path: PathBuf::FROM("/data/downloads/.librqbit-state"),
            listen_port_range: [6881, 6889],
            max_concurrent_downloads: 5,
            seed_ratio_threshold: 1.0,
            seed_time_threshold_hours: 72,
            tracker_seed_policies: HashMap::new(),
            progress_throttle_seconds: 2,
            extraction_temp_dir: PathBuf::FROM("/data/downloads/.extraction"),
            peer_connect_timeout_seconds: 10,
            max_connections_per_torrent: 0,
            magnet_resolve_timeout_seconds: 120,
            max_extraction_depth: 3,
            extraction_cleanup_hours: 48,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSubtitlesConfig {
    pub api_key: SecretString,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Maximum API requests per second.
    pub rate_limit_per_second: u32,
}

impl Default for OpenSubtitlesConfig {
    fn default() -> Self {
        Self {
            api_key: SecretString::new(),
            username: None,
            password: None,
            rate_limit_per_second: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for KomideConfig {
    fn default() -> Self {
        Self {
            podcast_poll_interval_minutes: 30,
            news_poll_interval_minutes: 15,
            podcast_dir: PathBuf::FROM("/data/podcasts"),
            news_retention_days: 30,
            news_retention_articles: 500,
            auto_download_latest_n: 3,
            fetch_timeout_secs: 30,
        }
    }
}

// ── External API integration (Syndesmos) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlexConfig {
    /// Base URL of the Plex Media Server, e.g. `http://localhost:32400`.
    pub url: String,
    /// X-Plex-Token for API authentication.
    pub token: SecretString,
    /// Maps Harmonia media type to the Plex library section ID.
    pub library_sections: HashMap<harmonia_common::MediaType, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastfmConfig {
    pub api_key: SecretString,
    pub shared_secret: SecretString,
    /// Populated after the user completes the Last.fm auth flow.
    pub session_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalConfig {
    pub client_id: String,
    pub client_secret: SecretString,
    /// OAuth2 access token; refreshed automatically when expired.
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    /// How often to sync the Tidal favorites list (minutes).
    pub sync_interval_minutes: u64,
}

impl Default for TidalConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: SecretString::new(),
            access_token: None,
            refresh_token: None,
            sync_interval_minutes: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyndesmosConfig {
    /// Plex integration  -  `None` disables Plex notify and collection management.
    pub plex: Option<PlexConfig>,
    /// Last.fm integration  -  `None` disables scrobbling and artist enrichment.
    pub lastfm: Option<LastfmConfig>,
    /// Tidal integration  -  `None` disables want-list sync.
    pub tidal: Option<TidalConfig>,
    /// Minutes a service's circuit breaker stays open after tripping.
    pub circuit_break_minutes: u64,
}

impl Default for SyndesmosConfig {
    fn default() -> Self {
        Self {
            plex: None,
            lastfm: None,
            tidal: None,
            circuit_break_minutes: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
