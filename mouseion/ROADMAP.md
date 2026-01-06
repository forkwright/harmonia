# Mouseion Roadmap

**Last Updated:** 2026-01-04
**Current Phase:** Phase 7 Complete - All Core Features Implemented
**Timeline:** 17-20 weeks to full *arr replacement (Week 20/20 complete - 100%)

---

## Vision

Transform Mouseion into a unified media manager replacing the entire *arr ecosystem:
- **Radarr** (movies)
- **Sonarr** (TV shows)
- **Lidarr** (music)
- **Readarr** (books)
- **Bazarr** (subtitles)
- **Prowlarr** (indexers)

Plus audiobooks, podcasts, and comics in a single application.

---

## Phase Overview

| Phase | Timeline | Focus | Status |
|-------|----------|-------|--------|
| 0: Foundation | Complete | MediaItems table, quality system foundation | ✅ Done |
| 1: Quality System | Weeks 1-2 | Parsers, polymorphic types, 103 definitions | ✅ Done |
| 2: Books/Audiobooks | Weeks 3-6 | Full CRUD, metadata, async/await | ✅ Done |
| 3: Music | Weeks 7-10 | Audiophile features, fingerprinting | ✅ Done |
| 4: Movies | Weeks 11-12 | Radarr parity | ✅ Done |
| 5: TV/Podcasts | Weeks 13-14 | Episode tracking, RSS | ✅ Done |
| 6: Infrastructure | Weeks 15-16 | Download clients, notifications, health checks | ✅ Done |
| 7: File Scanning | Weeks 17-20 | Music scanning, movie org, history, covers | ✅ Done |
| 8: Polish & Enhancements | Weeks 21-24 | Movie org, subtitles, auto-tagging, import pipeline | 📋 Planning |

---

## Phase 0: Foundation ✅ (Complete)

**Deliverables:**
- [x] Unified `MediaItems` table supporting 7 media types
- [x] 103 quality definitions (Movies, Books, Audiobooks, Music, Podcasts)
- [x] Hierarchical parent tables (Authors, Artists, TVShows, BookSeries)
- [x] Database migrations with reversibility
- [x] Streaming endpoint with HTTP 206 Range support
- [x] Chapter Markers API

**Quality Distribution:**
- Movies: 30 definitions (IDs 0-31)
- Books: 6 definitions (IDs 100-109)
- Audiobooks: 5 definitions (IDs 200-209)
- Music: 54 definitions (IDs 300-389)
- Podcasts: 7 definitions (IDs 400-409)

---

## Phase 1: Quality System (Weeks 1-2) ✅

**Goal:** Production-ready quality parsers and polymorphic implementations

**Features:**
- [x] Port all quality definitions from archive (103 definitions)
- [x] Quality parsers for all media types (BookQualityParser, AudiobookQualityParser, MusicQualityParser)
- [x] Polymorphic MediaItem implementations (Book, Audiobook base classes)
- [x] Type-safe repository pattern with Dapper
- [x] Test framework with 50%+ coverage (131/134 tests for quality parsers)

**Success Criteria:**
- ✅ Zero nullability warnings in Core
- ✅ All quality parsers tested (BookQualityParser: 72 tests, AudiobookQualityParser: 36 tests, MusicQualityParser: 43 tests)
- ✅ Database migrations verified on SQLite

---

## Phase 2: Books & Audiobooks (Weeks 3-6) ✅

**Goal:** Production-ready book/audiobook management with Akroasis integration

**Features:**
- [x] Author/BookSeries/Book/Audiobook models and repositories
- [x] Metadata providers (OpenLibrary, Audnexus) with Polly resilience
- [x] MyAnonamouse indexer integration
- [x] Search and import workflows via API controllers
- [x] Hierarchical monitoring (author → series → book)
- [x] API endpoints: `/api/v3/authors`, `/api/v3/books`, `/api/v3/audiobooks`, `/api/v3/series`, `/api/v3/lookup`
- [x] Narrator-aware audiobook logic (statistics by narrator)
- [x] Full async/await conversion with CancellationToken support
- [x] DryIoc 6.2.0 stable (downgraded from 8.0.0-preview-04)
- [ ] Progress tracking endpoint for Akroasis (deferred to Akroasis Phase 1)

