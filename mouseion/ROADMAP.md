# Mouseion Roadmap

**Last Updated:** 2025-12-31
**Current Phase:** Phase 0 Complete → Starting Phase 1
**Timeline:** 16-20 weeks to full *arr replacement

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
| 0: Foundation | Complete | MediaItems table, quality system | ✅ Done |
| 1: Quality System | Weeks 1-2 | Parsers, polymorphic types, tests | 🔄 In Progress |
| 2: Books/Audiobooks | Weeks 3-6 | Full CRUD, metadata, Akroasis integration | ⏳ Next |
| 3: Music | Weeks 7-10 | Audiophile features, fingerprinting | ⏳ Planned |
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

## Phase 1: Quality System (Weeks 1-2) 🔄

**Goal:** Production-ready quality parsers and polymorphic implementations

**Features:**
- [ ] Port all quality definitions from archive
- [ ] Quality parsers for all media types (book formats, audiobook codecs, music bitrates)
- [ ] Polymorphic MediaItem implementations (Movie, Book, Audiobook, etc.)
- [ ] Type-safe repository pattern (remove reflection)
- [ ] Test framework with 50%+ coverage

**Success Criteria:**
- Zero nullability warnings
- All quality parsers tested
- Database migrations verified on SQLite + PostgreSQL

---

## Phase 2: Books & Audiobooks (Weeks 3-6) ⏳

**Goal:** Production-ready book/audiobook management with Akroasis integration

**Features:**
- [ ] Author/BookSeries/Book/Audiobook models and repositories
- [ ] Metadata providers (OpenLibrary, Audnexus)
- [ ] MyAnonamouse indexer integration
- [ ] Search and import workflows
- [ ] Hierarchical monitoring (author → series → book)
- [ ] API endpoints: `/api/v3/authors`, `/api/v3/books`, `/api/v3/audiobooks`
- [ ] Narrator-aware audiobook logic
- [ ] Progress tracking endpoint for Akroasis

**Success Criteria:**
- Add 100 authors with metadata in <5 min
- MyAnonamouse search <2 sec
- Audiobook streaming with chapters working
- Quality upgrades functional (MP3-128 → M4B)
- Akroasis Android client integration ready

**Integration Points:**
- `/api/v3/audiobooks` - Full CRUD
- `/api/v3/stream/{id}` - Already implemented ✅
- `/api/v3/chapters/{id}` - Already implemented ✅
- `/api/v3/progress/{id}` - New endpoint

---

## Phase 3: Music (Weeks 7-10) ⏳

**Goal:** Audiophile-grade music management

**Features:**
- [ ] Artist/Album/Track models and repositories
- [ ] MusicBrainz metadata integration
- [ ] AcoustID fingerprinting (duplicate detection)
- [ ] Music quality parser (60+ definitions: lossy, lossless, hi-res, DSD)
- [ ] Spectral analysis (fake hi-res detection via FFmpeg)
- [ ] TagLib integration for file analysis
- [ ] Quality upgrades (MP3-320 → FLAC 16/44.1 → FLAC 24/96)
- [ ] API endpoints: `/api/v3/artists`, `/api/v3/albums`, `/api/v3/tracks`

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

## Phase 4: Movies (Weeks 11-12) ⏳

**Goal:** Maintain Radarr feature parity

**Features:**
- [ ] Movie model (extends MediaItem)
- [ ] TMDb metadata provider
- [ ] Movie quality parser (reuse IDs 0-31)
- [ ] File scanning and organization
- [ ] Calendar view
- [ ] API endpoints: `/api/v3/movies`, `/api/v3/moviefiles`

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
