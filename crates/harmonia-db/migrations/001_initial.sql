-- =============================================================================
-- 001_initial.sql — Full schema for Harmonia
-- All tables in FK dependency order.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- Users & auth
-- -----------------------------------------------------------------------------

CREATE TABLE users (
    id            BLOB NOT NULL PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('admin', 'member')),
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_login_at TEXT
);

CREATE INDEX idx_users_username ON users(username);

CREATE TABLE refresh_tokens (
    id         BLOB NOT NULL PRIMARY KEY,
    user_id    BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    expires_at TEXT NOT NULL,
    revoked    INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens(token_hash);

CREATE TABLE api_keys (
    id              BLOB NOT NULL PRIMARY KEY,
    user_id         BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    short_token     TEXT NOT NULL,
    long_token_hash TEXT NOT NULL UNIQUE,
    label           TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_used_at    TEXT,
    revoked         INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_api_keys_user ON api_keys(user_id);
CREATE INDEX idx_api_keys_short_token ON api_keys(short_token);

-- -----------------------------------------------------------------------------
-- Quality profiles and rank tables
-- -----------------------------------------------------------------------------

CREATE TABLE quality_profiles (
    id                         INTEGER PRIMARY KEY,
    name                       TEXT NOT NULL,
    media_type                 TEXT NOT NULL CHECK(media_type IN (
                                   'music', 'audiobook', 'book', 'comic',
                                   'podcast', 'movie', 'tv'
                               )),
    min_quality_score          INTEGER NOT NULL,
    upgrade_until_score        INTEGER NOT NULL,
    min_custom_format_score    INTEGER NOT NULL DEFAULT 0,
    upgrade_until_format_score INTEGER NOT NULL DEFAULT 0,
    upgrades_allowed           INTEGER NOT NULL DEFAULT 1,
    UNIQUE(name, media_type)
);

CREATE INDEX idx_quality_profiles_media_type ON quality_profiles(media_type);

CREATE TABLE music_quality_ranks (
    rank   INTEGER PRIMARY KEY,
    format TEXT NOT NULL UNIQUE,
    score  INTEGER NOT NULL
);

INSERT INTO music_quality_ranks (rank, format, score) VALUES
    (1, 'FLAC_24BIT',  100),
    (2, 'FLAC_16BIT',   90),
    (3, 'ALAC',          85),
    (4, 'MP3_320_CBR',   70),
    (5, 'MP3_V0_VBR',    65),
    (6, 'MP3_256',       55),
    (7, 'MP3_128',       30);

CREATE TABLE audiobook_quality_ranks (
    rank   INTEGER PRIMARY KEY,
    format TEXT NOT NULL UNIQUE,
    score  INTEGER NOT NULL
);

INSERT INTO audiobook_quality_ranks (rank, format, score) VALUES
    (1, 'M4B_AAC_128K_PLUS',  100),
    (2, 'M4B_AAC_64K',         80),
    (3, 'MP3_128K_PLUS',       70),
    (4, 'MP3_64K',             50);

CREATE TABLE video_quality_ranks (
    rank   INTEGER PRIMARY KEY,
    format TEXT NOT NULL UNIQUE,
    score  INTEGER NOT NULL
);

INSERT INTO video_quality_ranks (rank, format, score) VALUES
    (1, 'UHD_BLURAY_HEVC_HDR',  100),
    (2, 'UHD_BLURAY_H264',       85),
    (3, 'BLURAY_1080P_HEVC',      80),
    (4, 'BLURAY_1080P_H264',      75),
    (5, 'WEBDL_1080P',            70),
    (6, 'BLURAY_720P',            60),
    (7, 'WEBDL_720P',             55),
    (8, 'SD_480P',                30),
    (9, 'SDTV',                   20);

CREATE TABLE book_quality_ranks (
    rank   INTEGER PRIMARY KEY,
    format TEXT NOT NULL UNIQUE,
    score  INTEGER NOT NULL
);

INSERT INTO book_quality_ranks (rank, format, score) VALUES
    (1, 'EPUB',  100),
    (2, 'MOBI',   60),
    (3, 'AZW3',   55),
    (4, 'PDF',    40);

CREATE TABLE comic_quality_ranks (
    rank   INTEGER PRIMARY KEY,
    format TEXT NOT NULL UNIQUE,
    score  INTEGER NOT NULL
);

INSERT INTO comic_quality_ranks (rank, format, score) VALUES
    (1, 'CBZ',  100),
    (2, 'CBR',   90),
    (3, 'PDF',   40);

CREATE TABLE podcast_quality_ranks (
    rank   INTEGER PRIMARY KEY,
    format TEXT NOT NULL UNIQUE,
    score  INTEGER NOT NULL
);

INSERT INTO podcast_quality_ranks (rank, format, score) VALUES
    (1, 'AAC_128K_PLUS',   100),
    (2, 'MP3_192K_PLUS',    80),
    (3, 'MP3_128K',         60),
    (4, 'MP3_64K',          30);

CREATE TABLE custom_formats (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    media_type TEXT NOT NULL CHECK(media_type IN (
                   'music', 'audiobook', 'book', 'comic',
                   'podcast', 'movie', 'tv'
               ))
);

CREATE TABLE custom_format_conditions (
    id        INTEGER PRIMARY KEY,
    format_id INTEGER NOT NULL REFERENCES custom_formats(id) ON DELETE CASCADE,
    field     TEXT NOT NULL,
    pattern   TEXT NOT NULL,
    score     INTEGER NOT NULL
);

-- Default quality profile seed data
INSERT INTO quality_profiles (name, media_type, min_quality_score, upgrade_until_score) VALUES
    ('Any',      'music',     1,  100),
    ('Lossless', 'music',    85,  100),
    ('Standard', 'music',    70,   90);

INSERT INTO quality_profiles (name, media_type, min_quality_score, upgrade_until_score) VALUES
    ('Any',           'audiobook',  1, 100),
    ('M4B Preferred', 'audiobook', 80, 100);

INSERT INTO quality_profiles (name, media_type, min_quality_score, upgrade_until_score) VALUES
    ('Any',  'book', 1,   100),
    ('EPUB', 'book', 100, 100);

INSERT INTO quality_profiles (name, media_type, min_quality_score, upgrade_until_score) VALUES
    ('Any', 'comic', 1,   100),
    ('CBZ', 'comic', 100, 100);

INSERT INTO quality_profiles (name, media_type, min_quality_score, upgrade_until_score) VALUES
    ('Any',          'podcast',  1, 100),
    ('High Quality', 'podcast', 80, 100);

INSERT INTO quality_profiles (name, media_type, min_quality_score, upgrade_until_score) VALUES
    ('Any', 'movie',  1, 100),
    ('HD',  'movie', 60, 100),
    ('UHD', 'movie', 85, 100);

INSERT INTO quality_profiles (name, media_type, min_quality_score, upgrade_until_score) VALUES
    ('Any', 'tv',  1, 100),
    ('HD',  'tv', 60, 100),
    ('UHD', 'tv', 85, 100);

-- -----------------------------------------------------------------------------
-- Entity registry
-- -----------------------------------------------------------------------------

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

CREATE INDEX idx_registry_entity_type ON media_registry(entity_type);
CREATE INDEX idx_registry_display_name ON media_registry(display_name);

CREATE TABLE registry_external_ids (
    registry_id BLOB NOT NULL REFERENCES media_registry(id) ON DELETE CASCADE,
    provider    TEXT NOT NULL CHECK(provider IN (
                    'musicbrainz', 'tmdb', 'tvdb', 'openlibrary',
                    'audible', 'audnexus', 'goodreads', 'imdb', 'lastfm'
                )),
    external_id TEXT NOT NULL,
    PRIMARY KEY (registry_id, provider)
);

CREATE INDEX idx_external_ids_provider ON registry_external_ids(provider, external_id);

-- -----------------------------------------------------------------------------
-- Music (4-level hierarchy)
-- -----------------------------------------------------------------------------

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

CREATE TABLE music_media (
    id         BLOB NOT NULL PRIMARY KEY,
    release_id BLOB NOT NULL REFERENCES music_releases(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    format     TEXT NOT NULL CHECK(format IN ('CD', 'Vinyl', 'Digital', 'Cassette', 'Other')),
    title      TEXT,
    UNIQUE(release_id, position)
);

CREATE INDEX idx_mm_release ON music_media(release_id);

CREATE TABLE music_tracks (
    id                   BLOB NOT NULL PRIMARY KEY,
    medium_id            BLOB NOT NULL REFERENCES music_media(id) ON DELETE CASCADE,
    position             INTEGER NOT NULL,
    title                TEXT NOT NULL,
    duration_ms          INTEGER,
    mb_recording_id      TEXT,
    acoustid_fingerprint TEXT,
    acoustid_id          TEXT,
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

-- -----------------------------------------------------------------------------
-- Audiobooks
-- -----------------------------------------------------------------------------

CREATE TABLE audiobooks (
    id                 BLOB NOT NULL PRIMARY KEY,
    registry_id        BLOB REFERENCES media_registry(id),
    title              TEXT NOT NULL,
    subtitle           TEXT,
    publisher          TEXT,
    isbn               TEXT,
    asin               TEXT,
    duration_ms        INTEGER,
    release_date       TEXT,
    language           TEXT,
    series_name        TEXT,
    series_position    REAL,
    file_path          TEXT,
    file_format        TEXT CHECK(file_format IN ('m4b', 'mp3', 'flac')),
    file_size_bytes    INTEGER,
    quality_score      INTEGER,
    quality_profile_id INTEGER REFERENCES quality_profiles(id),
    source_type        TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                           'local', 'torrent', 'usenet', 'manual', 'rss'
                       )),
    added_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_ab_registry ON audiobooks(registry_id);
CREATE INDEX idx_ab_asin ON audiobooks(asin);
CREATE INDEX idx_ab_isbn ON audiobooks(isbn);
CREATE UNIQUE INDEX idx_ab_file_path ON audiobooks(file_path) WHERE file_path IS NOT NULL;

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

CREATE TABLE audiobook_authors (
    audiobook_id BLOB NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    person_id    BLOB NOT NULL REFERENCES media_registry(id),
    role         TEXT NOT NULL DEFAULT 'author' CHECK(role IN (
                     'author', 'narrator', 'translator', 'editor'
                 )),
    PRIMARY KEY (audiobook_id, person_id, role)
);

-- -----------------------------------------------------------------------------
-- Books
-- -----------------------------------------------------------------------------

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

-- -----------------------------------------------------------------------------
-- Comics
-- -----------------------------------------------------------------------------

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

CREATE TABLE comic_creators (
    comic_id  BLOB NOT NULL REFERENCES comics(id) ON DELETE CASCADE,
    person_id BLOB NOT NULL REFERENCES media_registry(id),
    role      TEXT NOT NULL CHECK(role IN (
                  'writer', 'penciller', 'inker', 'colorist', 'letterer', 'editor'
              )),
    PRIMARY KEY (comic_id, person_id, role)
);

-- -----------------------------------------------------------------------------
-- Podcasts
-- -----------------------------------------------------------------------------

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

CREATE TABLE podcast_episodes (
    id              BLOB NOT NULL PRIMARY KEY,
    subscription_id BLOB NOT NULL REFERENCES podcast_subscriptions(id) ON DELETE CASCADE,
    guid            TEXT NOT NULL,
    title           TEXT,
    description     TEXT,
    episode_number  INTEGER,
    season_number   INTEGER,
    publication_date TEXT,
    duration_ms     INTEGER,
    enclosure_url   TEXT,
    file_path       TEXT,
    file_size_bytes INTEGER,
    file_format     TEXT,
    quality_score   INTEGER,
    source_type     TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                        'local', 'torrent', 'usenet', 'manual', 'rss'
                    )),
    listened        INTEGER NOT NULL DEFAULT 0,
    added_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(subscription_id, guid)
);

CREATE INDEX idx_pe_subscription ON podcast_episodes(subscription_id);
CREATE INDEX idx_pe_pub_date ON podcast_episodes(subscription_id, publication_date);
CREATE UNIQUE INDEX idx_pe_file_path ON podcast_episodes(file_path) WHERE file_path IS NOT NULL;

-- -----------------------------------------------------------------------------
-- News
-- -----------------------------------------------------------------------------

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

-- -----------------------------------------------------------------------------
-- Movies
-- -----------------------------------------------------------------------------

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

CREATE TABLE movie_cast (
    movie_id       BLOB NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    person_id      BLOB NOT NULL REFERENCES media_registry(id),
    role           TEXT NOT NULL CHECK(role IN ('director', 'actor', 'writer', 'producer')),
    character_name TEXT,
    PRIMARY KEY (movie_id, person_id, role)
);

-- -----------------------------------------------------------------------------
-- TV
-- -----------------------------------------------------------------------------

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

CREATE TABLE tv_seasons (
    id            BLOB NOT NULL PRIMARY KEY,
    series_id     BLOB NOT NULL REFERENCES tv_series(id) ON DELETE CASCADE,
    season_number INTEGER NOT NULL,
    title         TEXT,
    episode_count INTEGER,
    air_date      TEXT,
    overview      TEXT,
    UNIQUE(series_id, season_number)
);

CREATE INDEX idx_tvs_series ON tv_seasons(series_id);

CREATE TABLE tv_episodes (
    id              BLOB NOT NULL PRIMARY KEY,
    season_id       BLOB NOT NULL REFERENCES tv_seasons(id) ON DELETE CASCADE,
    episode_number  INTEGER NOT NULL,
    title           TEXT,
    air_date        TEXT,
    runtime_min     INTEGER,
    overview        TEXT,
    tmdb_episode_id INTEGER,
    file_path       TEXT,
    file_format     TEXT,
    file_size_bytes INTEGER,
    resolution      TEXT,
    codec           TEXT,
    hdr_type        TEXT CHECK(hdr_type IN ('HDR10', 'HDR10Plus', 'DolbyVision', 'HLG', NULL)),
    quality_score   INTEGER,
    source_type     TEXT NOT NULL DEFAULT 'local' CHECK(source_type IN (
                        'local', 'torrent', 'usenet', 'manual', 'rss'
                    )),
    added_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(season_id, episode_number)
);

CREATE INDEX idx_tve_season ON tv_episodes(season_id);
CREATE INDEX idx_tve_tmdb ON tv_episodes(tmdb_episode_id);
CREATE UNIQUE INDEX idx_tve_file_path ON tv_episodes(file_path) WHERE file_path IS NOT NULL;

CREATE TABLE tv_series_cast (
    series_id      BLOB NOT NULL REFERENCES tv_series(id) ON DELETE CASCADE,
    person_id      BLOB NOT NULL REFERENCES media_registry(id),
    role           TEXT NOT NULL CHECK(role IN ('director', 'actor', 'writer', 'producer')),
    character_name TEXT,
    PRIMARY KEY (series_id, person_id, role)
);

-- -----------------------------------------------------------------------------
-- Want / Release / Have lifecycle
-- -----------------------------------------------------------------------------

CREATE TABLE wants (
    id                 BLOB NOT NULL PRIMARY KEY,
    media_type         TEXT NOT NULL CHECK(media_type IN (
                           'music_album', 'audiobook', 'book', 'comic',
                           'podcast', 'movie', 'tv_series'
                       )),
    title              TEXT NOT NULL,
    registry_id        BLOB REFERENCES media_registry(id),
    quality_profile_id INTEGER NOT NULL REFERENCES quality_profiles(id),
    status             TEXT NOT NULL DEFAULT 'searching' CHECK(status IN (
                           'searching', 'paused', 'fulfilled'
                       )),
    source             TEXT CHECK(source IN (
                           'manual', 'tidal_sync', 'request', 'rss_feed'
                       )),
    source_ref         TEXT,
    added_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    fulfilled_at       TEXT
);

CREATE INDEX idx_wants_type_status ON wants(media_type, status);
CREATE INDEX idx_wants_registry ON wants(registry_id);

CREATE TABLE releases (
    id                  BLOB NOT NULL PRIMARY KEY,
    want_id             BLOB NOT NULL REFERENCES wants(id) ON DELETE CASCADE,
    indexer_id          INTEGER NOT NULL,
    title               TEXT NOT NULL,
    size_bytes          INTEGER NOT NULL,
    quality_score       INTEGER NOT NULL,
    custom_format_score INTEGER NOT NULL DEFAULT 0,
    download_url        TEXT NOT NULL,
    protocol            TEXT NOT NULL CHECK(protocol IN ('torrent', 'nzb')),
    info_hash           TEXT,
    found_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    grabbed_at          TEXT,
    rejected_reason     TEXT
);

CREATE INDEX idx_releases_want ON releases(want_id);
CREATE INDEX idx_releases_info_hash ON releases(info_hash);

CREATE TABLE haves (
    id               BLOB NOT NULL PRIMARY KEY,
    want_id          BLOB NOT NULL REFERENCES wants(id),
    release_id       BLOB REFERENCES releases(id),
    media_type       TEXT NOT NULL,
    media_type_id    BLOB NOT NULL,
    quality_score    INTEGER NOT NULL,
    file_path        TEXT NOT NULL,
    file_size_bytes  INTEGER NOT NULL,
    status           TEXT NOT NULL DEFAULT 'pending' CHECK(status IN (
                         'pending', 'downloading', 'importing', 'complete', 'failed'
                     )),
    imported_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    upgraded_from_id BLOB REFERENCES haves(id)
);

CREATE INDEX idx_haves_want ON haves(want_id);
CREATE INDEX idx_haves_type_id ON haves(media_type, media_type_id);
CREATE UNIQUE INDEX idx_haves_file_path ON haves(file_path);
