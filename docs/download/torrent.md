# Torrent Download — Ergasia's librqbit Integration

> Ergasia wraps librqbit for BitTorrent session management, download lifecycle, and seeding policy enforcement.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Ergasia ownership), [architecture/communication.md](../architecture/communication.md) (events), [data/want-release.md](../data/want-release.md) (releases table).

---

## Session Management

Ergasia owns a single `librqbit::Session` instance for the lifetime of the process. All torrents share one session — no per-download sessions.

### `ErgasiaSession` Struct

```rust
pub struct ErgasiaSession {
    session: Arc<Session>,
    policy: SeedingPolicy,
    seed_tracker: Arc<DashMap<DownloadId, SeedHandle>>,
}
```

- `session` — the librqbit `Session`, shared across all torrents via `Arc`
- `policy` — default seeding policy for this instance; per-tracker overrides consulted at seed time
- `seed_tracker` — live handles for active seeding monitor tasks, keyed by `DownloadId`

### Session Initialization

Session is created once at startup with `SessionOptions`:

```rust
let opts = SessionOptions {
    disable_dht: false,           // DHT enabled — default peer discovery
    disable_dht_persistence: false,
    dht_config: None,             // use librqbit defaults
    persistence: Some(persistence_factory(&config.session_state_path)),
    listen_port_range: Some(config.listen_port_range.clone()),
    enable_upnp_port_forwarding: false,
    // peer opts:
    peer_connect_timeout: Some(config.peer_connect_timeout),
    peer_read_write_timeout: Some(Duration::from_secs(10)),
    ..Default::default()
};
let session = Session::new_with_opts(config.download_dir.clone(), opts).await
    .context(SessionInitSnafu)?;
```

Key guarantees:
- **DHT enabled** — default peer discovery. PEX enabled by librqbit defaults.
- **Fast resume** — `persistence` enabled. librqbit persists piece completion state to `session_state_path`. After restart, torrents resume without re-verifying all pieces.
- **Single session** — Ergasia does NOT expose librqbit's built-in HTTP API. All external access to download state goes through Ergasia's own trait surface (`start_download`, `cancel_download`, `get_progress`).
- **Connection limits** — `peer_connect_timeout` is configurable via `[ergasia]`. Max connections per torrent defaults to librqbit's internal limit; Horismos can override via `max_connections_per_torrent`.

---

## Download State Machine

Ergasia maintains its own download state on top of librqbit's internal tracking. This is Ergasia's domain model — not a direct mapping to librqbit internals.

```
                    ┌──────────┐
     Syntaxis enqueues│         ▼
                    Queued ──► Initializing ──► Downloading ──► Completed
                       │                             │               │
                    Failed ◄────────────────── (any state)      Seeding
                    (retries exhausted)                              │
                                                         SeedPolicySatisfied
                                                                     │
                                                                  Deleted
```

### State Definitions

| State | Description | Ergasia Action |
|-------|-------------|----------------|
| `Queued` | Work item received via mpsc from Syntaxis. Not yet handed to librqbit. | Waits for capacity slot (`max_concurrent_downloads`). |
| `Initializing` | librqbit resolving metadata — magnet link DHT lookup, piece map construction, integrity check on previously downloaded data. | Monitors `TorrentStats` for transition to downloading. |
| `Downloading` | Active piece download. `TorrentStats.state` is `Downloading`. | Polls `api_stats_v1` every 2 seconds. Emits `DownloadProgress` events (throttled). |
| `Completed` | All pieces verified. `TorrentStats.finished = true`. | Emits `DownloadCompleted` event. Signals Syntaxis. Spawns seeding monitor task. |
| `Seeding` | Post-completion upload. Torrent continues seeding from `config.download_dir`. Taxis has already hardlinked the files to the library. | Seeding monitor task polls every 60 seconds. |
| `SeedPolicySatisfied` | Seeding monitor determined policy threshold met (ratio OR time). | Pauses torrent via `api_torrent_action_pause`. Calls `Taxis::on_seed_complete(download_id)` directly. Emits `SeedPolicySatisfied` event for observability. |
| `Failed` | All retry attempts exhausted. | Emits `DownloadFailed` event. Records failure reason. |
| `Deleted` | Torrent removed from session after cleanup completes. | Calls `api_torrent_action_forget`. Removes entry from state map. |

