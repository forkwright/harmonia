# Canonical Filesystem Layout

Source of truth for kathodos (ex-taxis) path templates and harmonia ingestion. All media types follow the same principles: predictable paths, TOML sidecar metadata, cover art co-located, no ambiguity.

## Principles

1. **One answer per question.** Given a media item's metadata, there is exactly one correct path. No per-library template options.
2. **Human-navigable.** Browsable in a file manager without harmonia running.
3. **Sidecar-first metadata.** TOML files alongside media carry metadata that survived import. The database is rebuilt from sidecars, not the other way around.
4. **Year-prefixed for sort.** `[{YYYY}]` prefix on release directories enables chronological browsing.

## Music

```
{library_root}/
└── {Artist Name}/
    ├── artist.toml
    ├── [{YYYY}] {Album Title}/             # studio album — no type tag
    ├── [{YYYY}] [EP] {Title}/
    ├── [{YYYY}] [Single] {Title}/
    ├── [{YYYY}] [Live] {Title}/
    ├── [{YYYY}] [Comp] {Title}/            # compilation
    └── [{YYYY}] [OST] {Title}/             # soundtrack
```

### Release directory contents

```
[2020] Elisabeth/
├── album.toml                              # MusicBrainz ID, genres, label, catalog#
├── cover.jpg                               # front cover (required)
├── back.jpg                                # back cover (optional)
├── 01-01 - Come as You Are.flac            # {disc}-{track} - {Title}.{ext}
├── 01-02 - Tremolo.flac
└── ...
```

### Release types

| Tag | Meaning |
|-----|---------|
| *(none)* | Studio album (default) |
| `[EP]` | Extended play |
| `[Single]` | Single |
| `[Live]` | Live recording |
| `[Comp]` | Compilation / various artists |
| `[OST]` | Original soundtrack |

### Track naming

`{disc}-{track} - {Title}.{ext}`

- Disc and track numbers are zero-padded to 2 digits: `01-01`, `02-14`
- Multi-disc releases use the disc prefix; single-disc releases still include `01-`
- File extension preserves the source format: `.flac`, `.opus`, `.mp3`

## Books (ebooks)

```
{library_root}/
└── {Author Name}/
    └── [{YYYY}] {Title}/
        ├── book.toml                       # ISBN, publisher, series, Goodreads ID
        ├── cover.jpg
        └── {Title}.epub                    # preferred: EPUB > PDF > MOBI
```

## Audiobooks

```
{library_root}/
└── {Author Name}/
    └── [{YYYY}] {Title}/
        ├── audiobook.toml                  # ISBN, narrator, Audnexus ID, duration
        ├── cover.jpg
        └── {Title}.m4b                     # single-file M4B preferred
```

For multi-file audiobooks (chapter-per-file):

```
[{YYYY}] {Title}/
├── audiobook.toml
├── cover.jpg
├── 01 - Chapter 1.mp3
├── 02 - Chapter 2.mp3
└── ...
```

## Podcasts

```
{library_root}/
└── {Show Name}/
    ├── show.toml                           # RSS feed URL, description, categories
    ├── cover.jpg
    └── [{YYYY-MM-DD}] {Episode Title}.mp3
```

Episodes use ISO date prefix for chronological sort. No subdirectories per season.

## TOML Sidecar Schema

All sidecar files share a `[meta]` section:

```toml
[meta]
source = "musicbrainz"          # enrichment source
source_id = "abc-123-def"       # external ID
imported_at = "2026-04-12T00:00:00Z"
quality_score = 0.95            # kritike assessment (0.0–1.0)
```

Format-specific fields are documented in the template issues (#158, #159).

## Path Sanitization

All path components are sanitized by kathodos:

- Unicode NFC normalization
- Replace `/ \ : * ? " < > |` with `-`
- Collapse whitespace to single space
- Trim leading/trailing whitespace and dots
- Max component length: 255 bytes (filesystem limit)
- No leading dot (hidden files)

See #160 for the smart sanitization implementation.
