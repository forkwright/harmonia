use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

use crate::error::{DbError, PoolInitSnafu};
use crate::migrate::run_migrations;
use snafu::ResultExt;

pub struct DbPools {
    pub read: SqlitePool,
    pub write: SqlitePool,
}

pub async fn init_pools(db_path: &str) -> Result<DbPools, DbError> {
    let base_opts = SqliteConnectOptions::new()
        .filename(db_path)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let write = SqlitePoolOptions::new()
        .max_connections(1) // CRITICAL: single writer  -  SQLite WAL constraint
        .connect_with(base_opts.clone().create_if_missing(true))
        .await
        .context(PoolInitSnafu)?;

    sqlx::query("PRAGMA journal_size_limit = 67108864")
        .execute(&write)
        .await
        .context(PoolInitSnafu)?;
    sqlx::query("PRAGMA temp_store = memory")
        .execute(&write)
        .await
        .context(PoolInitSnafu)?;

    run_migrations(&write).await?;

    let read_pool_size = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .max(2);

    let read = SqlitePoolOptions::new()
        .max_connections(read_pool_size)
        .min_connections(2)
        .connect_with(base_opts.read_only(true))
        .await
        .context(PoolInitSnafu)?;

    Ok(DbPools { read, write })
}
