use std::io::Write;

use snafu::ResultExt;

use crate::cli::DbMigrateArgs;
use crate::error::{ConfigSnafu, DatabaseSnafu, HostError, OutputSnafu};

pub(crate) async fn run_db_migrate(
    args: DbMigrateArgs,
    out: &mut impl Write,
) -> Result<(), HostError> {
    let (config, warnings) =
        horismos::load_config(Some(args.config.as_path())).context(ConfigSnafu)?;

    for warning in &warnings {
        writeln!(
            out,
            "config warning: [{}] {}",
            warning.field, warning.message
        )
        .context(OutputSnafu {
            operation: "write database migration config warning",
        })?;
    }

    let db_path = config.database.db_path.to_string_lossy();
    let pools = apotheke::init_pools(
        &db_path,
        config.database.read_pool_size,
        config.database.write_pool_max,
    )
    .await
    .context(DatabaseSnafu)?;
    pools.read.close().await;
    pools.write.close().await;

    writeln!(
        out,
        "database migrations applied: {}",
        config.database.db_path.display()
    )
    .context(OutputSnafu {
        operation: "write database migration summary",
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_jwt_secret() -> &'static str {
        "db-migrate-test-secret-that-is-at-least-32-bytes"
    }

    #[tokio::test]
    async fn run_db_migrate_applies_migrations_to_configured_database() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("harmonia.toml");
        let db_path = dir.path().join("harmonia.db");

        std::fs::write(
            &config_path,
            format!(
                "[exousia]\njwt_secret = \"{}\"\n\n[database]\ndb_path = \"{}\"\n",
                valid_jwt_secret(),
                db_path.display()
            ),
        )
        .unwrap();

        let mut out = Vec::new();
        run_db_migrate(
            DbMigrateArgs {
                config: config_path,
            },
            &mut out,
        )
        .await
        .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(
            output.contains("database migrations applied"),
            "expected migration summary, got: {output}"
        );
        assert!(db_path.exists());
    }
}
