use std::io::Write;
use std::sync::Arc;

use apotheke::DbPools;
use exousia::{AuthService, CreateUserRequest, ExousiaServiceImpl, UserRole};
use rand::Rng;
use snafu::ResultExt;
use tracing::info;

use crate::error::{AuthSnafu, DatabaseSnafu, HostError};

pub async fn ensure_admin_user(
    auth: &Arc<ExousiaServiceImpl>,
    db: &DbPools,
    out: &mut impl Write,
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
    let _ = writeln!(
        out,
        "============================================================"
    );
    let _ = writeln!(out, "  First run detected. Admin password: {password}");
    let _ = writeln!(out, "  Change it immediately.");
    let _ = writeln!(
        out,
        "============================================================"
    );

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
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{EnvFilter, fmt};

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
    use std::sync::Arc;

    use super::*;

    #[test]
    fn generated_password_is_48_hex_chars() {
        let pw = generate_password();
        assert_eq!(pw.len(), 48);
        assert!(pw.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn ensure_admin_user_creates_and_writes_password() {
        let db_file = tempfile::NamedTempFile::new().unwrap();
        let db = apotheke::init_pools(db_file.path().to_str().unwrap())
            .await
            .unwrap();
        let db = Arc::new(db);
        let auth = Arc::new(ExousiaServiceImpl::new(
            db.clone(),
            horismos::ExousiaConfig::default(),
        ));

        let mut out = Vec::new();
        ensure_admin_user(&auth, &db, &mut out).await.unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("First run detected"),
            "expected first-run banner, got: {output}"
        );
        assert!(
            output.contains("Admin password:"),
            "expected password line, got: {output}"
        );
        assert!(
            output.contains("Change it immediately"),
            "expected change-password reminder, got: {output}"
        );
    }
}
