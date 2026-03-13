# Audiobook design

> M4B chapter extraction, Audnexus integration, narrator metadata, and per-user position tracking.
> See [media/lifecycle.md](lifecycle.md) for audiobook sub-states (imported → chapter_extracted → enriched → organized).
> See [media/metadata-providers.md](metadata-providers.md) for Audnexus and OpenLibrary provider configs.
> See [data/media-schemas.md](../data/media-schemas.md) for `audiobooks`, `audiobook_chapters`, and `audiobook_progress` tables.

---

## Audiobook lifecycle extension

Audiobooks extend the shared lifecycle with two sub-states between `imported` and `organized`:

```
imported -> chapter_extracted -> enriched -> organized -> available
```

| Sub-state | Owner | What Happens |
|-----------|-------|--------------|
| `chapter_extracted` | Epignosis | Chapter markers parsed from M4B via `mp4ameta`, OR fetched from Audnexus. `audiobook_chapters` table populated. |
| `enriched` | Epignosis | Full metadata from Audnexus (narrator, series position, description) and/or OpenLibrary (ISBN, author) resolved. |

Chapter extraction precedes enrichment: chapter data is needed before the item can be fully navigable, and the ASIN resolved during enrichment may improve chapter data quality in a follow-up step.

Both steps proceed to the next state even on partial failure:
- No chapters found → single-chapter fallback → still transitions to `chapter_extracted`
- Audnexus unavailable → OpenLibrary metadata only → still transitions to `enriched`

---

## M4B chapter extraction

M4B files are MP4 containers with chapter track metadata. Chapter markers define playback navigation breakpoints.

### Extraction via mp4ameta

Chapter extraction runs in `spawn_blocking`; file I/O and tag parsing are blocking operations:

```rust
fn extract_chapters(file_path: &Path) -> Result<Vec<RawChapter>, ChapterError> {
    let tag = mp4ameta::Tag::read_from_path(file_path)
        .context(ReadTagSnafu { path: file_path.to_owned() })?;

    let chapters: Vec<RawChapter> = tag.chapters()
        .map(|ch| RawChapter {
            position: ch.index as i64,
            title: ch.title.clone().unwrap_or_default(),
            start_ms: ch.start_time.as_millis() as i64,
            end_ms: ch.end_time.as_millis() as i64,
        })
        .collect();

    Ok(chapters)
}
```

### Fallback behavior

| Condition | Behavior |
|-----------|----------|
| M4B with chapter markers | Extract and store all chapters |
| M4B with no embedded chapters | Single-chapter fallback: `start_ms=0`, `end_ms=audiobook.duration_ms`, `title=audiobook.title` |
| Non-M4B file (MP3, FLAC) | No embedded chapter extraction. Skip M4B step. Await Audnexus chapter data. |

A single-chapter fallback always results in a valid `audiobook_chapters` row, so the item always transitions to `chapter_extracted` and never stays stuck at `imported` due to missing chapters.

### Chapter source tracking

```sql
ALTER TABLE audiobook_chapters ADD COLUMN source TEXT NOT NULL DEFAULT 'fallback'
    CHECK(source IN ('audnexus', 'embedded', 'fallback'));
```

`source` tracks chapter provenance for diagnostic and conflict-resolution purposes.

---

## Chapter data priority

When multiple chapter sources are available, a priority order determines which data is used:

1. **Audnexus chapter data:** authoritative, sourced from Audible publisher metadata, consistent naming
2. **Embedded M4B chapter markers:** publisher or user-created markers; variable quality
3. **Single-chapter fallback:** no chapters available

### Conflict resolution

When both Audnexus and embedded M4B chapters are present:

1. Compare chapter counts:
   - Counts match (within ±1): use Audnexus data. Better titles, consistent timestamps. Delete embedded rows.
   - Counts differ significantly (>2 difference): log WARN with both counts. Use Audnexus data as active chapters. Retain M4B chapters with `source='embedded'` for reference; do not delete.

2. Write active chapters with `source='audnexus'`. If Audnexus later provides an updated chapter list (manual re-fetch), the existing `source='audnexus'` rows are replaced in full.

This gives preference to the authoritative source without silently discarding embedded data when there is a meaningful discrepancy.

---

## Audnexus integration

Audnexus (`https://api.audnex.us`) is the canonical provider for audiobook narrator and chapter metadata. It sources data from Audible's publisher feeds.

