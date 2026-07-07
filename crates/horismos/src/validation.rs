use tracing::warn;

use crate::Config;
use crate::error::{HorismosError, ValidationSnafu};

// WHY: pure data — diagnostic warning from config validation.
#[derive(Debug)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

pub fn validate_config(config: &Config) -> Result<Vec<ValidationWarning>, HorismosError> {
    let mut warnings = Vec::new();

    validate_jwt_secret(config)?;
    validate_ports(config)?;
    validate_timeouts(config)?;
    validate_limits(config)?;
    validate_token_ttls(config)?;
    validate_provider_credentials(config, &mut warnings)?;
    collect_library_warnings(config, &mut warnings);

    Ok(warnings)
}

// WHY: 100 years — a TTL past this is a typo'd unit, not a policy choice, and
// absurd values are the input that once fed a silent expiry-math overflow.
const MAX_REFRESH_TOKEN_TTL_DAYS: u64 = 36_500;

fn validate_token_ttls(config: &Config) -> Result<(), HorismosError> {
    if config.exousia.access_token_ttl_secs == 0 {
        return ValidationSnafu {
            message: "exousia.access_token_ttl_secs must be greater than 0".to_string(),
        }
        .fail();
    }
    let days = config.exousia.refresh_token_ttl_days;
    if days == 0 || days > MAX_REFRESH_TOKEN_TTL_DAYS {
        return ValidationSnafu {
            message: format!(
                "exousia.refresh_token_ttl_days ({days}) must be between 1 and {MAX_REFRESH_TOKEN_TTL_DAYS}"
            ),
        }
        .fail();
    }
    Ok(())
}

fn validate_limits(config: &Config) -> Result<(), HorismosError> {
    if config.syndesis.jitter_buffer_max_frames == 0 {
        return ValidationSnafu {
            message: "syndesis.jitter_buffer_max_frames must be greater than 0".to_string(),
        }
        .fail();
    }
    if config.syndesis.max_sessions == 0 {
        return ValidationSnafu {
            message: "syndesis.max_sessions must be greater than 0".to_string(),
        }
        .fail();
    }
    if config.zetesis.max_response_body_bytes == 0 {
        return ValidationSnafu {
            message: "zetesis.max_response_body_bytes must be greater than 0".to_string(),
        }
        .fail();
    }
    if config.taxis.scan_concurrency == 0 {
        return ValidationSnafu {
            message: "taxis.scan_concurrency must be greater than 0 — it sizes a semaphore \
                      that library scans acquire from; 0 permits blocks the first scan forever"
                .to_string(),
        }
        .fail();
    }
    if config.database.write_pool_max == 0 {
        return ValidationSnafu {
            message: "database.write_pool_max must be greater than 0".to_string(),
        }
        .fail();
    }
    if config.kritike.quality_check_concurrency == 0 {
        return ValidationSnafu {
            message: "kritike.quality_check_concurrency must be greater than 0".to_string(),
        }
        .fail();
    }
    if config.zetesis.cloudflare_bypass_enabled
        && config
            .zetesis
            .cf_proxy_url
            .as_deref()
            .is_none_or(|url| url.trim().is_empty())
    {
        return ValidationSnafu {
            message:
                "zetesis.cf_proxy_url must be set when zetesis.cloudflare_bypass_enabled is true"
                    .to_string(),
        }
        .fail();
    }
    Ok(())
}

