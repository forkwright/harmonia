# Media Schemas

> Per-type table definitions for all eight media types in Harmonia.
> Related: [entity-registry.md](entity-registry.md) for registry FK pattern, [quality-profiles.md](quality-profiles.md) for quality score context, [want-release.md](want-release.md) for lifecycle tables that reference these tables.

## Design Principles

- **Table-per-type.** Each media type has its own dedicated tables. No polymorphic base table with a discriminator column.
- **No nullable type-specific columns.** Every column in every table is meaningful for that table. NULL columns signal absent optional data (external IDs, optional metadata), not type-mismatch padding.
- **No JSON blobs.** All attributes are proper relational columns. Extensible metadata goes in separate normalized tables or per-type provider tables.
- **Shared fields repeated per type.** `title`, `added_at`, and `quality_profile_id` appear in each top-level table. Repetition is intentional — it avoids a polymorphic base that would impose cross-type queries and join overhead.
- **Registry linkage via junction tables.** Every top-level table links to `media_registry` via junction tables following the pattern in `entity-registry.md`. The `registry_id` column on top-level tables is optional and asynchronously populated by Epignosis.
- **All timestamps are ISO8601 UTC strings.** No Unix epoch integers. Format: `strftime('%Y-%m-%dT%H:%M:%SZ', 'now')`.

---

## Music

Music uses a MusicBrainz-aligned four-level hierarchy: release group → release → medium → track. This hierarchy reflects how music is actually published — the abstract album concept is separate from a specific pressing, which is separate from individual discs, which contain individual tracks.

### `music_release_groups`

The abstract album concept — "Led Zeppelin IV" as an idea, independent of any specific pressing.

```sql
CREATE TABLE music_release_groups (
    id                   BLOB NOT NULL PRIMARY KEY,
    registry_id          BLOB REFERENCES media_registry(id),
    title                TEXT NOT NULL,
    rg_type              TEXT NOT NULL CHECK(rg_type IN (
                             'album', 'single', 'ep', 'compilation', 'live', 'other'
                         )),
    mb_release_group_id  TEXT,
    year                 INTEGER,
    quality_profile_id   INTEGER REFERENCES quality_profiles(id),
    added_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_mrg_registry ON music_release_groups(registry_id);
CREATE INDEX idx_mrg_mb_id ON music_release_groups(mb_release_group_id);
```

### `music_releases`

A concrete edition — a specific pressing, reissue, or digital release.

```sql
CREATE TABLE music_releases (
    id               BLOB NOT NULL PRIMARY KEY,
    release_group_id BLOB NOT NULL REFERENCES music_release_groups(id) ON DELETE CASCADE,
    title            TEXT NOT NULL,
    release_date     TEXT,
    country          TEXT,
    label            TEXT,
    catalog_number   TEXT,
    mb_release_id    TEXT,
    added_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_mr_release_group ON music_releases(release_group_id);
CREATE INDEX idx_mr_mb_id ON music_releases(mb_release_id);
```

### `music_media`

A disc, side, or digital medium within a release.

```sql
CREATE TABLE music_media (
    id         BLOB NOT NULL PRIMARY KEY,
    release_id BLOB NOT NULL REFERENCES music_releases(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    format     TEXT NOT NULL CHECK(format IN ('CD', 'Vinyl', 'Digital', 'Cassette', 'Other')),
    title      TEXT,
    UNIQUE(release_id, position)
);

CREATE INDEX idx_mm_release ON music_media(release_id);
```

### `music_tracks`

Individual songs. The leaf node — only tracks carry file paths and quality scores.

