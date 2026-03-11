# Music Design

> Track-level monitoring, MusicBrainz hierarchy matching, AcoustID fingerprinting, and ReplayGain R128 computation.
> See [media/lifecycle.md](lifecycle.md) for music sub-states (imported → fingerprinting → enriched → organized).
> See [media/metadata-providers.md](metadata-providers.md) for MusicBrainz, AcoustID, and Last.fm provider configs.
> See [media/import-rename.md](import-rename.md) for the music naming template.
> See [data/media-schemas.md](../data/media-schemas.md) for `music_release_groups`, `music_releases`, `music_media`, `music_tracks` tables.
> See [data/quality-profiles.md](../data/quality-profiles.md) for `music_quality_ranks` evaluation.

---

## Track-Level Monitoring

Music is the priority media type — each track is monitored individually. A single track carries its own lifecycle state, quality metadata, and provenance record.

### Per-Track State

Every `music_tracks` row has a `status` column (via `haves.status` or a dedicated `music_tracks.status` column) tracking the track through its sub-state sequence:

```
imported -> fingerprinting -> enriched -> organized -> available
```

Individual tracks on the same album can be at different states simultaneously. One track can be `available` while another is `fingerprinting`. The album-level state is never stored — it is always derived at query time.

### Per-Track Quality Metadata

All quality metadata is stored at the track level (leaf node):

| Column | Type | Meaning |
|--------|------|---------|
| `codec` | TEXT | Audio codec (flac, mp3, aac, opus, alac, wav, ogg) |
| `bit_depth` | INTEGER | Bit depth in bits (16, 24, 32) — NULL for lossy codecs |
| `sample_rate` | INTEGER | Sample rate in Hz (44100, 48000, 88200, 96000, 192000) |
| `file_size_bytes` | INTEGER | File size |
| `replay_gain_track_db` | REAL | Track-level ReplayGain gain value in dB |
| `replay_gain_album_db` | REAL | Album-level ReplayGain gain value in dB |
| `quality_score` | INTEGER | Evaluated score from `music_quality_ranks` |

### Album-Level Derived State

Album state is derived from the aggregate of all track states on a release. Not stored in the database — computed on read:

| Derived Album State | Condition |
|--------------------|-----------|
| `partial` | Some tracks `available`, some still processing or missing |
| `complete` | All expected tracks `available` (expected count from MusicBrainz release) |
| `upgrading` | At least one track being replaced by a higher-quality version |

Expected track count comes from `music_media.track_count` (if stored) or from the MusicBrainz release response. A release with 10 tracks where 8 are `available` and 2 are `imported` is `partial`.

### Quality Insight Queries

The schema columns enable direct quality analysis without additional tables:

**"What quality is my collection?"**
```sql
SELECT
    codec,
    COUNT(*) as track_count,
    ROUND(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER (), 1) as pct,
    AVG(bit_depth) as avg_bit_depth,
    ROUND(AVG(CAST(sample_rate AS REAL) / 1000), 1) as avg_sample_rate_khz
FROM music_tracks
WHERE file_path IS NOT NULL
GROUP BY codec
ORDER BY track_count DESC;
```

**"What quality is this album?"**
```sql
SELECT t.title, t.codec, t.bit_depth, t.sample_rate, t.replay_gain_track_db
FROM music_tracks t
JOIN music_media m ON t.medium_id = m.id
WHERE m.release_id = ?
ORDER BY m.position, t.position;
```

**"Are there upgrade candidates?"**
```sql
SELECT t.id, t.file_path, t.quality_score, qp.cutoff_quality_score
FROM music_tracks t
JOIN music_media m ON t.medium_id = m.id
JOIN music_releases r ON m.release_id = r.id
JOIN music_release_groups rg ON r.release_group_id = rg.id
JOIN quality_profiles qp ON rg.quality_profile_id = qp.id
WHERE t.quality_score < qp.cutoff_quality_score
  AND t.file_path IS NOT NULL;
```

