# Spec 03: Advanced Features

**Status:** Draft
**Priority:** Medium
**Issues:** #57, #58, #60, #61

## Goal

Intelligence layer on top of the media library. Smart Lists that auto-populate from external sources, delay profiles for quality-conscious acquisition, podcast transcription, taste analytics, and multi-zone playback. These are differentiators — features no single *arr tool provides, generalized across all 10 media types.

## Phases

### Phase 1: Smart playlists
- [ ] Audio analysis-based playlist generation (#60)
- [ ] Dynamic playlists from quality/genre/era filters
- [ ] Playlist CRUD API endpoints

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
Quality-conscious acquisition — wait for better releases before grabbing. Cinephage's pattern, applied across media types.

- [ ] DelayProfile entity — media type, preferred quality cutoff, delay period (hours), bypass for preferred quality, tags
- [ ] Delay evaluation in download decision pipeline: if release meets minimum but not preferred quality, hold for delay period
- [ ] Bypass: if release meets or exceeds preferred quality, grab immediately regardless of delay
- [ ] Per-tag scoping: different delay profiles for different libraries (e.g., 4K movies wait 7 days, 1080p grabs immediately)
- [ ] UI: delay countdown visible on pending items
- [ ] Integration with existing QualityDefinition weight system for cutoff comparison

### Phase 4: Analytics
- [ ] Taste profile from listening/reading/watching history (#57)
- [ ] Consumption statistics (daily/weekly/monthly) across all media types
- [ ] "On This Day" / historical playback data
- [ ] Per-media-type breakdown: hours listened, pages read, episodes watched

### Phase 5: Transcription
- [ ] Whisper API integration for podcast transcription (#61)
- [ ] Full-text search across transcripts
- [ ] Chapter marker generation from transcripts

### Phase 6: Multi-zone
- [ ] WebSocket-based synchronized playback (#58)
- [ ] Zone management API
- [ ] Latency compensation

## Dependencies

- Smart playlists need audio analysis data (TagLib metadata exists, spectral analysis in Phase 7)
- Smart Lists use existing ImportList refresh infrastructure but are distinct from ImportLists (discovery vs. import)
- Delay profiles integrate with download decision pipeline in Mouseion.Core/Download
- Transcription needs Whisper API access (local or OpenAI)
- Multi-zone is complex — significant new infrastructure

## Notes

- Smart playlists (Phase 1) has lowest dependency and highest immediate value.
- Smart Lists (Phase 2) are NOT ImportLists. ImportLists sync from a user's personal collection on another service. Smart Lists query public discovery APIs with filters. Different intent, different UX, may share some infrastructure.
- Delay profiles (Phase 3) are critical for indexer reputation — grabbing the first release and re-grabbing a better one wastes bandwidth and annoys trackers. Wait-and-grab-once is better.
- Multi-zone (#58) is labeled "moon shot" — deprioritize.
- Taste profile needs substantial consumption history to be meaningful.
