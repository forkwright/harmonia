# Mouseion Roadmap

**Last Updated:** 2026-01-02
**Current Phase:** Phase 3/4 Metadata Providers Complete → File Scanning Next
**Timeline:** 16-20 weeks to full *arr replacement (Week 9/20 complete - 45%)

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
| 3: Music | Weeks 7-10 | Audiophile features, fingerprinting | 🔄 Next |
| 4: Movies | Weeks 11-12 | Radarr parity | ⏳ Planned |
| 5: TV/Podcasts | Weeks 13-14 | Episode tracking, RSS | ⏳ Planned |
| 6: Infrastructure | Weeks 15-20 | Subtitles, import lists, production polish | ⏳ Planned |

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

## Phase 3: Music (Weeks 7-10) 🔄

**Goal:** Audiophile-grade music management

**Features:**
- [x] Artist/Album/Track models and repositories
- [x] MusicBrainz metadata integration
- [ ] Music file scanning and import ← **Next**
- [ ] TagLib integration for file analysis ← **Next**
- [ ] AcoustID fingerprinting (duplicate detection)
- [x] Music quality parser (54 definitions: lossy, lossless, hi-res, DSD)
- [ ] Spectral analysis (fake hi-res detection via FFmpeg)
- [ ] Quality upgrades (MP3-320 → FLAC 16/44.1 → FLAC 24/96)
- [x] API endpoints: `/api/v3/artists`, `/api/v3/albums`, `/api/v3/tracks`

**Quality System:**
- Lossy: MP3/AAC/OGG/Opus at 128-320kbps
- FLAC: 16/44.1 → 24/192 (CD → Hi-Res)
- DSD: DSD64/128/256/512 (SACD)
- Special: MQA, MQA Studio, ALAC, APE, WavPack

**Success Criteria:**
- Spectral analysis detects 90%+ fake hi-res
- AcoustID matches duplicates accurately
- Quality detection works for all 54 music definitions

---

## Phase 4: Movies (Weeks 11-12) 🔄

**Goal:** Maintain Radarr feature parity

**Features:**
- [x] Movie model (extends MediaItem)
- [x] TMDb metadata provider
- [x] Movie quality parser (reuse IDs 0-31)
- [ ] Movie file scanning and organization ← **Next**
- [ ] Calendar view
- [x] API endpoints: `/api/v3/movies`, `/api/v3/moviefiles`

**Success Criteria:**
- Feature parity with Radarr
- Zero regressions
- TMDB metadata fetch <1 sec
- Downloads organize correctly

---

## Phase 5: TV Shows & Podcasts (Weeks 13-14) ⏳

**Goal:** Sonarr replacement + podcast management

**TV Shows:**
- [ ] TVShow/Season/Episode models
- [ ] TVDB/TMDb metadata
- [ ] Air date monitoring
- [ ] Season pack handling
- [ ] Anime scene numbering
- [ ] API endpoints: `/api/v3/tvshows`, `/api/v3/seasons`, `/api/v3/episodes`

**Podcasts:**
- [ ] Podcast/PodcastEpisode models
- [ ] RSS feed parsing and polling
- [ ] Auto-download new episodes
- [ ] API endpoints: `/api/v3/podcasts`, `/api/v3/podcast-episodes`

**Success Criteria:**
- Auto-download TV episodes on air date
- Season packs extract correctly
- Podcast RSS polling <1 hour, 99% compatibility

---

## Phase 6: Infrastructure & Polish (Weeks 15-20) ⏳

**Goal:** Production-ready release with all *arr features

**Weeks 15-16: Supporting Infrastructure**
- [ ] Import lists (Goodreads, MusicBrainz, IMDB)
- [ ] Analytics dashboard
- [ ] Custom formats
- [ ] Auto-tagging rules
- [ ] Subtitles foundation (OpenSubtitles, hash-based matching, SRT/VTT/ASS conversion)

**Weeks 17-18: Production Readiness**
- [ ] Performance optimization
- [ ] Security hardening
- [ ] Accessibility improvements
- [ ] Health checks + startup validation
- [ ] Environment-specific configuration

**Weeks 19-20: Documentation & Deployment**
- [ ] Complete API documentation (OpenAPI/Swagger)
- [ ] Docker/Podman production images
- [ ] Deployment guides
- [ ] User guides

**Success Criteria:**
- All BLOCKER/CRITICAL bugs resolved
- Security audit complete
- 40% test coverage overall
- Zero nullability warnings across codebase

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
