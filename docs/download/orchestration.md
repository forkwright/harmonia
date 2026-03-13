# Download orchestration: Syntaxis queue, post-processing, and import pipeline

> Syntaxis owns the download queue, priority rules, concurrency control, and post-processing pipeline. Taxis owns library import, hardlink/move strategy, and file cleanup.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Syntaxis + Taxis ownership), [architecture/communication.md](../architecture/communication.md) (mpsc channel + events), [download/torrent.md](torrent.md) (Ergasia state machine + DownloadCompleted), [download/archive.md](archive.md) (extraction step), [download/usenet.md](usenet.md) (Usenet pipeline), [data/want-release.md](../data/want-release.md) (releases + haves tables).

---

## Queue architecture

Syntaxis owns the download queue. Items arrive from Episkope via direct call and are dispatched to Ergasia via mpsc channel.

### `QueueItem` type

```rust
pub struct QueueItem {
    pub id: Uuid,                    // download_queue.id — UUIDv7
    pub want_id: WantId,             // FK to wants
    pub release_id: ReleaseId,       // FK to releases
    pub download_url: String,        // magnet URI, .torrent URL, or .nzb URL
    pub protocol: DownloadProtocol,  // Torrent | Usenet
    pub priority: u8,                // 1–4, see Priority Tiers
    pub tracker_id: Option<i64>,     // FK to indexers.id (Torrent only)
    pub info_hash: Option<String>,   // torrent info hash (Torrent only)
}

pub enum DownloadProtocol {
    Torrent,
    Usenet,
}
```

### Queue flow

```
Episkope (or UI) calls Syntaxis.enqueue(item)
    |
Syntaxis inserts row into download_queue (status = 'queued')
    |
Priority 4 (interactive)? → bypass queue, send directly to Ergasia mpsc
    |
Otherwise: wait for capacity slot (max_concurrent_downloads, max_per_tracker)
    |
Slot available → update download_queue status to 'downloading'
              → set releases.grabbed_at = now()
              → send QueueItem to Ergasia via mpsc
```

**Ergasia mpsc channel:** bounded, capacity = `config.aggelia.download_queue_size` (default 512). When full, Syntaxis blocks on send; backpressure propagates to Episkope's enqueue calls. This is intentional: the queue does not grow without bound.

---

## Priority tiers

Higher number = processed first. Within the same tier, FIFO order is preserved.

| Priority | Tier | Trigger | Behavior |
|----------|------|---------|----------|
| 4 | Interactive | User-initiated from UI | Bypass queue entirely; sent directly to Ergasia mpsc |
| 3 | Wanted-missing | Episkope triggered | Item in want list with no matching have |
| 2 | Quality-upgrade | Kritike triggered | Better version available; current have below quality ceiling |
| 1 | Routine-check | Scheduled RSS monitoring | Background acquisition from RSS/schedule |

**Priority assignment:** Episkope passes a `priority: u8` field when calling `Syntaxis.enqueue()`. Kritike passes `priority: 2`. The UI HTTP handler passes `priority: 4`.

**Re-prioritization:** If a user interactively requests a download already queued at priority 1, 2, or 3, Syntaxis upgrades its priority to 4 and sends directly to Ergasia if a slot is available.

---

## Concurrency control

```toml
[syntaxis]
max_concurrent_downloads = 5   # total active downloads at once
max_per_tracker = 3            # max downloads per indexer.id simultaneously
```

- **Slot tracking:** Syntaxis maintains an in-memory counter of active downloads per tracker. When Ergasia emits `DownloadCompleted` or `DownloadFailed`, Syntaxis decrements the counter and dequeues the next eligible item.
- **max_per_tracker** applies to torrent downloads only (keyed by `tracker_id`). Usenet downloads count against `max_concurrent_downloads` but have no per-tracker limit.
- **Backpressure:** If both limits are at capacity, `Syntaxis.enqueue()` stores the item in `download_queue` and returns immediately; the item will be dispatched when a slot opens.

---

## Post-processing pipeline

Syntaxis owns the post-processing pipeline. It is triggered by the `DownloadCompleted` event from Ergasia.

```
DownloadCompleted { download_id, path } received by Syntaxis
    |
update download_queue status = 'post_processing'
    |
Step 1: Scan download path for archive files
    - Check file extensions: .rar, .r00, .zip, .7z
    - Confirm with magic byte check (see archive.md)
    - If archives found: proceed to Step 2
    - If no archives: skip to Step 3
    |
Step 2: Extract archives
    - Call Ergasia.extract(download_path)
    - Receive ExtractionResult { extracted_path, files, archive_format, nested_levels }
    - Update working path to extracted_path
    - On extraction failure: mark queue item 'failed', do NOT retry
    |
Step 3: Trigger import
update download_queue status = 'importing'
    |
    - Call Taxis.import(CompletedDownload { ... })
    - Taxis: resolve metadata via Epignosis, rename, hardlink/copy to library
    - Taxis returns ImportResult { library_path, media_id, media_type }
    |
Step 4: Post-import bookkeeping
    - Taxis emits ImportCompleted event via Aggelia
    - Taxis creates haves row
    - wants.status → 'fulfilled' (if quality threshold met)
    - update download_queue status = 'completed', completed_at = now()
    |
Step 5: Cleanup coordination
    - Torrent: seeding continues from original download_path
      (hardlink means library copy is independent — no action needed)
    - Usenet: no seeding — Taxis moves files and deletes extraction temp dir
    - SeedPolicySatisfied (Ergasia → Taxis direct call): Taxis deletes download copy
```

