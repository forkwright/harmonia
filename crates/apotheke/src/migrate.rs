use snafu::ResultExt;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;

use crate::error::{DbError, MigrationSnafu};

pub static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), DbError> {
    MIGRATOR.run(pool).await.context(MigrationSnafu)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies raw migration SQL through `before_version` (exclusive),
    /// bypassing migrator bookkeeping so a later migration can be exercised
    /// against a mid-history schema.
    async fn apply_until(pool: &SqlitePool, before_version: i64) {
        for migration in MIGRATOR.iter() {
            if migration.version < before_version {
                sqlx::raw_sql(&migration.sql).execute(pool).await.unwrap();
            }
        }
    }

    async fn apply_version(pool: &SqlitePool, version: i64) {
        let migration = MIGRATOR
            .iter()
            .find(|m| m.version == version)
            .unwrap_or_else(|| panic!("no migration with version {version}"));
        sqlx::raw_sql(&migration.sql).execute(pool).await.unwrap();
    }

    // WHY: migration 012 rebuilds indexers + indexer_categories to widen the
    // protocol CHECK — the rebuild must carry every existing row across.
    #[tokio::test]
    async fn indexer_protocol_rebuild_preserves_existing_rows() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        apply_until(&pool, 12).await;

        sqlx::query(
            "INSERT INTO indexers (id, name, url, protocol, api_key, priority)
             VALUES (3, 'Pre-existing', 'https://old.example/api', 'torznab', 'key', 10)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO indexer_categories (indexer_id, category_id, name)
             VALUES (3, 2000, 'Movies')",
        )
        .execute(&pool)
        .await
        .unwrap();

        apply_version(&pool, 12).await;

        let (id, name, protocol, api_key, priority) =
            sqlx::query_as::<_, (i64, String, String, String, i64)>(
                "SELECT id, name, protocol, api_key, priority FROM indexers",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            (
                id,
                name.as_str(),
                protocol.as_str(),
                api_key.as_str(),
                priority
            ),
            (3, "Pre-existing", "torznab", "key", 10)
        );

        let categories = sqlx::query_as::<_, (i64, i64, String)>(
            "SELECT indexer_id, category_id, name FROM indexer_categories",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(categories, vec![(3, 2000, "Movies".to_string())]);
    }

    #[tokio::test]
    async fn indexer_protocol_check_admits_cardigann_and_rejects_unknown() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO indexers (name, url, protocol) VALUES ('C', 'sample-id', 'cardigann')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let err =
            sqlx::query("INSERT INTO indexers (name, url, protocol) VALUES ('B', 'x', 'bogus')")
                .execute(&pool)
                .await
                .unwrap_err();
        assert!(err.to_string().contains("CHECK"), "got: {err}");
    }
}