---

## Album-to-Release Matching

### MusicBrainz Hierarchy Alignment

The data model maps directly to MusicBrainz's four-level hierarchy:

| MusicBrainz Entity | Harmonia Table | Key Column |
|-------------------|----------------|------------|
| Release Group | `music_release_groups` | `mb_release_group_id` |
| Release | `music_releases` | `mb_release_id` |
| Medium | `music_media` | mapped from release medium position |
| Recording | `music_tracks` | `mb_recording_id` |

- A **release group** is the abstract album concept ("OK Computer" as an idea, independent of any pressing).
- A **release** is a specific edition ("1997 UK CD", "2009 remaster", "2016 vinyl reissue").
- A **medium** is a disc, side, or digital medium within a release (disc 1 of a double album).
- A **recording** is the specific performance captured on a track.

### Matching Flow on Import

Taxis reads embedded tags via `lofty` in `spawn_blocking`, then attempts identity resolution:

**Step 1: Tag-based matching (highest confidence)**

If the file has embedded MusicBrainz ID tags (`MUSICBRAINZ_TRACKID`, `MUSICBRAINZ_ALBUMID`, `MUSICBRAINZ_RELEASEGROUPID`):
1. Call `GET /ws/2/recording/{mb_recording_id}?inc=releases+artist-credits&fmt=json`
2. Walk the release hierarchy to find or create the `music_release_groups` → `music_releases` → `music_media` → `music_tracks` rows
3. Populate all `mb_*` columns from the response

**Step 2: Search-based matching (lower confidence)**

If no MBID tags are present:
1. Read: artist name, album title, track title, track number, disc number, year from tags
2. Call `GET /ws/2/release?query=artist:{artist} AND release:{album}&inc=recordings&fmt=json`
3. Score each candidate release:

| Signal | Points |
|--------|--------|
| Exact track title match (case-insensitive) | +50 |
| Artist match (fuzzy, Levenshtein distance < 3) | +30 |
| Track count matches expected | +10 |
| Year matches (±1 year tolerance) | +10 |

4. **Threshold:** Score ≥ 70 → automatic match. Score < 70 → add to manual review queue, item proceeds to `enriched` with partial metadata (tag values only).
5. On match: populate `mb_*` columns, walk hierarchy, create/update rows.

**Step 3: On no match**

All MB fields remain NULL. Item stays at `enriched` state (usable, playable). A `IdentityAmbiguous` WARN log records the artist/album for later resolution. The manual matching UI (Phase 7) allows the user to search and assign the correct MusicBrainz release.

### Recommended Schema Additions

Two columns not in the Phase 4 schema but required for full depth:

```sql
ALTER TABLE music_releases ADD COLUMN source_type TEXT
    CHECK(source_type IN ('cd', 'vinyl', 'digital', 'web', 'sacd', 'dvd_audio', 'unknown'))
    DEFAULT 'unknown';

ALTER TABLE music_tracks ADD COLUMN acoustid_fingerprint TEXT;
```

Additional metadata depth columns:

```sql
ALTER TABLE music_tracks ADD COLUMN recording_date TEXT;
ALTER TABLE music_release_groups ADD COLUMN original_year INTEGER;
ALTER TABLE music_releases ADD COLUMN disambiguation TEXT;
```

---

## AcoustID Fingerprinting

Fingerprinting runs automatically on every music import, dispatched as a post-import syntaxis task.

### When It Runs

After Taxis creates the `music_tracks` row:
1. Taxis emits `ImportCompleted { media_type: Music, track_id }` via Aggelia
2. Epignosis subscriber dispatches `FingerprintTrack { track_id }` to syntaxis (priority: Normal)
3. Track transitions to `fingerprinting` state

### Fingerprint Backend Trait

