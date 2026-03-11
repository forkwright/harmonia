# Subtitle Management

> Prostheke-owned subtitle search, download, and format management via OpenSubtitles REST v3.
> See [architecture/subsystems.md](../architecture/subsystems.md) for Prostheke ownership boundaries.
> See [media/metadata-providers.md](metadata-providers.md) for OpenSubtitles rate limits and ProviderQueue pattern.
> See [media/lifecycle.md](lifecycle.md) for the import-triggered subtitle search hook.

---

## Prostheke Ownership

Prostheke owns all subtitle acquisition and management. No other subsystem searches for or downloads subtitle files.

| Relation | Subsystem | Direction |
|----------|-----------|-----------|
| Calls Epignosis | Prostheke → Epignosis | Media identity lookup (IMDB/TMDB ID for OpenSubtitles search) |
| Called by Taxis | Taxis → Prostheke | Post-import hook on `ImportCompleted` for movies/TV |
| Calls Aggelia | Prostheke → Aggelia | Emits `SubtitleAcquired` event when a subtitle file is stored |

Prostheke's `sync_timing` trait method is defined (see `architecture/subsystems.md`) but operates as manual-trigger-only in v1. No scheduled subtitle synchronization.

---

## OpenSubtitles REST v3 Integration

Base URL: `https://api.opensubtitles.com/api/v1`

### Authentication

OpenSubtitles REST v3 uses a two-layer auth scheme:

**API key** — included as `X-Api-Key` header on every request. Required even for unauthenticated endpoints.

**JWT token** — required for download. Obtain via login:

```
POST /api/v1/login
Body: { "username": "...", "password": "..." }
Response: { "token": "...", "base_url": "..." }
```

Token cached by Epignosis (same JWT refresh lock pattern as TVDB — `tokio::sync::Mutex<Option<OsToken>>`, first waiter refreshes, others block and reuse). Token lifetime: approximately 24 hours.

Secrets stored in `secrets.toml`:

```toml
[prostheke]
opensubtitles_api_key = "..."       # required — X-Api-Key header
opensubtitles_username = "..."      # required for download
opensubtitles_password = "..."      # required for download
```

### Search

**Movies:**
```
GET /api/v1/subtitles?imdb_id={imdb_id}&languages={lang_csv}
    also accepts: tmdb_id, query (title fallback if no IMDB ID)
```

**TV episodes:**
```
GET /api/v1/subtitles?parent_imdb_id={series_imdb_id}&season_number={n}&episode_number={n}&languages={lang_csv}
    also accepts: parent_tmdb_id
```

Response fields used:

| Field | Purpose |
|-------|---------|
| `data[].id` | Subtitle entry ID |
| `data[].attributes.language` | ISO 639-2 language code |
| `data[].attributes.format` | File format (srt, ass, ssa) |
| `data[].attributes.download_count` | Popularity signal |
| `data[].attributes.ratings` | Quality signal |
| `data[].attributes.files[0].file_id` | Required for download request |

Result ranking: prefer by `ratings` descending within format tier (SRT → ASS → SSA).

### Download

```
POST /api/v1/download
Headers: Authorization: Bearer {token}, X-Api-Key: {key}
Body: { "file_id": "..." }
Response: { "link": "...", "remaining": 18, "requests": 20, "reset_time": "..." }
```

`link` is a temporary pre-signed URL. Download the file immediately after receiving the link.

**Quota tracking:**
- `remaining` from the response body is the authoritative quota count for this 24-hour window
- If `remaining == 0` before a download: log WARN, emit `SubtitleQuotaExhausted`, stop all subtitle downloads until `reset_time`
- Free tier: 20 downloads per 24 hours. VIP: higher limits.

### Rate Limiting

All OpenSubtitles requests route through Epignosis's `ProviderQueue` at 1 req/s.

Download quota (`remaining`) is tracked separately from the API rate limit. A single request to the download endpoint consumes one quota unit regardless of file size.

---

## Search Trigger Conditions

Prostheke searches for subtitles automatically when:

1. `ImportCompleted` event received from Aggelia
2. `media_type` is `movie` or `tv_episode` — not music, audiobooks, books, comics, or podcasts
3. No existing subtitle file for the configured preferred languages (`subtitle_languages` config)
4. `auto_search_on_import = true` (default)
5. `subtitle_languages` list is non-empty

**Manual re-trigger:**
```
POST /api/prostheke/search-subtitles/{media_id}
```
No automatic re-search after initial import. If the first search returns no results, the user triggers re-search via API or UI.

**Batch imports:**

When a full TV season is imported, each episode's subtitle search is a separate syntaxis task:
```
FetchSubtitles { media_type: TvEpisode, episode_id }
FetchSubtitles { media_type: TvEpisode, episode_id }
...
```

Not one batch search per season. Individual tasks respect rate limiting and allow partial-season success.

---

## Format Preference and Handling

### Format Priority

```
SRT > ASS > SSA
```

- **SRT (SubRip Text) — preferred.** Text-based, minimal syntax, widest player compatibility. No font or styling dependencies.
- **ASS/SSA (Advanced SubStation Alpha) — accepted fallback.** Styling information may or may not render correctly in all players; Harmonia treats the content as valid and delivers as-is.
- **IDX/SUB (VobSub) — not supported in v1.** Bitmap-based format requiring image processing. Complex to handle correctly.

**Selection logic:**

1. Fetch results for the requested languages
2. Filter to supported formats (SRT, ASS, SSA)
3. Sort: SRT first, then ASS, then SSA; within each format by `ratings` descending
4. Attempt download from top result
5. If download fails or validation fails: try next result
6. If no valid result after exhausting all candidates: log `NoSubtitleFound`

