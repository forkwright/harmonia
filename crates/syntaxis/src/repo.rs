//! `download_queue` table operations.

use sqlx::SqlitePool;
use uuid::Uuid;

use harmonia_db::DbError;
use snafu::ResultExt;

use harmonia_db::error::QuerySnafu;

/// A raw DB row FROM `download_queue`.
///
/// All columns are selected so that `sqlx::FromRow` can deserialise any query result.
/// Some fields are only consumed in tests or future pipeline stages; suppressing the
/// lint avoids forcing premature use of every column.
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub(crate) struct QueueRow {
    pub id: Vec<u8>,
    pub want_id: Vec<u8>,
    pub release_id: Vec<u8>,
    pub download_url: String,
    pub protocol: String,
    pub priority: i64,
    pub tracker_id: Option<i64>,
    pub info_hash: Option<String>,
    pub status: String,
    pub added_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub failed_reason: Option<String>,
    pub retry_count: i64,
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_queue_item(
    pool: &SqlitePool,
    id: Uuid,
    want_id: &[u8],
    release_id: &[u8],
    download_url: &str,
    protocol: &str,
    priority: u8,
    tracker_id: Option<i64>,
    info_hash: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO download_queue
         (id, want_id, release_id, download_url, protocol, priority, tracker_id, info_hash)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(want_id)
    .bind(release_id)
    .bind(download_url)
    .bind(protocol)
    .bind(i64::try_from(priority).unwrap_or_default())
    .bind(tracker_id)
    .bind(info_hash)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "download_queue",
    })?;
    Ok(())
}

#[cfg(test)]
pub(crate) async fn get_queue_item(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<QueueRow>, DbError> {
    sqlx::query_as::<_, QueueRow>(
        "SELECT id, want_id, release_id, download_url, protocol, priority,
                tracker_id, info_hash, status, added_at, started_at,
                completed_at, failed_reason, retry_count
         FROM download_queue WHERE id = ?",
    )
    .bind(id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "download_queue",
    })
}

pub(crate) async fn update_status(
    pool: &SqlitePool,
    id: Uuid,
    status: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE download_queue SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id.as_bytes().as_slice())
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "download_queue",
        })?;
    Ok(())
}

pub(crate) async fn update_priority(
    pool: &SqlitePool,
    id: Uuid,
    priority: u8,
) -> Result<(), DbError> {
    sqlx::query("UPDATE download_queue SET priority = ? WHERE id = ?")
        .bind(i64::try_from(priority).unwrap_or_default())
        .bind(id.as_bytes().as_slice())
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "download_queue",
        })?;
    Ok(())
}

/// Returns all rows with any of the given statuses, ordered by priority DESC, added_at ASC.
pub(crate) async fn list_by_status(
    pool: &SqlitePool,
    statuses: &[&str],
) -> Result<Vec<QueueRow>, DbError> {
    // WHY: sqlx doesn't support binding Vec<&str> to IN clauses directly.
    // We build the placeholders dynamically and use a raw query string.
    if statuses.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: String = statuses
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i == 0 {
                "?".to_string()
            } else {
                ",?".to_string()
            }
        })
        .collect();
    let sql = format!(
        "SELECT id, want_id, release_id, download_url, protocol, priority,
                tracker_id, info_hash, status, added_at, started_at,
                completed_at, failed_reason, retry_count
         FROM download_queue
         WHERE status IN ({placeholders})
         ORDER BY priority DESC, added_at ASC"
    );
    let mut query = sqlx::query_as::<_, QueueRow>(&sql);
    for s in statuses {
        query = query.bind(*s);
    }
    query.fetch_all(pool).await.context(QuerySnafu {
        table: "download_queue",
    })
}

