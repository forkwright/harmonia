# Media Lifecycle State Machine

> Shared lifecycle state machine for all 7 media types, with per-type extensions.
> See [architecture/subsystems.md](../architecture/subsystems.md) for state ownership by subsystem.
> See [data/want-release.md](../data/want-release.md) for acquisition half (wants/releases/haves tables).
> See [data/media-schemas.md](../data/media-schemas.md) for per-type table schemas.

---

## Core State Machine

Six states apply to all 7 media types (music, audiobooks, books, comics, podcasts, movies, TV):

```
discovered -> wanted -> downloading -> imported -> [type-extensions] -> organized -> available
```

State transitions are explicit and one-directional in normal flow. No implicit jumps. No backward transitions except retry paths and quality upgrades (documented in Transition Rules below).

Two entry paths exist:

- **Scanner path:** File found on disk → `discovered` → may skip `wanted` + `downloading` (file already exists) → `imported`
- **Acquisition path:** User adds want → `wanted` → `downloading` → `imported`

State is tracked in two separate places:

- **Acquisition half:** `wants.status` column (`searching` / `paused` / `fulfilled`) — maps to `wanted` through `downloading`
- **Library half:** `status` column on per-type tables (e.g., `music_tracks.status`) — tracks `imported` through `available`

---

## State Ownership Map

| State | Owner | Trigger | What Happens |
|-------|-------|---------|--------------|
| `discovered` | Taxis (scanner) | File found in library directory not matching any `haves` row | Taxis creates a preliminary record, attempts media type detection |
| `discovered` | Episkope | Indexer search returned a match for an active want | Episkope found a release candidate (acquisition path entry) |
| `wanted` | Episkope | User adds to want list OR scanner-discovered item matched to identity | Episkope begins monitoring for releases; `wants.status = 'searching'` |
| `downloading` | Syntaxis + Ergasia | Release selected, download queued and started | Syntaxis manages queue, Ergasia executes torrent/NNTP download |
| `imported` | Taxis | Download completed and post-processed OR scanner file accepted | Taxis creates `haves` row, file accepted into library staging |
| `organized` | Taxis | Metadata resolved, file renamed to final library path | Taxis calls Epignosis for metadata, renames per template, hardlinks/moves |
| `available` | Paroche | File in final library location, metadata complete | Paroche can serve this item to clients. Terminal state. |

`discovered` has two entry points: the scanner path enters at `discovered` and may skip `wanted` + `downloading` if the file already exists on disk. The acquisition path enters at `wanted` and goes through `downloading`.

---

## Per-Type Extensions

Sub-states slot between `imported` and `organized`. These represent type-specific processing that must complete before the file can be renamed to its final library path.

### Music (most detailed)

```
imported -> fingerprinting -> enriched -> organized
```

- **`fingerprinting`:** AcoustID fingerprint computation in progress (Epignosis). CPU-bound work runs in `spawn_blocking`. Non-fatal: if fingerprinting fails, item still progresses to `enriched` with a WARN log. Fingerprint result stored on `music_tracks.acoustid_fingerprint`.
- **`enriched`:** MusicBrainz metadata lookup complete. Track has `mb_recording_id`, release has `mb_release_id`, artist credits resolved.

Music tracks are each tracked **individually** — a single track can be `fingerprinting` while another track on the same album is already `available`. Album-level state is derived from the aggregate of track states:

| Derived Album State | Condition |
|--------------------|-----------|
| `partial` | Some tracks `available`, some still processing |
| `complete` | All tracks `available` |
| `upgrading` | At least one track being replaced by a higher-quality version |

### Audiobook

```
imported -> chapter_extracted -> enriched -> organized
```

- **`chapter_extracted`:** M4B chapter markers parsed via `mp4ameta` OR Audnexus chapter data fetched. Chapter timestamps stored in `audiobook_chapters` table. If neither source provides chapters, item uses a single-chapter fallback (full duration) and proceeds to `enriched`.
- **`enriched`:** Audnexus metadata (narrator, series position) and/or OpenLibrary metadata (ISBN, author) resolved.

Chapter data source priority: (1) Audnexus chapter data (authoritative); (2) embedded M4B chapter markers; (3) single-chapter fallback.

### Movies and TV

```
imported -> enriched -> organized
```

- **`enriched`:** TMDB (movies canonical, TV enrichment) or TVDB (TV canonical) metadata resolved. Poster, cast, overview, ratings populated.

### Books and Comics

```
imported -> enriched -> organized
```

- **`enriched`:** OpenLibrary metadata (books) or ComicVine metadata (comics) resolved.

### Podcasts (exception)

```
downloaded -> available
```

Podcasts bypass the full want lifecycle. Subscriptions auto-download new episodes. Episodes go directly from `downloaded` to `available`.

- No `organized` state — podcasts use a fixed naming scheme, no user-configurable rename templates.
- No `fingerprinting`, `chapter_extracted`, or `enriched` states.
- Per-episode want lifecycle applies only if the user explicitly creates a want for a specific episode.

See `want-release.md` Podcast Exception for the full rationale.

---

## State Enum Pattern

Rust representation for database-persisted state:

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum MediaItemState {
    Discovered,
    Wanted,
    Downloading,
    Imported,
    Fingerprinting,    // music only
    ChapterExtracted,  // audiobook only
    Enriched,
    Organized,
    Available,
    Failed,
}