### ASIN resolution chain

Audnexus requires an ASIN (Audible/Amazon Standard Identification Number):

```
1. audiobooks.asin already populated (from download metadata, embedded tag, or user input)
       |-> Direct GET /books/{asin} lookup

2. No ASIN: attempt Audnexus title+author search
       GET /books?title={title}&author={author}
       (verify endpoint exists at implementation time — see metadata-providers.md)
       |-> If match: populate asin, proceed with full lookup
       |-> If no match or ambiguous: continue to step 3

3. Fall back to OpenLibrary for book identity
       GET /search.json?q={title}&author={author}
       No chapter data from this path — chapters will be embedded only (or fallback)
       |-> Populate audiobooks.isbn from OpenLibrary edition

4. All resolution failed: audiobook has title + author from filename/tags only
       Status: 'enriched' with partial metadata
       Manual ASIN entry via UI resolves in Phase 7
```

### Audnexus data mapping

| Audnexus Endpoint | Harmonia Target |
|-------------------|----------------|
| `GET /books/{asin}` | `audiobooks.asin`, `audiobooks.narrator` (via junction), `audiobooks.description`, `audiobooks.publisher`, `audiobooks.duration_ms`, `audiobooks.series_name`, `audiobooks.series_position` |
| `GET /books/{asin}/chapters` | `audiobook_chapters` rows (with `source='audnexus'`) |
| `GET /authors/{asin}` | `media_registry` person entry for narrator; `registry_external_ids` entry with Audnexus author ASIN |

### Narrator metadata

Narrators are stored as `media_registry` person entities:
1. Audnexus returns narrator ASIN and name from `GET /books/{asin}`
2. Check `registry_external_ids` for existing narrator with this ASIN
3. If not found: create `media_registry` person row + `registry_external_ids` entry
4. Link narrator to audiobook via `audiobook_authors` junction table with `role='narrator'`
5. `GET /authors/{asin}` enriches the narrator entity with biography and image URL

---

## EPUB audiobook handling

Some audiobooks are packaged as EPUB 3 with embedded audio via Media Overlays (SMIL files).

**Detection:** File is `.epub` and contains SMIL files referencing audio files in the EPUB package.

**Chapter extraction:** Parse EPUB spine + SMIL documents to determine:
- Chapter boundaries (SMIL `seq` elements with `epub:textref` attributes)
- Audio file references and time fragments for each chapter

**v1 scope:** EPUB audiobook detection is designed but full EPUB audiobook support is deferred to v2. In v1, EPUB files import as `books` (not `audiobooks`) unless the user manually overrides the media type assignment. The detection path is implemented so mis-typed EPUBs can be corrected.

---

## Per-user playback position tracking

Position tracking is at chapter + millisecond offset granularity. Each user has independent progress per audiobook.

### Schema (existing)

```sql
-- From data/media-schemas.md
CREATE TABLE audiobook_progress (
    id               BLOB NOT NULL PRIMARY KEY,
    audiobook_id     BLOB NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    user_id          BLOB NOT NULL,
    chapter_position INTEGER NOT NULL,   -- which chapter (chapter.position value)
    offset_ms        INTEGER NOT NULL DEFAULT 0,  -- offset within the chapter
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(audiobook_id, user_id)
);
```

`UNIQUE(audiobook_id, user_id)` ensures one progress record per user per audiobook. Playback updates use upsert.

### Position update flow

Akroasis client sends position updates during playback:

```
PUT /api/audiobooks/{id}/progress
Body: { "chapter_position": 3, "offset_ms": 45230 }

Response: { "chapter_position": 3, "offset_ms": 45230, "updated_at": "..." }
```

Taxis handles the upsert:
```sql
INSERT INTO audiobook_progress (id, audiobook_id, user_id, chapter_position, offset_ms, updated_at)
VALUES (?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
ON CONFLICT(audiobook_id, user_id) DO UPDATE SET
    chapter_position = excluded.chapter_position,
    offset_ms = excluded.offset_ms,
    updated_at = excluded.updated_at;
```

Minimum client update interval: `audiobook_position_sync_interval_seconds` (default 30). The client should not send updates more frequently than this.

### Position retrieval