```sql
CREATE TABLE music_tracks (
    id                   BLOB NOT NULL PRIMARY KEY,
    medium_id            BLOB NOT NULL REFERENCES music_media(id) ON DELETE CASCADE,
    position             INTEGER NOT NULL,
    title                TEXT NOT NULL,
    duration_ms          INTEGER,
    mb_recording_id      TEXT,
    acoustid_fingerprint TEXT,         -- AcoustID chromaprint fingerprint
    acoustid_id          TEXT,         -- AcoustID identifier (from API lookup)
    file_path            TEXT,
    file_size_bytes      INTEGER,
    bit_depth            INTEGER,
    sample_rate          INTEGER,
    codec                TEXT,
    quality_score        INTEGER,
    replay_gain_track_db REAL,
    replay_gain_album_db REAL,
    source_type          TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                             'local', 'torrent', 'usenet', 'manual', 'rss'
                         )),
    added_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(medium_id, position)
);

CREATE INDEX idx_mt_medium ON music_tracks(medium_id);
CREATE INDEX idx_mt_mb_recording ON music_tracks(mb_recording_id);
CREATE INDEX idx_mt_acoustid ON music_tracks(acoustid_id) WHERE acoustid_id IS NOT NULL;
CREATE UNIQUE INDEX idx_mt_file_path ON music_tracks(file_path) WHERE file_path IS NOT NULL;
```

### Junction Tables

```sql
CREATE TABLE music_release_group_artists (
    release_group_id BLOB NOT NULL REFERENCES music_release_groups(id) ON DELETE CASCADE,
    artist_id        BLOB NOT NULL REFERENCES media_registry(id),
    role             TEXT NOT NULL DEFAULT 'primary' CHECK(role IN (
                         'primary', 'featuring', 'remixer', 'producer', 'composer'
                     )),
    PRIMARY KEY (release_group_id, artist_id, role)
);

CREATE TABLE music_track_artists (
    track_id  BLOB NOT NULL REFERENCES music_tracks(id) ON DELETE CASCADE,
    artist_id BLOB NOT NULL REFERENCES media_registry(id),
    role      TEXT NOT NULL DEFAULT 'primary' CHECK(role IN (
                  'primary', 'featuring', 'remixer', 'producer', 'composer'
              )),
    PRIMARY KEY (track_id, artist_id, role)
);
```

Track-level artist credits are separate from release group credits to support per-track featuring and remixer attributions without polluting the album-level artist list.

---

## Audiobooks

Audiobooks are the second most detailed schema. The chapter table enables per-chapter playback navigation; the progress table tracks per-user position at chapter + millisecond offset granularity.

### `audiobooks`

```sql
CREATE TABLE audiobooks (
    id               BLOB NOT NULL PRIMARY KEY,
    registry_id      BLOB REFERENCES media_registry(id),
    title            TEXT NOT NULL,
    subtitle         TEXT,
    publisher        TEXT,
    isbn             TEXT,
    asin             TEXT,
    duration_ms      INTEGER,
    release_date     TEXT,
    language         TEXT,
    series_name      TEXT,
    series_position  REAL,
    file_path        TEXT,
    file_format      TEXT CHECK(file_format IN ('m4b', 'mp3', 'flac')),
    file_size_bytes  INTEGER,
    quality_score    INTEGER,
    quality_profile_id INTEGER REFERENCES quality_profiles(id),
    source_type      TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                         'local', 'torrent', 'usenet', 'manual', 'rss'
                     )),
    added_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_ab_registry ON audiobooks(registry_id);
CREATE INDEX idx_ab_asin ON audiobooks(asin);
CREATE INDEX idx_ab_isbn ON audiobooks(isbn);
CREATE UNIQUE INDEX idx_ab_file_path ON audiobooks(file_path) WHERE file_path IS NOT NULL;
```

`series_position` uses REAL to support fractional positions (book 1.5 in a series, prequel novellas).

### `audiobook_chapters`

```sql
CREATE TABLE audiobook_chapters (
    id           BLOB NOT NULL PRIMARY KEY,
    audiobook_id BLOB NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    position     INTEGER NOT NULL,
    title        TEXT,
    start_ms     INTEGER NOT NULL,
    end_ms       INTEGER NOT NULL,
    UNIQUE(audiobook_id, position)
);

CREATE INDEX idx_ac_audiobook ON audiobook_chapters(audiobook_id);
```

### `audiobook_progress`

Per-user playback position at chapter + millisecond offset granularity.

```sql
CREATE TABLE audiobook_progress (
    id               BLOB NOT NULL PRIMARY KEY,
    audiobook_id     BLOB NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    user_id          BLOB NOT NULL,
    chapter_position INTEGER NOT NULL,
    offset_ms        INTEGER NOT NULL DEFAULT 0,
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(audiobook_id, user_id)
);

CREATE INDEX idx_ap_user ON audiobook_progress(user_id);
```

