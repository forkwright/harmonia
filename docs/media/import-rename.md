# Import and rename pipeline: naming templates, conflict resolution, and file operations

> Taxis owns the import and rename pipeline. Files enter the `imported` state when the pipeline begins; they reach `organized` when successfully renamed and registered.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Taxis ownership), [download/orchestration.md](../download/orchestration.md) (CompletedDownload type, hardlink strategy), [data/want-release.md](../data/want-release.md) (haves creation, wants fulfillment), [media/lifecycle.md](lifecycle.md) (imported → organized states).

---

## Pipeline overview

Two entry points feed the same pipeline:

| Entry Point | Trigger | Source Context |
|------------|---------|---------------|
| Download import | `Taxis.import(CompletedDownload)` called by Syntaxis after post-processing | `download_path`, `want_id`, `release_id` from the completed download |
| Scanner import | Scanner discovers a new file in a library directory | `file_path`, library `media_type` from library config |

Both paths converge on the same five-step pipeline: identify → resolve metadata → compute target path → execute file operation → register.

---

## Import pipeline steps

```
Input: file_path + source context (download or scan)
    |
Step 1: Identify media type
    - Download import: from want's media_type or file extension
    - Scanner import: from library config media_type (resolved in scanner.md)
    |
Step 2: Resolve metadata (Epignosis, direct call)
    - Epignosis.resolve_identity(): determine what this file IS (match to registry entity)
    - Epignosis.enrich(): fetch full metadata from canonical provider
    - Resolution failure: hold file in 'imported' state, queue for manual matching in the UI
    |
Step 3: Compute target path
    - Load naming template for this library (custom template or type default)
    - Resolve template tokens against enriched metadata
    - Sanitize path segments (filesystem-safe characters)
    |
Step 4: Conflict check and file operation
    - Check whether target path exists on disk (spawn_blocking)
    - Apply conflict resolution strategy (see Conflict Resolution)
    - Execute: hardlink (same FS download) | copy (cross FS download) | rename (scanner import)
    |
Step 5: Register and emit
    - Create or update haves row (Taxis writes to DB)
    - Check quality gate: if quality_score >= wants.profile.upgrade_until_score → wants.status = 'fulfilled'
    - Emit ImportCompleted event via Aggelia
    |
Step 6: Post-import tasks (dispatched via syntaxis, not blocking the import pipeline)
    - Music: AcoustID fingerprinting + ReplayGain R128 computation
    - All types: Epignosis metadata enrichment tasks (background)
    - Movie/TV: Prostheke subtitle search
    - All types: Kritike quality registration
```

**Metadata resolution failure path:** If Epignosis cannot identify the file, Taxis leaves it in `imported` state and records the path in the UI's manual matching queue. The file is not deleted. The user can manually match it from the UI, which retriggers from Step 3 with the matched identity.

---

## Naming template syntax

Sonarr/Radarr-style `{Token}` syntax. Tokens are resolved at import time against enriched metadata.

**Rules:**

- Tokens are enclosed in curly braces: `{Token Name}`
- Numeric padding: `{Track Number:00}` zero-pads to N digits (`:00` = 2 digits, `:000` = 3 digits)
- Literal text between tokens is preserved verbatim
- Filesystem-unsafe characters in token values are replaced with `_`: `/ \ : * ? " < > |`
- Leading/trailing whitespace in resolved token values is trimmed
- Multiple consecutive spaces collapsed to single space
- Empty token value: the token and its immediately adjacent separator are removed. `{Title} ({Year})` with no year resolves to `{Title}` (not `{Title} ()`)
- Template validation at config load time: unknown tokens error immediately, not at resolve time

---

## Token reference

### Music

