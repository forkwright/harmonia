use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use aggelmata::ids::{DownloadId, ReleaseId, WantId};
use aggelmata::{HarmoniaEvent, create_event_bus};
use archon::import::ImportAdapter;
use ergasia::{DownloadProgress, DownloadState, ErgasiaError, ExtractionResult};
use horismos::{
    LibraryConfig, MediaType as HorismosMediaType, Section, SyntaxisConfig, TaxisConfig,
    WatcherMode,
};
use std::sync::Arc;
use syntaxis::{DownloadQueue, ImportService, QueueItem, QueueManager};
use tokio::sync::mpsc;

use super::{TestError, test_db};

// ── real-file fixture: a minimal, hand-built lofty-readable tagged FLAC ────

fn vorbis_comment_block(tags: &[(&str, &str)]) -> Vec<u8> {
    let vendor = b"kathodos-integration-test";
    let mut data = Vec::new();
    data.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    data.extend_from_slice(vendor);
    data.extend_from_slice(&(tags.len() as u32).to_le_bytes());
    for (k, v) in tags {
        let comment = format!("{k}={v}");
        data.extend_from_slice(&(comment.len() as u32).to_le_bytes());
        data.extend_from_slice(comment.as_bytes());
    }
    data
}

/// Big-endian bit packer FOR the FLAC STREAMINFO block's odd-width fields
/// (20-bit sample rate, 3-bit channel count, 5-bit bit depth, 36-bit sample
/// total) — hand-computing those byte boundaries is error-prone, so this
/// packs them programmatically instead.
struct BitWriter {
    buf: Vec<u8>,
    acc: u64,
    nbits: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            buf: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    fn write_bits(&mut self, value: u64, bits: u32) {
        self.acc = (self.acc << bits) | (value & ((1u64 << bits) - 1));
        self.nbits += bits;
        while self.nbits >= 8 {
            self.nbits -= 8;
            self.buf.push(((self.acc >> self.nbits) & 0xFF) as u8);
        }
    }

    fn finish(self) -> Vec<u8> {
        self.buf
    }
}

fn streaminfo_block() -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.write_bits(4096, 16); // min block size
    bw.write_bits(4096, 16); // max block size
    bw.write_bits(0, 24); // min frame size (unknown)
    bw.write_bits(0, 24); // max frame size (unknown)
    bw.write_bits(44100, 20); // sample rate
    bw.write_bits(1, 3); // channels - 1 (2 channels)
    bw.write_bits(15, 5); // bits per sample - 1 (16 bits)
    bw.write_bits(0, 36); // total samples (unknown — no audio frames follow)
    let mut block = bw.finish();
    block.extend_from_slice(&[0u8; 16]); // MD5 signature (all-zero = unknown, valid per spec)
    debug_assert_eq!(block.len(), 34);
    block
}

/// Builds a minimal valid FLAC file (STREAMINFO + VORBIS_COMMENT, zero audio
/// frames — lofty's properties reader tolerates an all-zero STREAMINFO and
/// no trailing audio data, see `flac::properties::read_properties`) with the
/// given tags, so `kathodos::import::tags::read_tags` exercises real lofty
/// parsing rather than a mock resolver.
fn make_tagged_flac(title: &str, artist: &str, track: &str, year: &str) -> Vec<u8> {
    let streaminfo = streaminfo_block();
    let vorbis = vorbis_comment_block(&[
        ("TITLE", title),
        ("ARTIST", artist),
        ("TRACKNUMBER", track),
        ("DATE", year),
    ]);

    let mut out = Vec::new();
    out.extend_from_slice(b"fLaC");

    out.push(0x00); // block type 0 (STREAMINFO), not last
    let len = streaminfo.len() as u32;
    out.extend_from_slice(&len.to_be_bytes()[1..4]);
    out.extend_from_slice(&streaminfo);

    out.push(0x84); // last-block flag (0x80) | block type 4 (VORBIS_COMMENT)
    let len2 = vorbis.len() as u32;
    out.extend_from_slice(&len2.to_be_bytes()[1..4]);
    out.extend_from_slice(&vorbis);

    out
}

// ── local download engine (configurable content_path, unlike the shared MockEngine) ─

struct ImportTestEngine {
    started_tx: mpsc::UnboundedSender<DownloadId>,
    content_dir: PathBuf,
}

impl ergasia::DownloadEngine for ImportTestEngine {
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
            percent_complete: 50,
            download_speed_bps: 0,
            upload_speed_bps: 0,
            peers_connected: 0,
            seeders: 0,
            eta_seconds: None,
            error: None,
        })
    }

    async fn content_path(&self, _download_id: DownloadId) -> Result<PathBuf, ErgasiaError> {
        Ok(self.content_dir.clone())
    }

    async fn extract(
        &self,
        _download_path: &std::path::Path,
        _output_dir: &std::path::Path,
    ) -> Result<Option<ExtractionResult>, ErgasiaError> {
        Ok(None)
    }
}

fn test_syntaxis_config() -> SyntaxisConfig {
    SyntaxisConfig {
        max_concurrent_downloads: 5,
        max_per_tracker: 3,
        retry_count: 2,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    }
}

async fn seed_profile_id(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT id FROM quality_profiles WHERE media_type = 'music' LIMIT 1")
        .fetch_one(pool)
        .await
        .unwrap()
}

// ── the completion→import regression (#602 keystone) ────────────────────────

