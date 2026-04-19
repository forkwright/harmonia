use std::sync::Arc;

use rand::Rng;
use snafu::ResultExt;
use tracing::info;

use apotheke::DbPools;
use exousia::{AuthService, CreateUserRequest, ExousiaServiceImpl, UserRole};

use crate::error::{AuthSnafu, DatabaseSnafu, HostError};

pub async fn ensure_admin_user(
    auth: &Arc<ExousiaServiceImpl>,
    db: &DbPools,
) -> Result<(), HostError> {
    let users = apotheke::repo::user::list_users(&db.read, 1, 0)
        .await
        .context(DatabaseSnafu)?;

    if !users.is_empty() {
        return Ok(());
    }

    let password = generate_password();

    auth.create_user(CreateUserRequest {
        username: "admin".to_string(),
        display_name: "Administrator".to_string(),
        password: password.clone(),
        role: UserRole::Admin,
    })
    .await
    .context(AuthSnafu)?;

    info!("First run detected. Admin user created.");
    println!("============================================================");
    println!("  First run detected. Admin password: {password}");
    println!("  Change it immediately.");
    println!("============================================================");

    Ok(())
}

fn generate_password() -> String {
    let mut rng = rand::rng();
    let mut bytes = [0u8; 24];
    rng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(48), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

pub fn init_tracing(config: &horismos::Config) -> Result<(), HostError> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let _ = config; // config reserved for future log-level configuration
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,archon=debug,paroche=debug,kathodos=debug,komide=debug")
    });

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .try_init()
        .map_err(|e| HostError::Tracing {
            message: e.to_string(),
            location: snafu::location!(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_password_is_48_hex_chars() {
        let pw = generate_password();
        assert_eq!(pw.len(), 48);
        assert!(pw.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
