# Spec 03: advanced features

**Status:** Feature-complete (Phases 1-4 implemented, Phase 5 deferred, Phase 6 moon shot)
**Priority:** Medium
**Issues:** none

## Goal

Intelligence layer on top of the media library. Smart playlists, Smart Lists that auto-populate from external sources, delay profiles for quality-conscious acquisition, podcast transcription, taste analytics, and multi-zone playback. These are differentiators: features no single *arr tool provides, generalized across all 10 media types.

## Phases

### Phase 1: smart playlists ✅
- [x] SmartPlaylist entity (ModelBase, filter JSON, track count, timestamps)
- [x] SmartPlaylistTrack join entity (playlist → track with position)
- [x] SmartPlaylistRepository: GetTracksAsync, SetTracksAsync (transactional), GetStaleAsync
- [x] SmartPlaylistService: CRUD + RefreshAsync (deserialize FilterRequest → LibraryFilterService → atomic track replace)
- [x] SmartPlaylistController: api/v3/smartplaylists (list/get/create/update/delete/refresh)
- [x] SmartPlaylistResourceValidator: name required, max 200, valid JSON filter
- [x] 31 tests (10 service, 4 entity, 11 controller, 6 validator)

### Phase 2: smart lists (auto-add from external sources) ✅
- [x] SmartList entity: source, query params, target media type, quality profile, root folder, refresh interval
- [x] SmartListSource enum: TMDbPopular, TMDbTopRated, TraktTrending, TraktAnticipated, AniListSeasonal, AniListPopular, GoodreadsList, MusicBrainzNewReleases, PodcastCharts
- [x] SmartListService: CRUD + RefreshAsync (fetch → deduplicate → auto-add) with parallel multi-source execution
- [x] SmartListController: api/v3/smartlists (list/get/create/update/delete/refresh/preview)
- [x] TMDb, Trakt, AniList, Goodreads, MusicBrainz source executors
- [x] Configurable filters: minimum rating, year range, language, max items
- [x] Migration 031

### Phase 3: delay profiles ✅
- [x] DelayProfile entity: media type, preferred quality cutoff, delay hours, bypass for preferred
- [x] DelayProfileService: CRUD + ShouldDelayAsync (quality comparison against cutoff)
- [x] DelayProfileController: api/v3/delayprofiles (list/get/create/update/delete/evaluate)
- [x] Evaluation endpoint: POST /evaluate with release info to check if delay applies
- [x] Per-tag scoping: different profiles for different libraries
- [x] Migration 031

### Phase 4: analytics ✅
- [x] ConsumptionStats: per-media-type breakdown (completed/in-progress, sessions, duration), daily activity heatmap, most active day/hour
- [x] TasteProfile: media type preferences (normalized 0-100), completion rates, consumption pattern classification (Binge/Steady/Sporadic)
- [x] AnalyticsRepository: Dapper queries against PlaybackSessions + MediaProgress with parallel execution
- [x] AnalyticsService: orchestrates 7 parallel queries, derives style classification and velocity stats
- [x] AnalyticsController: GET /api/v3/analytics/consumption?period=30d, /taste, /activity?period=90d

### Phase 5: transcription (deferred, low priority)
- [ ] Whisper API integration for podcast transcription
- [ ] Full-text search across transcripts
- [ ] Chapter marker generation from transcripts

### Phase 6: multi-zone (deferred, moon shot)
- [ ] WebSocket-based synchronized playback
- [ ] Zone management API
- [ ] Latency compensation

## Dependencies

- Smart Lists (Phase 2) use existing ImportList refresh infrastructure but are distinct from ImportLists
- Delay profiles (Phase 3) integrate with download decision pipeline in Mouseion.Core/Download
- Transcription needs Whisper API access (local or OpenAI)
- Multi-zone is significant new infrastructure; don't start before core features stable

## Notes

- Smart Lists (Phase 2) are NOT ImportLists. ImportLists sync from a user's personal collection. Smart Lists query public discovery APIs with filters. Different intent.
- Delay profiles (Phase 3) are critical for indexer reputation; wait-and-grab-once vs grab-then-upgrade.
- Multi-zone (Phase 6) labeled "moon shot"; deprioritize.