**Success Criteria:**
- ✅ All 134 tests passing (115 unit + 19 integration)
- ✅ Zero compilation errors, zero nullability warnings
- ✅ Async/await throughout (repositories, services, controllers)
- ⏳ MyAnonamouse search performance (not benchmarked yet)
- ⏳ Audiobook streaming with chapters (infrastructure exists, needs end-to-end test)
- ⏳ Akroasis integration (blocked on Akroasis Phase 1 completion)

**Integration Points:**
- `/api/v3/audiobooks` - Full CRUD
- `/api/v3/stream/{id}` - Already implemented ✅
- `/api/v3/chapters/{id}` - Already implemented ✅
- `/api/v3/progress/{id}` - New endpoint

**Modernization (Post-Phase 2):**
- [x] Full async/await conversion with CancellationToken support (Phase A)
- [x] Pagination (50/page default, prevents memory exhaustion) (Phase B)
- [x] FluentValidation with declarative rules (Phase B)
- [x] Swagger/OpenAPI documentation at /swagger (Phase B)
- [x] IMemoryCache with 15-minute TTL for metadata providers (Phase B)
- [x] Code deduplication - Generic AddMediaItemService<T> (63% reduction) (Phase C)
- [x] Type-safe MediaType enum constants (replaced magic numbers) (Phase C)
- [ ] Modern C# features (records, required, init) - deferred

**Post-Modernization Status:**
- Build: Zero errors, zero warnings
- Tests: 134/134 passing (115 unit + 19 integration)
- Performance: Production-ready for 100+ concurrent users
- Architecture: Clean base patterns for future media types (Music, TV, Podcasts, Comics)

---

## Phase 3: Music (Weeks 7-10) ✅

**Goal:** Audiophile-grade music management

**Features:**
- [x] Artist/Album/Track models and repositories
- [x] MusicBrainz metadata integration
- [x] AcoustID fingerprinting (duplicate detection)
- [x] Music quality parser (54 definitions: lossy, lossless, hi-res, DSD)
- [x] Gazelle metadata integration
- [x] Release monitoring with AcoustID fingerprint matching
- [x] Quality parsing for all music definitions
- [x] API endpoints: `/api/v3/artists`, `/api/v3/albums`, `/api/v3/tracks`

**Quality System:**
- Lossy: MP3/AAC/OGG/Opus at 128-320kbps
- FLAC: 16/44.1 → 24/192 (CD → Hi-Res)
- DSD: DSD64/128/256/512 (SACD)
- Special: MQA, MQA Studio, ALAC, APE, WavPack

**Success Criteria:**
- ✅ AcoustID matches duplicates accurately
- ✅ Quality detection works for all 54 music definitions
- ✅ Release monitoring functional with fingerprinting

---

## Phase 4: Movies (Weeks 11-12) ✅

**Goal:** Maintain Radarr feature parity

**Features:**
- [x] Movie model (extends MediaItem)
- [x] TMDb metadata provider with poster/backdrop management
- [x] Movie quality parser (30 definitions: IDs 0-31)
- [x] Calendar view with air date monitoring
- [x] Movie import decision engine
- [x] Movie quality upgrades and monitoring
- [x] API endpoints: `/api/v3/movies`, `/api/v3/moviefiles`, `/api/v3/calendar`

**Success Criteria:**
- ✅ Feature parity with Radarr core functions
- ✅ Zero regressions in movie functionality
- ✅ TMDB metadata fetch <1 sec
- ✅ Calendar view functional

---

## Phase 5: TV Shows & Podcasts (Weeks 13-14) ✅

**Goal:** Sonarr replacement + podcast management

**TV Shows:**
- [x] TVShow/Season/Episode models
- [x] TVDB/TMDb metadata integration
- [x] Air date monitoring with automatic download
- [x] Season pack handling and extraction
- [x] Anime scene numbering support
- [x] API endpoints: `/api/v3/tvshows`, `/api/v3/seasons`, `/api/v3/episodes`

