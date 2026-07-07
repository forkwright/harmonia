mod config;
mod diff;
mod error;
mod handle;
mod secrets;
mod subsystems;
mod validation;

use std::path::Path;

pub use config::Config;
pub use diff::{ConfigChange, LIVE, UNWIRED, diff_config};
pub use error::HorismosError;
use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
pub use handle::{
    ConfigHandle, ConfigManager, ConfigOverrides, ReloadOutcome, Section, SectionWatcher,
};
use snafu::ResultExt;
pub use subsystems::{
    AggeliaConfig, AitesisConfig, DatabaseConfig, EpignosisConfig, ErgasiaConfig, ExousiaConfig,
    KomideConfig, KritikeConfig, LastfmConfig, LibraryConfig, MediaType, OpenSubtitlesConfig,
    ParocheConfig, PlexConfig, ProsthekeConfig, SearchSubsystemConfig, SyndesmosConfig,
    SyntaxisConfig, TaxisConfig, TidalConfig, WatcherMode,
};
pub use validation::ValidationWarning;

use crate::error::ConfigParseSnafu;
use crate::secrets::secrets_path;
use crate::validation::validate_config;

/// Load and validate configuration.
///
/// Applies providers in priority order (lowest to highest):
/// 1. Compiled-in `Default` values
/// 2. `harmonia.toml` (or the given path)
/// 3. `secrets.toml` (sibling of config file, gitignored)
/// 4. `HARMONIA__SECTION__KEY` environment variables
///
/// Returns the validated config along with any non-fatal warnings.
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

#[cfg(test)]
mod tests {
    use figment::Jail;

    use super::*;

