# Import pipeline: naming templates, conflict resolution, and file operations

> Kathodos owns import execution (`crates/kathodos/src/import/`). `archon::import::ImportAdapter` (`crates/archon/src/import.rs`) is the only production caller today: it wires a completed download (`syntaxis::CompletedDownload`) into Kathodos' `ImportPipeline`. Scanner-triggered import (`ImportOrigin::Scanner`) is implemented and unit-tested but has no production caller — see [media/scanner.md](scanner.md) for what the scanner subsystem actually runs today.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Kathodos ownership, `ImportCompleted` event), [download/orchestration.md](../download/orchestration.md) (queue → import trigger, hardlink/copy/EXDEV mechanics, seeding continuation), [data/want-release.md](../data/want-release.md) (wants/releases/haves tables), [media/scanner.md](scanner.md) (the caveat on invented config-field claims applies equally here).

---

## What actually runs today

The only wired entry point is a completed torrent/NNTP download. `ImportAdapter::import_inner`:

1. Loads the want and release rows, maps the want's media type to a library. Every `wants.media_type` value (`music_album`, `audiobook`, `book`, `comic`, `podcast`, `movie`, `tv_series`) resolves to a `horismos::MediaType` library type via `kathodos::import::identify::resolve_media_type`; a want whose type has no configured library errors with `NoMatchingLibrary`.
2. Checks a haves fast-path: if a `complete` have for this exact release is still present on disk, the import is a no-op (the want is (re)marked `fulfilled` and nothing runs).
3. Enumerates the downloaded file(s) and runs each through `kathodos::import::ImportPipeline::process`.
4. Finalizes in one `BEGIN IMMEDIATE` transaction: deletes any stale have at the resulting `file_path`, inserts the new have (`haves.status = "complete"`), and sets `wants.status = "fulfilled"`.

`ImportPipeline` is always invoked with `naming_template: None` — there is no per-library custom template today. `horismos::LibraryConfig` (`crates/horismos/src/subsystems.rs`) has no `naming_template` field, and `horismos::TaxisConfig` has no `max_conflict_suffix`, `import_timeout_seconds`, or `bulk_rename_concurrency` field. There is no dry-run preview endpoint, template-validation endpoint, or bulk-rename job anywhere in the workspace.

---

## Pipeline steps (`ImportPipeline::process`, `crates/kathodos/src/import/mod.rs`)

```
1. Resolve metadata      — MetadataResolver::resolve_identity(path, media_type)
2. Compute target path   — TemplateEngine::parse(template, media_type).resolve(tokens)
3. Idempotency check     — same_file(source, target): true if target is already
                            a hardlink of source, or (cross-device) byte-identical
4. Conflict check        — resolve_conflict(target, existing_quality, new_quality,
                            is_same_item, max_suffix)
5. File operation        — hardlink_or_copy (Download origin) or rename (Scanner origin)
6. Emit ImportCompleted  — via Aggelia; skipped for AlreadyPresent/Skipped outcomes
```

---

## Metadata resolution: tags → hints → filename

The production resolver (`archon::import::DownloadResolver`) builds template tokens in priority order:

1. **Embedded file tags** (`kathodos::import::tags::read_tags`, via lofty) — artist, album artist, track/disc number, title, year.
2. **Want/release DB hints** — the want's title and resolved artist/author, and the release title, used directly as the `Album Title`/`Movie Title`/`Title` token and as an artist/author fallback.
3. **Filename parsing** (`epignosis::parse_filename`) — artist, track number, and title parsed from the file name when tags are absent.
4. **Best-effort year** — a plausible (1900-2099) 4-digit run within the release title, used only when neither tags nor the filename provide one.

A tag-read failure or an untagged file is not fatal; the resolver falls through to hints/filename with a log line and keeps going.

---

## Naming templates

`{Token}` syntax, Sonarr/Radarr-style. Real implementation: `crates/kathodos/src/import/template.rs`.

- Tokens are `{Token Name}`; numeric padding via `{Token Name:00}` (digit count = the number of zeros after the colon).
- Unknown tokens error at `TemplateEngine::parse` time (`TaxisError::UnknownToken`) — immediate, not at resolve time.
- A missing token's value is dropped silently at resolve time; `TaxisError::TemplateResolution` exists in the error enum for this case but nothing in the workspace constructs it today.
- Empty parenthetical/bracket groups left by a dropped token (` ()`, ` []`, `()`, `[]`) are stripped; multiple consecutive spaces collapse to one.
- Values are sanitized for filesystem-unsafe characters (`crate::sanitize::sanitize_component`).

