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

/// Opens the read/write SQLite pools.
///
/// `read_pool_size` sizes the read pool; `0` auto-detects FROM
/// `available_parallelism` (the documented sentinel on
/// `horismos::DatabaseConfig::read_pool_size`). `write_pool_max` sizes the
/// write pool directly — callers are expected to reject `0` upstream (see
/// `horismos` validation); it is clamped here too as a defense-in-depth floor,
/// since `max_connections(0)` would leave every write `.acquire()` blocked
/// forever, the same hang class `taxis.scan_concurrency` had.
pub async fn init_pools(
    db_path: &str,
    read_pool_size: u32,
    write_pool_max: u32,
) -> Result<DbPools, DbError> {
    let base_opts = SqliteConnectOptions::new()
        .filename(db_path)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    // WARNING: SQLite WAL allows only one writer to hold the lock at a time;
    // sqlx's default 5s busy_timeout serializes contenders past a single
    // connection. `write_pool_max` lets an operator raise the queue depth;
    // it defaults to 1 (a single physical connection) unless configured.
    let write = SqlitePoolOptions::new()
        .max_connections(write_pool_max.max(1))
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

    let read_pool_size = if read_pool_size == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4)
            .max(2)
    } else {
        read_pool_size
    };
    let read_min_connections = read_pool_size.min(2);

    let read = SqlitePoolOptions::new()
        .max_connections(read_pool_size)
        .min_connections(read_min_connections)
        .connect_with(base_opts.read_only(true))
        .await
        .context(PoolInitSnafu)?;

    Ok(DbPools { read, write })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn init_pools_honors_configured_pool_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pools = init_pools(db_path.to_str().unwrap(), 3, 2).await.unwrap();

        assert_eq!(pools.read.options().get_max_connections(), 3);
        assert_eq!(pools.write.options().get_max_connections(), 2);
    }

    #[tokio::test]
    async fn init_pools_zero_read_pool_size_auto_detects() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pools = init_pools(db_path.to_str().unwrap(), 0, 1).await.unwrap();

        let expected = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4)
            .max(2);
        assert_eq!(pools.read.options().get_max_connections(), expected);
    }

    #[tokio::test]
    async fn init_pools_clamps_zero_write_pool_max_to_one() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pools = init_pools(db_path.to_str().unwrap(), 0, 0).await.unwrap();

        assert_eq!(pools.write.options().get_max_connections(), 1);
    }
}
