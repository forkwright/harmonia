use apotheke::DbError;
use apotheke::error::QuerySnafu;
use snafu::ResultExt;
use sqlx::{Sqlite, SqliteConnection, SqlitePool};

use crate::types::{IndexerCaps, IndexerCategory};

#[derive(Clone, sqlx::FromRow)]
pub struct IndexerRow {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub protocol: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub cf_bypass: bool,
    pub status: String,
    pub last_tested: Option<String>,
    pub caps_json: Option<String>,
    pub priority: i32,
    pub added_at: String,
}
impl std::fmt::Debug for IndexerRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexerRow")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("url", &self.url)
            .field("protocol", &self.protocol)
            .field("api_key", &"[redacted]")
            .field("enabled", &self.enabled)
            .field("cf_bypass", &self.cf_bypass)
            .field("status", &self.status)
            .field("last_tested", &self.last_tested)
            .field("caps_json", &self.caps_json)
            .field("priority", &self.priority)
            .field("added_at", &self.added_at)
            .finish()
    }
}

/// Parameters for [`insert_indexer`].
///
/// Groups the columns written into the `indexers` row on insert. Columns with
/// database defaults (`enabled`, `status`, `last_tested`, `caps_json`,
/// `added_at`) are intentionally omitted.
// WHY: wire DTO — parameter bundle for the insert_indexer SQL call.
pub struct InsertIndexerParams<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub protocol: &'a str,
    pub api_key: Option<&'a str>,
    pub cf_bypass: bool,
    pub priority: i32,
}

pub async fn insert_indexer(
    pool: &SqlitePool,
    params: InsertIndexerParams<'_>,
) -> Result<i64, DbError> {
    let InsertIndexerParams {
        name,
        url,
        protocol,
        api_key,
        cf_bypass,
        priority,
    } = params;
    let result = sqlx::query_scalar::<_, i64>(
        "INSERT INTO indexers (name, url, protocol, api_key, cf_bypass, priority)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(name)
    .bind(url)
    .bind(protocol)
    .bind(api_key)
    .bind(cf_bypass)
    .bind(priority)
    .fetch_one(pool)
    .await
    .context(QuerySnafu { table: "indexers" })?;

    Ok(result)
}

pub async fn get_indexer(pool: &SqlitePool, id: i64) -> Result<Option<IndexerRow>, DbError> {
    let row = sqlx::query_as::<_, IndexerRow>("SELECT * FROM indexers WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context(QuerySnafu { table: "indexers" })?;

    Ok(row)
}

pub async fn list_indexers(pool: &SqlitePool) -> Result<Vec<IndexerRow>, DbError> {
    let rows = sqlx::query_as::<_, IndexerRow>("SELECT * FROM indexers ORDER BY priority ASC")
        .fetch_all(pool)
        .await
        .context(QuerySnafu { table: "indexers" })?;

    Ok(rows)
}

pub async fn get_eligible_indexers(pool: &SqlitePool) -> Result<Vec<IndexerRow>, DbError> {
    let rows = sqlx::query_as::<_, IndexerRow>(
        "SELECT * FROM indexers
         WHERE enabled = TRUE AND status != 'failed'
         ORDER BY priority ASC",
    )
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "indexers" })?;

    Ok(rows)
}

pub async fn update_indexer_status<'e, E>(executor: E, id: i64, status: &str) -> Result<(), DbError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    sqlx::query("UPDATE indexers SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(executor)
        .await
        .context(QuerySnafu { table: "indexers" })?;

    Ok(())
}

pub async fn update_indexer_caps<'e, E>(
    executor: E,
    id: i64,
    caps_json: &str,
    last_tested: &str,
) -> Result<(), DbError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    sqlx::query("UPDATE indexers SET caps_json = ?, last_tested = ? WHERE id = ?")
        .bind(caps_json)
        .bind(last_tested)
        .bind(id)
        .execute(executor)
        .await
        .context(QuerySnafu { table: "indexers" })?;

    Ok(())
}

