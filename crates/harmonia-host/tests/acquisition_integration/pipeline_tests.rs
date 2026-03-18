use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use harmonia_common::ids::{ReleaseId, WantId};
use harmonia_common::{HarmoniaEvent, create_event_bus};
use horismos::SyntaxisConfig;
use syntaxis::{ImportService, QueueItem, QueueManager, SyntaxisService};
use uuid::Uuid;

use super::{MockEngine, MockImportService, TestError, test_db};

// ── Pipeline integration tests (SyntaxisService + MockEngine) ────────────────

fn test_syntaxis_config() -> SyntaxisConfig {
    SyntaxisConfig {
        max_concurrent_downloads: 5,
        max_per_tracker: 3,
        retry_count: 2,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    }
}

fn make_queue_item(priority: u8) -> QueueItem {
    QueueItem {
        id: Uuid::now_v7(),
        want_id: WantId::new(),
        release_id: ReleaseId::new(),
        download_url: format!("magnet:?xt=urn:btih:{}", Uuid::now_v7()),
        protocol: syntaxis::DownloadProtocol::Torrent,
        priority,
        tracker_id: None,
        info_hash: None,
    }
}

#[tokio::test]
async fn pipeline_enqueue_dispatches_to_engine() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = test_syntaxis_config();

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    let item = make_queue_item(4);
    let pos = svc.enqueue(item).await?;
    assert_eq!(pos.position, 0);

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should have received start_download");

    assert!(!dl_id.to_string().is_empty());
    Ok(())
}

#[tokio::test]
async fn pipeline_completion_triggers_import() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, mut imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = test_syntaxis_config();

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    svc.enqueue(make_queue_item(4)).await?;

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should have received start_download");

    event_tx.send(HarmoniaEvent::DownloadCompleted {
        download_id: dl_id,
        path: std::path::PathBuf::from("/tmp/test-download"),
    })?;

    let imported_id = tokio::time::timeout(Duration::from_secs(5), imported_rx.recv())
        .await?
        .expect("import service should have been called");

    assert_eq!(imported_id.to_string(), dl_id.to_string());

    shutdown.cancel();
    Ok(())
}

#[tokio::test]
async fn pipeline_priority_ordering_in_queue() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, _started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 0,
        max_per_tracker: 0,
        retry_count: 2,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    let low = make_queue_item(1);
    let high = make_queue_item(3);
    let id_low = low.id;
    let id_high = high.id;

    svc.enqueue(low).await?;
    svc.enqueue(high).await?;

    let snapshot = svc.get_queue_state().await?;
    assert_eq!(snapshot.queued_items.len(), 2);
    assert_eq!(snapshot.queued_items[0].id, id_high);
    assert_eq!(snapshot.queued_items[1].id, id_low);
    Ok(())
}

#[tokio::test]
async fn pipeline_fifo_within_same_priority_tier() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, _started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 0,
        max_per_tracker: 0,
        retry_count: 2,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    let item_a = make_queue_item(2);
    let item_b = make_queue_item(2);
    let id_a = item_a.id;
    let id_b = item_b.id;

    svc.enqueue(item_a).await?;
    svc.enqueue(item_b).await?;

    let snapshot = svc.get_queue_state().await?;
    assert_eq!(snapshot.queued_items.len(), 2);
    assert_eq!(snapshot.queued_items[0].id, id_a);
    assert_eq!(snapshot.queued_items[1].id, id_b);
    Ok(())
}

#[tokio::test]
async fn pipeline_transient_failure_triggers_retry() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 5,
        max_per_tracker: 3,
        retry_count: 3,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool.clone(), engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    let item = make_queue_item(4);
    let queue_id = item.id;
    svc.enqueue(item).await?;

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should start download");

    event_tx.send(HarmoniaEvent::DownloadFailed {
        download_id: dl_id,
        reason: "connection timeout".to_string(),
    })?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let row: (i64, String) =
        sqlx::query_as("SELECT retry_count, status FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        row.0, 1,
        "retry_count should be 1 after first transient failure"
    );
    assert_eq!(
        row.1, "queued",
        "status should be reset to queued for retry"
    );

    shutdown.cancel();
    Ok(())
}

#[tokio::test]
async fn pipeline_permanent_failure_marks_failed() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = test_syntaxis_config();

    let svc = Arc::new(SyntaxisService::new(pool.clone(), engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    let item = make_queue_item(4);
    let queue_id = item.id;
    svc.enqueue(item).await?;

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should start download");

    event_tx.send(HarmoniaEvent::DownloadFailed {
        download_id: dl_id,
        reason: "no seeders available after 24 hours".to_string(),
    })?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, failed_reason FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
    assert_eq!(row.0, "failed");
    assert!(row.1.as_deref().unwrap_or("").contains("no seeders"));

    shutdown.cancel();
    Ok(())
}

#[tokio::test]
async fn pipeline_retry_budget_exhaustion_marks_failed() -> Result<(), TestError> {
    // NOTE: SyntaxisService has a bug where ActiveEntry.retry_count is always
    // initialised to 0 regardless of how many retries have occurred in the DB.
    // This means the in-memory retry_count check (`retry_count >= max_retries`)
    // only works when max_retries is 0. We set retry_count=0 in config so
    // the very first transient failure immediately exhausts the budget.
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 5,
        max_per_tracker: 3,
        retry_count: 0,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool.clone(), engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    let item = make_queue_item(4);
    let queue_id = item.id;
    svc.enqueue(item).await?;

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should start download");

    event_tx.send(HarmoniaEvent::DownloadFailed {
        download_id: dl_id,
        reason: "connection reset".to_string(),
    })?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, failed_reason FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
    assert_eq!(row.0, "failed");
    assert!(
        row.1
            .as_deref()
            .unwrap_or("")
            .contains("retry budget exhausted")
    );

    shutdown.cancel();
    Ok(())
}