```
GET /api/audiobooks/{id}/progress

Response:
{
    "chapter_position": 3,
    "offset_ms": 45230,
    "chapter_title": "Chapter 3: The Trial",
    "total_chapters": 28,
    "percent_complete": 14.7,    // (chapter_position / total_chapters) * 100
    "updated_at": "2026-03-02T10:30:00Z"
}
```

`percent_complete` is a chapter-count approximation (each chapter equally weighted). A duration-weighted calculation requires summing chapter durations, which is more accurate but also available from the schema.

### Multi-device sync

Last-write-wins: whichever device has the most recent `updated_at` wins. No conflict resolution UI in v1. This is sufficient for household use where a single user listens on one device at a time.

If two devices report position in rapid succession (e.g., syncing after offline period): the server accepts whichever arrives last. The client may display a "progress updated from another device" notification using the returned `updated_at`.

### Marking complete

When `offset_ms >= chapter.end_ms` for the last chapter, the audiobook is considered complete:

```sql
ALTER TABLE audiobook_progress ADD COLUMN completed_at TEXT;  -- nullable ISO8601 UTC
```

Server sets `completed_at = strftime(...)` when the playback position reaches the end of the last chapter. Completed audiobooks retain their last `chapter_position + offset_ms`; re-listening starts from any chapter the user selects.

---

## Audiobook import flow

End-to-end sequence for a new audiobook file:

```
File enters import pipeline (download or scan)
    |
Taxis: detect as audiobook
    - .m4b extension -> audiobook
    - .mp3 or .m4a in a library configured with media_type=audiobook -> audiobook
    - .epub with embedded audio -> book (re-assign manually in v1)
Taxis: create audiobooks row with status='imported'
Taxis: create haves row, emit ImportCompleted via Aggelia
    |
Post-import hooks dispatched via syntaxis (asynchronous):
    |
Step 1: Chapter extraction (Epignosis, in spawn_blocking)
    - M4B: mp4ameta chapter parsing -> store in audiobook_chapters (source='embedded')
    - Non-M4B: skip extraction, single-chapter fallback stored (source='fallback')
    - audiobooks.status = 'chapter_extracted'
    |
Step 2: Metadata enrichment (Epignosis)
    - Resolve ASIN via resolution chain (tag -> Audnexus search -> OpenLibrary -> manual)
    - GET /books/{asin}: fetch narrator, series, description, publisher, duration
    - GET /books/{asin}/chapters: fetch Audnexus chapter data
    - Apply chapter priority resolution (Audnexus > embedded > fallback)
    - Update audiobook_chapters with final authoritative data
    - GET /authors/{asin}: fetch narrator entity for media_registry
    - Fall back to OpenLibrary for ISBN and author if Audnexus unavailable
    - audiobooks.status = 'enriched'
    |
Step 3: Organize (Taxis)
    - Rename to template path: {Author Name}/{Series}/{Title}.{Extension}
    - Update audiobooks.file_path to final library path
    - audiobooks.status = 'organized' -> 'available'
```

---

## Error handling

All audiobook errors are non-fatal to the import completion. Missing data does not block `available` state.

| Error | Severity | Action |
|-------|----------|--------|
| `ChapterExtractionFailed { path, source }` | Non-fatal | Single-chapter fallback applied. Log WARN. |
| `AsinResolutionFailed { title, author }` | Non-fatal | Import proceeds with tag/filename metadata only. |
| `AudnexusUnavailable` | Non-fatal | Use embedded chapters + OpenLibrary metadata if available. |
| `ChapterCountMismatch { embedded, audnexus }` | Non-fatal | Log WARN, use Audnexus data, retain embedded for reference. |
| `PositionSyncFailed { user_id, audiobook_id }` | Non-fatal | Retry with exponential backoff (3 attempts). Log ERROR after all retries. |

`PositionSyncFailed` is the only error type that affects a user action directly. The client should display a "Failed to save progress" notification after retries are exhausted.

---

## Horismos configuration

Audiobook-specific additions to the `[taxis]` config section:

```toml
[taxis]
# Chapter source priority order
# Determines which chapter source wins when multiple are available
# Valid values: "audnexus", "embedded", "fallback"
audiobook_chapter_source_priority = ["audnexus", "embedded", "fallback"]

# Minimum interval between position update requests from the client
# Prevents excessive write load on the progress table
audiobook_position_sync_interval_seconds = 30
```
