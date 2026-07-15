//! #609 acceptance: drive search -> enqueue -> observe completion ENTIRELY
//! through the MCP acquisition surface — the real serve-hosted Unix-domain
//! socket bridge over a real `syntaxis::DownloadQueue`, no HTTP router, no
//! exousia auth, no startup banner.
//!
//! The "MCP surface" here is the bridge socket's newline-delimited JSON-RPC
//! wire — exactly what `harmonia mcp`'s stdio process forwards. A raw
//! `UnixStream` client speaking that wire is the honest end-to-end driver.

use std::sync::Arc;
use std::time::Duration;

use apotheke::DbPools;
use apotheke::migrate::MIGRATOR;
use archon::mcp_bridge::{BridgeContext, spawn};
use ergasia::{DownloadProgress, DownloadState, ErgasiaError, ExtractionResult};
use paroche::state::{
    DynQueueManager, DynSearchService, EnqueueItem, ResolvedRelease, ServiceError, ServiceFut,
};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use syntaxis::{CompletedDownload, DownloadQueue, ImportService, QueueManager};
use themelion::ids::DownloadId;
use themelion::{HarmoniaEvent, create_event_bus};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

type TestError = Box<dyn std::error::Error + Send + Sync>; // kanon:ignore RUST/box-dyn-error -- integration test helper

// ── A stub indexer that both searches and resolves a release by reference ──

struct AcceptanceSearch {
    magnet: String,
    release_id: Uuid,
}

impl DynSearchService for AcceptanceSearch {
    fn search(&self, _query: Value) -> ServiceFut<Value> {
        let magnet = self.magnet.clone();
        let release_id = self.release_id;
        Box::pin(async move {
            Ok(json!({
                "results": [{
                    "release_id": release_id.to_string(),
                    "title": "Kind of Blue - FLAC",
                    "download_url": magnet,
                    "protocol": "torrent"
                }]
            }))
        })
    }
    fn test_indexer(&self, _indexer_id: i64) -> ServiceFut<Value> {
        Box::pin(async { Ok(json!({})) })
    }
    fn refresh_caps(&self, _indexer_id: i64) -> ServiceFut<Value> {
        Box::pin(async { Ok(json!({})) })
    }
    fn cached_results(&self, _query_id: Uuid) -> ServiceFut<Value> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
    fn resolve_release(&self, release_id: Uuid) -> ServiceFut<ResolvedRelease> {
        let magnet = self.magnet.clone();
        let known = self.release_id;
        Box::pin(async move {
            if release_id == known {
                Ok(ResolvedRelease {
                    download_url: magnet,
                    protocol: "torrent".to_string(),
                    info_hash: None,
                })
            } else {
                Err(ServiceError::NotFound)
            }
        })
    }
}

// ── The live queue-manager adapter over a real DownloadQueue<MockEngine> ───

struct QueueAdapter(Arc<DownloadQueue<MockEngine>>);

impl DynQueueManager for QueueAdapter {
    fn enqueue(&self, item: EnqueueItem) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            let protocol = syntaxis::DownloadProtocol::parse(&item.protocol)
                .ok_or_else(|| ServiceError::InvalidInput("unsupported protocol".to_string()))?;
            service
                .enqueue(syntaxis::QueueItem {
                    id: item.queue_id,
                    want_id: themelion::WantId::from_uuid(item.want_id),
                    release_id: themelion::ReleaseId::from_uuid(item.release_id),
                    download_url: item.download_url,
                    protocol,
                    priority: item.priority,
                    tracker_id: None,
                    info_hash: item.info_hash,
                    retry_count: 0,
                })
                .await
                .map(|_| ())
                .map_err(|e| ServiceError::Internal(e.to_string()))
        })
    }
    fn cancel(&self, queue_id: Uuid) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .cancel_by_queue_id(queue_id)
                .await
                .map_err(|e| match e {
                    syntaxis::SyntaxisError::ItemNotFound { .. } => ServiceError::NotFound,
                    other => ServiceError::Internal(other.to_string()),
                })
        })
    }
    fn reprioritize(&self, queue_id: Uuid, priority: u8) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .reprioritize_by_queue_id(queue_id, priority)
                .await
                .map_err(|e| ServiceError::Internal(e.to_string()))
        })
    }
}

// ── A MockEngine that dispatches and reports its started ids ───────────────

struct MockEngine {
    started_tx: mpsc::UnboundedSender<DownloadId>,
}

impl ergasia::DownloadEngine for MockEngine {
    async fn start_download(
        &self,
        request: ergasia::DownloadRequest,
    ) -> Result<DownloadId, ErgasiaError> {
        let _ = self.started_tx.send(request.download_id);
        Ok(request.download_id)
    }
    async fn cancel_download(&self, _download_id: DownloadId) -> Result<(), ErgasiaError> {
        Ok(())
    }
    async fn get_progress(
        &self,
        download_id: DownloadId,
    ) -> Result<DownloadProgress, ErgasiaError> {
        Ok(DownloadProgress {
            download_id,
            state: DownloadState::Downloading,
            percent_complete: 100,
            download_speed_bps: 0,
            upload_speed_bps: 0,
            peers_connected: 1,
            seeders: 1,
            eta_seconds: Some(0),
            error: None,
        })
    }
    async fn content_path(
        &self,
        _download_id: DownloadId,
    ) -> Result<std::path::PathBuf, ErgasiaError> {
        Ok(std::path::PathBuf::from("/data/downloads/kind-of-blue"))
    }
    async fn extract(
        &self,
        _download_path: &std::path::Path,
        _output_dir: &std::path::Path,
    ) -> Result<Option<ExtractionResult>, ErgasiaError> {
        Ok(None)
    }
}

