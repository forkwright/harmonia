use std::borrow::Cow;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aggelmata::ids::DownloadId;
use aggelmata::{EventSender, HarmoniaEvent};
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use horismos::{ErgasiaConfig, Section};
use jiff::Timestamp;
use librqbit::api::TorrentIdOrHash;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ConnectionOptions, DhtSessionConfig,
    ListenerMode, ListenerOptions, ManagedTorrent, Session, SessionOptions,
    SessionPersistenceConfig, TorrentStats, TorrentStatsState,
};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, instrument};

use crate::error::{
    AddTorrentSnafu, ContentPathSnafu, DeleteActionSnafu, ErgasiaError, MagnetResolveTimeoutSnafu,
    PauseActionSnafu, SessionInitSnafu, TorrentMapPersistenceSnafu, TorrentNotFoundSnafu,
};
use crate::progress::DownloadProgress;
use crate::seeding::SeedingPolicy;

const TORRENT_MAP_FILE: &str = "harmonia-torrent-map.json";

/// Interval between seed-monitor policy checks. `pub(crate)` so tests inject
/// a sub-second cadence; the check runs BEFORE the first sleep, so a zero
/// time threshold satisfies without waiting out an interval.
pub(crate) const SEED_POLL_INTERVAL: Duration = Duration::from_secs(60);

// WHY: 8 MiB — frequent enough that a crash forfeits at most one small
// upload increment from the persisted ratio accounting, rare enough that a
// healthy seeder is not rewriting the side-table on every poll tick.
const WATERMARK_PERSIST_BYTES: u64 = 8 * 1024 * 1024;

pub struct SeedHandle {
    pub cancel: CancellationToken,
}

// WHY: librqbit persists its own session state but knows nothing about
// harmonia's DownloadId, so the download_id <-> librqbit id mapping is
// persisted in a side-table colocated with the session state and reloaded on
// startup — otherwise every persisted torrent is unmanageable after a restart.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedTorrentMap {
    torrents: Vec<PersistedTorrentEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedTorrentEntry {
    download_id: DownloadId,
    torrent_id: usize,
    // NOTE: `serde(default)` keeps side-table files written before the seed
    // fields existed loading cleanly (absent => fresh seed state).
    #[serde(default)]
    seed_started_at: Option<Timestamp>,
    #[serde(default)]
    uploaded_watermark: u64,
}

// WHY: the value side of torrent_map — librqbit has no cumulative uploaded
// counter that survives a restart (`TorrentStats.uploaded_bytes` is an
// in-memory per-live-epoch counter that reads 0 while Paused) and no record
// of when seeding began, so both are carried here and persisted in the
// side-table: the ratio and time thresholds keep enforcing across restarts
// instead of resetting on every process start (#590).
#[derive(Debug, Clone, Copy)]
pub(crate) struct TorrentRecord {
    torrent_id: usize,
    /// Wall-clock instant the download was first observed finished; the
    /// seed-time clock continues from here across restarts.
    seed_started_at: Option<Timestamp>,
    /// Total bytes uploaded across all live epochs observed so far.
    uploaded_watermark: u64,
}

impl TorrentRecord {
    fn new(torrent_id: usize) -> Self {
        Self {
            torrent_id,
            seed_started_at: None,
            uploaded_watermark: 0,
        }
    }
}

/// Cumulative upload for ratio accounting: the persisted prior-epoch
/// watermark plus the current live epoch's counter. librqbit resets the
/// epoch counter on every restart and pause, so summing (never re-adding a
/// previously persisted epoch) is what keeps the total monotonic without
/// double counting.
fn cumulative_uploaded(watermark_base: u64, epoch_uploaded: u64) -> u64 {
    watermark_base.saturating_add(epoch_uploaded)
}

pub struct TorrentSession {
    session: Arc<Session>,
    pub seed_tracker: Arc<DashMap<DownloadId, SeedHandle>>,
    torrent_map: DashMap<DownloadId, TorrentRecord>,
    // WHY: librqbit can return AlreadyManaged for a duplicate info-hash, so
    // two DownloadIds may end up mapped to the same torrent_id. This reverse
    // ref count is the source of truth for whether delete_torrent may reach
    // into librqbit, or must only drop its own mapping entry — see
    // acquire_torrent_ref / release_torrent_ref.
    torrent_refcounts: DashMap<usize, usize>,
    map_path: PathBuf,
    persist_lock: tokio::sync::Mutex<()>,
    // WHY: a live Section (not a frozen copy) — `magnet_resolve_timeout_seconds`
    // is read per add_torrent call and the seed thresholds are read per
    // seed-monitor poll tick, so a config reload applies without a restart.
    // Every other ergasia leaf consumed here is restart-class: `publish()`
    // holds those back in the effective config, so the construction-time
    // snapshot never drifts from what the Section serves.
    config: Section<ErgasiaConfig>,
    /// Seed-monitor poll cadence; `SEED_POLL_INTERVAL` in production, shorter
    /// in tests.
    seed_poll_interval: Duration,
    // WHY: completion is push-based — a lifecycle watcher per download emits
    // DownloadCompleted/DownloadFailed on this bus so syntaxis settles the
    // queue row and runs the import pipeline in production (#602).
    event_tx: EventSender,
}

/// Mirrors librqbit's `get_default_subfolder_for_torrent` name resolution
/// (librqbit-9.0.1 session.rs:1175-1216) tier for tier: the info-dict name if
/// non-empty, else the magnet display name if non-empty, else the largest
/// file's stem. `None` only when all three are unavailable.
///
/// NOTE: librqbit additionally path-validates tiers 1-2 at add time — a live
/// torrent's name already passed that check, so only emptiness is re-guarded
/// here (upstream guards `!s.is_empty()` but `TorrentMetadata::new` does not,
/// so an empty info name IS observable through metadata).
fn resolve_multi_file_subfolder(
    meta_name: Option<&str>,
    handle_name: Option<&str>,
    largest_stem: Option<&std::ffi::OsStr>,
) -> Option<PathBuf> {
    meta_name
        .filter(|n| !n.is_empty())
        .map(PathBuf::from)
        .or_else(|| handle_name.filter(|n| !n.is_empty()).map(PathBuf::from))
        .or_else(|| largest_stem.filter(|s| !s.is_empty()).map(PathBuf::from))
}

impl TorrentSession {
    // WHY: returns Arc<Self> — every successful add (and every restored
    // torrent) spawns a lifecycle watcher that holds the session, so
    // construction must already own the Arc the watchers clone.
    pub async fn new(
        config_section: Section<ErgasiaConfig>,
        event_tx: EventSender,
    ) -> Result<Arc<Self>, ErgasiaError> {
        Self::with_seed_poll_interval(config_section, event_tx, SEED_POLL_INTERVAL).await
    }

