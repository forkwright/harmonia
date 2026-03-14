//! Post-processing pipeline: archive scan → extraction → import trigger.
//!
//! Triggered by `DownloadCompleted` events from Ergasia. Each step is idempotent:
//! if the service restarts mid-pipeline, re-processing produces the same result.

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use sqlx::SqlitePool;
use tracing::{error, info, instrument};
use uuid::Uuid;

use ergasia::DownloadEngine;
use harmonia_common::ids::{DownloadId, ReleaseId, WantId};

use crate::error::SyntaxisError;
use crate::repo;
use crate::types::{CompletedDownload, DownloadProtocol};

/// Trait boundary for the Taxis import service. Wired in P3-08.
///
/// Uses `Pin<Box<dyn Future>>` for dyn compatibility — callers can store
/// `Arc<dyn ImportService>` without the object-safety issues of native async fn.
pub trait ImportService: Send + Sync {
    fn import(
        &self,
        completed: CompletedDownload,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>;
}

/// Per-item metadata passed to the post-processing pipeline.
///
/// Groups the download-specific context that is known at dispatch time,
/// keeping `run_pipeline`'s argument list under the Clippy limit.
#[derive(Debug, Clone)]
pub(crate) struct PipelineItem {
    pub queue_id: Uuid,
    pub want_id: WantId,
    pub release_id: ReleaseId,
    pub protocol: DownloadProtocol,
    pub tracker_id: Option<i64>,
}

/// Runs the post-processing pipeline for a completed download.
///
/// Steps:
/// 1. Mark status = 'post_processing'
/// 2. Scan for archives; extract if found
/// 3. Mark status = 'importing', call import service
/// 4. Mark status = 'completed'
#[instrument(skip_all, fields(download_id = %download_id))]
pub(crate) async fn run_pipeline<E: DownloadEngine>(
    pool: &SqlitePool,
    engine: &Arc<E>,
    import_svc: &Arc<dyn ImportService>,
    download_id: DownloadId,
    download_path: &Path,
    item: PipelineItem,
) -> Result<(), SyntaxisError> {
    let PipelineItem {
        queue_id,
        want_id,
        release_id,
        protocol,
        tracker_id: _tracker_id,
    } = item;
    repo::update_status(pool, queue_id, "post_processing")
        .await
        .map_err(|source| SyntaxisError::Database {
            source,
            location: snafu::location!(),
        })?;

    let source_path = download_path.to_path_buf();

    // Step 1: try extraction. On failure, mark failed and return immediately.
    let working_path = match engine.extract(download_path, download_path) {
        Ok(Some(result)) => {
            info!(extracted_path = %result.extracted_path.display(), "extracted archives");
            result.extracted_path
        }
        Ok(None) => {
            // No archives found — work with the original path.
            download_path.to_path_buf()
        }
        Err(source) => {
            let msg = source.to_string();
            error!(error = %msg, "archive extraction failed");
            repo::mark_failed(pool, queue_id, &format!("extraction failed: {msg}"))
                .await
                .map_err(|db_source| SyntaxisError::Database {
                    source: db_source,
                    location: snafu::location!(),
                })?;
            return Err(SyntaxisError::ExtractionFailed {
                source,
                location: snafu::location!(),
            });
        }
    };

    // Step 2: trigger import.
    repo::update_status(pool, queue_id, "importing")
        .await
        .map_err(|source| SyntaxisError::Database {
            source,
            location: snafu::location!(),
        })?;

    let completed = CompletedDownload {
        download_id,
        download_path: working_path,
        source_path,
        want_id,
        release_id,
        protocol,
        requires_copy: false,
    };

    if let Err(reason) = import_svc.import(completed).await {
        error!(%reason, "import failed");
        repo::mark_failed(pool, queue_id, &format!("import failed: {reason}"))
            .await
            .map_err(|source| SyntaxisError::Database {
                source,
                location: snafu::location!(),
            })?;
        return Err(SyntaxisError::ImportFailed {
            reason,
            location: snafu::location!(),
        });
    }

    // Step 3: bookkeeping complete.
    let now = jiff::Timestamp::now().to_string();
    repo::mark_completed(pool, queue_id, &now)
        .await
        .map_err(|source| SyntaxisError::Database {
            source,
            location: snafu::location!(),
        })?;

    info!("post-processing pipeline complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use ergasia::{DownloadProgress, ErgasiaError, ExtractionResult};
    use harmonia_common::ids::{DownloadId, ReleaseId, WantId};
    use harmonia_db::migrate::MIGRATOR;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use super::PipelineItem;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    async fn make_queue_row(pool: &SqlitePool, id: Uuid) {
        let want_id = Uuid::now_v7().as_bytes().to_vec();
        let release_id = Uuid::now_v7().as_bytes().to_vec();
        repo::insert_queue_item(
            pool,
            id,
            &want_id,
            &release_id,
            "magnet:?xt=urn:btih:test",
            "torrent",
            2,
            None,
            None,
        )
        .await
        .unwrap();
    }

    // --- mock engines ---

    struct NoArchiveEngine;
    impl DownloadEngine for NoArchiveEngine {
        async fn start_download(
            &self,
            _req: ergasia::DownloadRequest,
        ) -> Result<DownloadId, ErgasiaError> {
            Ok(DownloadId::new())
        }
        async fn cancel_download(&self, _id: DownloadId) -> Result<(), ErgasiaError> {
            Ok(())
        }
        async fn get_progress(&self, _id: DownloadId) -> Result<DownloadProgress, ErgasiaError> {
            unimplemented!()
        }
        fn extract(
            &self,
            _path: &Path,
            _out: &Path,
        ) -> Result<Option<ExtractionResult>, ErgasiaError> {
            Ok(None)
        }
    }

    struct FailingExtractEngine;
    impl DownloadEngine for FailingExtractEngine {
        async fn start_download(
            &self,
            _req: ergasia::DownloadRequest,
        ) -> Result<DownloadId, ErgasiaError> {
            Ok(DownloadId::new())
        }
        async fn cancel_download(&self, _id: DownloadId) -> Result<(), ErgasiaError> {
            Ok(())
        }
        async fn get_progress(&self, _id: DownloadId) -> Result<DownloadProgress, ErgasiaError> {
            unimplemented!()
        }
        fn extract(
            &self,
            _path: &Path,
            _out: &Path,
        ) -> Result<Option<ExtractionResult>, ErgasiaError> {
            Err(ErgasiaError::ExtractFile {
                path: PathBuf::from("/tmp/corrupt.rar"),
                error: "corrupt archive".to_string(),
                location: snafu::location!(),
            })
        }
    }

    // --- mock import services ---

    struct OkImportService;
    impl ImportService for OkImportService {
        fn import(
            &self,
            _c: CompletedDownload,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
            Box::pin(async { Ok(()) })
        }
    }

    struct FailImportService;
    impl ImportService for FailImportService {
        fn import(
            &self,
            _c: CompletedDownload,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
            Box::pin(async { Err("metadata not found".to_string()) })
        }
    }

    #[tokio::test]
    async fn pipeline_success_marks_completed() {
        let pool = setup().await;
        let queue_id = Uuid::now_v7();
        make_queue_row(&pool, queue_id).await;

        let engine: Arc<NoArchiveEngine> = Arc::new(NoArchiveEngine);
        let import: Arc<dyn ImportService> = Arc::new(OkImportService);

        let result = run_pipeline(
            &pool,
            &engine,
            &import,
            DownloadId::new(),
            Path::new("/data/downloads/album"),
            PipelineItem {
                queue_id,
                want_id: WantId::new(),
                release_id: ReleaseId::new(),
                protocol: DownloadProtocol::Torrent,
                tracker_id: None,
            },
        )
        .await;

        assert!(result.is_ok());
        let row = repo::get_queue_item(&pool, queue_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, "completed");
    }

    #[tokio::test]
    async fn pipeline_extraction_failure_marks_failed() {
        let pool = setup().await;
        let queue_id = Uuid::now_v7();
        make_queue_row(&pool, queue_id).await;

        let engine: Arc<FailingExtractEngine> = Arc::new(FailingExtractEngine);
        let import: Arc<dyn ImportService> = Arc::new(OkImportService);

        let result = run_pipeline(
            &pool,
            &engine,
            &import,
            DownloadId::new(),
            Path::new("/data/downloads/album"),
            PipelineItem {
                queue_id,
                want_id: WantId::new(),
                release_id: ReleaseId::new(),
                protocol: DownloadProtocol::Torrent,
                tracker_id: None,
            },
        )
        .await;

        assert!(result.is_err());
        let row = repo::get_queue_item(&pool, queue_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, "failed");
    }

    #[tokio::test]
    async fn pipeline_import_failure_marks_failed() {
        let pool = setup().await;
        let queue_id = Uuid::now_v7();
        make_queue_row(&pool, queue_id).await;

        let engine: Arc<NoArchiveEngine> = Arc::new(NoArchiveEngine);
        let import: Arc<dyn ImportService> = Arc::new(FailImportService);

        let result = run_pipeline(
            &pool,
            &engine,
            &import,
            DownloadId::new(),
            Path::new("/data/downloads/album"),
            PipelineItem {
                queue_id,
                want_id: WantId::new(),
                release_id: ReleaseId::new(),
                protocol: DownloadProtocol::Torrent,
                tracker_id: None,
            },
        )
        .await;

        assert!(result.is_err());
        let row = repo::get_queue_item(&pool, queue_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, "failed");
    }
}