**Podcasts:**
- [x] Podcast/PodcastEpisode models
- [x] RSS feed parsing and polling
- [x] Auto-download new episodes with schedule
- [x] Episode deduplication
- [x] API endpoints: `/api/v3/podcasts`, `/api/v3/podcast-episodes`

**Success Criteria:**
- ✅ Auto-download TV episodes on air date
- ✅ Season packs extract correctly
- ✅ Podcast RSS polling <1 hour, 99% compatibility

---

## Phase 6: Infrastructure & Polish (Weeks 15-16) ✅

**Goal:** Production-ready release with all *arr features

**Download Client Integration:**
- [x] Download client managers (qBittorrent, Transmission, NZBGet)
- [x] Download status polling and synchronization
- [x] Failed download handling and retry logic
- [x] History tracking for completed downloads

**Notifications & Monitoring:**
- [x] Notification providers (Email, Webhook, Pushover, etc.)
- [x] Custom notification triggers per media type
- [x] Health checks with startup validation
- [x] System status monitoring endpoints

**Import Lists & Organization:**
- [x] Import lists (Goodreads, MusicBrainz, IMDB)
- [x] Auto-tagging rules
- [x] Custom formats for quality profiles

**Success Criteria:**
- ✅ All download clients functional
- ✅ Notifications working for all media types
- ✅ Health checks passing on startup
- ✅ Import lists discovering new items

---

## Phase 7: File Scanning & Advanced Features (Weeks 17-20) ✅

**Goal:** Complete file scanning and advanced features for all media types

