# Spec 01: Akroasis Integration

**Status:** Active (Phase 1-3 complete)
**Priority:** High
**Issues:** #58

## Goal

Complete the API surface that Akroasis needs for full playback experience. Progress tracking, session management, cross-device sync, adaptive streaming, media server webhook ingestion, and OPDS content serving. This is the bridge between Mouseion's library management and every client that consumes it — Akroasis, Jellyfin, e-readers, podcast apps.

## Phases

### Phase 1: Progress & sessions
- [x] POST /api/v3/progress/{mediaId} — save playback position
- [x] GET /api/v3/progress/{mediaId} — restore position
- [x] GET /api/v3/continue — in-progress items across media types
- [x] Session tracking (start/stop/duration per playback)

### Phase 2: Cross-device sync
- [x] Queue state persistence (server-side queue for multi-device)
- [x] Playback transfer endpoint (hand off between devices)
- [x] Conflict resolution for concurrent position updates

### Phase 3: Streaming enhancements
- [x] Adaptive transcoding endpoint (lossless → opus/aac by client preference)
- [x] Bandwidth estimation hints in stream response headers
- [x] Cover art resize endpoint (thumbnails for mobile)

### Phase 4: Media server auto-tracking
Jellyfin, Emby, and Plex all fire webhooks on playback events. Mouseion should ingest these to track progress regardless of which client the user plays through — not just Akroasis.

- [ ] POST /api/v3/webhooks/jellyfin — receive Jellyfin playback webhooks (play, pause, stop, scrobble)
- [ ] POST /api/v3/webhooks/emby — receive Emby playback webhooks (same event mapping)
- [ ] POST /api/v3/webhooks/plex — receive Plex webhook payloads (Tautulli-compatible)
- [ ] Map external media IDs (Jellyfin/Emby/Plex item IDs) to Mouseion MediaItem IDs via metadata matching (TMDB/TVDB/MusicBrainz)
- [ ] Convert webhook events into MediaProgress/PlaybackSession updates (reuse Phase 1 models)
- [ ] Deduplicate: if Akroasis and Jellyfin both report the same playback, don't double-count
- [ ] Mark-as-watched threshold: configurable percentage (default 90%) to auto-set `IsComplete`

### Phase 5: OPDS 1.2 content serving
OPDS (Open Publication Distribution System) lets e-readers browse and download directly from Mouseion. Required for Book, Comic, and Manga media types — KOReader, Calibre, Moon+ Reader, Panels, and Chunky all speak OPDS natively.

- [ ] GET /opds/v1.2/catalog — root OPDS catalog (navigation feed)
- [ ] GET /opds/v1.2/books — paginated Atom feed of Book media items with acquisition links
- [ ] GET /opds/v1.2/comics — paginated feed for Comic items
- [ ] GET /opds/v1.2/manga — paginated feed for Manga items
- [ ] OPDS search endpoint (OpenSearch descriptor) — title, author, series queries
- [ ] Faceted navigation — by author, series, genre, recently added, in-progress
- [ ] Direct file acquisition links (EPUB, CBZ, CBR, PDF) with proper MIME types
- [ ] Cover image thumbnails in feed entries (opds:image links)
- [ ] API key authentication via URL parameter (`?apikey=`) for headerless OPDS clients
- [ ] OPDS Page Streaming Extension (OPDS-PSE) for comic/manga page-by-page reading

## Dependencies

- Session tracking needs new DB table + migration
- Phase 4 requires media server users to configure webhook URLs pointing at Mouseion
- Phase 5 OPDS requires Book/Comic/Manga CRUD already working (Phases 2, 9A, 9C — all complete)
- OPDS API key auth depends on Spec 06 (Auth) for proper key management, but can ship with existing `AuthOptions.ApiKey` initially

## Notes

- Progress API partially scaffolded in Phase 2 but never completed.
- Akroasis currently uses mock data for continue-listening; wiring to real API is blocked on this spec.
- Jellyfin webhook plugin: github.com/jellyfin/jellyfin-plugin-webhook — fires HTTP POST on play/pause/stop/scrobble with full item metadata.
- Yamtrack's approach: dedicated webhook endpoints per media server, map external IDs to internal via metadata API lookups. Same pattern fits here.
- OPDS 1.2 spec: specs.opds.io/opds-1.2 — Atom-based, well-specified, wide client support.
- Stump's OPDS implementation is the reference — Rust/Axum, supports PSE for page streaming, handles auth via URL params for clients that can't set headers.
- OPDS and Akroasis streaming coexist — OPDS serves files for download/offline reading, streaming endpoint serves for real-time playback.