---

## Hardlink/move import strategy

Taxis owns the import decision. Locked decisions apply:

### During seeding (torrent)

1. Taxis calls `std::fs::hard_link(download_path, library_path)`
2. **Same filesystem:** hardlink succeeds, zero disk overhead, single inode
3. **Cross-filesystem (`EXDEV` error):** fall back to `std::fs::copy(download_path, library_path)`. Set `CompletedDownload.requires_copy = true`.
4. Seeding continues from `download_path` (the original location). The library path is the hardlinked or copied version.

### After seed policy satisfied

Ergasia calls `Taxis.on_seed_complete(download_id)` directly (authoritative cleanup signal):

1. **Hardlink was used:** `std::fs::remove_file(download_path)` (inode persists via library hardlink)
2. **Copy was used:** `std::fs::remove_file(download_path)` (library copy is the sole copy)
3. Delete empty parent directories in the download directory

### For Usenet (no seeding)

1. Taxis moves files directly: `std::fs::rename(extraction_path, library_path)`
2. **Cross-filesystem:** copy then delete source, same EXDEV pattern
3. No seeding cleanup needed

---

## `CompletedDownload` type

Passed from Syntaxis to Taxis when triggering import:

```rust
pub struct CompletedDownload {
    pub download_id: DownloadId,
    pub download_path: PathBuf,    // path to the completed download (or extracted files)
    pub source_path: PathBuf,      // original download dir — for cleanup coordination
    pub want_id: WantId,
    pub release_id: ReleaseId,
    pub protocol: DownloadProtocol,  // Torrent | Usenet
    pub requires_copy: bool,         // set by Taxis if hard_link fails (EXDEV)
}
```

`source_path` is the original download directory. For torrents with archive extraction, `download_path` points to the extracted content while `source_path` still points to the raw download dir containing the `.rar`/`.zip` files. Taxis uses `source_path` for archive cleanup after seeding.

---

## Queue persistence

SQLite table for restart recovery. The in-memory queue is rebuilt from rows with `status IN ('queued', 'downloading', 'post_processing', 'importing')` at startup.

```sql
CREATE TABLE download_queue (
    id           BLOB NOT NULL PRIMARY KEY,  -- UUIDv7 BLOB
    want_id      BLOB NOT NULL,
    release_id   BLOB NOT NULL,
    download_url TEXT NOT NULL,
    protocol     TEXT NOT NULL CHECK (protocol IN ('torrent', 'nzb')),
    priority     INTEGER NOT NULL DEFAULT 1,
    tracker_id   INTEGER,
    info_hash    TEXT,
    status       TEXT NOT NULL DEFAULT 'queued' CHECK (status IN (
                     'queued', 'downloading', 'post_processing', 'importing', 'completed', 'failed'
                 )),
    added_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    started_at   TEXT,
    completed_at TEXT
);

CREATE INDEX idx_download_queue_status_priority ON download_queue(status, priority DESC);
```

**Startup reconciliation:** At startup, Syntaxis loads all non-terminal rows (status not 'completed' or 'failed') and reconstructs in-memory state. Downloads in `downloading` or `post_processing` state at shutdown are re-queued from the top; they may re-download or resume depending on Ergasia's persistence state.

---

## Error handling and retry

### Download failure

Syntaxis receives `DownloadFailed { download_id, reason }` from Ergasia.

| Error Class | Retry Strategy |
|------------|----------------|
| Transient (network error, tracker timeout) | 3 retries: 30s, 2m, 10m exponential backoff |
| Permanent (no seeders after `stalled_download_timeout_hours`, corrupt torrent) | Mark `download_queue.status = 'failed'`, update `releases.rejected_reason` |
| Stalled (no progress for `stalled_download_timeout_hours`) | Treated as permanent failure; no retry |

After retry budget exhausted: `download_queue.status = 'failed'`, `releases.rejected_reason = <reason>`.

### Extraction failure

Syntaxis logs at error level, marks queue item `failed`. Does NOT retry; extraction failures indicate corrupt archives, not transient conditions.

### Import failure

Syntaxis logs at error level, marks queue item `failed`. Taxis cleans up any partial import (partial hardlinks, partial renames).

---

## Horismos configuration: `[syntaxis]` section

```toml
[syntaxis]
# Maximum simultaneous active downloads across all protocols
max_concurrent_downloads = 5

# Maximum downloads from a single tracker simultaneously (torrent only)
max_per_tracker = 3

# Download retry attempts before permanent failure
retry_count = 3

# Base interval for retry backoff (seconds); actual intervals: base, base*4, base*20
retry_backoff_base_seconds = 30

# Hours of no progress before a download is marked permanently failed
stalled_download_timeout_hours = 24
```

`SyntaxisConfig` struct in `crates/horismos/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyntaxisConfig {
    pub max_concurrent_downloads: usize,       // default: 5
    pub max_per_tracker: usize,                // default: 3
    pub retry_count: u32,                      // default: 3
    pub retry_backoff_base_seconds: u64,       // default: 30
    pub stalled_download_timeout_hours: u64,   // default: 24
}

impl Default for SyntaxisConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 5,
            max_per_tracker: 3,
            retry_count: 3,
            retry_backoff_base_seconds: 30,
            stalled_download_timeout_hours: 24,
        }
    }
}
```