```rust
pub trait FingerprintBackend: Send + Sync {
    async fn fingerprint(&self, file_path: &Path) -> Result<Fingerprint, FingerprintError>;
}

pub struct Fingerprint {
    pub fingerprint: String,  // Chromaprint fingerprint string
    pub duration_seconds: f64,
}
```

### Two Implementations

**`RustyChromaprintBackend` (preferred)**

Pure Rust. No system dependencies.

```rust
// All runs in spawn_blocking — CPU-bound
fn compute_rusty(file_path: &Path) -> Result<Fingerprint, FingerprintError> {
    // 1. Decode first 120s via Symphonia -> interleaved PCM f32 samples
    // 2. Feed to rusty_chromaprint::Fingerprinter
    // 3. Get fingerprint string + duration
}
```

**`FpcalcSubprocessBackend` (fallback)**

Shells out to `fpcalc -json {path}`. Requires the `chromaprint` system package.

```rust
async fn compute_fpcalc(file_path: &Path) -> Result<Fingerprint, FingerprintError> {
    let output = Command::new("fpcalc")
        .args(["-json", file_path.to_str().unwrap()])
        .output()
        .await?;
    // Parse JSON: {"fingerprint": "...", "duration": 234.5}
}
```

**Selection:** At startup, Epignosis tries `RustyChromaprintBackend`. If the crate is unavailable or produces fingerprints that fail AcoustID validation against known test fixtures, it falls back to `FpcalcSubprocessBackend`. Configurable:

```toml
[epignosis]
fingerprint_backend = "auto"  # "auto" | "rusty_chromaprint" | "fpcalc"
```

### AcoustID Lookup Flow

1. Compute fingerprint in `spawn_blocking` (CPU-bound)
2. `POST https://api.acoustid.org/v2/lookup` with `fingerprint`, `duration`, `client` params
3. Response: list of recording MBIDs with confidence scores

| Confidence | Action |
|------------|--------|
| > 0.8 | Accept match. Update `mb_recording_id` if different from tag-based match. |
| 0.5 – 0.8 | Log ambiguous match at WARN. Retain tag-based ID. Store fingerprint. |
| < 0.5 | Discard AcoustID result. Retain tag-based ID. Store fingerprint. |

4. Store `acoustid_fingerprint` on `music_tracks` row — prevents re-computation on re-import
5. Cache: `acoustid:fingerprint:{fingerprint_string}` → permanent TTL

### Non-Fatal Behavior

Fingerprinting failure (corrupt audio, unsupported codec, `fpcalc` not installed) does NOT block import:
- Log WARN with path and error
- Set `acoustid_fingerprint = NULL`
- Track transitions to `enriched` (same as success path)

---

## ReplayGain R128 Computation

EBU R128 loudness computation runs automatically on every music import, in parallel with fingerprinting (both are post-import syntaxis tasks).

### When It Runs

Dispatched via syntaxis with `ComputeLoudness { track_id }` task (priority: Low — can run after `available`).

### Implementation

```rust
// Run inside spawn_blocking — CPU-bound
fn compute_loudness(file_path: &Path) -> Result<LoudnessResult, LoudnessError> {
    // 1. Decode full file via Symphonia -> interleaved PCM f32 samples
    // 2. Create ebur128::EbuR128 analyzer (Mode::I | Mode::LRA)
    // 3. Feed all frames via add_frames_f32()
    // 4. Extract integrated_lufs via loudness_global()
    // 5. Extract loudness range via loudness_range()
    // 6. Compute replay_gain_track_db = -18.0 - integrated_lufs (ReplayGain 2.0 target)
}

pub struct LoudnessResult {
    pub integrated_lufs: f64,         // e.g., -14.3 LUFS
    pub loudness_range_lu: f64,       // e.g., 8.2 LU
    pub replay_gain_track_db: f64,    // -18.0 - integrated_lufs
}
```

- Store `replay_gain_track_db` on `music_tracks` (column already exists in schema)