### State Transition Triggers

| Transition | Trigger | Owner |
|-----------|---------|-------|
| `Queued` → `Initializing` | Capacity slot available; `session.add_torrent()` called | Ergasia queue processor |
| `Initializing` → `Downloading` | `TorrentStats` shows non-zero progress | Ergasia poll loop |
| `Initializing` → `Failed` | Metadata resolution timeout or invalid magnet URI | librqbit error, caught by Ergasia |
| `Downloading` → `Completed` | `TorrentStats.finished = true` | Ergasia poll loop |
| `Downloading` → `Failed` | 3 consecutive poll errors OR tracker reports torrent invalid | Ergasia retry logic |
| `Completed` → `Seeding` | Ergasia spawns seeding monitor task immediately on completion | Ergasia |
| `Seeding` → `SeedPolicySatisfied` | Monitor: `ratio >= threshold` OR `elapsed >= time_threshold` | Seeding monitor task |
| `SeedPolicySatisfied` → `Deleted` | Taxis calls back `on_cleanup_complete(download_id)` after hardlink promotion | Taxis → Ergasia |
| Any state → `Failed` | Retry budget exhausted (network errors, tracker errors) | Ergasia retry logic |

---

## Seeding Policy Monitor

librqbit has no built-in ratio or time seeding policy. Ergasia implements this externally by polling torrent statistics and comparing against configured thresholds.

### `SeedingPolicy` Struct

```rust
pub struct SeedingPolicy {
    pub ratio_threshold: f64,        // default: 1.0 — stop seeding at 1:1 upload ratio
    pub time_threshold: Duration,    // default: 72 hours
}

impl SeedingPolicy {
    pub fn is_satisfied(&self, uploaded_bytes: u64, downloaded_bytes: u64, seeding_since: Instant) -> bool {
        let ratio = if downloaded_bytes == 0 {
            0.0
        } else {
            uploaded_bytes as f64 / downloaded_bytes as f64
        };
        let elapsed = seeding_since.elapsed();

        ratio >= self.ratio_threshold || elapsed >= self.time_threshold
    }
}
```

**Default policy** (from locked decisions): 1.0x ratio OR 72 hours — whichever is met first.

### Per-Tracker Overrides

Private trackers often require higher ratios or longer seed times. The `[ergasia]` config section carries a `tracker_seed_policies` map:

```toml
[ergasia]
seed_ratio_threshold = 1.0
seed_time_threshold_hours = 72

[ergasia.tracker_seed_policies]
"tracker.alpharatio.cc" = { ratio_threshold = 2.0, time_threshold_hours = 168 }
"tracker.btn.ag" = { ratio_threshold = 1.5, time_threshold_hours = 120 }
```

When a seeding monitor task starts, it queries the torrent's tracker URL list and checks `tracker_seed_policies` for an override. The most restrictive matching policy wins: if multiple tracker URLs match, use the highest `ratio_threshold` and longest `time_threshold`.

### Monitor Task

One monitor task per completed download, spawned by Ergasia on the `Completed` → `Seeding` transition:

```rust
async fn run_seeding_monitor(
    session_api: Arc<dyn TorrentApi>,
    download_id: DownloadId,
    torrent_id: usize,
    policy: SeedingPolicy,
    seeding_since: Instant,
    event_tx: broadcast::Sender<HarmoniaEvent>,
    taxis: Arc<dyn TaxisClient>,
    ct: CancellationToken,
) {
    let span = tracing::info_span!("seeding_monitor", download_id = %download_id);
    async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => {},
                _ = ct.cancelled() => break,
            }

            let stats = match session_api.api_stats_v1(torrent_id).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to query seeding stats — will retry");
                    continue;
                }
            };

            if policy.is_satisfied(stats.uploaded_bytes, stats.downloaded_bytes, seeding_since) {
                // 1. Pause the torrent — stop seeding
                if let Err(e) = session_api.api_torrent_action_pause(torrent_id).await {
                    tracing::error!(error = %e, "failed to pause torrent after seed policy satisfied");
                }

                // 2. Inform Taxis — direct call (authoritative signal for cleanup)
                taxis.on_seed_complete(download_id).await.ok();

                // 3. Emit informational event — observability, UI can show seeding complete
                event_tx.send(HarmoniaEvent::SeedPolicySatisfied {
                    download_id,
                    uploaded_bytes: stats.uploaded_bytes,
                    downloaded_bytes: stats.downloaded_bytes,
                }).ok();

                break;
            }
        }
    }
    .instrument(span)
    .await
}
```