/// Returns rows with status 'queued' or 'downloading', ordered by priority DESC.
#[cfg(test)]
pub(crate) async fn list_active(pool: &SqlitePool) -> Result<Vec<QueueRow>, DbError> {
    sqlx::query_as::<_, QueueRow>(
        "SELECT id, want_id, release_id, download_url, protocol, priority,
                tracker_id, info_hash, status, added_at, started_at,
                completed_at, failed_reason, retry_count
         FROM download_queue
         WHERE status IN ('queued', 'downloading')
         ORDER BY priority DESC, added_at ASC",
    )
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "download_queue",
    })
}

pub(crate) async fn mark_completed(
    pool: &SqlitePool,
    id: Uuid,
    completed_at: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE download_queue SET status = 'completed', completed_at = ? WHERE id = ?")
        .bind(completed_at)
        .bind(id.as_bytes().as_slice())
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "download_queue",
        })?;
    Ok(())
}

pub(crate) async fn mark_failed(pool: &SqlitePool, id: Uuid, reason: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE download_queue SET status = 'failed', failed_reason = ? WHERE id = ?")
        .bind(reason)
        .bind(id.as_bytes().as_slice())
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "download_queue",
        })?;
    Ok(())
}

pub(crate) async fn increment_retry_count(pool: &SqlitePool, id: Uuid) -> Result<(), DbError> {
    sqlx::query("UPDATE download_queue SET retry_count = retry_count + 1 WHERE id = ?")
        .bind(id.as_bytes().as_slice())
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "download_queue",
        })?;
    Ok(())
}

