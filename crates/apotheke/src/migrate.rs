use sqlx::{SqlitePool, migrate::Migrator};

use crate::error::{DbError, MigrationSnafu};
use snafu::ResultExt;

pub static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), DbError> {
    MIGRATOR.run(pool).await.context(MigrationSnafu)
}