**Completed (PR #18, #20, #29):**
- [x] Music file scanning and import (AudioFileAnalyzer, AudioAnalysisService)
- [x] TagLib integration for file analysis (via TagLibSharp)
- [x] Spectral analysis (fake hi-res detection via FFmpeg)
- [x] Quality upgrades (via MusicQualityParser, upgrade decision engine)
- [x] History tracking for all media types (MediaItemHistory)
- [x] Media covers management (MediaCoverService with SixLabors.ImageSharp)
- [x] AcoustID fingerprinting for duplicate detection
- [x] Gazelle indexer support (RED, OPS)

**Success Criteria:**
- ✅ Music files scanned with quality detection
- ✅ History tracking functional for imports/upgrades
- ✅ Cover art management integrated
- ✅ All Phase 7 features complete

---

## Phase 8: Polish & Enhancements (Weeks 21-24)

**Goal:** Production polish and advanced features beyond *arr parity

**Priority Features:**

1. **Movie File Organization** (Issue #96) ✅ (PR #110)
   - [x] Configurable naming patterns (`{Movie Title} ({Year}) - {Quality}`)
   - [x] Automatic folder creation per movie
   - [x] Rename on import/upgrade with dry-run mode
   - [x] File move strategies (copy, hardlink, symlink)

2. **Subtitles Foundation** (Issue #97) ✅ (PR #111)
   - [x] OpenSubtitles API integration (v1 REST API)
   - [x] Hash-based subtitle matching (MovieHash, OSDBHash)
   - [x] Language preference configuration
   - [x] Automatic download on movie import
   - [x] Manual search and download via API

3. **Auto-Tagging Improvements** (Issue #98)
   - [ ] Genre extraction from metadata providers (TMDb, MusicBrainz, Goodreads)
   - [ ] Language detection for audio/video files (FFmpeg audio streams)
   - [ ] Custom tag rules (e.g., "tag as 'Audiophile' if FLAC 24-bit")
   - [ ] Automatic tag application on import
   - [ ] Tag management API (add, remove, bulk update)

4. **Advanced File Import Pipeline** (Issue #99)
   - [ ] Import strategies: hardlink (preferred), copy, move, symlink
   - [ ] Automatic strategy selection based on filesystem
   - [ ] Import verification (hash check after copy)
   - [ ] RecycleBinService for safe deletion
   - [ ] Atomic operations with rollback on failure

**Technical Debt (Issues #100-107):**
- [ ] Quality detection system (UpgradeSpecification.cs)
- [ ] TVDB API implementation (TVDBProxy.cs)
- [ ] Notification persistence (NotificationController.cs)
- [ ] Calendar file checking (MovieCalendarService.cs)
- [x] Chapter parsing M4B/MP3 (Issue #104) - MP3 complete (PR #109), M4B partial (PR #113)
- [x] Rate limiting middleware (Issue #94) - Complete (PR #112)
- [x] Path.Combine security review (Issue #62) - Complete (41 instances validated)
- [x] Thread safety fixes (Issue #67) - Complete (2 static fields fixed)
- [ ] Sync-over-async refactoring (22 instances) - Issue #106
- [ ] Replace Thread.Sleep with Task.Delay (3 instances) - Issue #106
- [ ] Add logging to catch blocks - Issue #107

**Success Criteria:**
- ✅ Movie files organized per naming convention (PR #110)
- ✅ Subtitles download automatically (PR #111)
- ✅ Rate limiting middleware implemented (PR #112)
- ✅ M4B chapter detection (partial - graceful fallback) (PR #113)
- ⏳ Auto-tagging rules functional
- ⏳ File import strategies support all modes
- ⏳ All TODO comments resolved
- ⏳ Zero sync-over-async anti-patterns

---

## API Design

### Hierarchical Endpoints

```
GET  /api/v3/authors                    # List all authors
GET  /api/v3/authors/{id}               # Author details
GET  /api/v3/authors/{id}/books         # Books by author
GET  /api/v3/artists/{id}/albums        # Albums by artist
GET  /api/v3/tvshows/{id}/episodes      # Episodes by show
```

### Unified Media Endpoints

```
GET  /api/v3/media?type=book&authorId=5    # Filtered by type
GET  /api/v3/media/{id}                     # Single media item
```

### Streaming (Akroasis Integration)

```
GET  /api/v3/stream/{mediaId}              # HTTP 206 range support ✅
GET  /api/v3/chapters/{mediaId}            # Chapter markers ✅
POST /api/v3/progress/{mediaId}            # Playback progress
```

---

## Success Metrics

### Code Quality
- **Nullability warnings:** 40+ → 0
- **Test coverage:** 0% → 40%
- **Technical debt:** <200 hours

### Performance
- **Metadata fetch:** <2 sec (books), <1 sec (movies/music)
- **Search response:** <500ms
- **Import speed:** 100 items in <5 min

### Functionality
- All media types support: add, monitor, search, import, quality upgrades
- Cross-media tagging and organization
- Unified quality profiles

---

## Integration Timeline

| Milestone | Week | External Integration |
|-----------|------|---------------------|
| Phase 2 Complete | 6 | **Akroasis**: Audiobooks with chapters, progress tracking |
| Phase 3 Complete | 10 | **Akroasis**: + Music playback |
| Phase 5 Complete | 14 | **Akroasis**: + Podcast playback |

---

## Archive Migration Status

**Source:** `mouseion-archive-20251230/` (2,709 files)

| Category | Completion | Notes |
|----------|-----------|-------|
| Quality System | 60% | Port remaining definitions + parsers |
| Books/Audiobooks | 60% | Production-viable, needs integration |
| Music | 40% | Strong foundation, needs fingerprinting |
| TV/Podcasts | 20% | Architecture only |
| Infrastructure | 10% | Import lists, analytics planned |

**Nothing Lost Guarantee:** All archive code tracked via `/archive-migration-tracker/` with:
- Pending → In Progress → Completed checklists
- Validation per file (compiles, tests, nullability fixed)
- Cross-reference at phase end

---

## Dependencies & Tech Stack

### Backend
- .NET 8.0 (C#)
- Dapper (ORM)
- FluentMigrator (database migrations)
- DryIoc (dependency injection)
- Serilog + OpenTelemetry (logging/telemetry)
- TagLibSharp (media file parsing)

### Database
- SQLite (default)
- PostgreSQL (optional)

### API
- ASP.NET Core
- SignalR (real-time updates)
- HTTP 206 Range (streaming)

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Development setup
- Coding standards
- Conventional commits
- PR workflow

---

## License

GPL-3.0-or-later (based on Radarr)
See [NOTICE.md](NOTICE.md) for attribution.