#[tokio::test]
async fn download_completion_imports_into_library() -> Result<(), TestError> {
    let pool = test_db().await?;
    let profile_id = seed_profile_id(&pool).await;

    let want_id = WantId::new();
    apotheke::repo::want::insert_want(
        &pool,
        &apotheke::repo::want::Want {
            id: want_id.as_bytes().to_vec(),
            media_type: "music_album".to_string(),
            title: "Test Album".to_string(),
            registry_id: None,
            quality_profile_id: profile_id,
            status: "searching".to_string(),
            source: None,
            source_ref: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            fulfilled_at: None,
        },
    )
    .await?;

    let release_id = ReleaseId::new();
    apotheke::repo::want::insert_release(
        &pool,
        &apotheke::repo::want::Release {
            id: release_id.as_bytes().to_vec(),
            want_id: want_id.as_bytes().to_vec(),
            indexer_id: 1,
            title: "Test Album FLAC".to_string(),
            size_bytes: 1_000_000,
            quality_score: 90,
            custom_format_score: 0,
            download_url: "magnet:?xt=urn:btih:import-test".to_string(),
            protocol: "torrent".to_string(),
            info_hash: None,
            found_at: "2026-01-01T00:00:00Z".to_string(),
            grabbed_at: None,
            rejected_reason: None,
        },
    )
    .await?;

    // A tempdir download directory: one lofty-tagged .flac + one junk .nfo,
    // proving walk_library's extension filter runs.
    let download_dir = tempfile::TempDir::new()?;
    std::fs::write(
        download_dir.path().join("01 - Track.flac"),
        make_tagged_flac("Test Track", "Test Artist", "1", "2024"),
    )?;
    std::fs::write(download_dir.path().join("release.nfo"), b"not a media file")?;

    // A tempdir library root, wired as the sole Music library.
    let library_root = tempfile::TempDir::new()?;
    let mut libraries = HashMap::new();
    libraries.insert(
        "music".to_string(),
        LibraryConfig {
            path: library_root.path().to_path_buf(),
            media_type: HorismosMediaType::Music,
            watcher_mode: WatcherMode::Auto,
            poll_interval_seconds: 300,
            scan_interval_hours: 24,
        },
    );
    let taxis = Section::fixed(TaxisConfig {
        libraries,
        ..TaxisConfig::default()
    });

    let (event_tx, _) = create_event_bus(64);
    let import_adapter: Arc<dyn ImportService> = Arc::new(ImportAdapter::new(
        pool.clone(),
        pool.clone(),
        taxis,
        event_tx.clone(),
    ));

    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let engine = Arc::new(ImportTestEngine {
        started_tx,
        content_dir: download_dir.path().to_path_buf(),
    });

    let svc = Arc::new(
        DownloadQueue::new(pool.clone(), engine, import_adapter, test_syntaxis_config()).await?,
    );
    let shutdown = tokio_util::sync::CancellationToken::new();
    let mut assert_rx = event_tx.subscribe();
    let _listener = svc.start(event_tx.subscribe(), shutdown.clone());

    let queue_id = uuid::Uuid::now_v7();
    svc.enqueue(QueueItem {
        id: queue_id,
        want_id,
        release_id,
        download_url: "magnet:?xt=urn:btih:import-test".to_string(),
        protocol: syntaxis::DownloadProtocol::Torrent,
        priority: 4,
        tracker_id: None,
        info_hash: None,
        retry_count: 0,
    })
    .await?;

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should have received start_download");

    event_tx.send(HarmoniaEvent::DownloadCompleted {
        download_id: dl_id,
        path: download_dir.path().to_path_buf(),
    })?;

    // Poll for terminal queue state — the pipeline runs on a spawned task.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut status = String::new();
    while tokio::time::Instant::now() < deadline {
        status = sqlx::query_scalar("SELECT status FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
        if status == "completed" || status == "failed" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        status, "completed",
        "the pipeline must reach a terminal 'completed' state"
    );

    // The imported file landed at its canonical templated path.
    let expected_path = library_root
        .path()
        .join("Test Artist")
        .join("Test Album (2024)")
        .join("01 - Test Track.flac");
    assert!(
        expected_path.exists(),
        "expected imported file at {expected_path:?}; library_root contains: {:?}",
        walk_dir_names(library_root.path())
    );

    // haves row is complete, want is fulfilled.
    let have_status: String = sqlx::query_scalar("SELECT status FROM haves WHERE want_id = ?")
        .bind(want_id.as_bytes().as_slice())
        .fetch_one(&pool)
        .await?;
    assert_eq!(have_status, "complete");

    let want_status: String = sqlx::query_scalar("SELECT status FROM wants WHERE id = ?")
        .bind(want_id.as_bytes().as_slice())
        .fetch_one(&pool)
        .await?;
    assert_eq!(want_status, "fulfilled");

    // ImportCompleted was published on the bus.
    let mut saw_import_completed = false;
    while let Ok(event) = tokio::time::timeout(Duration::from_millis(200), assert_rx.recv()).await {
        if matches!(event?, HarmoniaEvent::ImportCompleted { .. }) {
            saw_import_completed = true;
            break;
        }
    }
    assert!(
        saw_import_completed,
        "expected an ImportCompleted event on the bus"
    );

    shutdown.cancel();
    Ok(())
}

fn walk_dir_names(root: &std::path::Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .collect()
}
