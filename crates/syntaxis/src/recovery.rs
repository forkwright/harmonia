//! Startup reconciliation: reload non-terminal queue rows INTO memory.
//!
//! At startup, any download not in a terminal state ('completed' or 'failed') is
//! reloaded and re-queued. Items in 'downloading', 'post_processing', or
//! 'importing' states are re-queued FROM the top so they can be retried.

use sqlx::SqlitePool;
use themelion::ids::{ReleaseId, WantId};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::SyntaxisError;
use crate::queue::PriorityQueue;
use crate::repo::{self, QueueRow};
use crate::types::{DownloadProtocol, QueueItem};

fn parse_protocol(s: &str) -> DownloadProtocol {
    match s {
        "torrent" => DownloadProtocol::Torrent,
        "nzb" => DownloadProtocol::Usenet,
        other => {
            warn!(
                protocol = other,
                "unknown protocol in download_queue; treating as torrent"
            );
            DownloadProtocol::Torrent
        }
    }
}

fn row_to_queue_item(row: &QueueRow) -> Option<QueueItem> {
    let id = Uuid::from_slice(&row.id).ok()?;
    let want_uuid = Uuid::from_slice(&row.want_id).ok()?;
    let release_uuid = Uuid::from_slice(&row.release_id).ok()?;
    Some(QueueItem {
        id,
        want_id: WantId::from_uuid(want_uuid),
        release_id: ReleaseId::from_uuid(release_uuid),
        download_url: row.download_url.clone(),
        protocol: parse_protocol(&row.protocol),
        // Clamp priority to 1–3 during recovery; interactive (4) items are re-queued
        // at priority 3 so they don't re-bypass on restart.
        priority: (u8::try_from(row.priority).unwrap_or_default()).clamp(1, 3),
        tracker_id: row.tracker_id,
        info_hash: row.info_hash.clone(),
    })
}

/// Loads all non-terminal rows FROM the database and inserts them INTO `queue`.
///
/// Returns the number of items reloaded.
pub(crate) async fn reload_queue(
    pool: &SqlitePool,
    queue: &mut PriorityQueue,
) -> Result<usize, SyntaxisError> {
    let non_terminal = ["queued", "downloading", "post_processing", "importing"];
    let rows = repo::list_by_status(pool, &non_terminal)
        .await
        .map_err(|source| SyntaxisError::Database {
            source,
            location: snafu::location!(),
        })?;

    let mut count = 0usize;
    for row in &rows {
        match row_to_queue_item(row) {
            Some(item) => {
                // Re-queue all non-terminal items. Items in downloading/post_processing/
                // importing are effectively re-queued FROM the start; Ergasia's
                // persistence layer may allow resumption.
                queue.insert(item);
                count += 1;
            }
            None => {
                warn!("could not parse download_queue row during recovery; skipping");
            }
        }
    }

    if count > 0 {
        info!(recovered = count, "startup reconciliation complete");
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use super::*;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn make_id_bytes(id: Uuid) -> Vec<u8> {
        id.as_bytes().to_vec()
    }

    async fn insert(pool: &SqlitePool, id: Uuid, status: &str, priority: u8) {
        let want_id = make_id_bytes(Uuid::now_v7());
        let release_id = make_id_bytes(Uuid::now_v7());
        repo::insert_queue_item(
            pool,
            id,
            &want_id,
            &release_id,
            "magnet:?xt=urn:btih:test",
            "torrent",
            priority,
            None,
            None,
        )
        .await
        .unwrap();
        if status != "queued" {
            repo::update_status(pool, id, status).await.unwrap();
        }
    }

    #[tokio::test]
    async fn reloads_queued_items() {
        let pool = setup().await;
        insert(&pool, Uuid::now_v7(), "queued", 2).await;
        insert(&pool, Uuid::now_v7(), "queued", 1).await;

        let mut queue = PriorityQueue::new();
        let count = reload_queue(&pool, &mut queue).await.unwrap();

        assert_eq!(count, 2);
        assert_eq!(queue.len(), 2);
    }

    #[tokio::test]
    async fn skips_terminal_items() {
        let pool = setup().await;
        insert(&pool, Uuid::now_v7(), "completed", 2).await;
        insert(&pool, Uuid::now_v7(), "failed", 1).await;
        insert(&pool, Uuid::now_v7(), "queued", 3).await;

        let mut queue = PriorityQueue::new();
        let count = reload_queue(&pool, &mut queue).await.unwrap();

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn reloads_in_progress_states() {
        let pool = setup().await;
        insert(&pool, Uuid::now_v7(), "downloading", 2).await;
        insert(&pool, Uuid::now_v7(), "post_processing", 2).await;
        insert(&pool, Uuid::now_v7(), "importing", 1).await;

        let mut queue = PriorityQueue::new();
        let count = reload_queue(&pool, &mut queue).await.unwrap();

        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn empty_db_returns_zero() {
        let pool = setup().await;
        let mut queue = PriorityQueue::new();
        let count = reload_queue(&pool, &mut queue).await.unwrap();
        assert_eq!(count, 0);
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn interactive_priority_clamped_to_three() {
        let pool = setup().await;
        // Insert with priority 4 directly in DB (simulating a pre-restart interactive item)
        let id = Uuid::now_v7();
        let want_id = make_id_bytes(Uuid::now_v7());
        let release_id = make_id_bytes(Uuid::now_v7());
        repo::insert_queue_item(
            &pool,
            id,
            &want_id,
            &release_id,
            "magnet:?",
            "torrent",
            4,
            None,
            None,
        )
        .await
        .unwrap();

        let mut queue = PriorityQueue::new();
        reload_queue(&pool, &mut queue).await.unwrap();

        let item = queue.dequeue().unwrap();
        assert_eq!(
            item.priority, 3,
            "priority 4 should be clamped to 3 on recovery"
        );
    }
}
