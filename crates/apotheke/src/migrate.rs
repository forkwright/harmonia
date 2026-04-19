use snafu::ResultExt;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;

use crate::error::{DbError, MigrationSnafu};

pub static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), DbError> {
    MIGRATOR.run(pool).await.context(MigrationSnafu)
}
