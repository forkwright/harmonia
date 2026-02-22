# Spec 07: Tracker Import Pipeline

**Status:** Feature-complete (all 5 phases implemented)
**Priority:** High
**Issues:** —

## Goal

Users won't migrate to Mouseion without their history. Every competing media manager supports importing from at least one external tracker. Mouseion's `IImportList` infrastructure already supports the pattern — `ImportListBase`, `ImportListDefinition`, `ImportListFetchResult`, `ImportListSyncService` — but only has implementations for TMDb and RSS. This spec adds first-class import from the trackers people actually use: Trakt, MAL, AniList, Goodreads, Last.fm, and ListenBrainz. Each is an `IImportList` implementation that maps external schemas to `ImportListItem` fields.

## Phases

### Phase 1: Trakt (movies + TV) ✅
- [x] TraktImportList : ImportListBase — OAuth 2.0 device code flow for authorization
- [x] TraktSettings — client ID, client secret, OAuth token storage, list selection, sync scope
- [x] Import sources: user watchlist, user collection, user ratings, user watch history, custom lists, popular/trending (public)
- [x] Map Trakt items to ImportListItem: TMDB ID (movies), TVDB ID (TV), IMDB ID, title, year, ratings
- [x] Watch history import: convert Trakt "watched_at" timestamps to MediaProgress records
- [x] Ratings import: store user rating alongside media item
- [x] Incremental sync: track last sync timestamp, only fetch changes since
- [x] Rate limiting: respect Trakt API limits (1000 calls/5min with OAuth)

### Phase 2: MAL + AniList (manga + anime) ✅
- [x] MALImportList : ImportListBase — OAuth 2.0 for MAL API v2
- [x] MAL import sources: user animelist, user mangalist (with status: watching/reading/completed/dropped/plan-to)
- [x] Map MAL items: MAL ID, title, status, score, episodes/chapters consumed, start/finish dates
- [x] AniListImportList : ImportListBase — OAuth via AniList API v2 (GraphQL)
- [x] AniList import sources: user media lists (anime + manga), with status and progress
- [x] Map AniList items: AniList ID → cross-reference to MAL ID where available
- [x] Extend ImportListItem: MalId (int), AniListId (int) fields
- [x] Status mapping: MAL/AniList statuses → Mouseion monitored state
- [x] Chapter/episode progress: import as MediaProgress

### Phase 3: Goodreads + OpenLibrary (books + audiobooks) ✅
- [x] GoodreadsImportList : ImportListBase — RSS feed import (Goodreads API deprecated, RSS shelves still work)
- [x] Goodreads import sources: user shelves (read, currently-reading, to-read, custom shelves)
- [x] Map Goodreads items: Goodreads ID, ISBN, title, author, rating, date read, shelf → status
- [x] OpenLibraryImportList : ImportListBase — public API, no auth required
- [x] OpenLibrary import sources: user reading log (want-to-read, currently-reading, already-read)
- [x] Map OpenLibrary items: OL work ID, ISBN, title, author, status
- [x] Cross-reference: BookCrossReferenceService — ISBN-based dedup with fuzzy title+author fallback
- [x] Audiobook detection: if edition format indicates audiobook, set MediaType.Audiobook

### Phase 4: Last.fm + ListenBrainz (music) ✅
- [x] LastFmImportList : ImportListBase — API key auth (no OAuth needed for read)
- [x] Last.fm import sources: user library (top artists, top albums, recent tracks), user loved tracks
- [x] Map Last.fm items: MusicBrainz ID (when available), artist, album, track, play count, last played
- [x] ListenBrainzImportList : ImportListBase — token auth
- [x] ListenBrainz import sources: user listens (with MusicBrainz IDs), user feedback (love/hate)
- [x] Map ListenBrainz items: MusicBrainz recording/release/artist IDs, listen timestamps
- [x] Scrobble history → consumption stats: aggregate play counts into analytics
- [x] Dedup: MusicCrossReferenceService — MBID-based dedup with fuzzy artist+title fallback

### Phase 5: Unified import UX ✅
- [x] Import wizard — POST /api/v3/import/preview/{listId} (dry-run: categorize as new/conflict/skipped/excluded)
- [x] Execute — POST /api/v3/import/execute/{listId}?autoResolve=true|false
- [x] Re-sync — POST /api/v3/import/resync/{listId} (bulk re-import with auto-resolve)
- [x] Conflict resolution — POST /api/v3/import/resolve/{id} (keep existing, use imported, or merge specific fields)
- [x] Import history — GET /api/v3/import/history (sessions with fetched/added/updated/skipped/failed counts)
- [x] Per-list history — GET /api/v3/import/history/list/{id}
- [x] Session detail — GET /api/v3/import/history/{id} (per-item outcomes + diffs)
- [x] Export — GET /api/v3/import/export/{format} (json, csv, trakt, goodreads, letterboxd)
- [x] Media type + date filtering on exports
- [x] ImportWizardService, ImportItemMatcher (cross-reference by TMDb/IMDb/ISBN/MusicBrainz/title)
- [x] ExportService (JSON, CSV, Trakt, Goodreads, Letterboxd formats)
- [x] Migration 032 (ImportSessions + ImportSessionItems)

## Dependencies

- Existing `IImportList` / `ImportListBase` / `ImportListSyncService` infrastructure handles discovery, scheduling, and sync
- `ImportListItem` already has TMDB, TVDB, IMDB, MusicBrainz, Goodreads, Audible, and podcast identifiers
- Watch/listen/read history import creates MediaProgress records — depends on Spec 01 progress API
- Per-user imports depend on Spec 06 (Auth) — without user identity, all imports go to "default" user

## Notes

- Export is strategically important: users who can export from Mouseion are less afraid to commit. Lock-in anxiety kills adoption.
- Goodreads deprecated their API in 2020 but RSS feeds for shelves still work.
- ListenBrainz has native MusicBrainz IDs on every listen, making it the cleanest import source for music.