### Post-Download Validation

Downloaded subtitle files are validated via the `subparse` crate before being stored:

```rust
// In spawn_blocking
fn validate_subtitle(file_path: &Path, format: SubtitleFormat) -> Result<(), SubtitleError> {
    match format {
        SubtitleFormat::Srt => subparse::parse_str(SubtitleFormat::SubRip, &content, 25.0)?,
        SubtitleFormat::Ass | SubtitleFormat::Ssa => subparse::parse_str(SubtitleFormat::SubStationAlpha, &content, 25.0)?,
    }
    // Ensure at least one subtitle entry parsed
}
```

If `subparse` fails to parse the downloaded file: discard it, log `SubtitleParseFailed`, and try the next search result. A file that fails parsing is not stored.

---

## Subtitle Storage

### File Location

Subtitle files are stored alongside their media files in the library:

**Movies:**
```
{Library Root}/{Movie Title} ({Year})/{Movie Title} ({Year}).{lang}.srt
```

Example: `{Library Root}/The Dark Knight (2008)/The Dark Knight (2008).en.srt`

**TV episodes:**
```
{Library Root}/{Series Title}/Season {N:02}/{Series Title} - S{N:02}E{N:02} - {Episode Title}.{lang}.srt
```

Example: `{Library Root}/Breaking Bad/Season 01/Breaking Bad - S01E01 - Pilot.en.srt`

Language code: ISO 639-1 two-letter code (e.g., `en`, `es`, `fr`, `de`, `pt`).

Multiple languages: one file per language per media item.

### Database Tracking

```sql
CREATE TABLE subtitles (
    id                  BLOB NOT NULL PRIMARY KEY,
    media_type          TEXT NOT NULL CHECK (media_type IN ('movie', 'tv_episode')),
    media_id            BLOB NOT NULL,      -- FK to movies.id or tv_episodes.id (soft FK)
    language            TEXT NOT NULL,       -- ISO 639-1
    format              TEXT NOT NULL CHECK (format IN ('srt', 'ass', 'ssa')),
    file_path           TEXT NOT NULL,
    opensubtitles_id    TEXT,                -- OpenSubtitles file_id for provenance tracking
    downloaded_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(media_id, language)               -- one subtitle per language per item
);

CREATE INDEX idx_sub_media ON subtitles(media_type, media_id);
CREATE INDEX idx_sub_language ON subtitles(language);
```

`UNIQUE(media_id, language)` enforces one subtitle file per language per media item. Re-downloading a subtitle for the same language replaces the existing row and file.

`media_id` is a soft FK (no `REFERENCES` constraint) because it may point to either `movies.id` or `tv_episodes.id`. The `media_type` column disambiguates. Application layer enforces referential integrity.

---

## Subtitle Sync (v1 Scope)

The `sync_timing` method on Prostheke is defined in `architecture/subsystems.md` but is manual-trigger-only in v1:

```
POST /api/prostheke/sync-subtitle/{subtitle_id}
```

**v1 behavior:** No-op stub. Returns `501 Not Implemented`.

**v2 plan:** FFmpeg-based subtitle timing correction using audio fingerprint alignment. Compares reference audio fingerprint against the actual audio track to compute an offset correction.

If subtitles are out of sync in v1, the user selects a different subtitle from OpenSubtitles via manual re-search, or replaces the subtitle file manually.

---

## Error Handling

`ProsthekeError` enum using `snafu`:

```rust
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum ProsthekeError {
    #[snafu(display("subtitle search failed for media {media_id} via {provider}"))]
    SubtitleSearchFailed {
        media_id: Uuid,
        provider: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subtitle download failed for file {subtitle_id}"))]
    SubtitleDownloadFailed {
        subtitle_id: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subtitle file at {path:?} failed format validation"))]
    SubtitleParseFailed {
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("OpenSubtitles download quota exhausted, resets at {reset_time}"))]
    SubtitleQuotaExhausted {
        reset_time: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("no {language} subtitle found for media {media_id}"))]
    NoSubtitleFound {
        media_id: Uuid,
        language: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
```

All subtitle errors are **non-fatal** to the media import process. A movie or TV episode with no subtitle is still `available`. Subtitle absence does not block Paroche from serving the media file.

Log levels:
- `SubtitleSearchFailed`, `SubtitleDownloadFailed`, `SubtitleParseFailed` — WARN (transient failures, user may retry)
- `SubtitleQuotaExhausted` — WARN once per quota period (not per search)
- `NoSubtitleFound` — INFO (expected outcome for obscure titles in some languages)

---

## Horismos Configuration

`[prostheke]` section in `horismos.toml`:

```toml
[prostheke]
# ISO 639-1 language codes to search for on import
# Empty list disables automatic subtitle search
subtitle_languages = ["en"]

# Format preference order (first supported format wins)
subtitle_format_preference = ["srt", "ass", "ssa"]

# Whether to search for subtitles automatically on movie/TV import
# Set to false to disable automatic search; use manual re-trigger endpoint only
auto_search_on_import = true
```

Secrets in `secrets.toml`:

```toml
[prostheke]
opensubtitles_api_key = "..."       # required for all OpenSubtitles requests
opensubtitles_username = "..."      # required for download quota
opensubtitles_password = "..."      # required for download quota
```

If `subtitle_languages` is non-empty and `opensubtitles_api_key` is absent: Horismos logs a startup WARN (not a fatal error — subtitle search will fail gracefully at runtime).