| Token | Source | Example |
|-------|--------|---------|
| `{Artist Name}` | `music_release_groups.artist_name` or artist credits | `Led Zeppelin` |
| `{Album Title}` | `music_releases.title` | `Led Zeppelin IV` |
| `{Year}` | `music_releases.year` (release year, not original) | `1971` |
| `{Track Number}` | `music_tracks.track_number` | `7` (`:00` → `07`) |
| `{Track Title}` | `music_tracks.title` | `When the Levee Breaks` |
| `{Disc Number}` | `music_media.position` | `1` |
| `{Quality}` | Derived from codec + bitrate/bit_depth | `FLAC 24-96` |
| `{Extension}` | File extension (lowercase) | `flac` |

### Movie

| Token | Source | Example |
|-------|--------|---------|
| `{Movie Title}` | `movies.title` | `Blade Runner 2049` |
| `{Year}` | `movies.year` | `2017` |
| `{Quality}` | Derived from resolution + codec | `Bluray-1080p` |
| `{Edition}` | `movies.edition` | `Director's Cut` |
| `{Extension}` | File extension | `mkv` |

### TV episode

| Token | Source | Example |
|-------|--------|---------|
| `{Series Title}` | `tv_series.title` | `Breaking Bad` |
| `{Season Number}` | `tv_seasons.season_number` | `5` |
| `{Episode Number}` | `tv_episodes.episode_number` | `16` |
| `{Episode Title}` | `tv_episodes.title` | `Felina` |
| `{Quality}` | Derived from resolution + codec | `HDTV-720p` |
| `{Extension}` | File extension | `mkv` |

### Audiobook

| Token | Source | Example |
|-------|--------|---------|
| `{Author Name}` | `audiobooks.author` | `Brandon Sanderson` |
| `{Title}` | `audiobooks.title` | `The Way of Kings` |
| `{Year}` | `audiobooks.year` | `2010` |
| `{Narrator}` | `audiobooks.narrator` | `Michael Kramer` |
| `{Series}` | `audiobooks.series_name` | `The Stormlight Archive` |
| `{Series Position}` | `audiobooks.series_position` | `1` |
| `{Extension}` | File extension | `m4b` |

### Book

| Token | Source | Example |
|-------|--------|---------|
| `{Author Name}` | `books.author` | `Frank Herbert` |
| `{Title}` | `books.title` | `Dune` |
| `{Year}` | `books.year` | `1965` |
| `{Extension}` | File extension | `epub` |

### Comic

| Token | Source | Example |
|-------|--------|---------|
| `{Series Name}` | `comic_series.name` | `Saga` |
| `{Volume Number}` | `comic_issues.volume` | `1` |
| `{Issue Number}` | `comic_issues.issue_number` | `1` (`:000` → `001`) |
| `{Issue Title}` | `comic_issues.title` | `Chapter One` |
| `{Year}` | `comic_issues.year` | `2012` |
| `{Extension}` | File extension | `cbz` |

### Podcast

| Token | Source | Example |
|-------|--------|---------|
| `{Podcast Title}` | `podcasts.title` | `Hardcore History` |
| `{Episode Title}` | `podcast_episodes.title` | `Supernova in the East I` |
| `{Publication Date}` | `podcast_episodes.published_at` (YYYY-MM-DD) | `2018-07-15` |
| `{Episode Number}` | `podcast_episodes.episode_number` | `62` |
| `{Extension}` | File extension | `mp3` |

---

## Default templates

Opinionated defaults matching *arr conventions. These apply when no custom `naming_template` is set for the library.

| Media Type | Default Template |
|-----------|-----------------|
| Music | `{Artist Name}/{Album Title} ({Year})/{Track Number:00} - {Track Title}.{Extension}` |
| Movie | `{Movie Title} ({Year})/{Movie Title} ({Year}) [{Quality}].{Extension}` |
| TV | `{Series Title}/Season {Season Number:00}/{Series Title} - S{Season Number:00}E{Episode Number:00} - {Episode Title}.{Extension}` |
| Audiobook | `{Author Name}/{Series}/{Title}.{Extension}` |
| Book | `{Author Name}/{Title}.{Extension}` |
| Comic | `{Series Name}/{Series Name} #{Issue Number:000}.{Extension}` |
| Podcast | `{Podcast Title}/{Publication Date} - {Episode Title}.{Extension}` |