`UNIQUE(audiobook_id, user_id)` ensures one progress record per user per book. Upsert on playback events.

### Junction Tables

```sql
CREATE TABLE audiobook_authors (
    audiobook_id BLOB NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    person_id    BLOB NOT NULL REFERENCES media_registry(id),
    role         TEXT NOT NULL DEFAULT 'author' CHECK(role IN (
                     'author', 'narrator', 'translator', 'editor'
                 )),
    PRIMARY KEY (audiobook_id, person_id, role)
);
```

---

## Books

### `books`

```sql
CREATE TABLE books (
    id                 BLOB NOT NULL PRIMARY KEY,
    registry_id        BLOB REFERENCES media_registry(id),
    title              TEXT NOT NULL,
    subtitle           TEXT,
    isbn               TEXT,
    isbn13             TEXT,
    openlibrary_id     TEXT,
    goodreads_id       TEXT,
    publisher          TEXT,
    publish_date       TEXT,
    language           TEXT,
    page_count         INTEGER,
    description        TEXT,
    file_path          TEXT,
    file_format        TEXT CHECK(file_format IN ('epub', 'mobi', 'azw3', 'pdf')),
    file_size_bytes    INTEGER,
    quality_score      INTEGER,
    quality_profile_id INTEGER REFERENCES quality_profiles(id),
    source_type        TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                           'local', 'torrent', 'usenet', 'manual', 'rss'
                       )),
    added_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_books_registry ON books(registry_id);
CREATE INDEX idx_books_isbn13 ON books(isbn13);
CREATE INDEX idx_books_openlibrary ON books(openlibrary_id);
CREATE UNIQUE INDEX idx_books_file_path ON books(file_path) WHERE file_path IS NOT NULL;
```

### Junction Tables

```sql
CREATE TABLE book_authors (
    book_id   BLOB NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    person_id BLOB NOT NULL REFERENCES media_registry(id),
    role      TEXT NOT NULL DEFAULT 'author' CHECK(role IN (
                  'author', 'translator', 'illustrator', 'editor'
              )),
    PRIMARY KEY (book_id, person_id, role)
);

CREATE TABLE book_genres (
    book_id BLOB NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    genre   TEXT NOT NULL,
    PRIMARY KEY (book_id, genre)
);
```

`book_genres` is a simple junction rather than a separate genres table. Genre taxonomy is not normalized in v1 — genres are free-text strings as returned by metadata providers.

---

## Comics

Comics use ComicInfo.xml field names directly as columns. ComicInfo.xml is the de facto metadata standard for comic archives (CBZ/CBR), and using its field names makes import/export round-trips lossless.

### `comics`

```sql
CREATE TABLE comics (
    id                  BLOB NOT NULL PRIMARY KEY,
    registry_id         BLOB REFERENCES media_registry(id),
    series_name         TEXT NOT NULL,
    volume              INTEGER,
    issue_number        REAL,
    title               TEXT,
    publisher           TEXT,
    release_date        TEXT,
    page_count          INTEGER,
    summary             TEXT,
    language            TEXT,
    comicinfo_writer    TEXT,
    comicinfo_penciller TEXT,
    comicinfo_inker     TEXT,
    comicinfo_colorist  TEXT,
    file_path           TEXT,
    file_format         TEXT CHECK(file_format IN ('cbz', 'cbr', 'pdf')),
    file_size_bytes     INTEGER,
    quality_score       INTEGER,
    quality_profile_id  INTEGER REFERENCES quality_profiles(id),
    source_type         TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                            'local', 'torrent', 'usenet', 'manual', 'rss'
                        )),
    added_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_comics_registry ON comics(registry_id);
CREATE INDEX idx_comics_series ON comics(series_name, volume, issue_number);
CREATE UNIQUE INDEX idx_comics_file_path ON comics(file_path) WHERE file_path IS NOT NULL;
```

`comicinfo_writer`, `comicinfo_penciller`, etc. are plain text strings from ComicInfo.xml, retained for lossless round-trip compatibility. The normalized creator relationships are in the junction table below.

