use snafu::ResultExt;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::{DbError, PoolInitSnafu, TransactionSnafu};
use crate::migrate::run_migrations;

// WHY: pure data — handles to all SQLite connection pools used by the apotheke repo layer.
pub struct DbPools {
    pub read: SqlitePool,
    pub write: SqlitePool,
}

/// Begins a write transaction with `BEGIN IMMEDIATE`.
///
/// WHY: SQLite's default deferred `BEGIN` acquires the write lock only at the
/// first write statement, so a read-check-write sequence can interleave with a
/// concurrent writer. `BEGIN IMMEDIATE` takes the write lock up front, making
/// the whole transaction serialize against other writers.
pub async fn begin_immediate(pool: &SqlitePool) -> Result<Transaction<'static, Sqlite>, DbError> {
    pool.begin_with("BEGIN IMMEDIATE")
        .await
        .context(TransactionSnafu)
}

pub async fn commit_tx(tx: Transaction<'static, Sqlite>) -> Result<(), DbError> {
    tx.commit().await.context(TransactionSnafu)
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