fn validate_jwt_secret(config: &Config) -> Result<(), HorismosError> {
    let secret = &config.exousia.jwt_secret;
    if secret.is_empty() || secret == "changeme" || secret == "default" {
        return ValidationSnafu {
            message: "exousia.jwt_secret must not be empty or a placeholder value — SET via secrets.toml or HARMONIA__EXOUSIA__JWT_SECRET".to_string(),
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

// #578: metadata-provider API credentials. Keyless operation is a
// legitimate posture (MusicBrainz + OpenLibrary need no key at all, and
// Google Books falls back to unauthenticated/rate-limited requests) — an
// absent key is a WARNING naming the degraded capability, never an error.
// An explicitly-set placeholder (empty string, "changeme", "default") IS an
// error: it can only come from a botched config edit, never from simply
// omitting the field.
fn validate_provider_credentials(
    config: &Config,
    warnings: &mut Vec<ValidationWarning>,
) -> Result<(), HorismosError> {
    validate_credential(
        "epignosis.acoustid_key",
        config.epignosis.acoustid_key.as_deref(),
        "AcoustID fingerprint identification unavailable",
        warnings,
    )?;
    validate_credential(
        "epignosis.tmdb_key",
        config.epignosis.tmdb_key.as_deref(),
        "movie and secondary TV metadata resolution unavailable",
        warnings,
    )?;
    validate_credential(
        "epignosis.tvdb_key",
        config.epignosis.tvdb_key.as_deref(),
        "TV metadata resolution unavailable",
        warnings,
    )?;
    validate_credential(
        "epignosis.comicvine_key",
        config.epignosis.comicvine_key.as_deref(),
        "comic metadata resolution unavailable",
        warnings,
    )?;
    validate_credential(
        "epignosis.google_books_key",
        config.epignosis.google_books_key.as_deref(),
        "Google Books fallback runs unauthenticated and rate-limited",
        warnings,
    )?;
    Ok(())
}

fn validate_credential(
    field: &str,
    value: Option<&str>,
    absent_degrades: &str,
    warnings: &mut Vec<ValidationWarning>,
) -> Result<(), HorismosError> {
    match value {
        None => {
            warnings.push(ValidationWarning {
                field: field.to_string(),
                message: format!("{field} unset — {absent_degrades}"),
            });
            Ok(())
        }
        Some(v) if v.is_empty() || v == "changeme" || v == "default" => ValidationSnafu {
            message: format!(
                "{field} must not be an empty string or a placeholder value — set a real key, \
                 or remove the field entirely for keyless operation"
            ),
        }
        .fail(),
        Some(_) => Ok(()),
    }
}

fn validate_ports(config: &Config) -> Result<(), HorismosError> {
    let port = config.paroche.port;
    if port < 1024 {
        return ValidationSnafu {
            message: format!("paroche.port ({port}) is below 1024 — Harmonia must not run as root"),
        }
        .fail();
    }
    let quic_port = config.paroche.renderer_quic_port;
    if quic_port < 1024 {
        return ValidationSnafu {
            message: format!(
                "paroche.renderer_quic_port ({quic_port}) is below 1024 — Harmonia must not run \
                 as root"
            ),
        }
        .fail();
    }
    if quic_port == port {
        return ValidationSnafu {
            message: format!(
                "paroche.renderer_quic_port ({quic_port}) must differ from paroche.port ({port})"
            ),
        }
        .fail();
    }
    Ok(())
}

fn validate_timeouts(config: &Config) -> Result<(), HorismosError> {
    if config.zetesis.request_timeout_secs == 0 {
        return ValidationSnafu {
            message: "zetesis.request_timeout_secs must be greater than 0".to_string(),
        }
        .fail();
    }
    if config.epignosis.provider_timeout_secs == 0 {
        return ValidationSnafu {
            message: "epignosis.provider_timeout_secs must be greater than 0".to_string(),
        }
        .fail();
    }
    let score = config.prostheke.min_match_score;
    if !(0.0..=1.0).contains(&score) {
        return ValidationSnafu {
            message: format!("prostheke.min_match_score ({score}) must be between 0.0 and 1.0"),
        }
        .fail();
    }
    Ok(())
}

fn collect_library_warnings(config: &Config, warnings: &mut Vec<ValidationWarning>) {
    for (name, library) in &config.taxis.libraries {
        if !library.path.exists() {
            let message = format!(
                "library '{}' path '{}' is not accessible at startup",
                name,
                library.path.display()
            );
            warn!(library = %name, path = %library.path.display(), "library path not accessible at startup");
            warnings.push(ValidationWarning {
                field: format!("taxis.libraries.{name}.path"),
                message,
            });
        }
    }
}