`issue_number` uses REAL to support decimal issue numbers (0.5 specials, annuals sometimes numbered 1.0).

### Junction Tables

```sql
CREATE TABLE comic_creators (
    comic_id  BLOB NOT NULL REFERENCES comics(id) ON DELETE CASCADE,
    person_id BLOB NOT NULL REFERENCES media_registry(id),
    role      TEXT NOT NULL CHECK(role IN (
                  'writer', 'penciller', 'inker', 'colorist', 'letterer', 'editor'
              )),
    PRIMARY KEY (comic_id, person_id, role)
);
```

---

## Podcasts

Podcasts use a two-table design: subscriptions (the feed) and episodes (individual items). Subscriptions are outside the want/release/have lifecycle — see `want-release.md` Podcast Exception for the rationale.

### `podcast_subscriptions`

```sql
CREATE TABLE podcast_subscriptions (
    id                 BLOB NOT NULL PRIMARY KEY,
    feed_url           TEXT NOT NULL UNIQUE,
    title              TEXT,
    description        TEXT,
    author             TEXT,
    image_url          TEXT,
    language           TEXT,
    last_checked_at    TEXT,
    auto_download      INTEGER NOT NULL DEFAULT 1,
    quality_profile_id INTEGER REFERENCES quality_profiles(id),
    added_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
```

`auto_download = 1` means new episodes are grabbed automatically. `auto_download = 0` means the feed is monitored but nothing is downloaded without manual action. `quality_profile_id` applies when auto-downloading.

### `podcast_episodes`

```sql
CREATE TABLE podcast_episodes (
    id               BLOB NOT NULL PRIMARY KEY,
    subscription_id  BLOB NOT NULL REFERENCES podcast_subscriptions(id) ON DELETE CASCADE,
    guid             TEXT NOT NULL,
    title            TEXT,
    description      TEXT,
    episode_number   INTEGER,
    season_number    INTEGER,
    publication_date TEXT,
    duration_ms      INTEGER,
    enclosure_url    TEXT,
    file_path        TEXT,
    file_size_bytes  INTEGER,
    file_format      TEXT,
    quality_score    INTEGER,
    source_type      TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                         'local', 'torrent', 'usenet', 'manual', 'rss'
                     )),
    listened         INTEGER NOT NULL DEFAULT 0,
    added_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(subscription_id, guid)
);

CREATE INDEX idx_pe_subscription ON podcast_episodes(subscription_id);
CREATE INDEX idx_pe_pub_date ON podcast_episodes(subscription_id, publication_date);
CREATE UNIQUE INDEX idx_pe_file_path ON podcast_episodes(file_path) WHERE file_path IS NOT NULL;
```

`UNIQUE(subscription_id, guid)` deduplicates episodes on feed refresh. `listened` is a boolean integer (0/1) for simple "played/unplayed" state; per-episode progress tracking is not in scope for v1.

---

## Movies

### `movies`

```sql
CREATE TABLE movies (
    id                 BLOB NOT NULL PRIMARY KEY,
    registry_id        BLOB REFERENCES media_registry(id),
    title              TEXT NOT NULL,
    original_title     TEXT,
    year               INTEGER,
    tmdb_id            INTEGER,
    imdb_id            TEXT,
    runtime_min        INTEGER,
    overview           TEXT,
    certification      TEXT,
    file_path          TEXT,
    file_format        TEXT,
    file_size_bytes    INTEGER,
    resolution         TEXT,
    codec              TEXT,
    hdr_type           TEXT CHECK(hdr_type IN ('HDR10', 'HDR10Plus', 'DolbyVision', 'HLG', NULL)),
    quality_score      INTEGER,
    quality_profile_id INTEGER REFERENCES quality_profiles(id),
    source_type        TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                           'local', 'torrent', 'usenet', 'manual', 'rss'
                       )),
    added_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_movies_registry ON movies(registry_id);
CREATE INDEX idx_movies_tmdb ON movies(tmdb_id);
CREATE INDEX idx_movies_imdb ON movies(imdb_id);
CREATE UNIQUE INDEX idx_movies_file_path ON movies(file_path) WHERE file_path IS NOT NULL;
```