**Why both a direct call and an event:**
- `taxis.on_seed_complete()` is the authoritative cleanup signal — Taxis promotes the hardlink and deletes the download copy. This is a direct call because Ergasia needs confirmation before transitioning to `Deleted`.
- `SeedPolicySatisfied` is an informational event for observability — the web UI can display seeding completion status. It does not wait for any subscriber.

---

## Progress Tracking

### `DownloadProgress` Event Emission

Ergasia emits `DownloadProgress` events during the `Downloading` state. To avoid flooding the broadcast channel:

- **Throttle**: max 1 event per 2 seconds per download (`config.progress_throttle_seconds`, default 2)
- **Delta filter**: only emit if `percent` changed by >= 1% since the last emission
- **Poll interval**: Ergasia polls `api_stats_v1` every 2 seconds during active download, every 60 seconds during seeding

```rust
struct ProgressThrottle {
    last_emit: Instant,
    last_percent: u8,
    throttle_duration: Duration,
}

impl ProgressThrottle {
    fn should_emit(&mut self, percent: u8) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_emit);
        let delta = percent.abs_diff(self.last_percent);

        if elapsed >= self.throttle_duration && delta >= 1 {
            self.last_emit = now;
            self.last_percent = percent;
            true
        } else {
            false
        }
    }
}
```

### Stats Exposed

`get_progress(id)` returns `DownloadProgress`:

```rust
pub struct DownloadProgress {
    pub download_id: DownloadId,
    pub state: DownloadState,
    pub percent_complete: u8,
    pub download_speed_bps: u64,
    pub upload_speed_bps: u64,
    pub peers_connected: u32,
    pub seeders: u32,
    pub eta_seconds: Option<u64>,
}
```

All fields sourced from `TorrentStats` returned by `api_stats_v1`.

---

## Key librqbit API Mapping

| Operation | librqbit Call | Notes |
|-----------|--------------|-------|
| Add torrent from magnet URI | `session.add_torrent(AddTorrent::from_url(magnet), opts)` | Returns torrent ID on success |
| Add torrent from file bytes | `session.add_torrent(AddTorrent::from_bytes(bytes), opts)` | For indexers that serve .torrent files |
| Get torrent statistics | `api.api_stats_v1(torrent_id)` | Returns `TorrentStats` with state, speeds, completion |
| Check completion | `TorrentStats.finished` | `bool` — true when all pieces verified |
| Get uploaded bytes | `TorrentStats.uploaded_bytes` | Used for ratio calculation in seeding monitor |
| Get downloaded bytes | `TorrentStats.downloaded_bytes` | Denominator for ratio |
| Pause torrent (seed complete) | `api.api_torrent_action_pause(torrent_id)` | Stops seeding; does not remove data |
| Delete from session (keep files) | `api.api_torrent_action_forget(torrent_id)` | Removes torrent from session; files remain on disk |
| Select specific files | `AddTorrentOptions { only_files: Some(vec![file_idx, ...]) }` | For multi-file torrents where only some files are wanted |
| Set output directory | `AddTorrentOptions { output_folder: Some(path) }` | Per-download output path; defaults to session root |
| List all torrents | `api.api_torrent_list()` | Used at startup to reconcile persisted state with in-memory state |

---

## Error Handling

`ErgasiaError` uses snafu per `.claude/rules/rust.md`:

```rust
#[derive(Debug, Snafu)]
pub enum ErgasiaError {
    #[snafu(display("failed to initialize librqbit session"))]
    SessionInit {
        source: librqbit::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to add torrent: {reason}"))]
    AddTorrent {
        reason: String,
        source: librqbit::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("torrent not found: {download_id}"))]
    TorrentNotFound {
        download_id: DownloadId,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to query torrent stats for {download_id}"))]
    StatsQuery {
        download_id: DownloadId,
        source: librqbit::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to pause torrent {download_id}"))]
    PauseAction {
        download_id: DownloadId,
        source: librqbit::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
```

### Retry Strategy

| Error Class | Retry Behaviour |
|------------|-----------------|
| Network errors (connection refused, timeout) | 3 retries with exponential backoff: 5s, 25s, 125s. After 3 failures → `Failed` state. |
| Tracker errors (tracker unreachable) | 3 retries. Private trackers may be momentarily down. |
| Invalid torrent / corrupt data | Fail immediately — no retry. Record reason in `DownloadFailed` event. |
| Magnet URI resolution timeout | Fail after configurable timeout (`magnet_resolve_timeout_secs`, default 120s). |
| Already exists in session | Not an error — log and return existing `DownloadId`. |

Errors are logged where they are handled (at the retry boundary or at final failure), not where they originate. This follows the snafu pattern: propagate with `.context()`, log at the decision point.

---

## Proposed New HarmoniaEvent Variants

The following variants should be added to `HarmoniaEvent` in `harmonia-common`:

```rust
/// Ergasia's seeding monitor determined ratio/time policy is satisfied.
/// Informational — the authoritative cleanup signal is the direct call to Taxis.
/// Subscribers: web UI (display seeding completion status)
SeedPolicySatisfied {
    download_id: DownloadId,
    uploaded_bytes: u64,
    downloaded_bytes: u64,
},
```

Note: `DownloadFailed` is already defined in `communication.md`. No addition needed.

---

## Horismos Configuration — `[ergasia]` Section

Full config additions for this document's design:

```toml
[ergasia]
# Base path where librqbit writes downloaded files
download_dir = "/data/downloads"

# librqbit session persistence — fast resume state
session_state_path = "/data/downloads/.librqbit-state"

# Port range for BitTorrent protocol (TCP + UDP)
listen_port_range = [6881, 6889]

# Maximum simultaneous active downloads (Queued torrents wait for a slot)
max_concurrent_downloads = 5

# Default seeding policy — applies to all torrents unless tracker override matches
seed_ratio_threshold = 1.0
seed_time_threshold_hours = 72

# Per-tracker seeding policy overrides
# Tracker URL substring match — most restrictive policy wins if multiple match
[ergasia.tracker_seed_policies]
# "tracker.example.com" = { ratio_threshold = 2.0, time_threshold_hours = 168 }

# How often to emit DownloadProgress events (minimum seconds between emissions)
progress_throttle_seconds = 2

# Staging area for archive extraction (before move to download_dir final location)
extraction_temp_dir = "/data/downloads/.extract-staging"

# Peer connection timeout in seconds
peer_connect_timeout_seconds = 10

# Maximum peer connections per torrent (0 = librqbit default)
max_connections_per_torrent = 0

# Magnet URI DHT resolution timeout in seconds
magnet_resolve_timeout_seconds = 120
```

Corresponding `ErgasiaConfig` struct additions in `crates/horismos/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErgasiaConfig {
    pub download_dir: PathBuf,
    pub session_state_path: PathBuf,
    pub listen_port_range: [u16; 2],
    pub max_concurrent_downloads: usize,          // default: 5
    pub seed_ratio_threshold: f64,                // default: 1.0
    pub seed_time_threshold_hours: u64,           // default: 72
    pub tracker_seed_policies: HashMap<String, TrackerSeedPolicy>,
    pub progress_throttle_seconds: u64,           // default: 2
    pub extraction_temp_dir: PathBuf,
    pub peer_connect_timeout_seconds: u64,        // default: 10
    pub max_connections_per_torrent: u32,         // default: 0 (librqbit default)
    pub magnet_resolve_timeout_seconds: u64,      // default: 120
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrackerSeedPolicy {
    pub ratio_threshold: f64,
    pub time_threshold_hours: u64,
}
```