    fn valid_jwt_secret() -> &'static str {
        "a-very-long-secret-key-that-is-at-least-32-bytes-long"
    }

    #[allow(
        clippy::result_large_err,
        reason = "figment::Jail::expect_with requires figment::Result; this lint is version-dependent"
    )]
    fn with_jail(run: impl FnOnce(&mut Jail)) {
        Jail::expect_with(|jail| {
            run(jail);
            Ok(())
        });
    }

    // ── Default config ────────────────────────────────────────────────────────

    #[test]
    fn default_config_has_correct_values() {
        let config = Config::default();
        assert_eq!(config.exousia.access_token_ttl_secs, 900);
        assert_eq!(config.exousia.refresh_token_ttl_days, 30);
        assert_eq!(config.paroche.port, 8096);
        assert_eq!(config.paroche.renderer_quic_port, 4433);
        assert_eq!(config.aggelia.buffer_size, 1024);
        assert_eq!(config.zetesis.request_timeout_secs, 30);
        assert_eq!(config.zetesis.max_response_body_bytes, 16 * 1024 * 1024);
        assert_eq!(config.epignosis.cache_ttl_secs, 86400);
        assert_eq!(config.kritike.quality_check_concurrency, 4);
    }

    #[test]
    fn default_komide_config_has_correct_values() {
        let config = Config::default();
        assert_eq!(config.komide.max_feed_bytes, 20 * 1024 * 1024);
        assert_eq!(config.komide.max_episode_bytes, 1024 * 1024 * 1024);
        assert_eq!(config.komide.max_backoff_minutes, 240);
        assert_eq!(config.komide.fetch_timeout_secs, 30);
    }

    #[test]
    fn default_syndesmos_config_has_correct_values() {
        let config = Config::default();
        assert!(config.syndesmos.plex.is_none());
        assert!(config.syndesmos.lastfm.is_none());
        assert!(config.syndesmos.tidal.is_none());
        assert_eq!(config.syndesmos.circuit_break_minutes, 5);
    }

    // ── TOML file overrides defaults ──────────────────────────────────────────

    #[test]
    fn toml_overrides_defaults() {
        with_jail(|jail| {
            jail.create_file(
                "harmonia.toml",
                &format!(
                    "[exousia]\naccess_token_ttl_secs = 1800\njwt_secret = \"{}\"\n\n[paroche]\nport = 9090\n",
                    valid_jwt_secret()
                ),
            )
            .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            assert_eq!(config.exousia.access_token_ttl_secs, 1800);
            assert_eq!(config.paroche.port, 9090);
        });
    }

    // ── Environment variables override TOML ───────────────────────────────────

    #[test]
    fn env_vars_override_toml() {
        with_jail(|jail| {
            jail.create_file(
                "harmonia.toml",
                &format!(
                    "[exousia]\naccess_token_ttl_secs = 900\njwt_secret = \"{}\"\n\n[paroche]\nport = 8096\n",
                    valid_jwt_secret()
                ),
            )
            .unwrap();
            jail.set_env("HARMONIA__PAROCHE__PORT", "7777");
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            assert_eq!(config.paroche.port, 7777);
        });
    }

    // ── secrets.toml is loaded ────────────────────────────────────────────────

    #[test]
    fn secrets_toml_is_loaded() {
        with_jail(|jail| {
            let secrets_secret = "secrets-toml-jwt-secret-long-enough-for-validation";
            jail.create_file("harmonia.toml", "[exousia]\naccess_token_ttl_secs = 900\n")
                .unwrap();
            jail.create_file(
                "secrets.toml",
                &format!("[exousia]\njwt_secret = \"{secrets_secret}\"\n"),
            )
            .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            assert_eq!(config.exousia.jwt_secret, secrets_secret);
        });
    }

    // ── Missing config file falls back to defaults ────────────────────────────

    #[test]
    fn missing_config_file_uses_defaults() {
        with_jail(|jail| {
            jail.set_env("HARMONIA__EXOUSIA__JWT_SECRET", valid_jwt_secret());
            let (config, _) = load_config(Some(Path::new("nonexistent.toml"))).unwrap();
            assert_eq!(config.exousia.access_token_ttl_secs, 900);
            assert_eq!(config.paroche.port, 8096);
        });
    }

    // ── JWT secret validation ─────────────────────────────────────────────────

    fn config_with_jwt(secret: &str) -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret = secret.to_string();
        config
    }

    #[test]
    fn validation_rejects_empty_jwt_secret() {
        let config = config_with_jwt("");
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("jwt_secret"));
    }

    #[test]
    fn validation_rejects_changeme_jwt_secret() {
        let config = config_with_jwt("changeme");
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("jwt_secret"));
    }

    #[test]
    fn validation_rejects_short_jwt_secret() {
        let config = config_with_jwt("tooshort");
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("jwt_secret"));
    }

    #[test]
    fn validation_accepts_valid_jwt_secret() {
        let config = config_with_jwt(valid_jwt_secret());
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn validation_rejects_absurd_token_ttls() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.exousia.refresh_token_ttl_days = 0;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("refresh_token_ttl_days"));

        let mut config = config_with_jwt(valid_jwt_secret());
        config.exousia.refresh_token_ttl_days = u64::MAX / 86400 + 1;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("refresh_token_ttl_days"));

        let mut config = config_with_jwt(valid_jwt_secret());
        config.exousia.access_token_ttl_secs = 0;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("access_token_ttl_secs"));
    }

    // ── Library path warnings ─────────────────────────────────────────────────

    #[test]
    fn validation_warns_on_inaccessible_library_paths() {
        let mut config = config_with_jwt(valid_jwt_secret());
        let library = LibraryConfig {
            path: std::path::PathBuf::from("/nonexistent/library/path"),
            ..LibraryConfig::default()
        };
        config.taxis.libraries.insert("music".to_string(), library);
        let warnings = validate_config(&config).unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| w.field.contains("taxis.libraries.music.path"))
        );
    }

    #[test]
    fn validation_no_warnings_for_accessible_library_paths() {
        let mut config = config_with_jwt(valid_jwt_secret());
        let library = LibraryConfig {
            path: std::path::PathBuf::from("/tmp"),
            ..LibraryConfig::default()
        };
        config.taxis.libraries.insert("music".to_string(), library);
        let warnings = validate_config(&config).unwrap();
        assert!(
            !warnings.iter().any(|w| w.field.contains("taxis.libraries")),
            "an accessible library path must not warn, independent of the \
             standing #578 absent-provider-credential warnings: {warnings:?}"
        );
    }

    // ── Port validation ───────────────────────────────────────────────────────

    #[test]
    fn validation_rejects_privileged_port() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.paroche.port = 80;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn validation_rejects_privileged_renderer_quic_port() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.paroche.renderer_quic_port = 443;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("renderer_quic_port"));
    }

    #[test]
    fn validation_rejects_renderer_quic_port_colliding_with_http_port() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.paroche.port = 9090;
        config.paroche.renderer_quic_port = 9090;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("must differ from paroche.port"));
    }

    #[test]
    fn validation_accepts_default_renderer_quic_port() {
        let config = config_with_jwt(valid_jwt_secret());
        assert_eq!(config.paroche.renderer_quic_port, 4433);
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn config_file_without_renderer_quic_port_gets_default() {
        with_jail(|jail| {
            jail.create_file(
                "harmonia.toml",
                &format!(
                    "[exousia]\njwt_secret = \"{}\"\n\n[paroche]\nport = 9090\n",
                    valid_jwt_secret()
                ),
            )
            .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            assert_eq!(config.paroche.renderer_quic_port, 4433);
        });
    }

    // ── Zetesis limits validation ─────────────────────────────────────────────

    #[test]
    fn validation_rejects_zero_max_response_body_bytes() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.zetesis.max_response_body_bytes = 0;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("max_response_body_bytes"));
    }

    // ── Taxis / database / kritike concurrency validation ─────────────────────

    #[test]
    fn validation_rejects_zero_scan_concurrency() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.taxis.scan_concurrency = 0;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("scan_concurrency"));
    }

    #[test]
    fn validation_accepts_nonzero_scan_concurrency() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.taxis.scan_concurrency = 1;
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn validation_rejects_zero_write_pool_max() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.database.write_pool_max = 0;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("write_pool_max"));
    }

    #[test]
    fn validation_accepts_zero_read_pool_size_as_auto_detect() {
        // WHY: read_pool_size = 0 is a documented sentinel (auto-detect FROM
        // available_parallelism), unlike write_pool_max — it must not be
        // rejected the way the other concurrency knobs are.
        let mut config = config_with_jwt(valid_jwt_secret());
        config.database.read_pool_size = 0;
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn validation_rejects_zero_quality_check_concurrency() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.kritike.quality_check_concurrency = 0;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("quality_check_concurrency"));
    }

    #[test]
    fn validation_rejects_cf_bypass_without_proxy_url() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.zetesis.cloudflare_bypass_enabled = true;
        config.zetesis.cf_proxy_url = None;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("cf_proxy_url"));
    }

    #[test]
    fn validation_rejects_cf_bypass_with_blank_proxy_url() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.zetesis.cloudflare_bypass_enabled = true;
        config.zetesis.cf_proxy_url = Some("   ".to_string());
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("cf_proxy_url"));
    }

    #[test]
    fn validation_accepts_cf_bypass_with_proxy_url() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.zetesis.cloudflare_bypass_enabled = true;
        config.zetesis.cf_proxy_url = Some("http://byparr:8191".to_string());
        assert!(validate_config(&config).is_ok());
    }

    // ── #578: provider credential validation ──────────────────────────────────

    #[test]
    fn validation_warns_on_absent_provider_credentials() {
        let config = config_with_jwt(valid_jwt_secret());
        let warnings = validate_config(&config).unwrap();
        let fields: Vec<&str> = warnings.iter().map(|w| w.field.as_str()).collect();
        assert!(fields.contains(&"epignosis.acoustid_key"));
        assert!(fields.contains(&"epignosis.tmdb_key"));
        assert!(fields.contains(&"epignosis.tvdb_key"));
        assert!(fields.contains(&"epignosis.comicvine_key"));
        assert!(fields.contains(&"epignosis.google_books_key"));
    }

    #[test]
    fn validation_absent_credential_warning_names_degraded_capability() {
        let config = config_with_jwt(valid_jwt_secret());
        let warnings = validate_config(&config).unwrap();
        let tmdb = warnings
            .iter()
            .find(|w| w.field == "epignosis.tmdb_key")
            .expect("absent tmdb_key must produce a warning");
        assert!(tmdb.message.contains("movie"));
    }

    #[test]
    fn validation_rejects_empty_string_provider_credential() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.epignosis.acoustid_key = Some(String::new());
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("epignosis.acoustid_key"));
    }

    #[test]
    fn validation_rejects_placeholder_provider_credential() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.epignosis.tmdb_key = Some("changeme".to_string());
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("epignosis.tmdb_key"));
    }

    #[test]
    fn validation_accepts_real_provider_credential_with_no_warning_for_that_field() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.epignosis.acoustid_key = Some("real-acoustid-key".to_string());
        let warnings = validate_config(&config).unwrap();
        assert!(
            !warnings.iter().any(|w| w.field == "epignosis.acoustid_key"),
            "a configured key must not also warn as absent"
        );
    }

    #[test]
    fn epignosis_config_debug_redacts_keys() {
        let config = EpignosisConfig {
            acoustid_key: Some("super-secret-acoustid-key".to_string()),
            ..EpignosisConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("super-secret-acoustid-key"));
    }

    // ── Serialize/Deserialize roundtrip ───────────────────────────────────────

    #[test]
    fn config_roundtrip() {
        let mut original = Config::default();
        original.exousia.jwt_secret = valid_jwt_secret().to_string();
        let json = serde_json::to_string(&original).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.exousia.jwt_secret, original.exousia.jwt_secret);
        assert_eq!(restored.paroche.port, original.paroche.port);
        assert_eq!(restored.aggelia.buffer_size, original.aggelia.buffer_size);
    }

    #[test]
    fn exousia_config_roundtrip() {
        let original = ExousiaConfig {
            access_token_ttl_secs: 1800,
            refresh_token_ttl_days: 60,
            jwt_secret: valid_jwt_secret().to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: ExousiaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.access_token_ttl_secs, 1800);
        assert_eq!(restored.refresh_token_ttl_days, 60);
    }

    #[test]
    fn taxis_config_roundtrip() {
        let mut original = TaxisConfig::default();
        let lib = LibraryConfig {
            path: std::path::PathBuf::from("/data/music"),
            media_type: MediaType::Music,
            watcher_mode: WatcherMode::Inotify,
            ..LibraryConfig::default()
        };
        original.libraries.insert("music".to_string(), lib);
        let json = serde_json::to_string(&original).unwrap();
        let restored: TaxisConfig = serde_json::from_str(&json).unwrap();
        assert!(restored.libraries.contains_key("music"));
        assert_eq!(
            restored.libraries["music"].path,
            std::path::PathBuf::from("/data/music")
        );
    }

    #[test]
    fn database_config_roundtrip() {
        let original = DatabaseConfig::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: DatabaseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.write_pool_max, 1);
        assert_eq!(restored.read_pool_size, 0);
    }

    #[test]
    fn aggelia_config_roundtrip() {
        let original = AggeliaConfig::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: AggeliaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.buffer_size, 1024);
    }

    #[test]
    fn syndesmos_config_roundtrip() {
        let original = SyndesmosConfig::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: SyndesmosConfig = serde_json::from_str(&json).unwrap();
        assert!(restored.plex.is_none());
        assert!(restored.lastfm.is_none());
        assert!(restored.tidal.is_none());
        assert_eq!(restored.circuit_break_minutes, 5);
    }
}
