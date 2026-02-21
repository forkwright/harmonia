# Spec 07: Tracker Import Pipeline

**Status:** Draft
**Priority:** High
**Issues:** —

## Goal

Users won't migrate to Mouseion without their history. Every competing media manager supports importing from at least one external tracker. Mouseion's `IImportList` infrastructure already supports the pattern — `ImportListBase`, `ImportListDefinition`, `ImportListFetchResult`, `ImportListSyncService` — but only has implementations for TMDb and RSS. This spec adds first-class import from the trackers people actually use: Trakt, MAL, AniList, Goodreads, Last.fm, and ListenBrainz. Each is an `IImportList` implementation that maps external schemas to `ImportListItem` fields.

## Phases

### Phase 1: Trakt (movies + TV)
Trakt is the dominant cross-platform watch tracker. 50M+ users, OAuth API, rich history data.

- [ ] TraktImportList : ImportListBase — OAuth 2.0 device code flow for authorization
- [ ] TraktSettings — client ID, client secret, OAuth token storage, list selection, sync scope
- [ ] Import sources: user watchlist, user collection, user ratings, user watch history, custom lists, popular/trending (public)
- [ ] Map Trakt items to ImportListItem: TMDB ID (movies), TVDB ID (TV), IMDB ID, title, year, ratings
- [ ] Watch history import: convert Trakt "watched_at" timestamps to MediaProgress records (mark as complete with date)
- [ ] Ratings import: store user rating alongside media item
- [ ] Incremental sync: track last sync timestamp, only fetch changes since (Trakt supports `start_at` parameter)
- [ ] Rate limiting: respect Trakt API limits (1000 calls/5min with OAuth)

### Phase 2: MAL + AniList (manga + anime)
MyAnimeList and AniList are the two dominant anime/manga trackers. Different APIs, same user base.

- [ ] MALImportList : ImportListBase — OAuth 2.0 for MAL API v2
- [ ] MAL import sources: user animelist, user mangalist (with status: watching/reading/completed/dropped/plan-to)
- [ ] Map MAL items: MAL ID, title, status, score, episodes/chapters consumed, start/finish dates
- [ ] AniListImportList : ImportListBase — OAuth via AniList API v2 (GraphQL)
- [ ] AniList import sources: user media lists (anime + manga), with status and progress
- [ ] Map AniList items: AniList ID → cross-reference to MAL ID where available, title (romaji/english/native), progress, score
- [ ] Extend ImportListItem: add MalId (int), AniListId (int) fields for anime/manga identifiers
- [ ] Status mapping: MAL/AniList statuses → Mouseion monitored state (Completed → IsComplete, Watching → in-progress, Plan to Watch → monitored)
- [ ] Chapter/episode progress: import as MediaProgress (e.g., "read 45 of 120 chapters")

### Phase 3: Goodreads + OpenLibrary (books + audiobooks)
Goodreads dominates book tracking. OpenLibrary is the open alternative. Import from both.

- [ ] GoodreadsImportList : ImportListBase — RSS feed import (Goodreads API deprecated, RSS shelves still work)
- [ ] Goodreads import sources: user shelves (read, currently-reading, to-read, custom shelves)
- [ ] Map Goodreads items: Goodreads ID, ISBN, title, author, rating, date read, shelf → status
- [ ] OpenLibraryImportList : ImportListBase — public API, no auth required
- [ ] OpenLibrary import sources: user reading log (want-to-read, currently-reading, already-read)
- [ ] Map OpenLibrary items: OL work ID, ISBN, title, author, status
- [ ] Cross-reference: when both Goodreads ID and ISBN are available, use ISBN to match existing Mouseion books
- [ ] Audiobook detection: if Goodreads edition format indicates audiobook, set MediaType.Audiobook instead of MediaType.Book

### Phase 4: Last.fm + ListenBrainz (music)
Last.fm has 20 years of scrobble data. ListenBrainz is the open alternative with MusicBrainz IDs.

- [ ] LastFmImportList : ImportListBase — API key auth (no OAuth needed for read)
- [ ] Last.fm import sources: user library (top artists, top albums, recent tracks), user loved tracks
- [ ] Map Last.fm items: MusicBrainz ID (when available), artist, album, track, play count, last played
- [ ] ListenBrainzImportList : ImportListBase — token auth
- [ ] ListenBrainz import sources: user listens (with MusicBrainz IDs), user feedback (love/hate)
- [ ] Map ListenBrainz items: MusicBrainz recording/release/artist IDs, listen timestamps
- [ ] Scrobble history → consumption stats: aggregate play counts into analytics (feeds Spec 03 Phase 4)
- [ ] Dedup: Last.fm and ListenBrainz users often have both — match by MusicBrainz ID to avoid double-import

### Phase 5: Unified import UX
- [ ] Import wizard: select source → authenticate → preview items → select what to import → execute
- [ ] Dry-run mode: show what would be imported (new items, updated progress, conflicts) without committing
- [ ] Conflict resolution UI: when imported item already exists, show diff (rating mismatch, progress mismatch) and let user choose
- [ ] Import history: log all imports with source, timestamp, items added/updated/skipped
- [ ] Bulk re-import: re-sync all from a source (useful after initial library build)
- [ ] Export: generate Trakt/MAL/AniList-compatible export files from Mouseion data (reciprocal)

## Dependencies

- Existing `IImportList` / `ImportListBase` / `ImportListSyncService` infrastructure handles discovery, scheduling, and sync
- `ImportListItem` already has TMDB, TVDB, IMDB, MusicBrainz, Goodreads, Audible, and podcast identifiers — most external IDs are covered
- Phase 1 Trakt OAuth needs secure token storage — can use existing config/settings pattern (`ImportListDefinition.Settings` JSON blob)
- Phase 2 MAL/AniList IDs need new fields on `ImportListItem` (MalId, AniListId)
- Watch/listen/read history import creates MediaProgress records — depends on Spec 01 Phase 1 progress API being stable
- Per-user imports depend on Spec 06 (Auth) Phase 1 — without user identity, all imports go to "default" user

## Notes

- Yamtrack supports Trakt, Simkl, MAL, AniList, Kitsu, TMDB, Jellyfin, Plex imports. Their import pipeline is the most comprehensive in the space.
- Trakt device code flow is better UX than redirect-based OAuth for server apps — user gets a code to enter on trakt.tv, no callback URL needed.
- Goodreads deprecated their API in 2020 but RSS feeds for shelves still work and are the standard import method. Each shelf has a public RSS URL if the profile is public.
- ListenBrainz has native MusicBrainz IDs on every listen, making it the cleanest import source for music.
- Last.fm API provides MusicBrainz IDs on some responses but not all — fall back to artist+album+track string matching when MBIDs are missing.
- Export (Phase 5) is strategically important: users who can export from Mouseion are less afraid to commit to it. Lock-in anxiety kills adoption.
