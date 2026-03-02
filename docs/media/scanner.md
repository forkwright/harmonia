# Library Scanner — File Watcher, NFS Fallback, and Scan Pipeline

> Taxis owns the library scanner. Files discovered by the scanner enter the `discovered` lifecycle state.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Taxis ownership), [architecture/communication.md](../architecture/communication.md) (LibraryScanCompleted event), [architecture/configuration.md](../architecture/configuration.md) ([taxis] section), [media/lifecycle.md](lifecycle.md) (discovered state).

---

## Scanner Architecture

Taxis combines real-time file watching with scheduled full scans. Both paths feed the same processing pipeline — the pipeline does not care whether a file was discovered by the watcher or the scheduled scan.

| Mode | Mechanism | Purpose |
|------|-----------|---------|
| Real-time | inotify (Linux) / FSEvents (macOS) via `notify` crate | Instant discovery on local filesystems |
| Scheduled | Full directory walk every `scan_interval_hours` (default: 6h) | Backstop for missed events and library reconciliation |
| NFS fallback | `PollWatcher` with configurable interval | Network-mounted filesystems where inotify is unreliable |

One watcher per library, not one global watcher. Each watcher runs in its own Tokio task.

---

## Library Configuration

Per-library settings live in `[taxis.libraries]` in `harmonia.toml`.

```toml
[taxis.libraries.music]
path = "/media/music"
media_type = "music"
watcher_mode = "auto"          # "inotify" | "poll" | "auto"
poll_interval_seconds = 30
auto_import = true             # false = manual review mode
scan_interval_hours = 6

[taxis.libraries.movies]
path = "/media/movies"
media_type = "movie"
watcher_mode = "auto"
auto_import = true
scan_interval_hours = 6

[taxis.libraries.audiobooks]
path = "/media/audiobooks"
media_type = "audiobook"
watcher_mode = "auto"
auto_import = true
scan_interval_hours = 6
```

**Design rules:**

- Each library has a single `media_type` — no mixed-type libraries. This simplifies media detection and naming templates.
- Multiple libraries can share the same `media_type` (e.g., two music libraries on different mounts).
- `watcher_mode = "auto"`: try `RecommendedWatcher`. If the mount is detected as NFS (via `/proc/mounts` or `nix::sys::statvfs`), fall back to `PollWatcher`.
- `auto_import = false` queues discovered files for manual review instead of importing immediately.

---

## File Watcher Implementation

Using `notify` 8.2.0. Both `RecommendedWatcher` (inotify) and `PollWatcher` are available from the same crate.

```rust
fn create_watcher(
    lib: &LibraryConfig,
    tx: mpsc::Sender<WatchEvent>,
) -> Result<Box<dyn Watcher>, TaxisError> {
    match detect_watcher_mode(lib) {
        WatcherMode::Inotify => {
            let mut w = RecommendedWatcher::new(
                move |res| { let _ = tx.blocking_send(WatchEvent::from(res)); },
                Config::default(),
            ).context(WatcherInitSnafu { library: lib.name.clone() })?;
            w.watch(&lib.path, RecursiveMode::Recursive)
                .context(WatcherInitSnafu { library: lib.name.clone() })?;
            Ok(Box::new(w))
        }
        WatcherMode::Poll => {
            let config = Config::default()
                .with_poll_interval(Duration::from_secs(lib.poll_interval_seconds));
            let mut w = PollWatcher::new(
                move |res| { let _ = tx.blocking_send(WatchEvent::from(res)); },
                config,
            ).context(WatcherInitSnafu { library: lib.name.clone() })?;
            w.watch(&lib.path, RecursiveMode::Recursive)
                .context(WatcherInitSnafu { library: lib.name.clone() })?;
            Ok(Box::new(w))
        }
    }
}
```

**Event debouncing:** A 500ms window collapses multiple rapid events for the same file into one. This prevents duplicate pipeline entries when editors write files in stages (save → temp → rename pattern).

---

## NFS Detection

Auto mode (`watcher_mode = "auto"`) checks `/proc/mounts` to determine whether the library path is on a network mount:

```rust
fn detect_watcher_mode(lib: &LibraryConfig) -> WatcherMode {
    match lib.watcher_mode {
        WatcherModeConfig::Inotify => WatcherMode::Inotify,
        WatcherModeConfig::Poll    => WatcherMode::Poll,
        WatcherModeConfig::Auto    => {
            if is_network_mount(&lib.path) {
                tracing::info!(library = %lib.name, "NFS mount detected — using PollWatcher");
                WatcherMode::Poll
            } else {
                WatcherMode::Inotify
            }
        }
    }
}

fn is_network_mount(path: &Path) -> bool {
    const NETWORK_FS_TYPES: &[&str] = &["nfs", "nfs4", "cifs", "smbfs", "smb", "fuse.sshfs"];
    // Parse /proc/mounts, find the deepest matching mount point for path,
    // return true if its fs_type is in NETWORK_FS_TYPES
    // ...
}
```

