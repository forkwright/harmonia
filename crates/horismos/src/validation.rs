use tracing::warn;

use crate::{
    Config,
    error::{HorismosError, ValidationSnafu},
};

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
    collect_library_warnings(config, &mut warnings);

    Ok(warnings)
}

fn validate_jwt_secret(config: &Config) -> Result<(), HorismosError> {
    let secret = &config.exousia.jwt_secret;
    if secret.is_empty() || secret == "changeme" || secret == "default" {
        return ValidationSnafu {
            message: "exousia.jwt_secret must not be empty or a placeholder value — set via secrets.toml or HARMONIA__EXOUSIA__JWT_SECRET".to_string(),
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

fn validate_ports(config: &Config) -> Result<(), HorismosError> {
    let port = config.paroche.port;
    if port < 1024 {
        return ValidationSnafu {
            message: format!("paroche.port ({port}) is below 1024 — Harmonia must not run as root"),
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