### Junction Tables

```sql
CREATE TABLE movie_cast (
    movie_id       BLOB NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    person_id      BLOB NOT NULL REFERENCES media_registry(id),
    role           TEXT NOT NULL CHECK(role IN ('director', 'actor', 'writer', 'producer')),
    character_name TEXT,
    PRIMARY KEY (movie_id, person_id, role)
);
```

---

## TV

TV uses a three-level hierarchy: series → season → episode. Only episodes carry file paths and quality scores.

### `tv_series`

```sql
CREATE TABLE tv_series (
    id                 BLOB NOT NULL PRIMARY KEY,
    registry_id        BLOB REFERENCES media_registry(id),
    title              TEXT NOT NULL,
    tmdb_id            INTEGER,
    tvdb_id            INTEGER,
    imdb_id            TEXT,
    status             TEXT NOT NULL CHECK(status IN ('continuing', 'ended', 'upcoming')),
    overview           TEXT,
    network            TEXT,
    quality_profile_id INTEGER REFERENCES quality_profiles(id),
    added_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_tv_registry ON tv_series(registry_id);
CREATE INDEX idx_tv_tmdb ON tv_series(tmdb_id);
CREATE INDEX idx_tv_tvdb ON tv_series(tvdb_id);
```

### `tv_seasons`

```sql
CREATE TABLE tv_seasons (
    id             BLOB NOT NULL PRIMARY KEY,
    series_id      BLOB NOT NULL REFERENCES tv_series(id) ON DELETE CASCADE,
    season_number  INTEGER NOT NULL,
    title          TEXT,
    episode_count  INTEGER,
    air_date       TEXT,
    overview       TEXT,
    UNIQUE(series_id, season_number)
);

CREATE INDEX idx_tvs_series ON tv_seasons(series_id);
```

### `tv_episodes`

```sql
CREATE TABLE tv_episodes (
    id               BLOB NOT NULL PRIMARY KEY,
    season_id        BLOB NOT NULL REFERENCES tv_seasons(id) ON DELETE CASCADE,
    episode_number   INTEGER NOT NULL,
    title            TEXT,
    air_date         TEXT,
    runtime_min      INTEGER,
    overview         TEXT,
    tmdb_episode_id  INTEGER,
    file_path        TEXT,
    file_format      TEXT,
    file_size_bytes  INTEGER,
    resolution       TEXT,
    codec            TEXT,
    hdr_type         TEXT CHECK(hdr_type IN ('HDR10', 'HDR10Plus', 'DolbyVision', 'HLG', NULL)),
    quality_score    INTEGER,
    source_type      TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                         'local', 'torrent', 'usenet', 'manual', 'rss'
                     )),
    added_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(season_id, episode_number)
);

CREATE INDEX idx_tve_season ON tv_episodes(season_id);
CREATE INDEX idx_tve_tmdb ON tv_episodes(tmdb_episode_id);
CREATE UNIQUE INDEX idx_tve_file_path ON tv_episodes(file_path) WHERE file_path IS NOT NULL;
```

### Junction Tables

```sql
CREATE TABLE tv_series_cast (
    series_id      BLOB NOT NULL REFERENCES tv_series(id) ON DELETE CASCADE,
    person_id      BLOB NOT NULL REFERENCES media_registry(id),
    role           TEXT NOT NULL CHECK(role IN ('director', 'actor', 'writer', 'producer')),
    character_name TEXT,
    PRIMARY KEY (series_id, person_id, role)
);
```

---

## News

News uses a two-table design: feeds (the RSS/Atom subscription) and articles (individual items). Like podcasts, news feeds are outside the want/release/have lifecycle — articles are fetched automatically on schedule. See `want-release.md` News Exception for the rationale.

### `news_feeds`

```sql
CREATE TABLE news_feeds (
    id                     BLOB NOT NULL PRIMARY KEY,
    title                  TEXT NOT NULL,
    url                    TEXT NOT NULL UNIQUE,
    site_url               TEXT,
    description            TEXT,
    category               TEXT,
    icon_url               TEXT,
    last_fetched_at        TEXT,
    fetch_interval_minutes INTEGER NOT NULL DEFAULT 60,
    is_active              INTEGER NOT NULL DEFAULT 1,
    added_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE UNIQUE INDEX idx_nf_url ON news_feeds(url);
```