**Detection failure behaviour:** If `/proc/mounts` cannot be parsed (unusual setup, nested mount, permission error), fall back to `RecommendedWatcher` with a warning log that NFS detection was inconclusive. Explicit `watcher_mode = "poll"` always overrides detection.

---

## Scan Pipeline

The same processing flow runs for both real-time watcher events and scheduled walk entries.

```
File event (create/modify/rename)  OR  scheduled walk entry
    |
Filter: check path against .harmoniaignore rules (ignore crate)
    → match: skip immediately
    |
spawn_blocking {
    Detect media type from extension + magic bytes
    Read file metadata: size, mtime
    If audio/video: read tags via lofty or mediainfo-style probe
}
    |
Query haves table by file_path:
    - No match:           NEW file   → dispatch to import pipeline
    - Match, same size+mtime:   UNCHANGED  → skip
    - Match, different size or mtime:   MODIFIED   → update metadata
    |
For delete events (file no longer on disk):
    - Set haves.status = 'missing'
    - If wants.status = 'searching' and quality headroom exists: no action (already searching)
    - If wants.status = 'fulfilled' and below ceiling: set wants.status = 'searching' to re-acquire
    - If wants.status = 'fulfilled' at quality ceiling: mark missing, do not re-search
```

---

## spawn_blocking Boundaries

**ALL file I/O runs in `spawn_blocking`.** The scanner task is async; it dispatches blocking work and processes results asynchronously.

| Operation | Why spawn_blocking |
|-----------|-------------------|
| Directory walking (`walkdir::WalkDir`) | Sync iterator — blocks the thread |
| File stat (`std::fs::metadata`) | Sync I/O syscall |
| Magic byte read (first 8 bytes) | Sync I/O — tiny but blocks |
| Audio tag reading (`lofty::read_from_path`) | Sync, CPU-bound for large files |
| Media type detection | CPU-bound extension + magic matching |

**Concurrency limit for scheduled scans:** A semaphore limits concurrent file reads to `config.taxis.scan_concurrency` (default: 4). This prevents the scheduled full-scan from saturating the blocking thread pool on large libraries. Real-time watcher events bypass the semaphore — they are single-file operations with low concurrency.

```rust
// Scheduled scan: acquire permit before each file read
let permit = scan_semaphore.acquire().await?;
let metadata = tokio::task::spawn_blocking(move || read_file_metadata(&path)).await??;
drop(permit);
```

---

## .harmoniaignore Support

Using the `ignore` crate (same gitignore semantics as `.gitignore`).

`.harmoniaignore` files can be placed in any library directory or subdirectory. Rules apply to that directory and all subdirectories below it.

**Syntax (gitignore-compatible):**

```gitignore
# Ignore incomplete downloads
*.part
*.downloading
*.!qB

# Ignore macOS metadata
.DS_Store
._*

# Ignore hidden directories
.*/

# Ignore processing artifacts
*.tmp
Thumbs.db
```

**Integration:**

- Scheduled scans: use `ignore::WalkBuilder` instead of raw `walkdir::WalkDir`. The builder automatically discovers and applies `.harmoniaignore` files during traversal.
- Real-time watcher events: path is checked against pre-loaded ignore rules via `ignore::gitignore::Gitignore` before entering the pipeline. Rules are reloaded when a `.harmoniaignore` file itself changes (detected by the watcher).

---

## Media Type Detection

Extension-first with magic byte verification. The library's configured `media_type` resolves ambiguous extensions.

| Extensions | Resolved Type | Magic Bytes Verified |
|-----------|--------------|---------------------|
| `.flac` | Music | `66 4C 61 43` (fLaC) |
| `.mp3` | Music (or Podcast — from library config) | `49 44 33` (ID3) or `FF FB` |
| `.m4a` | Music (or Podcast — from library config) | `66 74 79 70` (ftyp) |
| `.ogg`, `.opus` | Music (or Podcast) | `4F 67 67 53` (OggS) |
| `.wav` | Music | `52 49 46 46` (RIFF) |
| `.wv` | Music | `77 76 70 6B` (wvpk) |
| `.aac` | Music | `FF F1` / `FF F9` |
| `.mkv` | Movie or TV (from library config) | `1A 45 DF A3` (EBML) |
| `.mp4`, `.m4v` | Movie or TV | `66 74 79 70` (ftyp) |
| `.avi` | Movie or TV | `52 49 46 46` (RIFF) |
| `.m4b` | Audiobook | `66 74 79 70` (ftyp, M4B brand) |
| `.epub` | Book | `50 4B 03 04` (PK — EPUB is ZIP) |
| `.pdf` | Book or Comic (from library config) | `25 50 44 46` (%PDF) |
| `.mobi`, `.azw3` | Book | `42 4F 4F 4B 4D 4F 42 49` (BOOKMOBI) |
| `.cbz` | Comic | `50 4B 03 04` (PK — CBZ is ZIP) |
| `.cbr` | Comic | `52 61 72 21` (Rar!) |
| `.cb7` | Comic | `37 7A BC AF 27 1C` (7z) |

