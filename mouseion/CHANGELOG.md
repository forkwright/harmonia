# Changelog

All notable changes to Mouseion will be documented in this file.

## [2026-01-06] - Phase 8 Progress: Movie Organization, Subtitles, Security, and M4B Detection

### Added
- **Movie File Organization** (PR #110, Issue #96) - Configurable movie file renaming and organization
  - Features: Naming patterns (`{Movie Title} ({Year}) - {Quality}`), automatic folder creation, dry-run mode
  - Strategies: Copy, hardlink, symlink file operations
  - API: POST /api/v3/movies/{id}/rename with preview support
  - Impact: Full Radarr-style file organization for movies

- **Subtitle Integration** (PR #111, Issue #97) - OpenSubtitles API for subtitle search and download
  - Provider: OpenSubtitles REST API v1 integration
  - Matching: Hash-based subtitle matching (MovieHash, OSDBHash)
  - Features: Language preference configuration, automatic download on import
  - API: GET /api/v3/subtitles/search, POST /api/v3/subtitles/download
  - Impact: Bazarr-style subtitle management integrated into Mouseion

- **Rate Limiting Middleware** (PR #112, Issue #94) - DoS prevention via AspNetCoreRateLimit
  - Package: AspNetCoreRateLimit 5.0.0
  - IP-based: 100 requests/minute, 1000 requests/hour
  - Client-based: 200 requests/minute (authenticated), 5000 requests/hour
  - Whitelist: /swagger, /health endpoints excluded
  - Impact: Production-ready DoS protection for API endpoints

- **M4B Chapter Detection** (PR #113, Issue #104 partial) - M4B format detection with graceful fallback
  - Detection: Recognizes TagLib.Mpeg4.File for M4B audiobooks
  - Fallback: Returns single-chapter for M4B files (full-book playback)
  - MP3 Support: Full ID3v2 CTOC/CHAP chapter parsing (PR #109)
  - Limitation: TagLibSharp doesn't expose MP4 chpl atoms (requires FFmpeg future enhancement)
  - Impact: M4B audiobooks play correctly, MP3 audiobooks have full chapter navigation

### Security
- **Path.Combine Review** (Issue #62) - Complete security audit of 41 Path.Combine instances
  - Finding: 38 safe (no user input), 3 already validated with IsPathTraversalSafe()
  - Status: Zero vulnerabilities, documentation added (SECURITY_ANALYSIS_PATH_COMBINE.md)

- **Thread Safety Fixes** (Issue #67) - Static field thread safety audit
  - Finding: 2 static fields reviewed, no active thread safety issues
  - Status: Complete, no changes required

**Phase Status**: Phase 8 in progress (4/9 priority items complete)
**Akroasis Integration**: M4B/MP3 chapters API ready for Phase 4 client integration

---

## [2026-01-04] - Phase 7 Complete: Music File Scanning, History, and Media Covers

### Added
- **Music File Scanning** (PR #29) - Complete music library analysis
  - Features: TagLib# integration, audio format detection, quality verification
  - Analysis: Spectral analysis for fake hi-res detection via FFmpeg
  - Quality: Dynamic range (DR) calculation, ReplayGain detection
  - Fingerprinting: AcoustID integration for duplicate detection
  - Indexers: Gazelle support (RED, OPS) for music acquisition
  - Database: Audio quality fields (BitDepth, DynamicRange, Lossless, Codec)
  - Search: Denormalized fields (ArtistName, AlbumName, Genre) for performance

- **History Tracking** (PR #29) - Complete audit trail for all media types
  - Model: MediaItemHistory with event types (download, upgrade, delete, rename)
  - Tracking: Automatic history entries on all media operations
  - API: History endpoints with filtering by media type and event type
  - Retention: Configurable history retention policies

- **Media Covers** (PR #29) - Cover art management for all media types
  - Service: MediaCoverService with SixLabors.ImageSharp
  - Features: Cover download from metadata providers, local caching
  - Resizing: On-demand cover resizing for different UI sizes
  - API: Cover endpoints with ETag support for efficient caching

**Phase Status**: Phase 7 complete (all archive migration work finished)
**Archive Status**: Archive deleted (migration complete)

---

## [2026-01-04] - Phase 2 Complete: Movies, TV, Podcasts, Download Clients & Notifications

### Added
- **Movie Implementation** (PR #23) - Complete calendar-driven movie management
  - Features: Release calendar, automatic release monitoring, import decision engine
  - Database: Movie model with release dates, quality tracking
  - API: Calendar endpoints with date-range filtering
  - Monitoring: Scheduled jobs check releases, trigger automatic downloads
  - Import Engine: Conflict resolution (existing files, custom naming)
  - Impact: Full Radarr movie feature parity

- **TV Show & Podcast Support** (PR #24) - Series/seasons/episodes paradigm
  - TV Shows: Series, seasons, episodes model with scene numbering
  - Data: TVDB integration for episode metadata and air dates
  - Scene Numbering: Maps TVDB numbering to release scene numbering
  - Podcasts: PodcastShow, PodcastEpisode with episode descriptions
  - RSS Parsing: Built-in feed parsing with PodcastIndex lookups
  - Metadata: Episode-level information (runtime, summary, guest info)
  - Pattern: Shared base classes (Series → Show, Movie) for code reuse

- **Download Client Support** (PR #25) - Multi-client integration framework
  - Clients: qBittorrent, Transmission, SABnzbd, NZBGet
  - Protocol: Unified REST API abstraction for all clients
  - Features: Add torrent/nzb, monitor status, pause/resume, remove completed
  - Configuration: Client connection profiles with failover support
  - Health Checks: Automatic client availability monitoring
  - Alternative: Built-in download instead of external - rejected (architectural cleanness, reliability)

- **Notification Support** (PR #26) - Multi-platform notification system
  - Channels: Discord, Slack, Telegram, Gotify webhooks
  - Format: Rich text templates per event type (media added, download completed, errors)
  - Apprise Integration: Support for 100+ additional notification services
  - Email: SMTP with HTML templates
  - Events: System events (startup, errors), media events (added, completed)
  - Impact: Users + AI agents can monitor library changes in real-time

- **Infrastructure & Monitoring** (PR #27) - Production monitoring foundation
  - Health Checks: Endpoint system for service availability (download clients, metadata providers)
  - Background Jobs: Scheduled tasks (release monitoring, health checks, cleanup)
  - System Monitoring: CPU, memory, disk tracking with alerts
  - Backup: Automatic database backup before destructive operations
  - Observability: Health check results in API responses for client awareness
  - Alternative: Separate monitoring service - rejected (integrate into application for simplicity)

### API Enhancements
- **Phase 2 Akroasis Integration** (PR #28) - Enhanced search and filtering
  - Search: Full-text search across all media types with type filters
  - Filters: Quality, release status, monitored status, date ranges
  - Facets: Aggregated counts by type/quality/status for UI drill-down
  - Batch Endpoints: Bulk operations (add multiple, update quality, toggle monitoring)
  - Delta Sync: Modified-since endpoint for client synchronization
  - Impact: Mobile/desktop clients can efficiently query and manage library

### Architecture Impact
- **Pattern**: Event-driven notifications (publish-subscribe)
- **Extensibility**: Download client abstraction supports new clients without core changes
- **Resilience**: Health checks and client failover prevent cascading failures
- **Scale**: Background job system removes blocking operations from request path

**Phase Status**: Phase 1 (core backend) complete, Phase 2 (client integration) complete
**Test Coverage**: Comprehensive integration tests for all new media types and download clients
**Performance**: Calendar queries optimized with date indexes, background jobs use async batching

---

## [2026-01-01] - Phase 2 Modernization: Code Quality Improvements

### Changed
- **Code Deduplication** - Extracted duplicate logic to generic base service
  - Before: AddBookService and AddAudiobookService had 85% duplicate code (324 total lines)
  - After: Generic AddMediaItemService<TMediaItem, TRepository> base class (182 total lines)
  - Impact: 63% code reduction, easier to add new media types (Music, TV, Podcasts, Comics)
  - Pattern: Template method with abstract hooks for media-specific operations
  - Files: src/Mouseion.Core/MediaItems/AddMediaItemService.cs (created)

- **Type-Safe MediaType Constants** - Replaced magic numbers with enum references
  - Before: Hard-coded integers (`MediaType = 4`, `MediaType = 5`) in SQL queries
  - After: Enum constants (`MediaType.Book`, `MediaType.Audiobook`)
  - Rationale: Prevent cross-contamination bugs, improve maintainability
  - Impact: 30+ query strings updated across Book/Audiobook repositories
  - Gotcha: Must cast to int in interpolated SQL: `{(int)MediaType.Book}`

**Build Status**: Zero errors, zero warnings
**Test Status**: 134/134 passing (115 unit + 19 integration)

---

## [2026-01-01] - Phase 2 Modernization: Production Features

### Added
- **Pagination** - Prevent memory exhaustion on large libraries
  - Implementation: PagedResult<T> wrapper with LIMIT/OFFSET SQL
  - Default: 50 items per page, max: 250
  - Metadata: TotalCount, TotalPages, HasNext, HasPrevious
  - Endpoints: All list endpoints (books, audiobooks, authors, series)
  - Alternative: Cursor-based pagination - rejected (simpler for small-scale)

- **FluentValidation** - Input validation at API boundary
  - Package: FluentValidation.AspNetCore 11.3.0
  - Rules: Title required (max 500 chars), Year 1000-2100, QualityProfileId > 0
  - Validators: BookResourceValidator, AudiobookResourceValidator, AuthorResourceValidator
  - Impact: Early validation failure with clear error messages

- **Swagger/OpenAPI** - Interactive API documentation
  - Package: Swashbuckle.AspNetCore 7.2.0
  - Endpoint: /swagger
  - Configuration: API v3 metadata, automatic schema generation
  - Benefit: Self-documenting API, easier client development

- **Metadata Caching** - Reduce external API rate limiting
  - Implementation: IMemoryCache with 15-minute TTL
  - Targets: OpenLibrary (books), Audnexus (audiobooks)
  - Package: Microsoft.Extensions.Caching.Abstractions 10.0.1
  - Impact: Cache hit logging for observability
  - Alternative: Redis distributed cache - rejected (overkill for single-user)

**Performance Target**: 100+ concurrent users, <100ms response time
**Test Impact**: Updated integration tests to expect PagedResult<T> instead of List<T>

---

## [2026-01-01] - Phase 2 Modernization: Async/Await Conversion

### Changed
- **Complete Async/Await Implementation** - Production-scale concurrency support
  - Before: All I/O operations synchronous (blocking threads)
  - After: Full async/await with CancellationToken support
  - Scope: All repositories, services, controllers, metadata providers
  - Pattern: `async Task<T>` methods with `CancellationToken ct = default`
  - ConfigureAwait: `.ConfigureAwait(false)` for all library code

- **Repository Layer** - Async data access
  - Modified: IBasicRepository<T>, BasicRepository<T>
  - Async methods: AllAsync, FindAsync, InsertAsync, UpdateAsync, DeleteAsync, GetPageAsync
  - Sync methods: Kept for backward compatibility (internal use only)
  - Implementation: Dapper async methods (QueryAsync, ExecuteAsync)

- **Service Layer** - Async business logic
  - Modified: AddBookService, AddAudiobookService, AddAuthorService
  - Modified: BookStatisticsService, AudiobookStatisticsService
  - Pattern: Generic AddMediaItemService<T> base class with async template methods

- **API Layer** - Async HTTP endpoints
  - Modified: BookController, AudiobookController, AuthorController, BookSeriesController
  - All actions: async Task<ActionResult<T>>
  - Cancellation: CancellationToken propagated from HTTP context

- **Metadata Providers** - Async HTTP calls
  - Modified: BookInfoProxy, AudiobookInfoProxy, ResilientMetadataClient
  - Modified: MyAnonamouseIndexer
  - Implementation: HttpClient async methods with Polly resilience

**DryIoc Version Change**:
- Before: DryIoc 8.0.0-preview-04 (unstable)
- After: DryIoc 6.2.0 (stable)
- Rationale: Production stability over experimental features

**Breaking Change Strategy**: Clean async-only break (no obsolete sync methods)
- Rationale: Long-term quality solution, clear migration path
- Impact: All consumers must update to async patterns
- Alternative: Keep sync methods with [Obsolete] - rejected (technical debt)

**Test Impact**: All 134 tests converted to async patterns (await, Task)

---

## [2026-01-01] - Phase 2: Books & Audiobooks Complete

### Added
- **Book Management** - Complete CRUD with metadata
  - Models: Book, BookMetadata, BookSeries
  - Repository: BookRepository with MediaType filtering
  - Service: AddBookService with author validation
  - Controller: BookController (REST API)
  - Statistics: BookStatisticsService (total, monitored counts)

- **Audiobook Management** - Narrator-aware tracking
  - Models: Audiobook, AudiobookMetadata
  - Repository: AudiobookRepository with MediaType filtering
  - Service: AddAudiobookService with author validation
  - Controller: AudiobookController (REST API)
  - Statistics: AudiobookStatisticsService

- **Metadata Providers**
  - Books: BookInfoProxy (OpenLibrary API integration)
  - Audiobooks: AudiobookInfoProxy (Audnexus API integration)
  - Resilience: Polly retry/circuit breaker wrapper (ResilientMetadataClient)

- **Indexers**
  - MyAnonamouse: Torrent indexer for books/audiobooks
  - Authentication: OAuth 1.0 signing
  - Search: Title and author-based queries

**Database Schema**:
- Table: MediaItems (polymorphic storage)
- MediaType: Book = 4, Audiobook = 5
- Columns: AuthorId (FK), BookSeriesId (FK), Monitored (bool)

**Quality Profiles**: 103 quality definitions across all media types

**Test Coverage**: 134 tests (115 unit + 19 integration)
- Integration: TestWebApplicationFactory with in-memory SQLite
- Isolation: IClassFixture for shared database per test class

---

## [2025-12-31] - CI Performance Optimization

### Changed
- **Docker Build Workflow** - Skip Docker builds on PRs for faster feedback loop
  - Before: Docker multi-arch builds ran on every PR (4+ minutes total CI time)
  - After: Docker builds only run on merge to develop/main (1.5 min PR CI time)
  - Rationale: PRs don't need production Docker images; local Podman testing catches issues faster
  - Requirement: MUST test Docker/Podman builds locally before pushing PR
  - Alternative: Keep Docker in PR CI - rejected due to slow feedback (60-70% time savings)
  - Benefit: Fast developer feedback loop without sacrificing quality gates

---

## [2025-12-30] - Fresh-Start Migration

### Phase 0: Mouseion.Common Foundation

Ground-up rewrite of Mouseion based on Radarr architecture with clean `Mouseion.*` namespace.

**Timeline**: 8 weeks (40 working days)
**Goal**: Backend for media player integration

---

## [2025-12-30] - Code Quality Standards

### Added
- **C# Code Standards Documentation** - Comprehensive coding standards in CLAUDE.md
  - Before: No documented standards
  - After: Self-documenting code philosophy, naming conventions, efficiency patterns, AI trope blacklist
  - Rationale: Establish quality foundation before scaling development
  - Impact: All future code follows documented patterns

---

## [2025-12-30] - Platform Guards

### Fixed
- **ServiceProvider Platform Guards** - Added runtime OS checks for Windows-specific APIs
  - Before: 86 CA1416 warnings for Windows-only ServiceController usage
  - After: 0 new warnings, graceful cross-platform degradation
  - Implementation: `OsInfo.IsWindows` guards + `#pragma warning disable CA1416`
  - Gotcha: Analyzer can't detect runtime guards, pragma still needed after guard
  - Alternative: Could suppress globally, but runtime guards provide better UX

---

## [2025-12-30] - PR #5: Disk Module

### Added
- **Disk Infrastructure** (22 files, 2357 lines)
  - DiskProviderBase - Cross-platform disk operations
  - DiskTransferService - Verified file transfers with rollback
  - FileSystemLookupService - Directory browsing
  - PathExtensions - Path manipulation utilities
  - OsPath - Value type for path handling
  - CleanseLogMessage - Log sanitization for security

**Performance**:
- 64KB buffers for file I/O
- Early exits (size check before byte-by-byte comparison)
- Smart FS optimizations (btrfs/zfs reflink support)
- Value type OsPath for hot paths

**Quality**:
- 0 new warnings introduced
- Self-documenting code, minimal comments
- Industry-standard C# naming
- Modern C# features (nullability, pattern matching, range syntax)

---

## [2025-12-30] - PR #4: Services & Infrastructure

### Added
- **TPL** - Rate limiting, debouncing
- **Crypto** - Hash computation
- **OAuth** - OAuth 1.0 signing
- **Options** - Configuration management
- **Instrumentation** - Serilog infrastructure
- **ServiceProvider** - Windows service management

---

## [2025-12-30] - PR #3: HTTP & Composition

### Added
- **HTTP Infrastructure** - Request/Response models
  - HttpRequest, HttpResponse, HttpUri
  - HttpHeader, HttpException types
  - User agent building
  - Proxy settings

- **Composition** - DryIoc DI container integration
- **Processes** - Process execution and monitoring

---

## [2025-12-30] - PR #2: Core Infrastructure

### Added
- **Serialization** - JSON (Newtonsoft + STJ) and XML
- **Cache** - In-memory caching with ICached<T>
- **Disk** - Basic disk provider interface

---

## [2025-12-30] - PR #1: Foundation

### Added
- **Core Infrastructure** - Essential foundation
  - EnvironmentInfo - OS, runtime, build detection
  - Extensions - String, IEnumerable, Dictionary helpers
  - Exceptions - Custom exception types
  - EnsureThat - Parameter validation
  - Reflection - Assembly utilities
  - Model - Base models

**Architecture**:
- .NET 8.0 target
- Nullable reference types enabled
- Serilog structured logging
- GPL-3.0 licensing (Radarr derivative)

---

## [2025-12-30] - Repository Structure

### Added
- **Fresh Repository** - Clean start with modern stack
  - Before: Radarr fork with `NzbDrone.*` namespace
  - After: Ground-up rewrite with `Mouseion.*` namespace
  - Rationale: Clean slate enables better API design
  - Stack: .NET 8.0, React 19, Serilog, OpenTelemetry

**Project Structure**:
```
src/
├── Mouseion.Common/   # Utilities, DI, HTTP
├── Mouseion.Core/     # Business logic (planned)
├── Mouseion.Api/      # REST API (planned)
└── Mouseion.Host/     # Entry point (planned)
```

---

## Future Milestones

### Phase 1: Core Backend (Weeks 1-4)
- Media streaming API (HTTP 206 range requests)
- Progress tracking for audiobooks/podcasts
- Podcast foundation (PodcastShows, PodcastEpisodes)
- Unified search endpoint
- High-resolution audio metadata
- OpenAPI spec generation

### Phase 2: Client Integration (Weeks 5-8)
- Delta sync endpoint (`modifiedSince`)
- File hash in responses
- Cover art dynamic resizing
- Chapter marker parsing (M4B/MP3)
- Listening history tracking
- Bulk operations

### Client Integration
- **Integration Point**: Week 3 (streaming endpoint ready)
- **Chapter Markers**: Week 4 (HIGH priority for audiobook UX)
- **Full Integration**: Week 8 (all features complete)

---

## Technical Debt

**Current**: 51 warnings (pre-existing from PRs #1-4)
- 46× CS86xx (nullability)
- 4× CA1416 (platform guards needed in RuntimeInfo.cs)
- 1× CS0618 (DryIoc obsolete API usage)

**Strategy**: Fix as we touch files, don't block progress