`url` is the RSS/Atom feed URL — unique constraint prevents duplicate subscriptions. `category` is a user-assigned label (free-text, no normalized taxonomy). `is_active = 0` suspends fetching without deleting the feed or its articles.

### `news_articles`

```sql
CREATE TABLE news_articles (
    id           BLOB NOT NULL PRIMARY KEY,
    feed_id      BLOB NOT NULL REFERENCES news_feeds(id) ON DELETE CASCADE,
    guid         TEXT NOT NULL,
    title        TEXT NOT NULL,
    url          TEXT NOT NULL,
    author       TEXT,
    content_html TEXT,
    summary      TEXT,
    published_at TEXT,
    is_read      INTEGER NOT NULL DEFAULT 0,
    is_starred   INTEGER NOT NULL DEFAULT 0,
    source_type  TEXT NOT NULL DEFAULT 'rss' CHECK(source_type IN ('rss', 'atom')),
    added_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(feed_id, guid)
);

CREATE INDEX idx_na_feed ON news_articles(feed_id);
CREATE INDEX idx_na_pub_date ON news_articles(feed_id, published_at);
CREATE INDEX idx_na_starred ON news_articles(is_starred) WHERE is_starred = 1;
```

`UNIQUE(feed_id, guid)` deduplicates articles on feed refresh — the same item_guid seen twice is an upsert, not a new row. `content_html` stores full article body when the feed provides it; `summary` stores the excerpt. `is_read` and `is_starred` are per-user in a multi-user context but stored flat in v1 (single-user assumption matches podcasts).

---

## Common Patterns

Every top-level table follows these conventions:

| Pattern | Rule |
|---------|------|
| Primary key | `id BLOB NOT NULL PRIMARY KEY` — UUIDv7, 16-byte BLOB |
| Registry link | `registry_id BLOB REFERENCES media_registry(id)` — NULLABLE, populated asynchronously by Epignosis |
| Timestamps | `added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))` — ISO8601 UTC |
| File columns | `file_path TEXT`, `file_size_bytes INTEGER`, `quality_score INTEGER` — NULL until item is imported |
| Source type | `source_type TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN ('local', 'torrent', 'usenet', 'manual', 'rss'))` — on all file-bearing leaf tables. Tracks acquisition method for seeding management, statistics, and re-acquisition. Set by Taxis on import. |
| Junction tables | Three-column composite PK: `(media_item_id, registry_id, role)` — see `entity-registry.md` |
| External IDs | Inline on the table (tmdb_id, mb_release_id, isbn) — not in a normalized table, because each type has its own provider set and one-to-one cardinality |

The normalized external ID table (`registry_external_ids`) applies to `media_registry` entries. Per-type tables carry external IDs inline because each type uses a small, fixed set of providers with one ID per provider per item.

---

## Index Strategy

Summary of index categories across all per-type tables:

| Category | Why |
|----------|-----|
| FK columns | `release_group_id`, `medium_id`, `series_id`, `season_id`, `subscription_id`, `audiobook_id` — all FK columns are indexed. SQLite does not auto-index FKs. |
| External provider IDs | `mb_release_group_id`, `mb_release_id`, `mb_recording_id`, `acoustid_id`, `tmdb_id`, `tvdb_id`, `imdb_id`, `isbn13`, `asin`, `openlibrary_id` — metadata provider lookups hit these on every enrichment cycle. `acoustid_id` uses a partial index (`WHERE acoustid_id IS NOT NULL`) since it is NULL until Syntaxis processes the track. |
| `file_path UNIQUE` | Partial index (`WHERE file_path IS NOT NULL`) on every file-bearing table — enforces one path per item, skips un-imported rows. |
| Composite sort indexes | `(series_name, volume, issue_number)` for comics, `(subscription_id, publication_date)` for podcast episodes — supports ordered listing queries. |
| `registry_id` | On every top-level table — cross-type entity lookup from a registry entry to its typed media items. |