**Ambiguity resolution:**

- `.mp3`/`.m4a`/`.ogg`/`.opus` in a library configured as `media_type = "podcast"` → podcast. In a `music` library → music.
- `.mkv`/`.mp4` in a `movie` library → movie. In a `tv` library → TV. If the library allows both, default to movie and let Epignosis resolve via metadata.
- `.pdf` in a `book` library → book. In a `comic` library → comic.
- Unknown extension + recognized magic bytes: import as the detected type, log at info level.
- Unknown extension + unrecognized magic bytes: `MediaDetect` error (see Error Handling).

---

## Scheduled Scan

Full library walk on a configurable schedule.

**Triggers:**

- Automatic: `tokio-cron-scheduler` at `scan_interval_hours` cadence (default: 6h).
- Manual: `POST /api/taxis/scan/{library_id}` — triggers an immediate scan.

**Walk procedure:**

1. Use `ignore::WalkBuilder` to walk the library path (respects `.harmoniaignore`).
2. For each file: compare against `haves` table by `file_path`.
3. Identify three categories:
   - **New:** path not in `haves` → dispatch to import pipeline (or manual review queue if `auto_import = false`).
   - **Missing:** path in `haves` but not on disk → set `haves.status = 'missing'`, evaluate want re-activation.
   - **Modified:** path in `haves`, size or mtime changed → re-read tags, update metadata.
4. Unchanged files (path matches, same size+mtime) → skip, no DB write.
5. On completion: emit `LibraryScanCompleted` event via Aggelia.

**LibraryScanCompleted payload:**

```rust
LibraryScanCompleted {
    library_id: LibraryId,
    new_items: u32,
    updated_items: u32,
    missing_items: u32,
}
```

This maps to the `LibraryScanCompleted` variant in the `HarmoniaEvent` enum (see `architecture/communication.md`). The subscriber breakdown: web UI refreshes the library view; Kritike runs health assessment on newly scanned items.

---

## Error Handling

Scanner errors are **per-file**, not per-scan. A single unreadable file does not abort the scan.

`TaxisError` scanner variants:

```rust
#[derive(Debug, Snafu)]
pub enum TaxisError {
    #[snafu(display("watcher init failed for library {library}"))]
    WatcherInit {
        library: String,
        source: notify::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("directory walk error at {path}"))]
    ScanWalk {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("cannot determine media type for {path}"))]
    MediaDetect {
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("tag read failed for {path}"))]
    TagRead {
        path: PathBuf,
        source: lofty::error::LoftyError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    // ... import variants in import-rename.md
}
```

Per-file errors are logged at `warn` level and the scan continues. `WatcherInit` is a startup failure — logged at `error` level, that library's watcher is not started (no crash, other libraries continue).

---

## Horismos Configuration — `[taxis]` Scanner Section

```toml
[taxis]
# Maximum concurrent file reads during scheduled scans
scan_concurrency = 4

# Debounce window for real-time watcher events (milliseconds)
event_debounce_ms = 500

# Whether to auto-detect NFS mounts and use PollWatcher
nfs_detection = true

[taxis.libraries.music]
path = "/media/music"
media_type = "music"
watcher_mode = "auto"          # "inotify" | "poll" | "auto"
poll_interval_seconds = 30
auto_import = true
scan_interval_hours = 6

# Additional libraries follow the same pattern
```

`TaxisConfig` struct in `crates/horismos/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaxisConfig {
    pub scan_concurrency: usize,        // default: 4
    pub event_debounce_ms: u64,         // default: 500
    pub nfs_detection: bool,            // default: true
    pub libraries: HashMap<String, LibraryConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryConfig {
    pub path: PathBuf,
    pub media_type: MediaType,
    pub watcher_mode: WatcherModeConfig, // Auto | Inotify | Poll
    pub poll_interval_seconds: u64,      // default: 30
    pub auto_import: bool,               // default: true
    pub scan_interval_hours: u64,        // default: 6
    pub naming_template: Option<String>, // overrides default template
}
```