/// Music track state alias — same variants, named for domain clarity at the call site.
/// Album-level state is derived from aggregate track states, not stored.
pub type MusicTrackState = MediaItemState;
```

A single enum covers all types. Not all variants are valid for all types — the state machine specifies valid transitions per type. The application layer enforces valid transitions; the database stores the raw string.

`Failed` is a terminal state reachable from `downloading`, `imported`, `fingerprinting`, `chapter_extracted`, or `enriched`. Failed items can be retried (transition back to the state before failure).

`#[non_exhaustive]` ensures that adding new sub-states in future media types does not require exhaustive match updates across all subscriber code.

---

## Transition Rules

Complete table of valid state transitions. No implicit transitions exist outside this table.

| From | To | Trigger | Owner | Notes |
|------|----|---------|-------|-------|
| `(none)` | `discovered` | Scanner finds file not in `haves` | Taxis | Scanner entry point |
| `(none)` | `wanted` | User adds want | Episkope | Acquisition entry point |
| `discovered` | `wanted` | Scanner-found item matched to identity | Episkope | Taxis notifies Episkope after identity resolution |
| `discovered` | `imported` | Scanner file accepted into library (skips acquisition) | Taxis | Direct import path for existing library files |
| `wanted` | `downloading` | Release selected, enqueued | Syntaxis | `wants.status` remains `searching` until import |
| `downloading` | `imported` | Download completed, post-processing done | Taxis | Taxis creates `haves` row; `wants.status` → `fulfilled` on quality gate pass |
| `downloading` | `failed` | Download failed, retries exhausted | Syntaxis | Release marked failed; want stays `searching` to find another release |
| `imported` | `fingerprinting` | Import completed, music track | Epignosis | Agoge dispatches `FingerprintTrack` task |
| `imported` | `chapter_extracted` | Import completed, audiobook | Epignosis | Agoge dispatches chapter extraction task |
| `imported` | `enriched` | Import completed, all non-music/non-audiobook types | Epignosis | Agoge dispatches `EnrichMetadata` task |
| `fingerprinting` | `enriched` | AcoustID lookup complete (success or non-fatal failure) | Epignosis | Proceeds to enriched even if fingerprint failed (non-fatal) |
| `fingerprinting` | `failed` | Fatal fingerprinting error (file unreadable) | Epignosis | Rare — file corruption. Retryable. |
| `chapter_extracted` | `enriched` | Chapter extraction complete (success or fallback) | Epignosis | Proceeds even if no chapters found (single-chapter fallback) |
| `enriched` | `organized` | Metadata complete, file renamed to final path | Taxis | Taxis calls Epignosis for full metadata, renames per template |
| `enriched` | `failed` | Canonical provider failed, retries exhausted | Epignosis | Item stays in failed until manual retry or provider recovers |
| `organized` | `available` | File at final library path, metadata stored | Paroche | Terminal state. Paroche can serve this item. |
| `available` | `downloading` | Quality upgrade triggered | Kritike | Existing `available` item remains while upgrade progresses. See Quality Upgrade Lifecycle. |
| `failed` | `downloading` | Retry triggered (was downloading when failed) | Syntaxis | Re-queued for download |
| `failed` | `fingerprinting` | Retry triggered (was fingerprinting when failed) | Epignosis | Re-queued for fingerprint |
| `failed` | `imported` | Retry triggered (was enriched/organized when failed) | Epignosis | Re-dispatches metadata enrichment |

---

## Quality Upgrade Lifecycle

How upgrades interact with state without disrupting the existing available item:

1. Kritike detects a better release exists (`quality_score` higher than current `haves` row, within profile ceiling)
2. Kritike emits `QualityUpgradeTriggered` via Aggelia
3. Episkope creates a new `wants` row with `upgrade_from_id` referencing the current have's want
4. New item goes through normal `wanted -> downloading -> imported -> ... -> available` flow
5. On `available`: old have is marked `upgraded` — not deleted. `haves.upgraded_from_id` retains the provenance chain.
6. Existing `available` item remains accessible to Paroche throughout the upgrade pipeline.

Two items can be in the pipeline simultaneously: the current `available` version and the upgrading version. Paroche serves the current version until the upgrade completes.

---

## Integration with Want / Release / Have Tables

Mapping between lifecycle states and the acquisition tables defined in `want-release.md`:

| Lifecycle State | Table Column | Value |
|----------------|-------------|-------|
| `wanted` (active monitoring) | `wants.status` | `searching` |
| `wanted` (user paused) | `wants.status` | `paused` |
| `available` (quality ceiling met) | `wants.status` | `fulfilled` |
| `downloading` | `releases.grabbed_at` | non-NULL |
| `imported` | `haves` row | created by Taxis |
| `organized` | `haves.file_path` | final library path set |
| `available` | per-type table `status` | `available` |

The `releases` table is acquisition-only — no lifecycle state column needed. Individual releases are either grabbed or rejected.

**Recommended addition to `haves` table:** A `status` column tracking library-side lifecycle states:

```sql
ALTER TABLE haves ADD COLUMN status TEXT NOT NULL DEFAULT 'imported'
    CHECK(status IN ('imported', 'organized', 'available', 'failed', 'upgraded'));
```

This allows Kritike and Taxis to query library health without joining to per-type tables. The `upgraded` status preserves the provenance chain while marking the item as superseded.