struct MockImportService;

impl ImportService for MockImportService {
    fn import(
        &self,
        _completed: CompletedDownload,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

async fn test_db() -> Result<SqlitePool, TestError> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

// ── MCP wire client ─────────────────────────────────────────────────────────

/// Speaks the bridge's newline-delimited JSON-RPC over one reused socket
/// connection — the same wire `harmonia mcp` forwards. Returns the tool
/// `result` object.
async fn call_tool(
    reader: &mut BufReader<UnixStream>,
    id: i64,
    name: &str,
    arguments: Value,
) -> Result<Value, TestError> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments }
    });
    let mut line = serde_json::to_string(&request)?;
    line.push('\n');
    reader.get_mut().write_all(line.as_bytes()).await?;
    reader.get_mut().flush().await?;

    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;
    let response: Value = serde_json::from_str(&response_line)?;
    Ok(response["result"].clone())
}

#[tokio::test]
async fn search_enqueue_and_observe_completion_over_the_mcp_surface() -> Result<(), TestError> {
    let pool = test_db().await?;
    let db = Arc::new(DbPools {
        read: pool.clone(),
        write: pool.clone(),
    });
    let (event_tx, _) = create_event_bus(64);

    // Real syntaxis DownloadQueue + a MockEngine, listener wired to the bus.
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let queue_svc = Arc::new(
        DownloadQueue::new(
            pool.clone(),
            Arc::new(MockEngine { started_tx }),
            Arc::new(MockImportService) as Arc<dyn ImportService>,
            horismos::SyntaxisConfig {
                max_concurrent_downloads: 5,
                max_per_tracker: 3,
                retry_count: 1,
                retry_backoff_base_seconds: 0,
                stalled_download_timeout_hours: 24,
            },
        )
        .await?,
    );
    let queue_shutdown = CancellationToken::new();
    let _listener = queue_svc.start(event_tx.subscribe(), queue_shutdown.clone());

    let release_id = Uuid::now_v7();
    let bridge_ctx = BridgeContext {
        search: Arc::new(AcceptanceSearch {
            magnet: "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd".to_string(),
            release_id,
        }),
        queue: Arc::new(QueueAdapter(Arc::clone(&queue_svc))),
        db,
    };

    let dir = tempfile::tempdir()?;
    let socket_path = dir.path().join("harmonia-mcp.sock");
    let bridge_shutdown = CancellationToken::new();
    let bridge_handle = spawn(socket_path.clone(), bridge_ctx, bridge_shutdown.clone()).await?;

    // Once the engine reports a dispatch, complete it on the event bus — a
    // MockEngine that "completes" driving the row through the pipeline.
    let completion_bus = event_tx.clone();
    tokio::spawn(async move {
        if let Some(download_id) = started_rx.recv().await {
            let _ = completion_bus.send(HarmoniaEvent::DownloadCompleted {
                download_id,
                path: std::path::PathBuf::from("/data/downloads/kind-of-blue"),
            });
        }
    });

    let stream = UnixStream::connect(&socket_path).await?;
    let mut reader = BufReader::new(stream);

    // 1. SEARCH — a release_id comes back, no raw credential leaks.
    let search_result = call_tool(
        &mut reader,
        1,
        "harmonia_search_releases",
        json!({ "query_text": "Kind of Blue", "media_type": "music" }),
    )
    .await?;
    assert_eq!(search_result["isError"], false, "{search_result:?}");
    let results = search_result["structuredContent"]["results"]
        .as_array()
        .expect("results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["release_id"], release_id.to_string());

    // 2. ENQUEUE by release_id — the credentialed url is resolved + validated
    //    server-side; the persisted row id comes back for observation.
    let enqueue_result = call_tool(
        &mut reader,
        2,
        "harmonia_enqueue_download",
        json!({ "release_id": release_id.to_string(), "priority": 4 }),
    )
    .await?;
    assert_eq!(enqueue_result["isError"], false, "{enqueue_result:?}");
    let download_id = enqueue_result["structuredContent"]["id"]
        .as_str()
        .expect("enqueue returns the queue row id")
        .to_string();

    // 3. POLL LIST until the row settles as completed — the completion
    //    observability surface, entirely over MCP.
    let mut final_status = String::new();
    for _ in 0..100 {
        let list_result = call_tool(
            &mut reader,
            3,
            "harmonia_list_downloads",
            json!({ "id": download_id }),
        )
        .await?;
        assert_eq!(list_result["isError"], false, "{list_result:?}");
        let downloads = list_result["structuredContent"]["downloads"]
            .as_array()
            .expect("downloads array");
        if let Some(row) = downloads.first() {
            final_status = row["status"].as_str().unwrap_or("").to_string();
            if final_status == "completed" {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        final_status, "completed",
        "the enqueued download must settle as completed, observed only through the MCP surface"
    );

    bridge_shutdown.cancel();
    bridge_handle.await?;
    queue_shutdown.cancel();
    Ok(())
}