### Token reference (per `themelion::MediaType`)

| Media type | Valid tokens |
|---|---|
| Music | Artist Name, Album Title, Year, Track Number, Track Title, Disc Number, Quality, Extension |
| Movie | Movie Title, Year, Quality, Edition, Extension |
| TV | Series Title, Season Number, Episode Number, Episode Title, Quality, Extension |
| Audiobook | Author Name, Title, Year, Narrator, Series, Series Position, Extension |
| Book | Author Name, Title, Year, Extension |
| Comic | Series Name, Volume Number, Issue Number, Issue Title, Year, Extension |
| Podcast | Podcast Title, Episode Title, Publication Date, Episode Number, Extension |
| News | Extension only — no dedicated token set today |

`Quality`, `Edition`, `Narrator`, `Series`, `Series Position`, and `Disc Number` are valid tokens the template engine will accept, but `DownloadResolver` never populates them — only Music/Movie/Book import today, and even for those it fills only the tokens named in the Metadata resolution section.

### Default templates (`template::default_template`)

| Media type | Default template |
|---|---|
| Music | `{Artist Name}/{Album Title} ({Year})/{Track Number:00} - {Track Title}.{Extension}` |
| Movie | `{Movie Title} ({Year})/{Movie Title} ({Year}) [{Quality}].{Extension}` |
| TV | `{Series Title}/Season {Season Number:00}/{Series Title} - S{Season Number:00}E{Episode Number:00} - {Episode Title}.{Extension}` |
| Audiobook | `{Author Name}/{Series}/{Title}.{Extension}` |
| Book | `{Author Name}/{Title}.{Extension}` |
| Comic | `{Series Name}/{Series Name} #{Issue Number:000}.{Extension}` |
| Podcast | `{Podcast Title}/{Publication Date} - {Episode Title}.{Extension}` |
| News | `{Extension}` |

These are compiled-in only; no config surface overrides them per library today.

---

## Conflict resolution (`crates/kathodos/src/import/conflict.rs`)

`resolve_conflict`'s contract:

| Existing target | Same media item | New quality vs. existing | Outcome |
|---|---|---|---|
| Missing | — | — | `Clear` — proceed at the computed path |
| Present | Yes | Higher | `Upgrade` — replace at the computed path |
| Present | Yes | Equal or lower | `Skip` — no file operation |
| Present | No / unknown | — | `Suffixed` — `_2`, `_3`, ... appended before the extension |

Exhausting the suffix range (`DEFAULT_MAX_SUFFIX = 99`) returns `TaxisError::ConflictResolution`. `ImportPipeline::with_max_conflict_suffix` exists to override that ceiling, but `ImportAdapter` never calls it — no horismos config field drives it, so it is always 99 in practice.

**`ImportPipeline::process` calls `resolve_conflict` with `existing_quality: None` and `is_same_item: true` unconditionally.** The DB-aware quality comparison in the conflict-resolution table is the function's contract, not something the one production caller exercises — in practice, any pre-existing target at the computed path takes the `Suffixed` branch. `ImportAdapter`'s own haves lookup (see the "What actually runs today" section) is what prevents duplicate imports for a want that is already fulfilled.

---

## File operations (`crates/kathodos/src/import/fileops.rs`)

| Operation | When | Mechanism |
|---|---|---|
| Hardlink | `ImportOrigin::Download`, same filesystem | `std::fs::hard_link` |
| Copy (EXDEV fallback) | `ImportOrigin::Download`, cross-filesystem | `std::fs::copy` |
| Rename | `ImportOrigin::Scanner` | `std::fs::rename`, with a copy-then-rename-then-delete fallback on EXDEV |

`same_file` (the idempotency short-circuit in step 3 of the pipeline) recognizes two cases: an exact `(dev, ino)` match (the target is a same-filesystem hardlink of the source), or — for a cross-device copy landing on a fresh inode — an equal-size, byte-for-byte content match.

See [download/orchestration.md](../download/orchestration.md) for how a completed download reaches this pipeline and its discussion of seeding continuation after a hardlink vs. a copy.
