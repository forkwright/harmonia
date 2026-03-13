# Entity registry

> Cross-type identity hub for people, franchises, series, publishers, and labels.
> Subsystem ownership: [subsystems.md](../architecture/subsystems.md). Epignosis resolves identity; Episkope references entities in wants.
> Runtime type alignment: [cargo.md](../architecture/cargo.md). `MediaId` wraps a UUID that resolves to registry entries.

## Purpose

The `media_registry` table is the join point for cross-type entity discovery. A person entity for "Frank Herbert" links his books, audiobook narrations, and Dune movies; the same UUID appears in junction tables for every media type he is associated with. A franchise entity for "Dune" links books, audiobooks, movies, and TV adaptations regardless of their individual metadata schemas. The registry holds identity only; it does not duplicate metadata that belongs to per-type tables.

Subsystem ownership follows the boundaries defined in `docs/architecture/subsystems.md`: Epignosis resolves identity (matches provider IDs to registry UUIDs, creates new registry entries when no match exists), and Episkope references registry entries when building wanted media records. The registry itself is a data dependency; no single subsystem "owns" it in the sense of being the exclusive writer.

---

## `Media_registry` table

```sql
CREATE TABLE media_registry (
    id           BLOB NOT NULL PRIMARY KEY,
    entity_type  TEXT NOT NULL CHECK(entity_type IN (
                     'person', 'franchise', 'series', 'publisher', 'label'
                 )),
    display_name TEXT NOT NULL,
    sort_name    TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
```

| Column | Type | Notes |
|--------|------|-------|
| `id` | `BLOB NOT NULL PRIMARY KEY` | UUIDv7 stored as 16-byte BLOB. Never TEXT. |
| `entity_type` | `TEXT NOT NULL` | Constrained to five values; see type definitions below. |
| `display_name` | `TEXT NOT NULL` | Human-readable name. "Frank Herbert", "Dune", "Penguin Books". |
| `sort_name` | `TEXT` | Sort form. "Herbert, Frank". NULL when same as display_name. |
| `created_at` | `TEXT NOT NULL` | ISO8601 UTC. Set on insert, never updated. |
| `updated_at` | `TEXT NOT NULL` | ISO8601 UTC. Updated when display_name or sort_name changes. |

### Entity types

| Value | What It Represents | Examples |
|-------|--------------------|---------|
| `person` | Any individual with a creative role across media types | Author, narrator, artist, actor, director, composer |
| `franchise` | A named universe or IP spanning multiple media types | "Dune", "Marvel Cinematic Universe", "Discworld" |
| `series` | An ordered sequence within a franchise or standalone | "The Lord of the Rings" (book series), "Breaking Bad" (TV) |
| `publisher` | Book publisher, record label parent, film studio | "Penguin Books", "Warner Bros.", "Sony Music" |
| `label` | Music-specific imprint or sub-label | "ECM Records", "Sub Pop", "Merge Records" |

---

## `Registry_external_ids` table

External provider IDs are stored in a separate normalized table. This avoids wide nullable columns on `media_registry` (one column per provider, most NULL per entity) and allows unlimited providers without schema changes.

```sql
CREATE TABLE registry_external_ids (
    registry_id  BLOB NOT NULL REFERENCES media_registry(id) ON DELETE CASCADE,
    provider     TEXT NOT NULL CHECK(provider IN (
                     'musicbrainz', 'tmdb', 'tvdb', 'openlibrary',
                     'audible', 'audnexus', 'goodreads', 'imdb', 'lastfm'
                 )),
    external_id  TEXT NOT NULL,
    PRIMARY KEY (registry_id, provider)
);
```

| Column | Type | Notes |
|--------|------|-------|
| `registry_id` | `BLOB NOT NULL` | FK to `media_registry(id)`. CASCADE on delete. |
| `provider` | `TEXT NOT NULL` | One of nine known providers. |
| `external_id` | `TEXT NOT NULL` | Provider-specific identifier. "mb:artist:uuid", "tt1234567", "OL12345A". |

### Supported providers

| Provider | Entity Types | ID Format |
|----------|-------------|-----------|
| `musicbrainz` | person (artist), label, series | MBID (UUID format) |
| `tmdb` | person, franchise, series | Integer ID |
| `tvdb` | series | Integer ID |
| `openlibrary` | person (author), publisher, series | `/authors/OL...`, `/works/OL...` |
| `audible` | person (narrator), series | ASIN |
| `audnexus` | person (narrator) | String slug |
| `goodreads` | person (author), series | Integer ID |
| `imdb` | person (actor/director), franchise, series | `tt...`, `nm...` |
| `lastfm` | person (artist), label | String name slug |

### Reverse lookup index

The primary key `(registry_id, provider)` optimizes "given registry entity, list all external IDs." The reverse query, "given a provider ID, find the registry entity," requires a separate index:

```sql
CREATE INDEX idx_external_ids_provider ON registry_external_ids(provider, external_id);
```

This index supports identity matching: Epignosis receives a MusicBrainz MBID from a metadata provider and uses this index to find the existing registry entry before deciding whether to create a new one.

---

## Junction table pattern

Every per-type schema follows the same pattern to link media items to registry entities. The template is a three-column junction table with a composite primary key.

