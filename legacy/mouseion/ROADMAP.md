# Mouseion Roadmap

## What's Done

| Phase | Focus | Status |
|-------|-------|--------|
| 0: Foundation | MediaItems table, quality system (103 definitions) | ✅ |
| 1: Quality | Parsers, polymorphic types, 131 tests | ✅ |
| 2: Books/Audiobooks | CRUD, OpenLibrary/Audnexus metadata, MyAnonamouse | ✅ |
| 3: Music | MusicBrainz, AcoustID fingerprinting, 54 quality defs | ✅ |
| 4: Movies | TMDb metadata, calendar, import engine | ✅ |
| 5: TV/Podcasts | TVDB/TMDb, RSS feeds, air date monitoring | ✅ |
| 6: Infrastructure | Download clients, notifications, health checks | ✅ |
| 7: File Scanning | TagLib, spectral analysis, covers, history | ✅ |
| 8: Polish | Movie org, subtitles, auto-tagging, import pipeline | ✅ |
| 9A: Manga | MangaDex + AniList APIs, 38 tests | ✅ |
| 9B: News/RSS | Feed parsing, read/starred state | ✅ |
| 9C: Comics | ComicVine API, 22 tests | ✅ |
| 9D: Integration | Health checks, unified stats | ✅ |

## Specs — All Feature-Complete

| Spec | Title | Status | Phases |
|------|-------|--------|--------|
| 01 | Akroasis integration (progress, sessions, sync, webhooks, OPDS) | ✅ Complete | 5/5 |
| 02 | Test coverage expansion | 🚧 Ongoing | 89+ unit tests, 18 controller test files |
| 03 | Advanced features (smart lists, delay profiles, analytics) | ✅ Complete | 4/4 (Phase 5-6 deferred) |
| 04 | Infrastructure polish (CI, workflows, DX) | ✅ Complete | 3/3 |
| 05 | Competitive analysis | ✅ Complete | Reference doc |
| 06 | Authentication & multi-user (local, OIDC, per-user state, permissions) | ✅ Complete | 5/5 |
| 07 | Tracker import pipeline (Trakt, MAL, AniList, Goodreads, Last.fm, LB) | ✅ Complete | 5/5 |
| 08 | Acquisition intelligence (clients, rate limits, dedup, debrid, orchestration) | ✅ Complete | 5/5 |

## Remaining Work

### Spec 02: Test coverage (ongoing)
~95% of core logic remains untested. This is the gap between feature-complete and production-ready. Current coverage:
- 89+ unit tests (quality parsers, services, entities)
- 18 controller integration test files (bulk, history, comic, manga, webcomic, podcasts, import lists, etc.)
- Missing: service-layer tests for the new features (analytics, smart lists, acquisition orchestrator, import wizard, auth, etc.)

### Deferred features
- **Spec 03 Phase 5**: Whisper transcription for podcasts — low priority
- **Spec 03 Phase 6**: Multi-zone synchronized playback — moon shot
- **Spec 08 Phase 1**: Download client settings UI — cosmetic

## Architecture Summary

**10 media types**: Movies, TV, Books, Audiobooks, Music, Podcasts, Manga, Comics, Webcomics, News/RSS

**Import sources**: Trakt, MAL, AniList, Goodreads, OpenLibrary, Last.fm, ListenBrainz, TMDb, RSS + export to Trakt/Goodreads/Letterboxd/JSON/CSV

**Download clients**: qBittorrent, Transmission, Deluge, SABnzbd, NZBGet + Real-Debrid/AllDebrid/Premiumize (.strm)

**Auth**: Local (JWT + refresh tokens), OIDC/OAuth 2.0 (any provider), API keys (scoped, PBKDF2-hashed)

**Intelligence**: Smart lists (auto-populate from external APIs), delay profiles (quality-conscious acquisition), consumption analytics + taste profiles, stateful dedup (indexer-friendly), acquisition orchestration (priority queue + strategy routing)