### Album-Level ReplayGain

Computed after all tracks on an album are individually analyzed:

```
album_integrated_lufs = AVERAGE of all track integrated_lufs values
replay_gain_album_db = -18.0 - album_integrated_lufs
```

This is a simplified v1 approach. Full EBU R128 album normalization performs a true multi-track analysis; the average-of-tracks approach is an acceptable approximation for household listening.

Store result on `music_tracks.replay_gain_album_db` (same value on all tracks in the album).

### Non-Fatal Behavior

Loudness computation failure (corrupt file, unsupported sample format) does NOT block import:
- Log WARN with path and error
- Set `replay_gain_track_db = NULL`, `replay_gain_album_db = NULL`
- Track proceeds to `available` without gain values

---

## Source Type Tracking

Source type records where a specific release came from — physical medium vs digital download vs web rip.

### Schema

```sql
ALTER TABLE music_releases ADD COLUMN source_type TEXT
    CHECK(source_type IN ('cd', 'vinyl', 'digital', 'web', 'sacd', 'dvd_audio', 'unknown'))
    DEFAULT 'unknown';
```

### Mapping from MusicBrainz `packaging` Field

When a release is matched in MusicBrainz, the `packaging` field provides a packaging type hint:

| MusicBrainz Packaging | Harmonia `source_type` |
|-----------------------|----------------------|
| `Jewel Case`, `Keep Case`, `Digipak`, `Slim Jewel Case`, `Super Jewel Box` | `cd` |
| `Gatefold Cover`, `Cardboard/Paper Sleeve` (with vinyl medium format) | `vinyl` |
| `None` (with medium format `Digital Media`) | `digital` |
| `None` (without medium format indicator) | `web` |
| `SACD Slipcase` or medium format `SACD` | `sacd` |
| Medium format `DVD-Audio` | `dvd_audio` |
| Anything else or missing | `unknown` |

The medium `format` column on `music_media` (CD, Vinyl, Digital, Cassette, Other) provides a second signal when `packaging` is ambiguous.

User can override `source_type` per-release via API — the override persists through re-enrichment.

### Quality Insight: Source Distribution

```sql
SELECT source_type, COUNT(DISTINCT r.id) as release_count,
       COUNT(t.id) as track_count
FROM music_releases r
JOIN music_media m ON m.release_id = r.id
JOIN music_tracks t ON t.medium_id = m.id
WHERE t.file_path IS NOT NULL
GROUP BY source_type;
```

---

## Music Metadata Depth

Harmonia targets Roon-level metadata depth for music.

### MusicBrainz Provides

- Artist credits with role types (primary, featuring, remixer, producer, composer) — stored in `music_release_group_artists` and `music_track_artists` junction tables
- Recording date — stored in `music_tracks.recording_date`
- Original release date (earliest year across all editions on the release group) — stored in `music_release_groups.original_year`
- Label and catalog number — stored in `music_releases.label`, `music_releases.catalog_number`
- Release disambiguation — stored in `music_releases.disambiguation` (e.g., "2009 remaster", "deluxe edition")

### Last.fm Enriches

- Genre tags (top 5 by weight) — stored in `music_track_tags` junction table
- Global play count — stored in `music_tracks` or metadata cache
- Similar artists — not stored in database, served from Epignosis cache only
- Artist biography — served from Epignosis cache, not persisted

### Genre Storage

```sql
CREATE TABLE music_track_tags (
    track_id   BLOB NOT NULL REFERENCES music_tracks(id) ON DELETE CASCADE,
    tag_name   TEXT NOT NULL,
    weight     INTEGER NOT NULL DEFAULT 100,  -- Last.fm tag weight (1-100)
    PRIMARY KEY (track_id, tag_name)
);
```

Tags are free-text strings as returned by Last.fm — not normalized to a fixed taxonomy. The top 5 by weight are stored per track.

---

## Complete Import Flow