```sql
-- Template: {media_type}_{role_type}
-- Concrete example: music release group → artists
CREATE TABLE music_release_group_artists (
    release_group_id  BLOB NOT NULL REFERENCES music_release_groups(id),
    artist_id         BLOB NOT NULL REFERENCES media_registry(id),
    role              TEXT NOT NULL DEFAULT 'primary',
    PRIMARY KEY (release_group_id, artist_id, role)
);
```

Every per-type schema in `docs/data/media-schemas.md` follows this pattern. Role values are type-appropriate:

| Media Type | Role Values |
|------------|-------------|
| Music | `primary`, `featuring`, `remixer`, `producer`, `composer` |
| Audiobook | `author`, `narrator`, `translator`, `editor` |
| Book | `author`, `translator`, `illustrator`, `editor` |
| Movie/TV | `director`, `actor`, `writer`, `producer` |
| Comic | `writer`, `penciller`, `inker`, `colorist`, `letterer` |
| Podcast | `host`, `guest` |

The `role` column is part of the composite primary key because the same person can appear in multiple roles for the same item (an author who also narrates their own audiobook, a director who co-wrote the screenplay).

---

## UUID strategy

New registry entries use UUIDv7: `uuid::Uuid::now_v7()`.

**Why UUIDv7:** UUIDv7 is time-ordered; the most-significant bits encode a millisecond timestamp. When inserted sequentially, new UUIDs land near the end of the B-tree index rather than at random positions, reducing page splits and improving insert locality. For a registry that grows continuously as Epignosis resolves new identities, this matters.

**Why BLOB not TEXT:** UUIDs stored as TEXT are 36 bytes including hyphens. UUIDs stored as BLOB are 16 bytes, 55% smaller. For a foreign key column that appears in every junction table across all media types, this compounds: a junction table with one million rows storing a 36-byte FK column uses 36 MB just for that column; the same table with BLOB uses 16 MB. Index operations on BLOB are proportionally faster.

**Rust serialization:**

```rust
use uuid::Uuid;

// Insert: convert to BLOB bytes
let id = Uuid::now_v7();
let id_bytes = id.as_bytes().to_vec();  // Vec<u8>, 16 bytes

// Query result: reconstruct UUID from BLOB
let id = Uuid::from_bytes(row.id.try_into().expect("UUID is 16 bytes"));
```

**Cargo.toml dependency:**

```toml
uuid = { version = "1", features = ["v7"] }
```

---

## Scope constraint

The `media_registry` table holds exactly six columns: `id`, `entity_type`, `display_name`, `sort_name`, `created_at`, `updated_at`. Nothing else.

Type-specific metadata (birth year, nationality, genre, biography) lives in per-type provider tables, not in the registry. External provider IDs go in `registry_external_ids`. Relationships to media items go in junction tables.

Adding any column to `media_registry` beyond the six above is a scope violation. The registry is an identity store, not a metadata aggregator. See RESEARCH.md Pitfall 4 (Registry Scope Creep) for the failure mode this prevents.

---

## Indexes

```sql
-- Lookup all people, all franchises, all labels
CREATE INDEX idx_registry_entity_type ON media_registry(entity_type);

-- Name search and sort — "show me all artists starting with H"
CREATE INDEX idx_registry_display_name ON media_registry(display_name);

-- Reverse identity matching — "given MusicBrainz MBID, find registry entity"
CREATE INDEX idx_external_ids_provider ON registry_external_ids(provider, external_id);
```

The `display_name` index supports prefix-scan queries (`WHERE display_name LIKE 'Herbert%'`). SQLite's B-tree index handles these efficiently. A full-text index (`CREATE VIRTUAL TABLE`) is deferred until Phase 8 determines whether in-process FTS5 is preferred over external search.

---

## Relationship to harmonia-common

The `MediaId` type in `harmonia-common` wraps a UUID:

```rust
// From docs/architecture/cargo.md
pub struct MediaId(Uuid);
```

When a media item has a resolved identity, Epignosis has matched it to a known entity, and its `MediaId` resolves to a row in `media_registry`. The same UUID appears as the `id` in the registry row and as the `registry_id` foreign key in the per-type junction table.

`HarmoniaEvent::ImportCompleted { media_id, media_type, path }` carries this `MediaId`. Subscribers that need entity metadata (Syndesmos notifying Plex, Epignosis enriching metadata) use the UUID to look up the registry entry and follow the appropriate junction table join.

Not all media items have a registry entry at import time. A newly downloaded item may not yet have a resolved identity. `registry_id` is nullable in per-type tables for this reason; Epignosis enriches identity asynchronously after import.

---

## Anti-patterns

| Pattern | What Goes Wrong | Correct Approach |
|---------|----------------|-----------------|
| Type-discriminated nullable columns | `person_birth_year`, `franchise_start_year` both on `media_registry`, one NULL per row | Move to per-type tables (person metadata table, franchise metadata table) |
| Flat JSON `metadata` column | `{"birth_year": 1920, "nationality": "US"}` on registry | Per-type relational columns in separate tables |
| Wide provider columns | `musicbrainz_id`, `tmdb_id`, `imdb_id` all on `media_registry` | Normalized `registry_external_ids` table |
| TEXT UUID primary keys | `id TEXT PRIMARY KEY` storing "550e8400-e29b-41d4-a716-446655440000" | `id BLOB NOT NULL PRIMARY KEY` storing 16-byte binary |