Custom templates are stored per library in `[taxis.libraries.{name}].naming_template`. A dry-run preview is mandatory before a custom template is applied to existing library content (see the Dry-run preview section).

---

## Conflict resolution

Per locked decision: never overwrite, never prompt in automated flows.

**Same file at different quality (upgrade scenario):**

1. Check if the existing file in `haves` represents the same media item (same `media_type_id`).
2. Compare quality scores using the quality rank tables (see `quality-profiles.md`).
3. New quality is higher → upgrade: replace `haves.file_path`, update `haves.quality_score`, set `haves.upgraded_from_id` to the previous have. The old file is replaced on disk via `std::fs::rename` (same FS) or copy-then-delete.
4. New quality is equal or lower → skip import, log at info level. No duplicate is created.

**Different item at colliding path:**

Genuine path collision (two different media items resolve to the same target path, e.g., two movies both titled "Dune" from different years with the same year). Append a numeric suffix to the filename (before the extension):

```
Dune (2021)/Dune (2021) [Bluray-1080p].mkv     ← original
Dune (2021)/Dune (2021) [Bluray-1080p]_2.mkv   ← collision resolved
Dune (2021)/Dune (2021) [Bluray-1080p]_3.mkv   ← second collision
```

Suffix pattern: `_{N}` inserted before the extension. Increment until a non-colliding path is found. Maximum suffix: `_99` (see `max_conflict_suffix`). Log at info level with both item identities.

**Rule: never overwrite.** If `std::path::Path::exists()` returns true and the item is not a quality upgrade of the same media, always suffix. The existing file is never touched without an explicit upgrade decision.

---

## Dry-run preview

Mandatory before applying a new naming template to existing library content.

**Endpoint:** `POST /api/taxis/dry-run`

**Request:**

```json
{
  "library_id": "...",
  "naming_template": "{Artist Name}/{Album Title} ({Year})/{Track Number:00} - {Track Title}.{Extension}",
  "limit": 100
}
```

**Response:** Array of before/after path pairs (no files are moved):

```json
[
  {
    "media_id": "...",
    "media_type": "music",
    "current_path": "/media/music/Led Zeppelin/Led Zeppelin IV/07 - When The Levee Breaks.flac",
    "proposed_path": "/media/music/Led Zeppelin/Led Zeppelin IV (1971)/07 - When the Levee Breaks.flac"
  }
]
```

`limit` defaults to 100; returns the first N items in the library sorted by path. The UI displays this before/after table before the user confirms.

**Template change flow:**

1. User edits naming template in UI.
2. UI calls `POST /api/taxis/dry-run`.
3. UI displays before/after preview table.
4. User confirms or adjusts.
5. On confirm: `POST /api/taxis/apply-template`; Taxis submits a bulk rename job to syntaxis.
6. syntaxis processes renames as a background task. Progress emitted via events. Errors are per-file (one failed rename does not abort the batch).

---

## Template resolution engine

```rust
pub struct TemplateEngine {
    segments: Vec<TemplateSegment>,
}

pub enum TemplateSegment {
    Literal(String),
    Token { name: String, padding: Option<usize> },
}

impl TemplateEngine {
    /// Parse a template string. Returns error for unknown tokens.
    pub fn parse(template: &str, media_type: MediaType) -> Result<Self, TaxisError>;

    /// Resolve template against enriched metadata to produce a relative path.
    pub fn resolve(&self, metadata: &ResolvedMetadata) -> Result<PathBuf, TaxisError>;
}
```

**Template parsing:** Called once at config load, not at import time. The parsed `Vec<TemplateSegment>` is cached. Unknown tokens produce a `TemplateResolution` error at parse time; this surfaces immediately when config is loaded, not silently at the first import.

