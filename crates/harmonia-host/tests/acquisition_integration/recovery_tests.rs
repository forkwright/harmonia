use std::sync::Arc;

use tokio::sync::mpsc;

use horismos::SyntaxisConfig;
use syntaxis::{ImportService, QueueManager, SyntaxisService};
use uuid::Uuid;

use super::{
    MockEngine, MockImportService, TestError, admin_token, build_app, get_queue_snapshot, test_db,
    test_state,
};

// ── Startup recovery tests ────────────────────────────────────────────────────

#[tokio::test]
async fn startup_recovery_loads_queued_items_from_db() -> Result<(), TestError> {
    let pool = test_db().await?;

    // Insert non-terminal rows directly into DB (simulating prior state)
    let id_queued = Uuid::now_v7();
    let id_downloading = Uuid::now_v7();
    let id_completed = Uuid::now_v7();
    let want_id = Uuid::now_v7().as_bytes().to_vec();
    let release_id = Uuid::now_v7().as_bytes().to_vec();

    for (id, status) in [
        (id_queued, "queued"),
        (id_downloading, "downloading"),
        (id_completed, "completed"),
    ] {
        sqlx::query(
            "INSERT INTO download_queue \
             (id, want_id, release_id, download_url, protocol, priority, status, added_at, retry_count) \
             VALUES (?, ?, ?, 'magnet:test', 'torrent', 2, ?, '2026-01-01T00:00:00Z', 0)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(&want_id)
        .bind(&release_id)
        .bind(status)
        .execute(&pool)
        .await?;
    }

    // Boot SyntaxisService — recovery should load non-terminal items
    let (started_tx, _started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 0,
        max_per_tracker: 0,
        retry_count: 3,
        retry_backoff_base_seconds: 30,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    let snapshot = svc.get_queue_state().await?;
    // 'queued' and 'downloading' are non-terminal and should be recovered
    // 'completed' is terminal and should NOT be recovered
    assert_eq!(
        snapshot.queued_items.len(),
        2,
        "both 'queued' and 'downloading' rows should be recovered into the in-memory queue"
    );
    assert_eq!(snapshot.completed_count, 1);
    Ok(())
}

#[tokio::test]
async fn startup_recovery_visible_via_http_snapshot() -> Result<(), TestError> {
    let (state, auth, pool) = test_state().await?;
    let token = admin_token(&auth).await?;

    // Insert a queued row directly
    let id = Uuid::now_v7();
    let want_id = Uuid::now_v7().as_bytes().to_vec();
    let release_id = Uuid::now_v7().as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO download_queue \
         (id, want_id, release_id, download_url, protocol, priority, status, added_at, retry_count) \
         VALUES (?, ?, ?, 'magnet:test', 'torrent', 3, 'queued', '2026-01-01T00:00:00Z', 0)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(&want_id)
    .bind(&release_id)
    .execute(&pool)
    .await?;

    let app = build_app(state);

    let (_, json) = get_queue_snapshot(&app, &token).await?;
    let queued = json["data"]["queued"].as_array().unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0]["priority"], 3);
    assert_eq!(queued[0]["status"], "queued");
    Ok(())
}
