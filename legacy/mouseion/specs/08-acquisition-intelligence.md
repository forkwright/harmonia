# Spec 08: acquisition intelligence

**Status:** Feature-complete (all 5 phases implemented)
**Priority:** Medium
**Issues:** none

## Goal

Make Mouseion a responsible, intelligent acquisition engine. The current state: one download client (qBittorrent), three indexer implementations (Torznab, Gazelle, MyAnonamouse), no rate limiting, no deduplication, no alternative acquisition modes. Huntarr proved that indexer-friendly behavior (rate caps, stateful dedup, queue awareness) is what keeps users from getting banned. Cinephage proved that `.strm` file generation is a viable alternative to downloading entirely. This spec expands the download client abstraction, adds acquisition intelligence, and introduces diskless streaming as a first-class mode.

## Phases

### Phase 1: download client expansion ✅
- [x] TransmissionClient : IDownloadClient: Transmission RPC API (v2.94+)
- [x] SABnzbdClient : IDownloadClient: SABnzbd API for Usenet (NZB downloads)
- [x] NZBGetClient : IDownloadClient: NZBGet/NZBGet-ng API for Usenet
- [x] DelugeClient : IDownloadClient: Deluge Web API
- [x] Health check per client: periodic connection test, alert on failure
- [x] Category/label support: auto-categorize downloads by media type

### Phase 2: indexer rate limiting and queue awareness ✅
- [x] Per-indexer rate limit configuration: max requests per hour, configurable per indexer
- [x] Rate limit state: track request count per indexer with sliding window (DB-persisted)
- [x] Queue-aware search: skip at-capacity indexers, schedule retry
- [x] Backoff on errors: exponential backoff on 429/503/timeout (1min → 5min → 30min → 4hr)
- [x] Indexer health dashboard: requests used/remaining, last error, backoff state

### Phase 3: stateful deduplication ✅
- [x] SearchHistoryEntry, GrabbedRelease, SkippedRelease entities
- [x] DeduplicationService: search history, grabbed/skipped state, download queue awareness
- [x] Intelligent re-search with media-type-specific cooldowns (TV=6h, Movie=12h, Music=24h, Books=3d)
- [x] Dedup across indexers: same release on multiple indexers → grab from preferred, skip duplicates
- [x] Migration 029

### Phase 4: .strm file generation (diskless streaming) ✅
- [x] Three debrid API clients: Real-Debrid, AllDebrid, Premiumize
  - Magnet → direct HTTPS stream URL resolution
  - Account info, connection testing, bandwidth tracking
- [x] StrmService: create, verify, refresh, delete .strm files
  - Movies/TV only (books, music, podcasts = download-only)
  - Tries debrid services in priority order with bandwidth limit enforcement
  - Expiration tracking + bulk verification
- [x] DebridServices + StrmFiles tables with full CRUD API
  - POST/GET/DELETE /api/v3/acquisition/debrid
  - GET/DELETE /api/v3/acquisition/strm/{mediaItemId}
  - POST /api/v3/acquisition/strm/verify
- [x] Migration 034

### Phase 5: acquisition orchestration ✅
- [x] AcquisitionOrchestrator: priority queue with smart defaults by source (UserTriggered=10, Import=30, SmartList=50, RssSync=70)
- [x] Strategy routing per media type: Download vs Strm vs MonitorOnly
- [x] Full state machine: Queued → Searching → Found → Grabbing → Complete/Failed
- [x] Exponential backoff retries (5min → 20min → 40min, 3 max)
- [x] Acquisition log: complete audit trail per media item (queued, searched, found, grabbed, strm_created, failed, cancelled)
- [x] Queue management API:
  - POST /api/v3/acquisition/enqueue
  - POST /api/v3/acquisition/process
  - GET /api/v3/acquisition/queue/stats
  - DELETE /api/v3/acquisition/queue/{id}
  - POST /api/v3/acquisition/queue/{id}/retry
  - GET /api/v3/acquisition/strategy/{mediaType}
  - GET /api/v3/acquisition/log[/{mediaItemId}]
- [x] Migration 034

## Dependencies

- `IDownloadClient` interface is stable; new clients implement without changes
- Phase 4 .strm requires debrid service accounts (user-provided credentials)
- Phase 4 is movies/TV only; requires files in Jellyfin/Emby library paths

## Notes

- Huntarr's rate limiting is its killer feature; users on private trackers report zero bans. Mouseion has this.
- .strm is polarizing: some users love diskless streaming (saves TB of storage), others want local copies. Both supported, download is the default.
- Real-Debrid/AllDebrid/Premiumize are the three major debrid services. All have REST APIs for torrent-to-stream resolution.
- Dedup prevents the #1 complaint on private tracker forums: automation tools hammering APIs and re-downloading.
