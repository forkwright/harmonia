pub mod audiobook;
pub mod book;
pub mod comic;
pub mod kosync;
pub mod movie;
pub mod music;
pub mod news;
pub mod play_history;
pub mod podcast;
pub mod quality;
pub mod registry;
pub mod renderer;
pub mod tv;
pub mod user;
pub mod want;
pub mod zone;

use snafu::ResultExt;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteQueryResult;

use crate::error::{DbError, NotFoundSnafu, QuerySnafu};

// WHY: DbError::NotFound carries a displayable id — raw UUID bytes are not.
pub(crate) fn id_hex(id: &[u8]) -> String {
    id.iter()
        .fold(String::with_capacity(id.len() * 2), |mut s, b| {
            use std::fmt::Write;
            // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
            write!(s, "{b:02x}").ok();
            s
        })
}

// WHY: a single-row UPDATE/DELETE that matches zero rows hit a missing target;
// returning Ok would report success for a write that changed nothing.
pub(crate) fn require_affected(
    result: SqliteQueryResult,
    table: &'static str,
    id: impl Into<String>,
) -> Result<(), DbError> {
    if result.rows_affected() == 0 {
        return NotFoundSnafu { table, id }.fail();
    }
    Ok(())
}

/// Total row count of `table`, for pagination metadata.
///
/// WARNING: `table` is interpolated into the SQL text — pass compile-time
/// table-name literals only, never caller-supplied input.
pub async fn count_rows(pool: &SqlitePool, table: &'static str) -> Result<i64, DbError> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar(&sql)
        .fetch_one(pool)
        .await
        .context(QuerySnafu { table })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::MIGRATOR;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn count_rows_tracks_inserts() {
        let pool = setup().await;
        assert_eq!(count_rows(&pool, "zones").await.unwrap(), 0);
        for i in 0..3 {
            zone::create_zone(&pool, &format!("z{i}"), &format!("Zone {i}"))
                .await
                .unwrap();
        }
        assert_eq!(count_rows(&pool, "zones").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn count_rows_unknown_table_errors() {
        let pool = setup().await;
        let err = count_rows(&pool, "no_such_table").await.unwrap_err();
        assert!(matches!(err, DbError::Query { .. }));
    }

    #[tokio::test]
    async fn id_hex_formats_bytes() {
        assert_eq!(id_hex(&[0x00, 0xff, 0x0a]), "00ff0a");
        assert_eq!(id_hex(&[]), "");
    }
}
