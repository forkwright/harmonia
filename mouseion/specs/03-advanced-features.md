# Spec 03: Advanced Features

**Status:** Active (Phase 1 complete)
**Priority:** Medium
**Issues:** —

## Goal

Intelligence layer on top of the media library. Smart playlists, Smart Lists that auto-populate from external sources, delay profiles for quality-conscious acquisition, podcast transcription, taste analytics, and multi-zone playback. These are differentiators — features no single *arr tool provides, generalized across all 10 media types.

## Phases

### Phase 1: Smart playlists ✅
- [x] SmartPlaylist entity (ModelBase, filter JSON, track count, timestamps)
- [x] SmartPlaylistTrack join entity (playlist → track with position)
- [x] SmartPlaylistRepository: GetTracksAsync, SetTracksAsync (transactional), GetStaleAsync
- [x] SmartPlaylistService: CRUD + RefreshAsync (deserialize FilterRequest → LibraryFilterService → atomic track replace)
- [x] SmartPlaylistController: api/v3/smartplaylists (list/get/create/update/delete/refresh)
- [x] SmartPlaylistResourceValidator: name required, max 200, valid JSON filter
- [x] 31 tests (10 service, 4 entity, 11 controller, 6 validator)

### Phase 2: Smart Lists (auto-add from external sources)
Cinephage's Smart Lists generalized across all media types. Dynamic queries against external metadata sources that auto-add matching items to the library.

- [ ] SmartList entity — source (TMDB/Trakt/AniList/MusicBrainz/Goodreads), query parameters, target media type, quality profile, root folder, refresh interval
- [ ] TMDB Smart Lists — discover by genre, year, rating, keywords, popularity (movies + TV)
- [ ] Trakt Smart Lists — trending, popular, anticipated, user watchlists, custom lists (movies + TV)
- [ ] AniList Smart Lists — by genre, score threshold, season, popularity (manga + anime)
- [ ] MusicBrainz Smart Lists — new releases by tag, area, artist type (music)
- [ ] Goodreads/OpenLibrary Smart Lists — by shelf, list, subject (books + audiobooks)
- [ ] Auto-add pipeline: fetch matches → deduplicate against existing library → apply quality profile → add as monitored → optionally trigger search
- [ ] Configurable filters: minimum rating, year range, exclude genres, language
- [ ] Refresh scheduler: per-list intervals (daily/weekly/monthly) via existing Jobs infrastructure

### Phase 3: Delay profiles
Quality-conscious acquisition — wait for better releases before grabbing.

- [ ] DelayProfile entity — media type, preferred quality cutoff, delay period (hours), bypass for preferred quality, tags
- [ ] Delay evaluation in download decision pipeline: if release meets minimum but not preferred quality, hold for delay period
- [ ] Bypass: if release meets or exceeds preferred quality, grab immediately regardless of delay
- [ ] Per-tag scoping: different delay profiles for different libraries (e.g., 4K movies wait 7 days, 1080p grabs immediately)
- [ ] Integration with existing QualityDefinition weight system for cutoff comparison

### Phase 4: Analytics
- [ ] Taste profile from listening/reading/watching history
- [ ] Consumption statistics (daily/weekly/monthly) across all media types
- [ ] Per-media-type breakdown: hours listened, pages read, episodes watched

### Phase 5: Transcription (low priority)
- [ ] Whisper API integration for podcast transcription
- [ ] Full-text search across transcripts
- [ ] Chapter marker generation from transcripts

### Phase 6: Multi-zone (deferred — moon shot)
- [ ] WebSocket-based synchronized playback
- [ ] Zone management API
- [ ] Latency compensation

## Dependencies

- Smart Lists (Phase 2) use existing ImportList refresh infrastructure but are distinct from ImportLists
- Delay profiles (Phase 3) integrate with download decision pipeline in Mouseion.Core/Download
- Transcription needs Whisper API access (local or OpenAI)
- Multi-zone is significant new infrastructure — don't start before core features stable

## Notes

- Smart Lists (Phase 2) are NOT ImportLists. ImportLists sync from a user's personal collection. Smart Lists query public discovery APIs with filters. Different intent.
- Delay profiles (Phase 3) are critical for indexer reputation — wait-and-grab-once vs grab-then-upgrade.
- Multi-zone (Phase 6) labeled "moon shot" — deprioritize.