    // WHY: pub(crate) test seam — the production 60s cadence would stall the
    // ratio-path integration tests; nothing outside the crate tunes this.
    #[instrument(skip_all, name = "ergasia_session_init")]
    pub(crate) async fn with_seed_poll_interval(
        config_section: Section<ErgasiaConfig>,
        event_tx: EventSender,
        seed_poll_interval: Duration,
    ) -> Result<Arc<Self>, ErgasiaError> {
        let config = config_section.get();
        let peer_opts = librqbit::PeerConnectionOptions {
            connect_timeout: Some(Duration::from_secs(config.peer_connect_timeout_seconds)),
            read_write_timeout: Some(Duration::from_secs(10)),
            ..Default::default()
        };

        let listen_port_start = config
            .listen_port_range
            .first()
            .copied()
            .unwrap_or_else(|| unreachable!("listen_port_range is [u16; 2]"));
        let listen_port_end = config
            .listen_port_range
            .get(1)
            .copied()
            .unwrap_or_else(|| unreachable!("listen_port_range is [u16; 2]"));

        // WHY: librqbit 9 replaced SessionOptions::listen_port_range (an
        // internal loop that tried each port in the range and bound the
        // first free one — librqbit 8 session.rs create_tcp_listener) with a
        // single ListenerOptions::listen_addr and no retry left inside the
        // crate. The listener bind is the first fallible step in
        // Session::new_with_opts — it runs before DHT/persistence are
        // touched — so retrying the whole call per candidate port reproduces
        // the old range scan exactly, without racing a separate probe bind
        // against librqbit's own.
        //
        // NOTE: dual-stack listening (below) does not change this. Any bind
        // failure — including one specific to dual-stack, e.g. the port
        // already held by an IPv6-only socket, or set_only_v6(false) itself
        // failing (librqbit-dualstack-sockets socket.rs) — still surfaces as
        // a plain Err from the same `.context("error starting listeners")?`
        // in Session::new_with_opts, still before DHT/persistence run, so
        // this loop retries it exactly like any other bind failure without
        // needing to know why it failed.
        let mut last_err = None;
        let mut session = None;
        for port in listen_port_start..listen_port_end {
            let opts = SessionOptions {
                // WHY: librqbit 8 always bound the TCP listener, the DHT
                // socket, and outbound peer connections to IPv4 only
                // (0.0.0.0, hardcoded — v8 had no IPv6 support at all).
                // librqbit 9 added real IPv6 support and defaults to
                // dual-stack; adopted here rather than pinned back to
                // IPv4-only (operator decision). `ipv4_only` is left at its
                // default (false) across the board, which fans out to three
                // places: DHT binds `[::]` (dht_listen_addr derives the IP
                // solely from this flag), the outbound StreamConnector
                // becomes dual-stack-capable, and the TCP listener below is
                // told to request dual-stack explicitly via its own
                // listen_addr (ipv4_only alone does not widen an address we
                // hand it — see that field's WHY note).
                dht: Some(DhtSessionConfig {
                    // WHY: pin DHT routing-table persistence inside this
                    // instance's own session_state_path rather than
                    // librqbit's global default
                    // (~/.cache/com.rqbit.dht/dht.json). A shared default
                    // races and corrupts across concurrent instances — and
                    // parallel tests — that initialize the persistent DHT at
                    // once; instance-local state keeps each session
                    // self-contained.
                    persistence: Some(librqbit::dht::DhtPersistenceConfig {
                        config_filename: Some(
                            PathBuf::from(&config.session_state_path).join("dht.json"),
                        ),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                persistence: Some(SessionPersistenceConfig::Json {
                    folder: Some(PathBuf::from(&config.session_state_path)),
                }),
                listen: Some(ListenerOptions {
                    // WHY: `[::]`, not `0.0.0.0` — with ipv4_only left at its
                    // default (false), librqbit-dualstack-sockets only
                    // widens an IPv6-unspecified address to accept IPv4 too
                    // (socket.rs: request_dualstack applies exclusively to
                    // the `SocketAddr::V6(UNSPECIFIED)` branch); handing it
                    // an explicit IPv4 address here would keep the listener
                    // IPv4-only regardless of the ipv4_only flag.
                    listen_addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port),
                    mode: ListenerMode::TcpOnly,
                    enable_upnp_port_forwarding: false,
                    ..Default::default()
                }),
                connect: Some(ConnectionOptions {
                    peer_opts: Some(peer_opts),
                    ..Default::default()
                }),
                ..Default::default()
            };

            match Session::new_with_opts(config.download_dir.clone(), opts).await {
                Ok(s) => {
                    session = Some(s);
                    break;
                }
                Err(e) => last_err = Some(e),
            }
        }

        let session = session.ok_or_else(|| {
            SessionInitSnafu {
                error: last_err.map(|e| e.to_string()).unwrap_or_else(|| {
                    format!("no free TCP ports in range {listen_port_start}..{listen_port_end}")
                }),
            }
            .build()
        })?;

        let torrent_session = Arc::new(Self {
            session,
            seed_tracker: Arc::new(DashMap::new()),
            torrent_map: DashMap::new(),
            torrent_refcounts: DashMap::new(),
            map_path: PathBuf::from(&config.session_state_path).join(TORRENT_MAP_FILE),
            persist_lock: tokio::sync::Mutex::new(()),
            config: config_section,
            seed_poll_interval,
            event_tx,
        });

        // INVARIANT: the session is not handed out until torrent_map reflects
        // every torrent librqbit restored from persisted state.
        torrent_session.reconcile_persisted_torrents().await?;

        Ok(torrent_session)
    }

    #[instrument(skip(self, magnet_uri), fields(download_id = %download_id))]
    pub async fn add_torrent_from_magnet(
        self: &Arc<Self>,
        download_id: DownloadId,
        magnet_uri: &str,
    ) -> Result<(usize, Arc<ManagedTorrent>), ErgasiaError> {
        let source = AddTorrent::Url(Cow::Owned(magnet_uri.to_owned()));
        self.add_torrent_inner(download_id, source, None).await
    }

    #[instrument(skip(self, torrent_bytes), fields(download_id = %download_id))]
    pub async fn add_torrent_from_bytes(
        self: &Arc<Self>,
        download_id: DownloadId,
        torrent_bytes: bytes::Bytes,
    ) -> Result<(usize, Arc<ManagedTorrent>), ErgasiaError> {
        let source = AddTorrent::TorrentFileBytes(torrent_bytes);
        self.add_torrent_inner(download_id, source, None).await
    }

    async fn add_torrent_inner(
        self: &Arc<Self>,
        download_id: DownloadId,
        source: AddTorrent<'static>,
        output_folder: Option<String>,
    ) -> Result<(usize, Arc<ManagedTorrent>), ErgasiaError> {
        let opts = Some(AddTorrentOptions {
            output_folder,
            // WHY: librqbit's own session restore sets overwrite
            // (session_persistence/mod.rs `into_add_torrent`) — existing data
            // is hash-checked, never blindly trusted, so this enables
            // retry-over-kept-files and completion via hash check alone.
            overwrite: true,
            ..Default::default()
        });

        let is_magnet = matches!(
            &source,
            AddTorrent::Url(url) if url.get(..7).is_some_and(|p| p.eq_ignore_ascii_case("magnet:"))
        );

        // WHY: the add runs as a spawned task so a deadline never cancels it
        // mid-add — librqbit registers the torrent in its session before its
        // final persistence await, and dropping that future there would leave
        // a live torrent harmonia never tracks. The task either finishes in
        // budget or finishes late and the reaper below deletes exactly what
        // it created.
        let add_session = Arc::clone(&self.session);
        let mut add_task = tokio::spawn(
            async move { add_session.add_torrent(source, opts).await }
                .instrument(tracing::info_span!("torrent_add", %download_id)),
        );

        // WHY: the deadline applies to magnet sources only — resolve waits on
        // the peer/DHT stream with no internal deadline, while a torrent-file
        // add is local and returns promptly; binding file adds to the magnet
        // knob would fail valid adds under an aggressive setting.
        let timeout_seconds = self.config.get().magnet_resolve_timeout_seconds;
        let joined = if is_magnet {
            match tokio::time::timeout(Duration::from_secs(timeout_seconds), &mut add_task).await {
                Ok(joined) => joined,
                Err(elapsed) => {
                    // WARNING: best-effort orphan cleanup — a late-completing
                    // add is deleted only when it Added a torrent (never
                    // AlreadyManaged: that torrent belongs to another
                    // download); a resolve that never completes is aborted
                    // after a generous cap.
                    let reap_session = Arc::clone(&self.session);
                    tokio::spawn(
                        async move {
                            let cap =
                                Duration::from_secs(timeout_seconds.saturating_mul(10).max(600));
                            match tokio::time::timeout(cap, &mut add_task).await {
                                Ok(Ok(Ok(AddTorrentResponse::Added(id, _)))) => {
                                    tracing::warn!(
                                        %download_id,
                                        torrent_id = id,
                                        "timed-out add completed late; deleting orphaned torrent"
                                    );
                                    if let Err(e) =
                                        reap_session.delete(TorrentIdOrHash::Id(id), false).await
                                    {
                                        tracing::warn!(
                                            %download_id,
                                            torrent_id = id,
                                            error = %e,
                                            "orphaned torrent delete failed"
                                        );
                                    }
                                }
                                Ok(_) => {}
                                Err(_) => {
                                    add_task.abort();
                                    tracing::warn!(
                                        %download_id,
                                        "timed-out add never completed within the reap cap; aborted"
                                    );
                                }
                            }
                        }
                        .instrument(tracing::info_span!("torrent_add_reaper", %download_id)),
                    );
                    return Err(elapsed).context(MagnetResolveTimeoutSnafu {
                        download_id,
                        timeout_seconds,
                    });
                }
            }
        } else {
            add_task.await
        };

        let response = joined
            .map_err(|e| {
                AddTorrentSnafu {
                    reason: "add_torrent task join failed".to_string(),
                    error: e.to_string(),
                }
                .build()
            })?
            .map_err(|e| {
                AddTorrentSnafu {
                    reason: "add_torrent call failed".to_string(),
                    error: e.to_string(),
                }
                .build()
            })?;

        match response {
            AddTorrentResponse::Added(id, handle)
            | AddTorrentResponse::AlreadyManaged(id, handle) => {
                self.torrent_map.insert(download_id, TorrentRecord::new(id));
                self.acquire_torrent_ref(id);
                if let Err(persist_err) = self.persist_torrent_map().await {
                    // WHY: fail loudly but non-destructively — the mapping is
                    // rolled back so the caller sees a consistent failure, while
                    // the torrent stays in librqbit (an AlreadyManaged torrent
                    // may belong to another download, so deleting it here could
                    // destroy live data).
                    self.torrent_map.remove(&download_id);
                    self.release_torrent_ref(id);
                    return Err(persist_err);
                }
                self.spawn_lifecycle_watcher(download_id, Arc::clone(&handle), true);
                Ok((id, handle))
            }
            AddTorrentResponse::ListOnly(_) => Err(AddTorrentSnafu {
                reason: "unexpected ListOnly response".to_string(),
                error: String::new(),
            }
            .build()),
        }
    }

    pub fn get_torrent(
        &self,
        download_id: DownloadId,
    ) -> Result<Arc<ManagedTorrent>, ErgasiaError> {
        let torrent_id = self
            .torrent_map
            .get(&download_id)
            .map(|v| v.torrent_id)
            .ok_or_else(|| TorrentNotFoundSnafu { download_id }.build())?;

        self.session
            .get(TorrentIdOrHash::Id(torrent_id))
            .ok_or_else(|| TorrentNotFoundSnafu { download_id }.build())
    }

    pub fn get_stats(&self, download_id: DownloadId) -> Result<TorrentStats, ErgasiaError> {
        let handle = self.get_torrent(download_id)?;
        Ok(handle.stats())
    }

    /// Reports honest, engine-derived progress for a download.
    pub fn progress(&self, download_id: DownloadId) -> Result<DownloadProgress, ErgasiaError> {
        let stats = self.get_stats(download_id)?;
        Ok(DownloadProgress::from_stats(download_id, &stats))
    }

    /// Resolves the on-disk content path: the containing directory for a
    /// multi-file download, the file itself for a single-file one.
    ///
    /// WHY: this recomputes what librqbit resolved at add time (session.rs
    /// `get_default_subfolder_for_torrent` + `add_torrent_internal`'s
    /// output_folder join) — the resolved folder is pub(crate) upstream, so
    /// it cannot be read back. `download_dir` is restart-class config
    /// (`publish()` holds changes back), so the recomputation cannot drift
    /// within a process lifetime. Name resolution mirrors librqbit's three
    /// tiers via `resolve_multi_file_subfolder`.
    pub fn content_path(&self, download_id: DownloadId) -> Result<PathBuf, ErgasiaError> {
        let handle = self.get_torrent(download_id)?;
        let download_dir = self.config.get().download_dir.clone();
        let (file_count, meta_name, first_file, largest_stem) = handle
            .with_metadata(|m| {
                (
                    m.file_infos.len(),
                    // WHY: librqbit 9 dropped TorrentMetadata::name (a
                    // cached Option<String>) in favour of computing it live
                    // from the validated info dict; ManagedTorrent::name()
                    // (torrent_state/mod.rs) uses this exact
                    // `m.info.name().map(Cow::into_owned)` pattern, so this
                    // mirrors the crate's own idiom rather than inventing one.
                    m.info.name().map(|n| n.into_owned()),
                    m.file_infos.first().map(|f| f.relative_filename.clone()),
                    m.file_infos
                        .iter()
                        .max_by_key(|f| f.len)
                        .and_then(|f| f.relative_filename.file_stem())
                        .map(|s| s.to_os_string()),
                )
            })
            .map_err(|e| {
                // WHY: metadata is None only while a magnet resolves — the
                // caller (reconcile / lifecycle watcher) retries later.
                ContentPathSnafu {
                    download_id,
                    reason: e.to_string(),
                }
                .build()
            })?;

        if file_count >= 2 {
            match resolve_multi_file_subfolder(
                meta_name.as_deref(),
                handle.name().as_deref(),
                largest_stem.as_deref(),
            ) {
                Some(subfolder) => Ok(download_dir.join(subfolder)),
                // WHY: an unresolvable name must be an ERROR, never the bare
                // download_dir — that is the session-wide shared root, and
                // handing it to the completion pipeline would extract and
                // import every sibling download's files as this release.
                None => Err(ContentPathSnafu {
                    download_id,
                    reason: "multi-file torrent has no resolvable name \
                             (no info name, magnet name, or file stem)"
                        .to_string(),
                }
                .build()),
            }
        } else {
            first_file
                .map(|relative| download_dir.join(relative))
                .ok_or_else(|| {
                    ContentPathSnafu {
                        download_id,
                        reason: "torrent metadata lists no files".to_string(),
                    }
                    .build()
                })
        }
    }

    /// Spawns the per-download lifecycle watcher that turns librqbit's
    /// completion signal into a bus event, then enforces the seeding policy
    /// until it is satisfied (#590).
    ///
    /// `announce_completion` is false for torrents restored from persisted
    /// state: they exist for seed continuity, while completion for re-queued
    /// rows flows through their new download_ids — announcing here would
    /// replay stale completions on every restart.
    fn spawn_lifecycle_watcher(
        self: &Arc<Self>,
        download_id: DownloadId,
        handle: Arc<ManagedTorrent>,
        announce_completion: bool,
    ) {
        // WHY: wait_until_completed never resolves for a paused torrent
        // (librqbit torrent_state/mod.rs loops while Paused) — a watcher
        // would pin the session Arc forever. Only restored torrents can be
        // paused (librqbit persists is_paused), and the only production
        // pauser is the seed monitor, so a restored paused torrent is
        // already seed-policy-satisfied — nothing left to watch or enforce
        // (`map_torrent_stats` reports it as such).
        if handle.is_paused() {
            return;
        }

        let cancel = CancellationToken::new();
        self.seed_tracker.insert(
            download_id,
            SeedHandle {
                cancel: cancel.clone(),
            },
        );
        let session = Arc::clone(self);
        // kanon:ignore RUST/spawn-no-instrument -- the spawned future IS instrumented (`.instrument(torrent_lifecycle)` below); the lint's line-distance window does not span this ~50-line watcher body
        tokio::spawn(
            async move {
                let finished = tokio::select! {
                    _ = cancel.cancelled() => false,
                    result = handle.wait_until_completed() => match result {
                        Ok(()) => {
                            if announce_completion {
                                match session.content_path(download_id) {
                                    Ok(path) => session.emit(HarmoniaEvent::DownloadCompleted {
                                        download_id,
                                        path,
                                    }),
                                    // WHY: warn and stand down — the syntaxis
                                    // reconcile pass observes the terminal
                                    // state and retries the path resolution.
                                    Err(e) => tracing::warn!(
                                        %download_id,
                                        error = %e,
                                        "download finished but its content path did not resolve"
                                    ),
                                }
                            }
                            true
                        }
                        Err(e) => {
                            if announce_completion {
                                session.emit(HarmoniaEvent::DownloadFailed {
                                    download_id,
                                    reason: e.to_string(),
                                });
                            } else {
                                tracing::warn!(
                                    %download_id,
                                    error = %e,
                                    "restored torrent errored while awaiting completion"
                                );
                            }
                            false
                        }
                    }
                };
                if finished {
                    session.monitor_seeding(download_id, &handle, &cancel).await;
                }
                // WHY: self-cleanup on every exit path — a download_id that
                // is never deleted (e.g. the pre-restart id of a row that
                // recovery re-queued under a fresh id) would otherwise leak
                // its seed_tracker entry forever. Removing an already-removed
                // key (the delete_torrent path) is a no-op.
                session.seed_tracker.remove(&download_id);
            }
            .instrument(tracing::info_span!("torrent_lifecycle", %download_id)),
        );
    }

    /// Enforces the live seeding policy on a finished torrent: polls stats,
    /// persists the upload watermark and seed-start instant, and pauses the
    /// torrent once `SeedingPolicy::is_satisfied`.
    ///
    /// WHY pause, not delete: `Session::pause` stops upload, durably persists
    /// `is_paused`, and keeps the torrent re-seedable; after a restart the
    /// paused+finished stats map to `SeedPolicySatisfied`
    /// (`map_torrent_stats`). Delete would forget the torrent, destroying
    /// both the re-seed ability and that restart-visible terminal state —
    /// delete stays cancel-only.
    async fn monitor_seeding(
        self: &Arc<Self>,
        download_id: DownloadId,
        handle: &Arc<ManagedTorrent>,
        cancel: &CancellationToken,
    ) {
        // WHY: the mapping can be gone (a delete raced completion) — then
        // there is nothing to enforce or account for.
        let Some((watermark_base, seed_started_at, newly_started)) =
            self.claim_seed_start(download_id)
        else {
            return;
        };
        if newly_started {
            self.persist_seed_state(download_id).await;
        }

        let mut last_persisted = watermark_base;
        loop {
            let stats = handle.stats();
            match stats.state {
                // Externally stopped — enforcement has nothing left to do.
                TorrentStatsState::Paused => break,
                TorrentStatsState::Error => {
                    tracing::warn!(
                        %download_id,
                        error = ?stats.error,
                        "torrent errored while seeding; seed monitor exiting"
                    );
                    break;
                }
                // NOTE: librqbit 9 added a `paused` field to `Initializing`
                // (whether an initializing torrent is also paused) — ignored
                // here, matching librqbit 8's coarser (fieldless) variant:
                // the seed monitor has nothing to enforce either way while
                // still initializing.
                TorrentStatsState::Initializing { .. } | TorrentStatsState::Live => {}
            }

            // INVARIANT: `stats.uploaded_bytes` counts the CURRENT live epoch
            // only (librqbit resets it on restart/pause and reads 0 while
            // Paused); the persisted watermark carries prior epochs.
            let uploaded_total = cumulative_uploaded(watermark_base, stats.uploaded_bytes);
            if uploaded_total.saturating_sub(last_persisted) >= WATERMARK_PERSIST_BYTES {
                self.record_uploaded_watermark(download_id, uploaded_total);
                self.persist_seed_state(download_id).await;
                last_persisted = uploaded_total;
            }

            // WHY: rebuilt from the live Section on every tick — this is
            // what makes the LIVE reclassification of the seed thresholds
            // true: a reload applies to in-flight seeding at the next poll.
            let policy = SeedingPolicy::from(&self.config.get());
            // WHY: max(ZERO) guards a persisted start ahead of the current
            // clock (skew across a restart) from panicking the conversion.
            let seeding_elapsed = Timestamp::now()
                .duration_since(seed_started_at)
                .max(jiff::SignedDuration::ZERO)
                .unsigned_abs();
            // NOTE: `total_bytes` is the ratio denominator — the only
            // download-size figure that is stable across restarts (librqbit
            // persists no cumulative downloaded counter either).
            if policy.is_satisfied(uploaded_total, stats.total_bytes, seeding_elapsed) {
                if uploaded_total > last_persisted {
                    self.record_uploaded_watermark(download_id, uploaded_total);
                    self.persist_seed_state(download_id).await;
                    last_persisted = uploaded_total;
                }
                let paused = match self.session.pause(handle).await {
                    Ok(()) => true,
                    // WHY: an external pause between the stats read and here
                    // is the desired end state, not a failure.
                    Err(_) if handle.is_paused() => true,
                    Err(e) => {
                        tracing::warn!(
                            %download_id,
                            error = %e,
                            "seed-policy pause failed; retrying next tick"
                        );
                        false
                    }
                };
                if paused {
                    tracing::info!(
                        %download_id,
                        uploaded_bytes = uploaded_total,
                        total_bytes = stats.total_bytes,
                        "seed policy satisfied; torrent paused"
                    );
                    self.emit(HarmoniaEvent::SeedPolicySatisfied {
                        download_id,
                        uploaded_bytes: uploaded_total,
                        downloaded_bytes: stats.total_bytes,
                    });
                    break;
                }
            }

            tokio::select! {
                _ = cancel.cancelled() => {
                    if uploaded_total > last_persisted {
                        self.record_uploaded_watermark(download_id, uploaded_total);
                        self.persist_seed_state(download_id).await;
                    }
                    return;
                }
                _ = tokio::time::sleep(self.seed_poll_interval) => {}
            }
        }
    }

    /// Copies out the seed bookkeeping for the monitor, recording the start
    /// instant on the first observation of a finished torrent.
    ///
    /// WHY now-on-absent: a restored finished torrent whose side-table
    /// predates the seed fields has no recorded start — starting its clock
    /// now over-seeds by at most the downtime (bounded, honest) instead of
    /// either never satisfying or fabricating a past instant.
    ///
    /// INVARIANT: the DashMap guard is dropped before return — persistence
    /// happens in the caller, never under a shard lock.
    fn claim_seed_start(&self, download_id: DownloadId) -> Option<(u64, Timestamp, bool)> {
        let mut record = self.torrent_map.get_mut(&download_id)?;
        match record.seed_started_at {
            Some(started_at) => Some((record.uploaded_watermark, started_at, false)),
            None => {
                let now = Timestamp::now();
                record.seed_started_at = Some(now);
                Some((record.uploaded_watermark, now, true))
            }
        }
    }

    // INVARIANT: max() keeps the watermark monotonic even if two monitors
    // for DownloadIds sharing one torrent_id interleave their writes.
    fn record_uploaded_watermark(&self, download_id: DownloadId, uploaded_total: u64) {
        if let Some(mut record) = self.torrent_map.get_mut(&download_id) {
            record.uploaded_watermark = record.uploaded_watermark.max(uploaded_total);
        }
    }

    /// Best-effort side-table write for seed bookkeeping.
    ///
    /// WHY: enforcement must keep running when the write fails — a full disk
    /// would otherwise stop the monitor and reopen unbounded seeding (#590);
    /// the next watermark growth or exit retries the write.
    async fn persist_seed_state(&self, download_id: DownloadId) {
        if let Err(e) = self.persist_torrent_map().await {
            tracing::warn!(
                %download_id,
                error = %e,
                "failed to persist seed state; enforcement continues"
            );
        }
    }

    fn emit(&self, event: HarmoniaEvent) {
        // WHY: a send error only means no live receivers (broadcast
        // semantics) — nothing to recover, but worth a trace for tests and
        // shutdown windows.
        if self.event_tx.send(event).is_err() {
            tracing::debug!("event bus has no receivers; lifecycle event dropped");
        }
    }

    pub async fn pause_torrent(&self, download_id: DownloadId) -> Result<(), ErgasiaError> {
        let handle = self.get_torrent(download_id)?;
        self.session.pause(&handle).await.map_err(|e| {
            PauseActionSnafu {
                download_id,
                error: e.to_string(),
            }
            .build()
        })
    }

    pub async fn delete_torrent(&self, download_id: DownloadId) -> Result<(), ErgasiaError> {
        // WHY: remove() is an atomic claim — of two concurrent deletes for the
        // same id, exactly one proceeds into librqbit; the other sees the entry
        // already gone and gets TorrentNotFound instead of a confusing
        // wrapped librqbit failure.
        let (_, record) = self
            .torrent_map
            .remove(&download_id)
            .ok_or_else(|| TorrentNotFoundSnafu { download_id }.build())?;
        let torrent_id = record.torrent_id;

        // WHY: the lifecycle watcher must not announce completion for a
        // download being deleted; cancel it under the claim that won the map
        // entry. NOTE: a failed librqbit delete below restores the mapping
        // but not the watcher — the syntaxis reconcile pass still observes
        // terminal states for the restored entry.
        if let Some((_, seed_handle)) = self.seed_tracker.remove(&download_id) {
            seed_handle.cancel.cancel();
        }

        // WHY: librqbit dedups a duplicate info-hash onto one torrent_id shared
        // by two DownloadIds (AlreadyManaged) — only the last DownloadId still
        // referencing torrent_id may delete it from librqbit; an earlier
        // caller just drops its own mapping entry and leaves the torrent live.
        let last_ref = self.release_torrent_ref(torrent_id);

        if last_ref
            && let Err(e) = self
                .session
                .delete(TorrentIdOrHash::Id(torrent_id), false)
                .await
        {
            // WHY: the torrent still exists in librqbit — restore the
            // mapping (seed bookkeeping included) and its ref so the caller
            // can retry instead of orphaning it.
            self.torrent_map.insert(download_id, record);
            self.acquire_torrent_ref(torrent_id);
            // WHY: re-persist after restoring. Cancelling the seed monitor
            // above can race its watermark persist, which would snapshot the
            // side-table with this entry already removed; because
            // persist_torrent_map snapshots the map under persist_lock, this
            // post-restore persist is the durable last write for this id, so
            // a crash after a failed delete cannot drop the still-live torrent
            // from disk and orphan it on the next reconcile.
            if let Err(persist_err) = self.persist_torrent_map().await {
                tracing::warn!(
                    %download_id,
                    error = %persist_err,
                    "failed to re-persist torrent map after a failed delete"
                );
            }
            return Err(DeleteActionSnafu {
                download_id,
                error: e.to_string(),
            }
            .build());
        }

        self.persist_torrent_map().await?;
        Ok(())
    }

    fn acquire_torrent_ref(&self, torrent_id: usize) {
        *self.torrent_refcounts.entry(torrent_id).or_insert(0) += 1;
    }

    // Decrements the ref count for `torrent_id` and reports whether this was
    // the last reference — i.e. whether the caller may now delete it from
    // librqbit.
    // INVARIANT: entry() holds the shard lock for `torrent_id` across the
    // whole read-modify-write, so two concurrent releases for the same
    // torrent_id can never both observe "last reference".
    fn release_torrent_ref(&self, torrent_id: usize) -> bool {
        match self.torrent_refcounts.entry(torrent_id) {
            Entry::Occupied(mut e) => {
                *e.get_mut() -= 1;
                if *e.get() == 0 {
                    e.remove();
                    true
                } else {
                    false
                }
            }
            // WHY: an untracked torrent_id has no evidence of another owner
            // (e.g. a mapping inserted directly without acquire_torrent_ref)
            // — fail toward attempting the real delete rather than silently
            // orphaning it.
            Entry::Vacant(_) => true,
        }
    }

    async fn reconcile_persisted_torrents(self: &Arc<Self>) -> Result<(), ErgasiaError> {
        let persisted = self.load_torrent_map().await?;

        let mut restored = 0usize;
        let mut dropped = 0usize;
        for entry in persisted {
            if let Some(handle) = self.session.get(TorrentIdOrHash::Id(entry.torrent_id)) {
                self.torrent_map.insert(
                    entry.download_id,
                    TorrentRecord {
                        torrent_id: entry.torrent_id,
                        seed_started_at: entry.seed_started_at,
                        uploaded_watermark: entry.uploaded_watermark,
                    },
                );
                self.acquire_torrent_ref(entry.torrent_id);
                self.spawn_lifecycle_watcher(entry.download_id, handle, false);
                restored += 1;
            } else {
                tracing::warn!(
                    download_id = %entry.download_id,
                    torrent_id = entry.torrent_id,
                    "dropping torrent map entry no longer present in the session"
                );
                dropped += 1;
            }
        }

        if dropped > 0 {
            self.persist_torrent_map().await?;
        }

        let live = self.session.with_torrents(|torrents| torrents.count());
        tracing::info!(restored, dropped, live, "reconciled persisted torrents");
        Ok(())
    }

    async fn load_torrent_map(&self) -> Result<Vec<PersistedTorrentEntry>, ErgasiaError> {
        let bytes = match tokio::fs::read(&self.map_path).await {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build());
            }
        };

        match serde_json::from_slice::<PersistedTorrentMap>(&bytes) {
            Ok(map) => Ok(map.torrents),
            Err(e) => {
                // WHY: a corrupt side-table must not brick startup — librqbit's
                // own session state is intact. Quarantine the file for forensics
                // and continue with an empty map (same managability as before
                // the side-table existed).
                let quarantine = self.map_path.with_extension("json.corrupt");
                tokio::fs::rename(&self.map_path, &quarantine).await.ok();
                tracing::warn!(
                    path = %self.map_path.display(),
                    quarantine = %quarantine.display(),
                    error = %e,
                    "torrent map file is corrupt; quarantined and starting with an empty map"
                );
                Ok(Vec::new())
            }
        }
    }

    async fn persist_torrent_map(&self) -> Result<(), ErgasiaError> {
        let _guard = self.persist_lock.lock().await;

        let torrents: Vec<PersistedTorrentEntry> = self
            .torrent_map
            .iter()
            .map(|kv| PersistedTorrentEntry {
                download_id: *kv.key(),
                torrent_id: kv.value().torrent_id,
                seed_started_at: kv.value().seed_started_at,
                uploaded_watermark: kv.value().uploaded_watermark,
            })
            .collect();

        let payload =
            serde_json::to_vec_pretty(&PersistedTorrentMap { torrents }).map_err(|e| {
                TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build()
            })?;

        if let Some(parent) = self.map_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build()
            })?;
        }

        // WHY: write-then-rename keeps the side-table atomic — a crash mid-write
        // leaves the previous map intact instead of a truncated file.
        let tmp_path = self.map_path.with_extension("json.tmp");
        tokio::fs::write(&tmp_path, &payload).await.map_err(|e| {
            TorrentMapPersistenceSnafu {
                path: tmp_path.clone(),
                error: e.to_string(),
            }
            .build()
        })?;
        tokio::fs::rename(&tmp_path, &self.map_path)
            .await
            .map_err(|e| {
                TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build()
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::LazyLock;

    use super::*;

    // WHY: each session binds listen ports and starts a DHT; serializing the
    // session tests avoids port/DHT-persistence races inside one test binary.
    static SESSION_TEST_LOCK: LazyLock<tokio::sync::Mutex<()>> =
        LazyLock::new(|| tokio::sync::Mutex::new(()));

    fn test_config(root: &Path, port_base: u16) -> ErgasiaConfig {
        ErgasiaConfig {
            download_dir: root.join("downloads"),
            session_state_path: root.join("state"),
            listen_port_range: [port_base, port_base + 8],
            ..ErgasiaConfig::default()
        }
    }

    fn test_event_tx() -> EventSender {
        aggelmata::create_event_bus(64).0
    }

    fn minimal_torrent_bytes(name: &str) -> bytes::Bytes {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"d4:infod6:lengthi11e4:name");
        buf.extend_from_slice(format!("{}:{}", name.len(), name).as_bytes());
        buf.extend_from_slice(b"12:piece lengthi16384e6:pieces20:");
        buf.extend_from_slice(&[0xAA; 20]);
        buf.extend_from_slice(b"ee");
        bytes::Bytes::from(buf)
    }

    async fn wait_for_session_state(state_dir: &Path) {
        for _ in 0..100 {
            let has_state = std::fs::read_dir(state_dir)
                .map(|entries| {
                    entries.flatten().any(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "json")
                            .unwrap_or(false)
                            && e.metadata().map(|m| m.len() > 2).unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            if has_state {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("librqbit session state was never persisted to {state_dir:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn torrent_map_rebuilt_after_restart() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24101);
        let download_id = DownloadId::new();

        {
            let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
                .await
                .unwrap();
            session
                .add_torrent_from_bytes(download_id, minimal_torrent_bytes("restart.bin"))
                .await
                .unwrap();
            assert!(session.get_stats(download_id).is_ok());
            wait_for_session_state(&config.session_state_path).await;
            session.session.stop().await;
        }

        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        assert!(
            session.get_stats(download_id).is_ok(),
            "download must be manageable after restart"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconcile_drops_stale_entries() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24201);
        let stale_id = DownloadId::new();

        std::fs::create_dir_all(&config.session_state_path).unwrap();
        let map_path = config.session_state_path.join(TORRENT_MAP_FILE);
        let stale = PersistedTorrentMap {
            torrents: vec![PersistedTorrentEntry {
                download_id: stale_id,
                torrent_id: 4242,
                seed_started_at: None,
                uploaded_watermark: 0,
            }],
        };
        std::fs::write(&map_path, serde_json::to_vec(&stale).unwrap()).unwrap();

        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        assert!(
            matches!(
                session.get_stats(stale_id),
                Err(ErgasiaError::TorrentNotFound { .. })
            ),
            "stale entry must be dropped, not resurrected"
        );

        let rewritten: PersistedTorrentMap =
            serde_json::from_slice(&std::fs::read(&map_path).unwrap()).unwrap();
        assert!(
            rewritten.torrents.is_empty(),
            "stale entry must be pruned from the persisted map"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_download_id_errors_torrent_not_found() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24401);
        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        let unknown = DownloadId::new();

        assert!(matches!(
            session.get_torrent(unknown),
            Err(ErgasiaError::TorrentNotFound { .. })
        ));
        assert!(matches!(
            session.get_stats(unknown),
            Err(ErgasiaError::TorrentNotFound { .. })
        ));
        assert!(matches!(
            session.delete_torrent(unknown).await,
            Err(ErgasiaError::TorrentNotFound { .. })
        ));
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_deletes_yield_one_ok_one_not_found() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24501);
        let download_id = DownloadId::new();

        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        session
            .add_torrent_from_bytes(download_id, minimal_torrent_bytes("race.bin"))
            .await
            .unwrap();

        let (a, b) = tokio::join!(
            session.delete_torrent(download_id),
            session.delete_torrent(download_id)
        );

        let ok_count = [&a, &b].iter().filter(|r| r.is_ok()).count();
        assert_eq!(ok_count, 1, "exactly one delete must win: {a:?} / {b:?}");
        let loser = if a.is_err() { a } else { b };
        assert!(
            matches!(loser, Err(ErgasiaError::TorrentNotFound { .. })),
            "loser must see TorrentNotFound, got {loser:?}"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shared_torrent_id_survives_first_owners_delete() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24551);
        let download_a = DownloadId::new();
        let download_b = DownloadId::new();
        let torrent_bytes = minimal_torrent_bytes("shared.bin");

        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        let (id_a, _) = session
            .add_torrent_from_bytes(download_a, torrent_bytes.clone())
            .await
            .unwrap();
        let (id_b, _) = session
            .add_torrent_from_bytes(download_b, torrent_bytes)
            .await
            .unwrap();
        assert_eq!(
            id_a, id_b,
            "duplicate info-hash must dedup onto one torrent_id (AlreadyManaged)"
        );

        session.delete_torrent(download_a).await.unwrap();

        assert!(
            session.get_stats(download_b).is_ok(),
            "download_b must still resolve after download_a's delete"
        );
        assert!(
            session.session.get(TorrentIdOrHash::Id(id_b)).is_some(),
            "the shared torrent must not be deleted from librqbit while \
             download_b still references it"
        );

        session.delete_torrent(download_b).await.unwrap();

        assert!(
            matches!(
                session.get_stats(download_b),
                Err(ErgasiaError::TorrentNotFound { .. })
            ),
            "download_b must be gone after its own delete"
        );
        assert!(
            session.session.get(TorrentIdOrHash::Id(id_b)).is_none(),
            "the shared torrent must be deleted once the last owner deletes it"
        );

        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_failure_reports_delete_action_and_restores_mapping() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24601);
        let download_id = DownloadId::new();

        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        // WHY: a mapping to a torrent id librqbit does not manage forces the
        // session.delete failure path deterministically.
        session
            .torrent_map
            .insert(download_id, TorrentRecord::new(999_999));

        let err = session.delete_torrent(download_id).await.unwrap_err();
        assert!(
            matches!(err, ErgasiaError::DeleteAction { .. }),
            "expected DeleteAction (not PauseAction), got {err:?}"
        );
        assert!(
            session.torrent_map.contains_key(&download_id),
            "mapping must be restored after a failed delete"
        );

        // WHY (#590 review): the failed-delete path re-persists the restored
        // mapping so the on-disk side-table matches memory — cancelling the
        // seed monitor can race a watermark persist that snapshots the map
        // with this entry already removed. Assert on the persisted file
        // directly: `reconcile` legitimately drops this bogus id as an orphan
        // (it is not in librqbit's live session), so a reconstruct cannot
        // distinguish the fix; the durable write is what this guards.
        let persisted = session.load_torrent_map().await.unwrap();
        assert!(
            persisted.iter().any(|e| e.download_id == download_id),
            "the restored mapping must be persisted to disk after a failed delete"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unresolvable_magnet_times_out_instead_of_hanging() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path(), 24701);
        config.magnet_resolve_timeout_seconds = 1;
        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        let download_id = DownloadId::new();

        // WHY: a random info-hash no peer will ever announce — librqbit's
        // magnet resolve blocks on the DHT stream indefinitely, so only the
        // configured timeout can settle this call.
        // NOTE: ManagedTorrent (the Ok payload) does not derive Debug, so
        // unwrap_err() is unavailable — match directly instead.
        let result = session
            .add_torrent_from_magnet(
                download_id,
                "magnet:?xt=urn:btih:0000000000000000000000000000000000000001",
            )
            .await;
        let Err(err) = result else {
            panic!("an unresolvable magnet must time out, not resolve");
        };

        assert!(
            matches!(err, ErgasiaError::MagnetResolveTimeout { .. }),
            "expected MagnetResolveTimeout, got {err:?}"
        );
        assert!(
            matches!(
                session.get_stats(download_id),
                Err(ErgasiaError::TorrentNotFound { .. })
            ),
            "a timed-out add must leave no download mapping behind"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn torrent_file_add_ignores_magnet_deadline() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path(), 24707);
        // WHY: zero instantly elapses any bounded await — a local torrent-file
        // add must still succeed because the deadline is magnet-only.
        config.magnet_resolve_timeout_seconds = 0;
        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .unwrap();
        let download_id = DownloadId::new();

        session
            .add_torrent_from_bytes(download_id, minimal_torrent_bytes("deadline-free"))
            .await
            .map(|_| ())
            .expect("a torrent-file add must not be bounded by the magnet deadline");
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn corrupt_torrent_map_is_quarantined() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24301);

        std::fs::create_dir_all(&config.session_state_path).unwrap();
        let map_path = config.session_state_path.join(TORRENT_MAP_FILE);
        std::fs::write(&map_path, b"{ not json").unwrap();

        let session = TorrentSession::new(Section::fixed(config.clone()), test_event_tx())
            .await
            .expect("corrupt side-table must not brick startup");
        assert!(
            map_path.with_extension("json.corrupt").exists(),
            "corrupt map must be quarantined for forensics"
        );
        session.session.stop().await;
    }

    // ── #602: lifecycle watcher announces honest terminal states ───────────

    /// Writes `payload` into the download dir and returns torrent bytes whose
    /// hash check will pass against it — a deterministic completed download
    /// with zero peers.
    async fn single_file_fixture(download_dir: &Path, name: &str, payload: &[u8]) -> bytes::Bytes {
        std::fs::create_dir_all(download_dir).unwrap();
        let file_path = download_dir.join(name);
        std::fs::write(&file_path, payload).unwrap();
        // WHY: librqbit 9 externalized the blocking-work spawner that
        // create_torrent uses internally (was BlockingSpawner::default(),
        // crate-private, in librqbit 8). A fresh, uncontended spawner here
        // reproduces the old behaviour exactly: create_torrent only ever
        // calls spawner.block_in_place(...) (gated on the current runtime
        // flavor, same as v8's Default), never the semaphore-limited path,
        // so the concurrency figure passed to `new` is inert for a spawner
        // nothing else shares — 1 matches librqbit's own one-off precedent
        // (dht_utils.rs test: BlockingSpawner::new(1)).
        let created = librqbit::create_torrent(
            &file_path,
            librqbit::CreateTorrentOptions::default(),
            &librqbit::spawn_utils::BlockingSpawner::new(1),
        )
        .await
        .unwrap();
        created.as_bytes().unwrap()
    }

    /// Two-file directory fixture; librqbit derives the multi-file subfolder
    /// from the torrent name (the directory basename).
    async fn multi_file_fixture(download_dir: &Path, dir_name: &str) -> bytes::Bytes {
        let content_dir = download_dir.join(dir_name);
        std::fs::create_dir_all(&content_dir).unwrap();
        std::fs::write(content_dir.join("a.bin"), b"first payload").unwrap();
        std::fs::write(content_dir.join("b.bin"), b"second payload").unwrap();
        // WHY: see single_file_fixture's WHY note above — same inert
        // per-call spawner.
        let created = librqbit::create_torrent(
            &content_dir,
            librqbit::CreateTorrentOptions::default(),
            &librqbit::spawn_utils::BlockingSpawner::new(1),
        )
        .await
        .unwrap();
        created.as_bytes().unwrap()
    }

    async fn recv_completion(
        rx: &mut aggelmata::EventReceiver,
        download_id: DownloadId,
    ) -> PathBuf {
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                match rx.recv().await {
                    Ok(HarmoniaEvent::DownloadCompleted {
                        download_id: id,
                        path,
                    }) if id == download_id => return path,
                    Ok(_) => continue,
                    Err(e) => panic!("event bus closed before completion arrived: {e}"),
                }
            }
        })
        .await
        .expect("DownloadCompleted must arrive for a hash-complete torrent")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn completed_fixture_announces_completion_with_single_file_path() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24801);
        let torrent_bytes = single_file_fixture(
            &config.download_dir,
            "payload.bin",
            b"deterministic fixture payload",
        )
        .await;

        let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
        let session = TorrentSession::new(Section::fixed(config.clone()), event_tx)
            .await
            .unwrap();
        let download_id = DownloadId::new();
        session
            .add_torrent_from_bytes(download_id, torrent_bytes)
            .await
            .unwrap();

        let path = recv_completion(&mut event_rx, download_id).await;
        assert_eq!(
            path,
            config.download_dir.join("payload.bin"),
            "single-file content path must be the file itself"
        );

        // The watcher stays alive past completion as the seed monitor (the
        // default policy — ratio 1.0 / 72h — is unsatisfied with zero
        // upload), so its seed_tracker entry must still be claimable for
        // cancellation. Self-removal on monitor exit is asserted by
        // seed_policy_pauses_completed_fixture_immediately.
        assert!(
            session.seed_tracker.contains_key(&download_id),
            "the seed monitor must keep its seed_tracker entry while seeding"
        );
        session.session.stop().await;
    }

    #[test]
    fn multi_file_subfolder_resolution_mirrors_librqbit_tiers() {
        use std::ffi::OsStr;
        let stem = Some(OsStr::new("largest-file"));
        // Tier 1: non-empty info name wins.
        assert_eq!(
            resolve_multi_file_subfolder(Some("info-name"), Some("magnet-name"), stem),
            Some(PathBuf::from("info-name"))
        );
        // Empty info name falls through to the handle (magnet) name.
        assert_eq!(
            resolve_multi_file_subfolder(Some(""), Some("magnet-name"), stem),
            Some(PathBuf::from("magnet-name"))
        );
        // Absent/empty names fall through to the largest file's stem.
        assert_eq!(
            resolve_multi_file_subfolder(None, None, stem),
            Some(PathBuf::from("largest-file"))
        );
        assert_eq!(
            resolve_multi_file_subfolder(Some(""), Some(""), stem),
            Some(PathBuf::from("largest-file"))
        );
        // Nothing resolvable: None — the caller must error, never hand back
        // the shared download_dir root.
        assert_eq!(resolve_multi_file_subfolder(None, None, None), None);
        assert_eq!(
            resolve_multi_file_subfolder(Some(""), None, Some(OsStr::new(""))),
            None
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn multi_file_completion_path_is_download_dir_joined_with_name() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24901);
        let torrent_bytes = multi_file_fixture(&config.download_dir, "album").await;

        let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
        let session = TorrentSession::new(Section::fixed(config.clone()), event_tx)
            .await
            .unwrap();
        let download_id = DownloadId::new();
        session
            .add_torrent_from_bytes(download_id, torrent_bytes)
            .await
            .unwrap();

        let path = recv_completion(&mut event_rx, download_id).await;
        assert_eq!(
            path,
            config.download_dir.join("album"),
            "multi-file content path must be the torrent-name directory"
        );
        assert_eq!(
            session.content_path(download_id).unwrap(),
            config.download_dir.join("album"),
            "content_path must agree with the announced path"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restored_watcher_does_not_reannounce_completion() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 25001);
        let torrent_bytes =
            single_file_fixture(&config.download_dir, "restart.bin", b"restart payload").await;
        let download_id = DownloadId::new();

        {
            let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
            let session = TorrentSession::new(Section::fixed(config.clone()), event_tx)
                .await
                .unwrap();
            session
                .add_torrent_from_bytes(download_id, torrent_bytes)
                .await
                .unwrap();
            recv_completion(&mut event_rx, download_id).await;
            wait_for_session_state(&config.session_state_path).await;
            session.session.stop().await;
        }

        let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
        let session = TorrentSession::new(Section::fixed(config.clone()), event_tx)
            .await
            .unwrap();
        assert!(
            session.get_stats(download_id).is_ok(),
            "precondition: the download must be restored"
        );
        // WHY: restored watchers run with announce_completion = false; a
        // bounded quiet window proves no stale completion replays on restart.
        let replay = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match event_rx.recv().await {
                    Ok(HarmoniaEvent::DownloadCompleted { .. }) => return,
                    Ok(_) => continue,
                    Err(_) => std::future::pending::<()>().await,
                }
            }
        })
        .await;
        assert!(
            replay.is_err(),
            "a restored finished torrent must not re-announce DownloadCompleted"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn progress_reports_seeding_for_completed_fixture() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 25101);
        let torrent_bytes =
            single_file_fixture(&config.download_dir, "seeded.bin", b"seeded payload").await;

        let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
        let session = TorrentSession::new(Section::fixed(config.clone()), event_tx)
            .await
            .unwrap();
        let download_id = DownloadId::new();
        session
            .add_torrent_from_bytes(download_id, torrent_bytes)
            .await
            .unwrap();
        recv_completion(&mut event_rx, download_id).await;

        let progress = session.progress(download_id).unwrap();
        assert_eq!(
            progress.state,
            crate::state::DownloadState::Seeding,
            "a finished live torrent must report Seeding, never Downloading"
        );
        assert_eq!(progress.percent_complete, 100);
        assert!(progress.error.is_none());
        session.session.stop().await;
    }

    // ── #590: seed-policy enforcement ───────────────────────────────────────

    #[test]
    fn cumulative_uploaded_sums_base_and_epoch_without_double_count() {
        // Fresh add: no prior epochs.
        assert_eq!(cumulative_uploaded(0, 500), 500);
        // Mid-epoch growth on top of a persisted base.
        assert_eq!(cumulative_uploaded(100, 50), 150);
        // After persisting 150 and restarting, the total becomes the new
        // base and the fresh epoch counter restarts at 0 — the persisted
        // epochs are never re-added.
        assert_eq!(cumulative_uploaded(150, 0), 150);
        assert_eq!(cumulative_uploaded(u64::MAX, 1), u64::MAX);
    }

    async fn recv_seed_satisfied(
        rx: &mut aggelmata::EventReceiver,
        download_id: DownloadId,
        timeout: Duration,
    ) -> (u64, u64) {
        tokio::time::timeout(timeout, async {
            loop {
                match rx.recv().await {
                    Ok(HarmoniaEvent::SeedPolicySatisfied {
                        download_id: id,
                        uploaded_bytes,
                        downloaded_bytes,
                    }) if id == download_id => return (uploaded_bytes, downloaded_bytes),
                    Ok(_) => continue,
                    Err(e) => panic!("event bus closed before SeedPolicySatisfied arrived: {e}"),
                }
            }
        })
        .await
        .expect("SeedPolicySatisfied must arrive once the policy is met")
    }

    async fn wait_for_seed_start_persisted(map_path: &Path) {
        for _ in 0..100 {
            if let Ok(bytes) = std::fs::read(map_path)
                && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
                && value["torrents"]
                    .as_array()
                    .is_some_and(|t| t.iter().any(|e| !e["seed_started_at"].is_null()))
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("seed_started_at was never persisted to {map_path:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn seed_policy_pauses_completed_fixture_immediately() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path(), 25201);
        // WHY: a zero time threshold satisfies on the monitor's FIRST check
        // (check-before-sleep), so no poll interval elapses on this path.
        config.seed_time_threshold_hours = 0;
        let payload: &[u8] = b"seed-policy fixture payload";
        let torrent_bytes = single_file_fixture(&config.download_dir, "policy.bin", payload).await;

        let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
        let session = TorrentSession::new(Section::fixed(config.clone()), event_tx)
            .await
            .unwrap();
        let download_id = DownloadId::new();
        session
            .add_torrent_from_bytes(download_id, torrent_bytes)
            .await
            .unwrap();

        recv_completion(&mut event_rx, download_id).await;
        let (uploaded, downloaded) =
            recv_seed_satisfied(&mut event_rx, download_id, Duration::from_secs(30)).await;
        assert_eq!(uploaded, 0, "a zero-peer fixture uploads nothing");
        assert_eq!(
            downloaded,
            payload.len() as u64,
            "the ratio denominator must be the torrent's total_bytes"
        );

        let stats = session.get_stats(download_id).unwrap();
        assert!(
            matches!(stats.state, TorrentStatsState::Paused),
            "a satisfied seed policy must pause the torrent, got {:?}",
            stats.state
        );
        assert!(stats.finished);
        assert_eq!(
            session.progress(download_id).unwrap().state,
            crate::state::DownloadState::SeedPolicySatisfied,
        );

        // The watcher self-removes its seed_tracker entry once the monitor
        // exits satisfied.
        let mut cleaned = false;
        for _ in 0..100 {
            if !session.seed_tracker.contains_key(&download_id) {
                cleaned = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            cleaned,
            "the watcher must self-remove its seed_tracker entry after the policy is satisfied"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn seed_clock_continues_from_persisted_start_across_restart() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path(), 25301);
        // Phase 1: thresholds nothing can satisfy — the monitor records the
        // seed start and keeps seeding.
        config.seed_ratio_threshold = 999.0;
        config.seed_time_threshold_hours = 999;
        let torrent_bytes =
            single_file_fixture(&config.download_dir, "clock.bin", b"seed clock payload").await;
        let download_id = DownloadId::new();
        let map_path = config.session_state_path.join(TORRENT_MAP_FILE);

        {
            let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
            let session = TorrentSession::new(Section::fixed(config.clone()), event_tx)
                .await
                .unwrap();
            session
                .add_torrent_from_bytes(download_id, torrent_bytes)
                .await
                .unwrap();
            recv_completion(&mut event_rx, download_id).await;
            wait_for_seed_start_persisted(&map_path).await;
            wait_for_session_state(&config.session_state_path).await;
            session.session.stop().await;
        }

        // Backdate the persisted seed start by two hours: only the persisted
        // wall-clock timestamp can satisfy a 1-hour threshold immediately
        // after restart — a monitor that reset its clock would report ~0s
        // elapsed and never fire inside this test's window.
        let raw = std::fs::read(&map_path).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let two_hours_ago = (Timestamp::now() - jiff::SignedDuration::from_hours(2)).to_string();
        value["torrents"][0]["seed_started_at"] = serde_json::Value::String(two_hours_ago);
        std::fs::write(&map_path, serde_json::to_vec(&value).unwrap()).unwrap();

        let mut restarted = config.clone();
        restarted.seed_ratio_threshold = 999.0;
        restarted.seed_time_threshold_hours = 1;
        let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
        let session = TorrentSession::new(Section::fixed(restarted), event_tx)
            .await
            .unwrap();
        assert!(
            session.get_stats(download_id).is_ok(),
            "precondition: the download must be restored"
        );

        recv_seed_satisfied(&mut event_rx, download_id, Duration::from_secs(30)).await;
        assert_eq!(
            session.progress(download_id).unwrap().state,
            crate::state::DownloadState::SeedPolicySatisfied,
            "a restored finished torrent paused by the monitor must map to SeedPolicySatisfied"
        );
        session.session.stop().await;
    }

    // NOTE: real two-session peer transfer over localhost — the fixture
    // tests above are the always-on guarantee for the policy mechanics; this
    // additionally proves the ratio numerator accumulates from real uploads.
    #[tokio::test(flavor = "multi_thread")]
    async fn seed_ratio_crossing_pauses_seeder() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path(), 25401);
        // Only the ratio can satisfy: the time threshold is out of reach.
        config.seed_ratio_threshold = 1.0;
        config.seed_time_threshold_hours = 999;
        let payload = vec![0xC3u8; 64 * 1024];
        let torrent_bytes = single_file_fixture(&config.download_dir, "ratio.bin", &payload).await;

        let (event_tx, mut event_rx) = aggelmata::create_event_bus(64);
        let seeder = TorrentSession::with_seed_poll_interval(
            Section::fixed(config.clone()),
            event_tx,
            Duration::from_millis(500),
        )
        .await
        .unwrap();
        let download_id = DownloadId::new();
        seeder
            .add_torrent_from_bytes(download_id, torrent_bytes.clone())
            .await
            .unwrap();
        recv_completion(&mut event_rx, download_id).await;
        let seeder_port = seeder
            .session
            .announce_port()
            .expect("seeder must expose a TCP listen port");

        // Leech: a bare librqbit session pointed straight at the seeder — no
        // DHT, no trackers, so the seeder is its only possible source.
        let leech_dir = dir.path().join("leech");
        std::fs::create_dir_all(&leech_dir).unwrap();
        // WHY: librqbit 9 dropped SessionOptions::listen_port_range (see the
        // WHY note in with_seed_poll_interval) — this reproduces the same
        // scan-the-range-by-retrying-construction loop for the bare leech
        // session. ipv4_only is left at its default (false, dual-stack) for
        // the same reason as the production session — the loopback outbound
        // connect to the seeder below works the same either way, since
        // ipv4_only governs binding, not whether an IPv4 peer is reachable.
        let mut leech_session = None;
        for port in 25501u16..25509 {
            let attempt = Session::new_with_opts(
                leech_dir.clone(),
                SessionOptions {
                    dht: None,
                    persistence: None,
                    listen: Some(ListenerOptions {
                        listen_addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port),
                        mode: ListenerMode::TcpOnly,
                        enable_upnp_port_forwarding: false,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await;
            if let Ok(s) = attempt {
                leech_session = Some(s);
                break;
            }
        }
        let leech_session = leech_session.expect("a free TCP port for the leech session");
        let response = leech_session
            .add_torrent(
                AddTorrent::TorrentFileBytes(torrent_bytes),
                Some(AddTorrentOptions {
                    initial_peers: Some(vec![std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        seeder_port,
                    ))]),
                    disable_trackers: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        let AddTorrentResponse::Added(_, leech_handle) = response else {
            panic!("leech add must register a fresh torrent");
        };
        tokio::time::timeout(
            Duration::from_secs(120),
            leech_handle.wait_until_completed(),
        )
        .await
        .expect("leech must finish downloading from the seeder")
        .expect("leech download must succeed");

        let (uploaded, downloaded) =
            recv_seed_satisfied(&mut event_rx, download_id, Duration::from_secs(60)).await;
        assert!(
            uploaded >= downloaded,
            "crossing 1.0 requires uploaded ({uploaded}) >= total ({downloaded})"
        );
        let stats = seeder.get_stats(download_id).unwrap();
        assert!(
            matches!(stats.state, TorrentStatsState::Paused),
            "the seeder must be paused after crossing the ratio, got {:?}",
            stats.state
        );

        leech_session.stop().await;
        seeder.session.stop().await;
    }
}
