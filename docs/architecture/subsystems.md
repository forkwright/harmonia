# Subsystem Architecture

> How Harmonia's subsystems communicate, what each owns, and where the boundaries are.
> Subsystem identities are defined in [lexicon.md](../lexicon.md).
> The dependency DAG lives in [architecture/subsystems.md](../architecture/subsystems.md).
> This document adds: ownership boundaries, interface contracts, and communication classification.

## Purpose and Scope

This document enriches the Phase 2 naming deliverables with the HOW. It does not restate subsystem identities — those belong to `docs/lexicon.md`, which is the single source of truth for subsystem names, pronunciations, and layer tests. It does not restate the dependency graph — that belongs to `docs/architecture/subsystems.md`. What this document adds is the operational layer: what each subsystem exclusively owns, what it must not own, what its public trait surface looks like, and — critically — whether each inter-subsystem communication path is a direct call or an event.

---

## Communication Classification Rule

**Locked decision:** "If the caller needs the result to continue, direct call. If the caller is announcing something happened, event."

This distinction maps cleanly onto two Rust patterns:

- **Direct call** — synchronous trait method call across a crate boundary. The caller holds the awaited result before proceeding. `Paroche` calls `Exousia::authorize()` and cannot stream until it gets a decision back. `Taxis` calls `Epignosis::resolve_metadata()` and cannot rename files until it knows what the media is.
- **Event** — fire-and-forget broadcast via Aggelia (the internal event bus). The emitter announces a past-tense fact and moves on. `Taxis` announces `ImportCompleted` — it does not wait to find out whether Syndesmos notified Plex, whether Kritike queued a quality check, or whether Prostheke started subtitle lookup. Those are reactions, not returns.

| Communication Type | Inter-Subsystem Paths |
|--------------------|----------------------|
| **Direct Calls** | Episkope → Zetesis (search for wanted media) |
| | Paroche → Exousia (authorize streaming request) |
| | Taxis → Epignosis (metadata lookup before rename) |
| | Aitesis → Exousia (check request authorization limits) |
| | Aitesis → Epignosis (validate requested media identity) |
| | Aitesis → Episkope (begin monitoring approved request) |
| | Episkope → Syntaxis (enqueue found items) |
| | Episkope → Epignosis (verify candidate identity) |
| | Syntaxis → Ergasia (execute download) |
| | Syntaxis → Taxis (trigger import after completion) |
| | Taxis → Kritike (register imported item for curation tracking) |
| | Taxis → Prostheke (trigger subtitle acquisition) |
| | Kritike → Episkope (trigger quality upgrade re-acquisition) |
| | Prostheke → Epignosis (media identity for subtitle lookup) |
| | Epignosis → Syndesmos (Last.fm artist data supply) |
| | Episkope → Syndesmos (Tidal want-list sync) |
| | All subsystems → Horismos (config read — passive, not listed separately below) |
| **Events via Aggelia** | `ImportCompleted` — emitted by Taxis on successful library import |
| | `QualityUpgradeTriggered` — emitted by Kritike when upgrade criteria met |
| | `DownloadProgress` — emitted by Ergasia during active download |
| | `DownloadCompleted` — emitted by Ergasia on successful completion |
| | `PlexNotifyRequired` — emitted by Taxis, consumed by Syndesmos |
| | `ScrobbleRequired` — emitted by Paroche (on playback), consumed by Syndesmos |
| | `TidalWantListSynced` — emitted by Syndesmos after Tidal sync |

Config reads are not listed individually. Every subsystem reads from Horismos at construction time (receiving `Arc<SubsystemConfig>`) — this is a passive dependency, not a call in the operational sense.

---

## Domain Ownership Table

Each subsystem owns a clearly bounded set of data and behavior. The "Must NOT Own" column prevents scope creep — if a subsystem starts accumulating concerns from another column, the boundary has eroded.

