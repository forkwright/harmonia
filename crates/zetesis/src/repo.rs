use snafu::ResultExt;
use sqlx::SqlitePool;

use harmonia_db::DbError;
use harmonia_db::error::QuerySnafu;

use crate::types::{IndexerCaps, IndexerCategory};

#[derive(Debug, Clone, sqlx::FromRow)]
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

pub async fn insert_indexer(
    pool: &SqlitePool,
    name: &str,
    url: &str,
    protocol: &str,
    api_key: Option<&str>,
    cf_bypass: bool,
    priority: i32,
) -> Result<i64, DbError> {
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

pub async fn update_indexer_status(
    pool: &SqlitePool,
    id: i64,
    status: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE indexers SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "indexers" })?;

    Ok(())
}

pub async fn update_indexer_caps(
    pool: &SqlitePool,
    id: i64,
    caps_json: &str,
    last_tested: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE indexers SET caps_json = ?, last_tested = ? WHERE id = ?")
        .bind(caps_json)
        .bind(last_tested)
        .bind(id)
        .execute(pool)
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

pub async fn upsert_indexer_categories(
    pool: &SqlitePool,
    indexer_id: i64,
    caps: &IndexerCaps,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM indexer_categories WHERE indexer_id = ?")
        .bind(indexer_id)
        .execute(pool)
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
        .bind(i64::try_from(cat_id).unwrap_or_default())
        .bind(&name)
        .execute(pool)
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
