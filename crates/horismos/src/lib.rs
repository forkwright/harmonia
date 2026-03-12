mod config;
mod diff;
mod error;
mod handle;
mod secrets;
mod subsystems;
mod validation;

pub use config::Config;
pub use diff::{ConfigChange, diff_config};
pub use error::HorismosError;
pub use handle::{ConfigHandle, ConfigManager};
pub use subsystems::{
    AggeliaConfig, DatabaseConfig, EpignosisConfig, ErgasiaConfig, ExousiaConfig, KomideConfig,
    KritikeConfig, LibraryConfig, MediaType, ParocheConfig, ProsthekeConfig, SyntaxisConfig,
    TaxisConfig, TrackerSeedPolicy, WatcherMode, ZetesisConfig,
};
pub use validation::ValidationWarning;

use std::path::Path;

use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use snafu::ResultExt;

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
#[expect(clippy::result_large_err, reason = "test closures via figment::Jail")]
mod tests {
    use figment::Jail;

    use super::*;

    fn valid_jwt_secret() -> &'static str {
        "a-very-long-secret-key-that-is-at-least-32-bytes-long"
    }

    // ── Default config ────────────────────────────────────────────────────────

    #[test]
    fn default_config_has_correct_values() {
        let config = Config::default();
        assert_eq!(config.exousia.access_token_ttl_secs, 900);
        assert_eq!(config.exousia.refresh_token_ttl_days, 30);
        assert_eq!(config.paroche.port, 8096);
        assert_eq!(config.aggelia.buffer_size, 1024);
        assert_eq!(config.aggelia.download_queue_size, 512);
        assert_eq!(config.zetesis.request_timeout_secs, 30);
        assert_eq!(config.epignosis.cache_ttl_secs, 86400);
        assert_eq!(config.kritike.scan_interval_hours, 24);
    }

    // ── TOML file overrides defaults ──────────────────────────────────────────

    #[test]
    fn toml_overrides_defaults() {
        Jail::expect_with(|jail: &mut Jail| {
            jail.create_file(
                "harmonia.toml",
                &format!(
                    "[exousia]\naccess_token_ttl_secs = 1800\njwt_secret = \"{}\"\n\n[paroche]\nport = 9090\n",
                    valid_jwt_secret()
                ),
            )?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            assert_eq!(config.exousia.access_token_ttl_secs, 1800);
            assert_eq!(config.paroche.port, 9090);
            Ok(())
        });
    }

    // ── Environment variables override TOML ───────────────────────────────────

    #[test]
    fn env_vars_override_toml() {
        Jail::expect_with(|jail: &mut Jail| {
            jail.create_file(
                "harmonia.toml",
                &format!(
                    "[exousia]\naccess_token_ttl_secs = 900\njwt_secret = \"{}\"\n\n[paroche]\nport = 8096\n",
                    valid_jwt_secret()
                ),
            )?;
            jail.set_env("HARMONIA__PAROCHE__PORT", "7777");
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            assert_eq!(config.paroche.port, 7777);
            Ok(())
        });
    }

    // ── secrets.toml is loaded ────────────────────────────────────────────────

    #[test]
    fn secrets_toml_is_loaded() {
        Jail::expect_with(|jail: &mut Jail| {
            let secrets_secret = "secrets-toml-jwt-secret-long-enough-for-validation";
            jail.create_file("harmonia.toml", "[exousia]\naccess_token_ttl_secs = 900\n")?;
            jail.create_file(
                "secrets.toml",
                &format!("[exousia]\njwt_secret = \"{secrets_secret}\"\n"),
            )?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            assert_eq!(config.exousia.jwt_secret, secrets_secret);
            Ok(())
        });
    }

    // ── Missing config file falls back to defaults ────────────────────────────

    #[test]
    fn missing_config_file_uses_defaults() {
        Jail::expect_with(|jail: &mut Jail| {
            jail.set_env("HARMONIA__EXOUSIA__JWT_SECRET", valid_jwt_secret());
            let (config, _) = load_config(Some(Path::new("nonexistent.toml"))).unwrap();
            assert_eq!(config.exousia.access_token_ttl_secs, 900);
            assert_eq!(config.paroche.port, 8096);
            Ok(())
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
        assert!(!warnings.is_empty());
        assert!(warnings[0].field.contains("taxis.libraries.music.path"));
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
        assert!(warnings.is_empty());
    }

    // ── Port validation ───────────────────────────────────────────────────────

    #[test]
    fn validation_rejects_privileged_port() {
        let mut config = config_with_jwt(valid_jwt_secret());
        config.paroche.port = 80;
        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("port"));
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
        assert_eq!(restored.download_queue_size, 512);
    }
}
