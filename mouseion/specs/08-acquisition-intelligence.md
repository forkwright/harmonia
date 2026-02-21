# Spec 08: Acquisition Intelligence

**Status:** Draft
**Priority:** Medium
**Issues:** —

## Goal

Make Mouseion a responsible, intelligent acquisition engine. The current state: one download client (qBittorrent), three indexer implementations (Torznab, Gazelle, MyAnonamouse), no rate limiting, no deduplication, no alternative acquisition modes. Huntarr proved that indexer-friendly behavior (rate caps, stateful dedup, queue awareness) is what keeps users from getting banned. Cinephage proved that `.strm` file generation is a viable alternative to downloading entirely. This spec expands the download client abstraction, adds acquisition intelligence, and introduces diskless streaming as a first-class mode.

## Phases

### Phase 1: Download client expansion
`IDownloadClient` already defines the interface. Only `QBittorrentClient` implements it. Add the clients people actually use.

- [ ] TransmissionClient : IDownloadClient — Transmission RPC API (v2.94+)
- [ ] SABnzbdClient : IDownloadClient — SABnzbd API for Usenet (NZB downloads)
- [ ] NZBGetClient : IDownloadClient — NZBGet/NZBGet-ng API for Usenet
- [ ] DelugeClient : IDownloadClient — Deluge Web API
- [ ] Download client settings UI: connection test, category mapping, priority, remove on completion
- [ ] Health check per client: periodic connection test, alert on failure (integrate with existing HealthCheck infrastructure)
- [ ] Category/label support: auto-categorize downloads by media type (movies/, music/, books/, etc.)

### Phase 2: Indexer rate limiting and queue awareness
Huntarr's core innovation: treat indexers as resources with capacity, not unlimited endpoints.

- [ ] Per-indexer rate limit configuration: max requests per hour, configurable per indexer
- [ ] Rate limit state: track request count per indexer with sliding window (store in DB, survive restarts)
- [ ] Queue-aware search: before searching an indexer, check if we're already at capacity — skip and schedule retry
- [ ] Backoff on errors: if indexer returns 429/503/timeout, exponential backoff (1min → 5min → 30min → 4hr)
- [ ] Indexer health dashboard: requests used/remaining per window, last error, current backoff state
- [ ] Aggregate rate budget: if Prowlarr is the upstream, respect its global rate limits on top of per-indexer limits

### Phase 3: Stateful deduplication
Don't re-search for items already grabbed, already skipped, or already in the download queue.

- [ ] Search history table: record every search (media item, indexer, timestamp, result count, best match)
- [ ] Grabbed state: when a release is sent to download client, record it — never re-grab the same release
- [ ] Skip state: when user manually skips/rejects a release, record it — don't resurface
- [ ] Download queue awareness: before searching, check if item is already in any download client's queue
- [ ] Intelligent re-search: only re-search if quality cutoff not met AND enough time has passed AND new releases are likely (based on media type release patterns)
- [ ] Dedup across indexers: same release on multiple indexers → grab from the preferred indexer, skip duplicates

### Phase 4: .strm file generation (diskless streaming)
Cinephage's genuinely novel pattern. Generate `.strm` files that Jellyfin/Emby/Kodi play directly from indexer/debrid sources without downloading to local storage.

- [ ] StrmAcquisitionMode: alternative to download — generate .strm file pointing to streamable URL
- [ ] Debrid service integration: Real-Debrid, AllDebrid, Premiumize — resolve torrent/magnet to direct HTTPS stream URL
- [ ] .strm file generation: write `<url>` to `{root_folder}/{organized_path}/{title}.strm`
- [ ] Jellyfin/Emby/Kodi auto-detect .strm files in library folders and play via URL
- [ ] Per-media-type mode: movies + TV support .strm (video streaming), books/music/podcasts stay download-only
- [ ] Hybrid mode: .strm for initial availability, download in background for permanent local copy
- [ ] Stale URL detection: periodically verify .strm URLs are still valid, re-resolve or alert on expiry
- [ ] Quality selection: when multiple stream qualities available, pick based on quality profile

### Phase 5: Acquisition orchestration
Tie it all together: smart decisions about when, where, and how to acquire media.

- [ ] Acquisition strategy per media type: download (default), strm (movies/TV), or monitor-only (news/RSS)
- [ ] Priority queue: high-priority items (user-triggered search) bypass rate limits, low-priority (RSS sync) waits
- [ ] Multi-indexer search strategy: search preferred indexers first, fall back to others only if no results
- [ ] Cost awareness: if debrid service has bandwidth limits, factor into acquisition decisions
- [ ] Acquisition log: full audit trail — what was searched, what was found, what was grabbed, what was skipped, why
- [ ] Notifications: integrate with existing notification system (Discord, Telegram, etc.) for grab/fail events

## Dependencies

- `IDownloadClient` interface is stable — new clients implement it without changes
- Phase 2 rate limiting needs persistent state — new DB table for indexer request tracking
- Phase 3 dedup needs search history table — new migration
- Phase 4 .strm requires debrid service accounts (user-provided credentials) or direct indexer stream URLs
- Phase 4 is movies/TV only — requires files to be in Jellyfin/Emby library paths
- Phase 5 priority queue may share infrastructure with Spec 03 delay profiles

## Notes

- Huntarr's rate limiting is its killer feature — users on private trackers report zero bans after switching. Mouseion needs this before scaling to more indexers.
- Download client abstraction is straightforward: Transmission RPC, SABnzbd API, NZBGet API are all well-documented JSON APIs. Each is ~200 lines of client code implementing the existing interface.
- .strm is polarizing: some users love diskless streaming (saves TB of storage), others want local copies. Support both, default to download, let power users opt into .strm.
- Real-Debrid/AllDebrid/Premiumize are the three major debrid services. All have REST APIs for torrent-to-stream resolution. Cinephage uses Real-Debrid primarily.
- Dedup is invisible to users but prevents the #1 complaint on private tracker forums: "automation tools hammer our API and re-download the same releases."
- Existing Torznab/Gazelle/MyAnonamouse indexers don't need changes — rate limiting wraps around them at the search orchestration layer, not inside individual indexer implementations.
