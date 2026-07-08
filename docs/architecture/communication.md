# Communication architecture

> Internal communication design for Harmonia: event bus topology, channel types, HarmoniaEvent enum, and subscription patterns.
> Subsystem identities are defined in [lexicon.md](../lexicon.md).
> The dependency DAG and communication classification live in [subsystems.md](subsystems.md).
> Event types live in `crates/themelion/src/aggelia/`; see [cargo.md](cargo.md).

## Purpose

This document specifies how Harmonia subsystems communicate at runtime. It does not restate subsystem identities or the dependency DAG; those belong to `docs/lexicon.md` and `docs/architecture/subsystems.md`. What this document adds is the implementation layer: which tokio channel type each communication path uses, what events the `HarmoniaEvent` enum carries, how subsystems subscribe and emit, and how archon wires the bus at startup.

Two patterns cover all inter-subsystem communication: direct trait calls for synchronous request-response, and Aggelia (the internal event bus) for fire-and-forget announcements. `docs/architecture/subsystems.md` classifies every inter-subsystem path as direct call or event; this document specifies the implementation details of those paths.

---

## The rule

**"If the caller needs the result to continue, direct call. If the caller is announcing something happened, event."**

This is the single most important communication design rule in Harmonia. Every new inter-subsystem path must be classified against it before implementation.

**Direct call examples:**

- `Paroche → Exousia::authorize()`: Paroche cannot stream until it has the authorization decision. The result determines the next step. Direct trait call.
- `Kathodos → Epignosis::resolve()`: Kathodos cannot determine the correct file name and library path until it knows what the media is. The result gates the rename. Direct trait call.
- `Aitesis → MonitorService::create_want()`: Aitesis cannot move an approved request to `Monitoring` until the injected monitor boundary accepts the want. The result is required to proceed. Direct trait call.

**Event examples:**

- `Kathodos` announces `ImportCompleted`: Kathodos has finished importing a media item. Whether Syndesmos notifies Plex, whether Kritike queues a quality check, whether Prostheke acquires subtitles; none of these are Kathodos's concern. It emits and moves on.
- `Ergasia` announces `DownloadProgress`: Ergasia is reporting a fact about active download state. The web UI may display it, or nothing may subscribe. Ergasia does not care.
- `Paroche` announces `ScrobbleRequired`: playback occurred; scrobbling is now warranted. Whether Last.fm is configured, whether Syndesmos is running; not Paroche's concern.

---

## Channel topology

**`tokio::sync::broadcast`:** pub/sub events where multiple subscribers each react independently. Every subscriber receives a copy of the event. Buffer size: 1024 (configurable via Horismos under `[aggelia] buffer_size`). Used for all `HarmoniaEvent` variants — `create_event_bus(buffer_size)` (`themelion::aggelia`) wraps `broadcast::channel`, and it is the ONLY config-driven channel in the system.