pub async fn delete_indexer(pool: &SqlitePool, id: i64) -> Result<(), DbError> {
    sqlx::query("DELETE FROM indexers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "indexers" })?;

    Ok(())
}

// INVARIANT: the caller supplies a connection inside an open transaction —
// the DELETE + INSERT sequence is only atomic under that transaction.
pub async fn upsert_indexer_categories(
    conn: &mut SqliteConnection,
    indexer_id: i64,
    caps: &IndexerCaps,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM indexer_categories WHERE indexer_id = ?")
        .bind(indexer_id)
        .execute(&mut *conn)
        .await
        .context(QuerySnafu {
            table: "indexer_categories",
        })?;

    fn collect_categories(cats: &[IndexerCategory], out: &mut Vec<(u32, String)>) {
        for cat in cats {
            out.push((cat.id, cat.name.clone()));
            collect_categories(&cat.subcategories, out);
        }
    }

    let mut flat = Vec::new();
    collect_categories(&caps.categories, &mut flat);

    for (cat_id, name) in flat {
        sqlx::query(
            "INSERT OR REPLACE INTO indexer_categories (indexer_id, category_id, name)
             VALUES (?, ?, ?)",
        )
        .bind(indexer_id)
        .bind(i64::from(cat_id))
        .bind(&name)
        .execute(&mut *conn)
        .await
        .context(QuerySnafu {
            table: "indexer_categories",
        })?;
    }

    Ok(())
}

pub async fn restore_degraded_cf_indexers(pool: &SqlitePool) -> Result<u64, DbError> {
    let result = sqlx::query(
        "UPDATE indexers SET status = 'active' WHERE status = 'degraded' AND cf_bypass = TRUE",
    )
    .execute(pool)
    .await
    .context(QuerySnafu { table: "indexers" })?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    use super::*;
    use crate::types::{SearchLimits, ServerInfo};

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn params<'a>(name: &'a str, cf_bypass: bool) -> InsertIndexerParams<'a> {
        InsertIndexerParams {
            name,
            url: "https://example.com/api",
            protocol: "torznab",
            api_key: Some("key"),
            cf_bypass,
            priority: 50,
        }
    }

    fn caps_with(categories: &[(u32, &str)]) -> IndexerCaps {
        IndexerCaps {
            server: ServerInfo {
                title: None,
                version: None,
            },
            limits: SearchLimits::default(),
            search_functions: vec![],
            categories: categories
                .iter()
                .map(|(id, name)| IndexerCategory {
                    id: *id,
                    name: (*name).to_string(),
                    subcategories: vec![],
                })
                .collect(),
        }
    }

    async fn category_rows(pool: &SqlitePool, indexer_id: i64) -> Vec<(i64, String)> {
        sqlx::query_as::<_, (i64, String)>(
            "SELECT category_id, name FROM indexer_categories
             WHERE indexer_id = ? ORDER BY category_id",
        )
        .bind(indexer_id)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn insert_and_get_indexer_roundtrip() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Roundtrip", false))
            .await
            .unwrap();

        let row = get_indexer(&pool, id).await.unwrap().unwrap();
        assert_eq!(row.id, id);
        assert_eq!(row.name, "Roundtrip");
        assert_eq!(row.url, "https://example.com/api");
        assert_eq!(row.protocol, "torznab");
        assert_eq!(row.api_key.as_deref(), Some("key"));
        assert!(row.enabled);
        assert!(!row.cf_bypass);
        assert_eq!(row.status, "active");
        assert_eq!(row.priority, 50);
    }

    #[tokio::test]
    async fn get_indexer_returns_none_for_unknown_id() {
        let pool = setup().await;
        assert!(get_indexer(&pool, 9999).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_indexers_orders_by_priority() {
        let pool = setup().await;
        let low = insert_indexer(
            &pool,
            InsertIndexerParams {
                priority: 90,
                ..params("Low", false)
            },
        )
        .await
        .unwrap();
        let high = insert_indexer(
            &pool,
            InsertIndexerParams {
                priority: 10,
                ..params("High", false)
            },
        )
        .await
        .unwrap();

        let rows = list_indexers(&pool).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, high);
        assert_eq!(rows[1].id, low);
    }

    #[tokio::test]
    async fn get_eligible_indexers_excludes_disabled() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Disabled", false))
            .await
            .unwrap();
        sqlx::query("UPDATE indexers SET enabled = FALSE WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(get_eligible_indexers(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_eligible_indexers_excludes_failed() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Failed", false))
            .await
            .unwrap();
        update_indexer_status(&pool, id, "failed").await.unwrap();

        assert!(get_eligible_indexers(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_eligible_indexers_includes_active_and_degraded() {
        let pool = setup().await;
        insert_indexer(&pool, params("Active", false))
            .await
            .unwrap();
        let degraded = insert_indexer(&pool, params("Degraded", false))
            .await
            .unwrap();
        update_indexer_status(&pool, degraded, "degraded")
            .await
            .unwrap();

        let eligible = get_eligible_indexers(&pool).await.unwrap();
        assert_eq!(eligible.len(), 2);
    }

    #[tokio::test]
    async fn update_indexer_status_persists() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Status", false))
            .await
            .unwrap();

        update_indexer_status(&pool, id, "degraded").await.unwrap();

        let row = get_indexer(&pool, id).await.unwrap().unwrap();
        assert_eq!(row.status, "degraded");
    }

    #[tokio::test]
    async fn update_indexer_caps_persists_json_and_timestamp() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Caps", false)).await.unwrap();

        update_indexer_caps(&pool, id, r#"{"caps":true}"#, "2026-01-01T00:00:00Z")
            .await
            .unwrap();

        let row = get_indexer(&pool, id).await.unwrap().unwrap();
        assert_eq!(row.caps_json.as_deref(), Some(r#"{"caps":true}"#));
        assert_eq!(row.last_tested.as_deref(), Some("2026-01-01T00:00:00Z"));
    }

    #[tokio::test]
    async fn delete_indexer_removes_row() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Doomed", false))
            .await
            .unwrap();

        delete_indexer(&pool, id).await.unwrap();

        assert!(get_indexer(&pool, id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn upsert_indexer_categories_replaces_previous_set() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Cats", false)).await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        upsert_indexer_categories(
            &mut tx,
            id,
            &caps_with(&[(1000, "Console"), (2000, "Movies")]),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        upsert_indexer_categories(&mut tx, id, &caps_with(&[(3000, "Audio")]))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let rows = category_rows(&pool, id).await;
        assert_eq!(rows, vec![(3000, "Audio".to_string())]);
    }

    #[tokio::test]
    async fn upsert_indexer_categories_flattens_subcategories() {
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Nested", false))
            .await
            .unwrap();

        let caps = IndexerCaps {
            categories: vec![IndexerCategory {
                id: 3000,
                name: "Audio".to_string(),
                subcategories: vec![IndexerCategory {
                    id: 3010,
                    name: "Audio/MP3".to_string(),
                    subcategories: vec![],
                }],
            }],
            ..caps_with(&[])
        };

        let mut tx = pool.begin().await.unwrap();
        upsert_indexer_categories(&mut tx, id, &caps).await.unwrap();
        tx.commit().await.unwrap();

        let rows = category_rows(&pool, id).await;
        assert_eq!(
            rows,
            vec![(3000, "Audio".to_string()), (3010, "Audio/MP3".to_string())]
        );
    }

    #[tokio::test]
    async fn upsert_indexer_categories_rolls_back_with_caller_transaction() {
        // WHY: the atomicity contract — a dropped (uncommitted) transaction
        // must leave the previous category set fully intact, DELETE included.
        let pool = setup().await;
        let id = insert_indexer(&pool, params("Atomic", false))
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        upsert_indexer_categories(&mut tx, id, &caps_with(&[(1000, "Console")]))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        upsert_indexer_categories(&mut tx, id, &caps_with(&[(2000, "Movies")]))
            .await
            .unwrap();
        drop(tx);

        let rows = category_rows(&pool, id).await;
        assert_eq!(rows, vec![(1000, "Console".to_string())]);
    }

    #[tokio::test]
    async fn restore_degraded_cf_indexers_only_affects_cf_bypass_degraded() {
        let pool = setup().await;
        let cf_degraded = insert_indexer(&pool, params("CfDegraded", true))
            .await
            .unwrap();
        let plain_degraded = insert_indexer(&pool, params("PlainDegraded", false))
            .await
            .unwrap();
        let cf_failed = insert_indexer(&pool, params("CfFailed", true))
            .await
            .unwrap();
        update_indexer_status(&pool, cf_degraded, "degraded")
            .await
            .unwrap();
        update_indexer_status(&pool, plain_degraded, "degraded")
            .await
            .unwrap();
        update_indexer_status(&pool, cf_failed, "failed")
            .await
            .unwrap();

        let affected = restore_degraded_cf_indexers(&pool).await.unwrap();

        assert_eq!(affected, 1);
        let restored = get_indexer(&pool, cf_degraded).await.unwrap().unwrap();
        assert_eq!(restored.status, "active");
        let untouched = get_indexer(&pool, plain_degraded).await.unwrap().unwrap();
        assert_eq!(untouched.status, "degraded");
        let still_failed = get_indexer(&pool, cf_failed).await.unwrap().unwrap();
        assert_eq!(still_failed.status, "failed");
    }
}