**Token resolution:** Look up each token name in the `ResolvedMetadata` map, apply optional numeric padding, sanitize the value (replace unsafe characters, trim whitespace). Assemble into a relative path with OS-appropriate separators.

**Template validation endpoint:** `POST /api/taxis/validate-template`; returns parse errors for unknown tokens or syntax issues without requiring a library scan. Used by the UI template editor for live feedback.

---

## File operations

All file operations run in `spawn_blocking`. Taxis owns the file system; no other subsystem moves or renames library files.

| Operation | When | Implementation |
|-----------|------|---------------|
| Hardlink | Download import, source and target on same filesystem | `std::fs::hard_link(source, target)` |
| Copy | Download import, cross-filesystem (`EXDEV` error on hardlink attempt) | `std::fs::copy(source, target)`, stream copy |
| Rename | Scanner import (file already in library) | `std::fs::rename(current, target)` |
| Cross-FS rename | Target on different filesystem | Copy to temp path, then rename temp → target |
| Bulk rename | Template change via syntaxis background task | `std::fs::rename` per file |

**Directory creation:** `std::fs::create_dir_all(target_parent)` before any file operation. Target directories may not exist if this is the first file for a new artist/season/author.

**Atomic operations:** `std::fs::rename` is atomic on the same filesystem (single inode move). Cross-filesystem moves write to a `.tmp` file in the target directory, then `rename` the temp into place; this makes the final appearance of the file atomic from the reader's perspective.

**Permission preservation:** New files inherit parent directory permissions. No explicit `chmod`; the `umask` at process startup governs default permissions.

**Post-import cleanup:** After a successful download import, Taxis removes empty parent directories from the download source location. Library directories are never cleaned up by Taxis directly; only the download staging area is cleaned.

---

## Error handling

Import errors are **per-file**. A failed import leaves the file in `imported` state for retry or manual resolution. The import pipeline never crashes on a per-file error.

`TaxisError` import variants:

```rust
#[derive(Debug, Snafu)]
pub enum TaxisError {
    #[snafu(display("metadata resolution failed for {path}"))]
    MetadataResolutionFailed {
        path: PathBuf,
        source: EpignosisError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("template token '{token}' missing from metadata (template: {template})"))]
    TemplateResolution {
        template: String,
        token: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("{operation} failed: {source_path} → {target_path}"))]
    FileOperation {
        operation: String,  // "hardlink" | "copy" | "rename"
        source_path: PathBuf,
        target_path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("conflict suffix exhausted at {target_path} (max {max})"))]
    ConflictResolution {
        target_path: PathBuf,
        max: usize,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("dry-run failed for library {library_id}"))]
    DryRunFailed {
        library_id: String,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    // ... scanner variants in scanner.md
}
```

Errors are logged at `warn` level with file path and error chain. The file remains at its original location with `imported` status so the user can inspect and retry via the UI.

---

## Horismos configuration: `[taxis]` import section

```toml
[taxis]
# Maximum _{N} suffix attempts before ConflictResolution error
max_conflict_suffix = 99

# Per-file import timeout (seconds): covers metadata resolution + file operation
import_timeout_seconds = 300

# Parallel rename jobs during bulk template change (via syntaxis)
bulk_rename_concurrency = 2

[taxis.libraries.music]
path = "/media/music"
media_type = "music"
# Optional: override default naming template for this library
naming_template = "{Artist Name}/{Album Title} ({Year})/{Track Number:00} - {Track Title}.{Extension}"
```

`TaxisConfig` import fields in `crates/horismos/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaxisConfig {
    // ... scanner fields from scanner.md ...
    pub max_conflict_suffix: usize,         // default: 99
    pub import_timeout_seconds: u64,        // default: 300
    pub bulk_rename_concurrency: usize,     // default: 2
    pub libraries: HashMap<String, LibraryConfig>,
}
```