Syntaxis → Ergasia is **not** a channel. There was a design for a bounded mpsc download queue (sized by an `aggelia.download_queue_size` field), but it never had a consumer and the field was removed from the schema (#598) — Syntaxis dispatches to Ergasia with a direct async trait call instead (`ergasia::DownloadEngine::start_download`, awaited from the dispatch task), which is the natural fit under this doc's own rule: Syntaxis needs the `Result<DownloadId, ErgasiaError>` to release or hold the slot it claimed. Concurrency is bounded by an in-process `SlotAllocator` (plain counters against `syntaxis.max_concurrent_downloads`/`.max_per_tracker`), not by channel capacity.

| Communication Path | Channel Type | Rationale |
|--------------------|-------------|-----------|
| `ImportCompleted` event | `broadcast` | Syndesmos (Plex notify), Kritike (quality check), Prostheke (subtitle lookup) all react independently |
| `QualityUpgradeTriggered` event | `broadcast` | Monitoring adapters may re-trigger acquisition; no guaranteed subscriber |
| `DownloadProgress` event | `broadcast` | Defined on the enum for the web UI / API layer, but has no production emitter — nothing constructs and sends it today |
| `DownloadCompleted` event | `broadcast` | Syntaxis triggers post-processing pipeline on completion |
| `DownloadFailed` event | `broadcast` | Syntaxis handles retry or failure escalation |
| `SeedPolicySatisfied` event | `broadcast` | Web UI may display seeding-complete status; the authoritative cleanup signal is a direct call to Kathodos, not this event |
| `SearchCompleted` event | `broadcast` | Monitoring adapters may evaluate search results for acquisition decisions |
| `PlexNotifyRequired` event | `broadcast` | Syndesmos is the sole consumer, but broadcast allows future subscribers |
| `ScrobbleRequired` event | `broadcast` | Syndesmos is the sole consumer |
| `TidalWantListSynced` event | `broadcast` | Monitoring adapters may add new want-list entries |
| `MetadataEnriched` event | `broadcast` | Library indexing layer and web UI react to enrichment completion |
| `LibraryScanCompleted` event | `broadcast` | Web UI and health reporting react to full scan completion |
| `SubtitleAcquired` event | `broadcast` | Paroche reacts to new subtitle availability for active streams |
| Syntaxis → Ergasia dispatch | direct call | Syntaxis needs the result to manage its slot — see above |

All direct calls between subsystems (Paroche → Exousia, Kathodos → Epignosis, Syntaxis → Ergasia, etc.) use synchronous trait method calls, not channels. Those paths are enumerated in `docs/architecture/subsystems.md`.

Two distinct `DownloadProgress` types exist and are easy to conflate: `themelion::HarmoniaEvent::DownloadProgress` (the broadcast event below — defined, never emitted) and `ergasia::DownloadProgress` (a different shape, returned by the `DownloadEngine::get_progress` polling call — see [download/torrent.md](../download/torrent.md)). Progress is available today only by polling the latter; there is no broadcast push.

---

## HarmoniaEvent enum

The complete event enum. Lives in `crates/themelion/src/aggelia/`. All event types in `themelion` are shared across all subsystems without circular dependencies; see [cargo.md](cargo.md) for why event types live in the shared leaf crate.

```rust
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum HarmoniaEvent {
    // Acquisition pipeline

    /// Kathodos successfully imported a media item into the library.
    /// Subscribers: Syndesmos (Plex notify), Kritike (quality assessment),
    ///              Prostheke (subtitle acquisition), web UI (library update)
    ImportCompleted {
        media_id: MediaId,
        media_type: MediaType,
        path: PathBuf,
    },

    /// Kritike determined a library item does not meet its quality profile.
    /// Subscribers: monitoring layer (re-trigger acquisition for the item)
    QualityUpgradeTriggered {
        media_id: MediaId,
        current_quality: QualityProfile,
    },

    /// Ergasia is reporting download progress during an active transfer.
    /// Subscribers: web UI / API layer (real-time progress display)
    DownloadProgress {
        download_id: DownloadId,
        percent: u8,
        bytes_downloaded: u64,
        bytes_total: u64,
    },

    /// Ergasia completed a download successfully.
    /// Subscribers: Syntaxis (trigger post-processing pipeline)
    DownloadCompleted {
        download_id: DownloadId,
        path: PathBuf,
    },

    /// Ergasia failed a download — all retries exhausted.
    /// Subscribers: Syntaxis (handle failure escalation, update queue state)
    DownloadFailed {
        download_id: DownloadId,
        reason: String,
    },

    /// Ergasia's seeding monitor determined ratio/time policy is satisfied.
    /// Informational — the authoritative cleanup signal is a direct call to Kathodos.
    /// Subscribers: web UI (display seeding completion status)
    SeedPolicySatisfied {
        download_id: DownloadId,
        uploaded_bytes: u64,
        downloaded_bytes: u64,
    },

    /// Zetesis completed a search against configured indexers.
    /// Subscribers: monitoring layer (evaluate candidates for acquisition)
    SearchCompleted {
        query_id: QueryId,
        result_count: usize,
    },

    // Integration events

    /// Kathodos imported new media — Plex library needs a scan notification.
    /// Subscribers: Syndesmos (call Plex refresh endpoint)
    PlexNotifyRequired {
        media_id: MediaId,
    },

    /// Paroche detected playback of a track — scrobbling is now warranted.
    /// Subscribers: Syndesmos (submit scrobble to Last.fm)
    ScrobbleRequired {
        track_id: MediaId,
        user_id: UserId,
    },

    /// Syndesmos completed a Tidal want-list sync.
    /// Subscribers: monitoring layer (add new want-list items to monitored set)
    TidalWantListSynced {
        added: Vec<MediaId>,
    },

    // Library events

    /// Epignosis completed metadata enrichment for a library item.
    /// Subscribers: web UI / API layer (update displayed metadata),
    ///              library indexer (update search index)
    MetadataEnriched {
        media_id: MediaId,
        media_type: MediaType,
    },

    /// A full library scan completed.
    /// Subscribers: web UI / API layer (refresh library view),
    ///              Kritike (run health assessment on newly scanned items)
    LibraryScanCompleted {
        items_scanned: usize,
        items_added: usize,
        items_removed: usize,
    },

    /// Prostheke acquired subtitle tracks for a media item.
    /// Subscribers: Paroche (update available subtitle tracks for active streams)
    SubtitleAcquired {
        media_id: MediaId,
        languages: Vec<String>,
    },
}
```

**Why `#[non_exhaustive]`:** New event variants will be added as the system grows. `#[non_exhaustive]` mandates a wildcard arm in all subscriber match statements: `_ => {}`. Without it, adding any new variant forces recompilation of every subscriber crate. Mandated by `standards/RUST.md` for all public enums that may grow.

**Why all fields are owned types:** `broadcast::Sender` requires `Clone`. Events cannot carry references; they must be fully owned values. All IDs are newtypes (not `&str`, not `&[u8]`). String fields that might hold large data (e.g., `reason`) are acceptable because failed downloads are rare; the typical high-frequency event (`DownloadProgress`) carries only primitive types.

---

## Startup wiring

archon creates all channels at startup and distributes handles via constructor injection. No subsystem imports a "bus crate"; this avoids the circular dependency pitfall described in `docs/architecture/cargo.md`.

```rust
// In archon's serve.rs
use themelion::create_event_bus;

// Create the broadcast bus — all HarmoniaEvent variants flow through this single sender
let (event_tx, _event_rx) = create_event_bus(boot_config.aggelia.buffer_size);

// Each subscribing subsystem gets a Receiver by calling .subscribe() on the Sender.
// The initial _event_rx is discarded — it exists only to keep the channel open
// until the first real subscriber subscribes.
let syndesmos_rx = event_tx.subscribe();
let kritike_rx   = event_tx.subscribe();
let prostheke_rx = event_tx.subscribe();
let paroche_rx   = event_tx.subscribe();
// ... each subsystem that subscribes gets its own Receiver clone

// Subsystems receive a Sender clone (to emit) and their own Receiver (to subscribe)
let syndesmos = Syndesmos::new(
    config_handle.section(|c| &c.syndesmos),
    event_tx.clone(),   // to emit TidalWantListSynced
    syndesmos_rx,       // to receive PlexNotifyRequired, ScrobbleRequired
    // ...
);

// Syntaxis holds an Arc<dyn DownloadEngine> and calls it directly — there is
// no channel between Syntaxis and Ergasia (see Channel topology above).
let engine: Arc<dyn ergasia::DownloadEngine> = Arc::new(TorrentSession::new(&config.ergasia).await?);
let syntaxis = DownloadQueue::new(pool.clone(), engine, event_tx.clone(), /* ... */);
```

**Key points:**
- `create_event_bus(aggelia.buffer_size)` is created once in archon.
- Each subsystem that emits events receives a `broadcast::Sender<HarmoniaEvent>` clone.
- Each subsystem that subscribes calls `.subscribe()` to create its `broadcast::Receiver<HarmoniaEvent>`; this happens before the subsystems start processing, so no events are missed.
- Syntaxis → Ergasia dispatch does not go through this bus, or through any channel — it is a direct call (see Channel topology above).
- No subsystem imports a crate to gain access to the bus. Handles are constructor-injected.

---

## Subscriber pattern

Each subscribing subsystem runs a dedicated event-handling task. The task is spawned during startup, receives a `broadcast::Receiver<HarmoniaEvent>`, and loops until the channel closes.

```rust
use tokio::sync::broadcast;
use tracing::Instrument;
use themelion::HarmoniaEvent;

async fn run_event_handler(
    mut rx: broadcast::Receiver<HarmoniaEvent>,
    // subsystem state as needed
) {
    let span = tracing::info_span!("syndesmos_event_handler");
    async move {
        loop {
            match rx.recv().await {
                Ok(HarmoniaEvent::PlexNotifyRequired { media_id }) => {
                    // React to this specific event
                    if let Err(e) = notify_plex(media_id).await {
                        tracing::error!(error = %e, "Plex notification failed");
                    }
                }
                Ok(HarmoniaEvent::ScrobbleRequired { track_id, user_id }) => {
                    // React to this specific event
                    if let Err(e) = submit_scrobble(track_id, user_id).await {
                        tracing::error!(error = %e, "scrobble submission failed");
                    }
                }
                Ok(_) => {
                    // Required wildcard — HarmoniaEvent is #[non_exhaustive]
                    // Unrecognized variants are silently ignored
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // This subscriber fell behind — n events were dropped from its buffer.
                    // Dropped notification events are acceptable — they are not commands.
                    tracing::warn!(dropped = n, "event receiver lagged — events dropped");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    // Channel closed — archon is shutting down
                    tracing::info!("event channel closed, shutting down event handler");
                    break;
                }
            }
        }
    }
    .instrument(span)
    .await
}
```

**Tracing spans:** Every spawned event-handling task uses `.instrument(span)` to propagate the tracing context. Never `tokio::spawn(async { ... })` without `.instrument()`. Mandated by `standards/RUST.md`.

**Lagged handling:** When a subscriber falls behind (slow event handler, burst of events), tokio's broadcast channel drops the oldest buffered events for that subscriber and delivers `RecvError::Lagged(n)`. The correct response is to log and continue; not to crash, not to re-subscribe. Notification events are not commands; missing a Plex notify or a scrobble signal is acceptable. The system recovers on the next event.

**Blocking in event handlers:** If an event handler needs to do substantial CPU-bound work, it must `tokio::spawn()` a new task from inside the match arm; the event loop itself must remain unblocked. I/O-bound work (external HTTP calls) is fine with `.await` since it does not block the executor.

---

## Emitter pattern

Emitting an event is fire-and-forget. The emitter does not wait for any subscriber to process the event, and it does not care whether subscribers exist.

```rust
// Fire-and-forget — .ok() discards the error when no subscribers are active.
// This is by design: optional integrations (Plex, Last.fm, Tidal) may not be
// configured, in which case no subscriber exists. That is not an error.
event_tx.send(HarmoniaEvent::ImportCompleted {
    media_id,
    media_type,
    path,
}).ok();
```

**Why `.ok()`:** `broadcast::Sender::send` returns `Err(SendError<T>)` when there are no active receivers. For notification events, no receivers is a valid operational state; the optional integration features (`plex`, `lastfm`, `tidal`) may not be enabled. `.ok()` converts the `Result` to `Option` and discards the `None`, which is idiomatic for this pattern.

**No blocking on emit:** `broadcast::Sender::send` does not require `.await`; it is synchronous. The emitter continues immediately after the call. If the broadcast buffer is full for some receivers (they've lagged), those receivers see `RecvError::Lagged` on their next read; the sender is never blocked by a slow subscriber.

---

## Anti-patterns

**Events must be past tense.** An event is an announcement of something that already occurred. Use `ImportCompleted`, not `StartImport`. Use `ScrobbleRequired` (the fact that scrobbling is now needed is a past event; the playback occurred), not `Scrobble` (a command). Commands belong in direct calls, not events.

**Events must not carry references.** `broadcast::Sender` requires `T: Clone`. All event fields must be owned types; no `&str`, no `&Path`, no `Arc<T>` holding references into subsystem-internal state. If the data is large and cloning is expensive, put an ID in the event (e.g., `media_id: MediaId`) and let subscribers fetch the data they need via direct call.

**Subscribers must not block the event loop.** If event processing requires heavy computation or a slow external call, `tokio::spawn()` from inside the match arm. The event-handling loop must remain responsive to subsequent events.

**No subsystem should subscribe to its own events.** `Kathodos` does not subscribe to `ImportCompleted`. `Ergasia` does not subscribe to `DownloadCompleted`. Events are for cross-subsystem reactions. Intra-subsystem state updates happen through direct state mutation within the subsystem.

**No event as a command.** If Subsystem A emits an event and Subsystem B *must* handle it for the system to be correct, that is not an event relationship; it is a direct call. Events are for reactions that are desirable but not required for the emitter's correctness. If Syndesmos does not react to `PlexNotifyRequired`, the media is still imported correctly; Plex just doesn't know yet. That is an acceptable event relationship.