End-to-end sequence for a new music file:

```
File enters import pipeline (download or scan)
    |
Taxis: detect as music (lofty tag probe in spawn_blocking)
Taxis: read embedded tags (artist, album, track, track#, disc#, year, MBID tags)
Taxis: create music_tracks row with status='imported'
    |
Taxis: call Epignosis.resolve_identity() — attempt tag-based or search-based MB matching
    Populates mb_release_group_id, mb_release_id, mb_recording_id if match found
    Creates/links music_release_groups, music_releases, music_media rows
    |
Taxis: compute target path via {Artist Name}/{Album Title} ({Year})/{Track:00} - {Title}.{Ext}
Taxis: hardlink/copy/move file to library path
Taxis: update music_tracks.file_path, status='imported' (or 'fingerprinting' immediately)
Taxis: create haves row, emit ImportCompleted via Aggelia
    |
Post-import hooks dispatched via syntaxis (asynchronous — do not block import return):
    |
    syntaxis -> Epignosis: FingerprintTrack { track_id }
        music_tracks.status = 'fingerprinting'
        spawn_blocking: decode audio -> compute Chromaprint fingerprint
        POST acoustid.org/v2/lookup
        Verify/update mb_recording_id on match (confidence > 0.8)
        Store acoustid_fingerprint on music_tracks
        music_tracks.status = 'enriched'
    |
    syntaxis -> Epignosis: ComputeLoudness { track_id }
        spawn_blocking: decode full file -> ebur128 -> compute R128 integrated LUFS
        replay_gain_track_db = -18.0 - integrated_lufs
        Store on music_tracks.replay_gain_track_db
        After all album tracks processed: compute album gain, store replay_gain_album_db
    |
    syntaxis -> Epignosis: EnrichMetadata { media_type: Music, track_id }
        Fetch full MusicBrainz recording + release + release group data
        Fetch Last.fm tags (top 5), global play count
        Populate recording_date, original_year, disambiguation, label, catalog_number
        Insert music_track_tags rows
        music_tracks.status = 'organized'
    |
music_tracks.status = 'available'
Album derived state: check all track statuses -> emit 'complete' if all available
```

---

## Error Handling

Music-specific error conditions. All are non-fatal to the import completion unless noted.

| Error | Severity | Action |
|-------|----------|--------|
| `FingerprintFailed { path, source }` | Non-fatal | `acoustid_fingerprint = NULL`, proceed to `enriched` |
| `LoudnessComputeFailed { path, source }` | Non-fatal | `replay_gain_track_db = NULL`, proceed to `available` |
| `MusicBrainzMatchAmbiguous { candidates }` | Non-fatal | Log WARN, proceed with tag metadata only, add to manual review queue |
| `TagReadFailed { path }` | Non-fatal | Fall back to filename-based parsing: `{Artist}/{Album}/{TrackN} - {Title}.ext` |
| `ProviderNotFound` from MusicBrainz (canonical) | Fatal | Item stays `failed`, syntaxis retries with exponential backoff |

**Tag read fallback parsing — filename structure assumed:**
```
{Artist Name}/{Album Title} ({Year})/{Track:00} - {Track Title}.{ext}
```
Values parsed from path are used as-is without MB enrichment until manual resolution.

---

## Horismos Configuration

Music-specific additions to the `[taxis]` config section:

```toml
[taxis]
# ... (other taxis config from scanner.md and import-rename.md)

# Music quality threshold for automatic match acceptance
music_mb_match_threshold = 70          # minimum score for automatic MB match (0-100)

# Fingerprint backend selection
# "auto" = try rusty_chromaprint first, fall back to fpcalc
# "rusty_chromaprint" = native Rust only (no system dependency)
# "fpcalc" = fpcalc subprocess only (requires chromaprint system package)
# (also configurable in [epignosis] section — taxis forwards the config)
music_fingerprint_backend = "auto"
```