| Subsystem | Owns | Public Trait Surface | Must NOT Own |
|-----------|------|---------------------|--------------|
| **Horismos** | All configuration values — paths, thresholds, API keys (non-secret), feature flags | `fn config() -> &Config`, `fn subsystem_config(name) -> &SubsystemConfig`, `fn validate_at_startup()` | Business logic, secrets in committed files |
| **Exousia** | User identities, password hashes, JWT issuance/validation, API key issuance/validation, refresh token lifecycle | `fn authenticate(credentials) -> Result<Session>`, `fn authorize(token, operation) -> Result<Permission>`, `fn issue_api_key(user_id, label) -> Result<ApiKey>` | Media-domain knowledge, per-subsystem access rules |
| **Syndesmos** | External API credentials (Plex, Last.fm, Tidal), retry logic, rate limiting for external services | `fn notify_plex_import(media_id)`, `fn scrobble(track_id, user_id)`, `fn sync_tidal_want_list() -> Result<Vec<MediaId>>` | Internal state, any logic beyond integration boundary |
| **Epignosis** | Metadata cache, provider credential management, rate limiting for metadata providers | `fn resolve(media_identity) -> Result<Metadata>`, `fn enrich(item) -> Result<EnrichedItem>`, `fn invalidate_cache(media_id)` | Media file paths, download state, library organization |
| **Zetesis** | Indexer credentials, protocol negotiation (Torznab/Newznab), Cloudflare bypass coordination | `fn search(query: SearchQuery) -> Result<Vec<SearchResult>>`, `fn test_indexer(config) -> Result<IndexerStatus>` | Download execution, queue management, media identity |
| **Ergasia** | BitTorrent session state, seeding rules, archive extraction, download progress tracking | `fn start_download(spec: DownloadSpec) -> Result<DownloadId>`, `fn cancel_download(id)`, `fn get_progress(id) -> Result<DownloadProgress>` | Queue priority, post-download pipeline, metadata |
| **Syntaxis** | Download queue, priority rules, bandwidth allocation, post-processing pipeline state | `fn enqueue(item: QueueItem) -> Result<QueuePosition>`, `fn cancel(item_id)`, `fn get_queue_state() -> Result<QueueSnapshot>` | Download execution (delegates to Ergasia), import (delegates to Taxis) |
| **Taxis** | Library directory structure, file naming schema enforcement, import state | `fn import(download: CompletedDownload) -> Result<LibraryItem>`, `fn rename_in_place(item_id) -> Result<()>` | Metadata resolution (delegates to Epignosis), subtitle acquisition (delegates to Prostheke) |
| **Kritike** | Curation rules, quality profile enforcement, library health state, cleanup rules | `fn assess(item_id) -> Result<QualityAssessment>`, `fn scan_library() -> Result<HealthReport>`, `fn register_imported(item_id)` | Acquisition pipeline logic, metadata enrichment |
| **Prostheke** | Subtitle files, subtitle provider credentials, language preferences enforcement | `fn acquire(media_id, languages) -> Result<Vec<SubtitleTrack>>`, `fn sync_timing(subtitle_id, audio_track) -> Result<()>` | Media file organization, metadata identity |
| **Paroche** | HTTP streaming state, OPDS catalog generation, transcoding session lifecycle | `fn stream(media_id, range) -> Result<StreamResponse>`, `fn get_opds_catalog() -> Result<OpdsFeed>`, `fn transcode(media_id, profile) -> Result<TranscodeSession>` | Authorization decisions (delegates to Exousia), library organization |
| **Aitesis** | Request workflow state (submission, approval, tracking), per-user request limits | `fn submit_request(user_id, media_identity) -> Result<RequestId>`, `fn get_status(request_id) -> Result<RequestStatus>`, `fn list_requests(user_id) -> Result<Vec<Request>>` | Acquisition pipeline, media identity resolution beyond validation |
| **Episkope** | Wanted media registry, release schedule tracking, acquisition trigger state | `fn add_wanted(identity: MediaIdentity) -> Result<WantedItem>`, `fn check_missing() -> Result<Vec<WantedItem>>`, `fn mark_acquired(item_id)` | Download execution, metadata enrichment, quality judgment |
| **Aggelia** | Internal event channel handles, `HarmoniaEvent` enum definition (lives in harmonia-common) | `HarmoniaEvent` enum, `broadcast::Sender<HarmoniaEvent>` distributed by harmonia-host, `broadcast::Receiver<HarmoniaEvent>` held per-subscriber | No subsystems — Aggelia carries messages, it does not call subsystems |

---

## Dependency Classification

Each edge from the topology.md DAG, classified by interaction type:

| Caller | Callee | Type | Reason |
|--------|--------|------|--------|
| Episkope | Zetesis | Direct call | Needs search results to decide what to enqueue |
| Episkope | Syntaxis | Direct call | Needs enqueue confirmation before marking wanted item as in-progress |
| Episkope | Epignosis | Direct call | Needs precise identity before treating a candidate as a match |
| Episkope | Syndesmos | Direct call | Tidal sync returns want-list additions that Episkope acts on |
| Zetesis | Horismos | Config read | Reads indexer credentials and configuration at construction |
| Zetesis | Exousia | Direct call | Verifies caller is authorized before serving search results |
| Ergasia | Horismos | Config read | Reads download directory and bandwidth limits at construction |
| Syntaxis | Ergasia | Direct call | Initiates or cancels downloads; needs confirmation |
| Syntaxis | Taxis | Direct call | Triggers import after download completion; hands off file paths |
| Taxis | Epignosis | Direct call | Needs metadata before it can determine the correct file name and path |
| Taxis | Horismos | Config read | Reads library root paths and naming schema at construction |
| Taxis | Kritike | Direct call | Registers imported item; Kritike acknowledges before Taxis marks import complete |
| Taxis | Prostheke | Direct call | Triggers subtitle acquisition; handoff is synchronous at import time |
| Kritike | Episkope | Direct call | Triggers upgrade re-acquisition; needs Episkope to accept the wanted item |
| Kritike | Horismos | Config read | Reads curation rules and quality thresholds at construction |
| Prostheke | Epignosis | Direct call | Needs precise media identity to look up matching subtitles |
| Prostheke | Horismos | Config read | Reads subtitle language preferences and provider credentials |
| Paroche | Exousia | Direct call | Needs authorization decision before streaming; cannot proceed without it |
| Paroche | Horismos | Config read | Reads transcoding profiles and stream limits at construction |
| Aitesis | Episkope | Direct call | Hands approved request to Episkope; needs acknowledgment |
| Aitesis | Epignosis | Direct call | Validates requested media identity before accepting the request |
| Aitesis | Exousia | Direct call | Checks per-user request limits before accepting submission |
| Epignosis | Horismos | Config read | Reads provider API keys and cache TTL at construction |
| Epignosis | Syndesmos | Direct call | Requests Last.fm artist data; needs data to return enriched metadata |
| Syndesmos | Horismos | Config read | Reads external API credentials at construction |
| Exousia | Horismos | Config read | Reads JWT secrets and session configuration at construction |

---

## Aggelia — Internal Event Bus

Aggelia (ἀγγελία — pronounced an-geh-LEE-ah) is the 14th backend subsystem: the internal announcement system that carries past-tense facts between subsystems without coupling the emitter to any subscriber.

**Where it lives:** Aggelia is not a standalone crate. Its types live in `crates/harmonia-common/src/aggelia/` — the shared leaf crate that all subsystems already depend on. This avoids the circular dependency pitfall: if event types lived in a separate `harmonia-events` crate that imported domain types from subsystem crates, and those subsystem crates also imported from `harmonia-events`, the graph would cycle.

**How handles are distributed:** Harmonia-host creates the broadcast channel at startup and distributes `Sender`/`Receiver` handles to each subsystem via constructor injection. No subsystem imports Aggelia as a crate dependency — they receive the handles as arguments. This means Aggelia's types are in harmonia-common (which every crate already depends on), but the channel lifecycle is owned by harmonia-host.

**Event naming convention:** All event variants are past tense. An event is an announcement of something that already occurred — not a command for something to happen. `ImportCompleted` (not `StartImport`), `DownloadProgress` (not `UpdateProgress`), `ScrobbleRequired` (not `Scrobble` — this names the fact that scrobbling is now needed, not a command to do it).

**What Aggelia carries:**

```rust
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum HarmoniaEvent {
    ImportCompleted { media_id: MediaId, media_type: MediaType, path: PathBuf },
    QualityUpgradeTriggered { media_id: MediaId, current_quality: QualityProfile },
    DownloadProgress { download_id: DownloadId, percent: u8 },
    DownloadCompleted { download_id: DownloadId, path: PathBuf },
    PlexNotifyRequired { media_id: MediaId },
    ScrobbleRequired { track_id: MediaId, user_id: UserId },
    TidalWantListSynced { added: Vec<MediaId> },
}
```

**Why `#[non_exhaustive]`:** New event types will be added as the system grows. `#[non_exhaustive]` on the enum means subscriber match arms can use `_ => {}` for unrecognized variants, preventing forced recompilation of all subscriber crates on every event addition. Mandated by `standards/RUST.md` for all public enums that may grow.

**Subscriber pattern:**

```rust
async fn handle_events(mut rx: broadcast::Receiver<HarmoniaEvent>) {
    loop {
        match rx.recv().await {
            Ok(HarmoniaEvent::ImportCompleted { media_id, .. }) => { /* react */ }
            Ok(_) => {}  // required — #[non_exhaustive]
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(dropped = n, "event receiver lagged");
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}
```

**Emitter pattern (fire-and-forget):**

```rust
event_tx.send(HarmoniaEvent::ImportCompleted {
    media_id,
    media_type,
    path,
}).ok();  // ok() — lagged subscribers are acceptable; emitter does not block
```

**Relationship to Syndesmos:** Syndesmos is the external API connector — it holds external credentials and speaks to Plex, Last.fm, and Tidal. Aggelia is entirely internal — it carries facts between subsystems within Harmonia. The names are distinct by design: σύνδεσμος (the bond connecting disparate parts) versus ἀγγελία (the announcement, the act of carrying tidings).

The full naming entry for Aggelia — with L1-L4 layer test — is in `docs/lexicon.md`.