pub(crate) async fn count_by_status(pool: &SqlitePool, status: &str) -> Result<u64, DbError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM download_queue WHERE status = ?")
        .bind(status)
        .fetch_one(pool)
        .await
        .context(QuerySnafu {
            table: "download_queue",
        })?;
    Ok(row.u64::try_from(0).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use harmonia_db::migrate::MIGRATOR;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn make_uuid() -> Uuid {
        Uuid::now_v7()
    }

    fn make_id_bytes() -> Vec<u8> {
        Uuid::now_v7().as_bytes().to_vec()
    }

    #[tokio::test]
    async fn insert_and_get_queue_item() {
        let pool = setup().await;
        let id = make_uuid();
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        insert_queue_item(
            &pool,
            id,
            &want_id,
            &release_id,
            "magnet:?xt=urn:btih:abc",
            "torrent",
            3,
            Some(1),
            Some("abc123"),
        )
        .await
        .unwrap();

        let row = get_queue_item(&pool, id).await.unwrap().unwrap();
        assert_eq!(row.status, "queued");
        assert_eq!(row.priority, 3);
        assert_eq!(row.protocol, "torrent");
        assert_eq!(row.tracker_id, Some(1));
        assert_eq!(row.info_hash, Some("abc123".to_string()));
        assert_eq!(row.retry_count, 0);
    }

    #[tokio::test]
    async fn update_status_changes_row() {
        let pool = setup().await;
        let id = make_uuid();
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        insert_queue_item(
            &pool,
            id,
            &want_id,
            &release_id,
            "magnet:?xt=urn:btih:def",
            "torrent",
            1,
            None,
            None,
        )
        .await
        .unwrap();

        update_status(&pool, id, "downloading").await.unwrap();
        let row = get_queue_item(&pool, id).await.unwrap().unwrap();
        assert_eq!(row.status, "downloading");
    }

    #[tokio::test]
    async fn mark_failed_sets_reason() {
        let pool = setup().await;
        let id = make_uuid();
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        insert_queue_item(
            &pool,
            id,
            &want_id,
            &release_id,
            "magnet:?xt=urn:btih:xyz",
            "torrent",
            2,
            None,
            None,
        )
        .await
        .unwrap();

        mark_failed(&pool, id, "no seeders").await.unwrap();
        let row = get_queue_item(&pool, id).await.unwrap().unwrap();
        assert_eq!(row.status, "failed");
        assert_eq!(row.failed_reason, Some("no seeders".to_string()));
    }

    #[tokio::test]
    async fn mark_completed_sets_timestamp() {
        let pool = setup().await;
        let id = make_uuid();
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        insert_queue_item(
            &pool,
            id,
            &want_id,
            &release_id,
            "magnet:?xt=urn:btih:zzz",
            "torrent",
            3,
            None,
            None,
        )
        .await
        .unwrap();

        mark_completed(&pool, id, "2026-01-01T00:00:00Z")
            .await
            .unwrap();
        let row = get_queue_item(&pool, id).await.unwrap().unwrap();
        assert_eq!(row.status, "completed");
        assert_eq!(row.completed_at, Some("2026-01-01T00:00:00Z".to_string()));
    }

    #[tokio::test]
    async fn list_by_status_returns_matching_rows() {
        let pool = setup().await;
        let id1 = make_uuid();
        let id2 = make_uuid();
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        insert_queue_item(
            &pool,
            id1,
            &want_id,
            &release_id,
            "magnet:?xt=1",
            "torrent",
            3,
            None,
            None,
        )
        .await
        .unwrap();
        insert_queue_item(
            &pool,
            id2,
            &want_id,
            &release_id,
            "magnet:?xt=2",
            "torrent",
            1,
            None,
            None,
        )
        .await
        .unwrap();
        mark_failed(&pool, id2, "timeout").await.unwrap();

        let queued = list_by_status(&pool, &["queued"]).await.unwrap();
        assert_eq!(queued.len(), 1);

        let all = list_by_status(&pool, &["queued", "failed"]).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn list_by_status_empty_slice_returns_empty() {
        let pool = setup().await;
        let rows = list_by_status(&pool, &[]).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn list_active_excludes_terminal_states() {
        let pool = setup().await;
        let id_queued = make_uuid();
        let id_failed = make_uuid();
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        insert_queue_item(
            &pool,
            id_queued,
            &want_id,
            &release_id,
            "magnet:?xt=a",
            "torrent",
            1,
            None,
            None,
        )
        .await
        .unwrap();
        insert_queue_item(
            &pool,
            id_failed,
            &want_id,
            &release_id,
            "magnet:?xt=b",
            "torrent",
            1,
            None,
            None,
        )
        .await
        .unwrap();
        mark_failed(&pool, id_failed, "corrupt").await.unwrap();

        let active = list_active(&pool).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active.get(0).copied().unwrap_or_default().status, "queued");
    }

    #[tokio::test]
    async fn priority_ordering_in_list_by_status() {
        let pool = setup().await;
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        let id_low = make_uuid();
        let id_high = make_uuid();

        // Insert low priority first
        insert_queue_item(
            &pool,
            id_low,
            &want_id,
            &release_id,
            "magnet:?xt=low",
            "torrent",
            1,
            None,
            None,
        )
        .await
        .unwrap();
        insert_queue_item(
            &pool,
            id_high,
            &want_id,
            &release_id,
            "magnet:?xt=high",
            "torrent",
            3,
            None,
            None,
        )
        .await
        .unwrap();

        let rows = list_by_status(&pool, &["queued"]).await.unwrap();
        assert_eq!(rows.len(), 2);
        // First row should be the higher priority item
        assert_eq!(rows.get(0).copied().unwrap_or_default().priority, 3);
        assert_eq!(rows.get(1).copied().unwrap_or_default().priority, 1);
    }

    #[tokio::test]
    async fn count_by_status_returns_correct_count() {
        let pool = setup().await;
        let want_id = make_id_bytes();
        let release_id = make_id_bytes();

        for _ in 0..3 {
            insert_queue_item(
                &pool,
                make_uuid(),
                &want_id,
                &release_id,
                "magnet:?xt=x",
                "torrent",
                1,
                None,
                None,
            )
            .await
            .unwrap();
        }

        let count = count_by_status(&pool, "queued").await.unwrap();
        assert_eq!(count, 3);
    }
}
