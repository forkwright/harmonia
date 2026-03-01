# Codebase Concerns

**Analysis Date:** 2026-03-01

## Tech Debt

**Duplicate MusicQualityParser:**
- Issue: Two separate implementations of the same parsing logic with different patterns
  - `mouseion/src/Mouseion.Core/Music/MusicQualityParser.cs` — instance-based, DI-registered
  - `mouseion/src/Mouseion.Core/Parser/MusicQualityParser.cs` — static class, called directly from MusicFileAnalyzer
- Files: `mouseion/src/Mouseion.Core/Music/MusicQualityParser.cs`, `mouseion/src/Mouseion.Core/Parser/MusicQualityParser.cs`, `mouseion/src/Mouseion.Core/MediaFiles/Analysis/MusicFileAnalyzer.cs`
- Impact: Confusing duplicate implementations, inconsistent patterns, difficult to maintain quality detection logic
- Fix approach: Consolidate into single DI-injectable instance. Refactor MusicFileAnalyzer to accept DI-injected parser instead of calling static methods (30 min, low risk)

**Nullable Reference Warnings (83 build warnings):**
- Issue: Codebase uses pre-existing CS8600/CS8602/CS8603/CS8604/CS8618/CS8625 nullable reference warnings throughout
- Files: `mouseion/src/Mouseion.Common/` and related, not introduced by recent changes
- Impact: Reduces code safety, makes refactoring harder, hides real null-reference risks
- Fix approach: Enable `<Nullable>enable</Nullable>` with `<WarningsAsErrors>` in `Mouseion.Common.csproj` and systematically fix warnings (estimated 2-3 hours)

**Unused Interfaces (13 dead interfaces):**
- Issue: 13 interfaces compile but have no external consumers
- Files: `IFileSystemLookupService, IProvidePidFile, IMouseionCloudRequestBuilder, IConsoleService, IPlatformInfo, IServiceFactory, IDebounceManager, IRSSFeedParser, IPodcastFileRepository, ISeasonRepository, ISceneMappingService, ITVDBProxy, IOidcStateRepository` in `mouseion/src/Mouseion.Core/`
- Impact: Code bloat, misleading API surface, unclear which abstractions are actually used
- Fix approach: Remove dead interfaces (coordinate with upstream Radarr fork tracking to confirm they won't be needed for planned features)

**Namespace/Folder Mismatches (Intentional):**
- Issue: Deliberate style choice — files organized in folders but using parent namespace for API simplicity
- Files: `mouseion/src/Mouseion.Core/Notifications/Messages/` uses `Mouseion.Core.Notifications` namespace, `mouseion/src/Mouseion.Core/Serializer/System.Text.Json/` and `Serializer/Newtonsoft.Json/`
- Impact: Confusing for new developers, inconsistent with .NET convention (namespace should match folder structure)
- Fix approach: Document this decision in CONTRIBUTING.md or align folders with namespaces when refactoring these areas

## Known Bugs

**Status: All Critical Bugs Resolved**

All 10 identified issues from audit (BUGS.md, 2026-02-22) have been fixed in PRs #205-215:
- 5 controllers missing `[Authorize]` attributes — fixed
- JWT secret regeneration on restart — persisted to `.jwt-secret` in data directory
- DryIoc only registered last IDebridClient — changed to `RegisterMany`
- Null dereference in GetActiveSessionsAsync — guard added
- int.Parse without validation in permissions — validation added
- OPDS/Webhook endpoints without auth — ApiKeyAuthFilter and WebhookSecretFilter added
- Housekeeping tasks referencing non-existent tables — rewritten for unified MediaItems schema
- Admin user not seeded on fresh install — `POST /api/v3/setup` endpoint added
- Webhook secret not discoverable — `GET /api/v3/webhooks/secret` endpoint added

## Security Considerations

**JWT Secret Management:**
- Risk: Secrets regenerated on restart could expose authorization tokens if persisted across instances
- Files: `mouseion/src/Mouseion.Api/Authentication/JwtAuthenticationHandler.cs`, `.jwt-secret` file in data directory
- Current mitigation: Secret persisted to disk in data directory (`.jwt-secret`). Requires filesystem-level access control.
- Recommendations:
  - Document requirement that `.jwt-secret` must not be world-readable (`chmod 600`)
  - Consider supporting external secret storage (environment variable override) for containerized deployments
  - Add startup validation that `.jwt-secret` permissions are correct (fail loud if world-readable)

**Webhook Secret Persistence:**
- Risk: Webhook secrets must be stable across restarts to validate incoming webhooks
- Files: `mouseion/src/Mouseion.Api/Webhooks/` (secret stored in database)
- Current mitigation: Persisted to database, discoverable via admin-only `GET /api/v3/webhooks/secret` endpoint
- Recommendations: Ensure database backups are encrypted; document that webhook secret compromise requires rotation

**OIDC State Storage:**
- Risk: OIDC state values must be temporary and single-use to prevent replay attacks
- Files: `mouseion/src/Mouseion.Core/Authentication/Oidc/OidcProvider.cs`
- Current mitigation: State values stored in database, not found in code comments or docs regarding TTL
- Recommendations: Add explicit TTL enforcement (e.g., 10 minutes) and cleanup job for expired OIDC states; document security model

**Hardcoded Upstream URLs:**
- Risk: `MouseionCloudRequestBuilder.cs` references real upstream APIs (`radarr.servarr.com`, `api.radarr.video`)
- Files: `mouseion/src/Mouseion.Core/Metadata/CloudRequestBuilders/MouseionCloudRequestBuilder.cs`
- Current mitigation: These are legitimate upstream endpoints, not secrets
- Recommendations: These cannot be changed until Mouseion has its own metadata service. Document in ROADMAP.md (spec 05 — competitive-analysis)

## Performance Bottlenecks

**SmartList Deduplication via Linear Scan:**
- Problem: SmartListService deduplicates results using in-memory LINQ without database-level dedup
- Files: `mouseion/src/Mouseion.Core/SmartLists/SmartListService.cs:214`
- Cause: Missing `FindByExternalIdAsync` on IMediaItemRepository prevents efficient dedup query
- Improvement path: Implement batched external ID lookup in repository, use SQL GROUP BY/DISTINCT at fetch time instead of post-processing

**Metadata Caching TTL:**
- Problem: 15-minute cache TTL for metadata responses may be too short for high-traffic deployments
- Files: Services using `IMemoryCache` (15-min default)
- Cause: No configurable TTL; hardcoded across multiple services
- Improvement path: Make TTL configurable via appsettings (separate for different media types — movies faster-changing than books)

**Background Service Exception Handling:**
- Problem: All background housekeeping tasks wrapped in catch-all exception handlers (PR #207) — failures logged but not surfaced
- Files: `mouseion/src/Mouseion.Host/BackgroundServices/`
- Cause: Risk of silent failures (e.g., housekeeping job fails, data inconsistency goes undetected)
- Improvement path: Implement health check endpoint (`/health/housekeeping`) that reports last-run status; add Healthchecks.Net integration

## Fragile Areas

**Test Coverage Gaps — 27 Untested Controllers:**
- Files: `PodcastEpisodes, Facets, LibraryStatistics, WebcomicEpisodes, IndexerHealth, Rename, MovieStatistics, Calendar, MovieImport, BookSeries, Blocklist, ComicIssues, MangaChapters, BookStatistics, MusicScan, SeriesStatistics, MediaFiles, Chapters, AudiobookStatistics, AlbumVersions, AlbumStatistics, MediaSync, Auth, ImportListExclusion, Trakt, MAL, Tracks` (27 controllers in `mouseion/src/Mouseion.Api/Controllers/`)
- Why fragile: Untested API surface risks silent regressions; authentication/authorization bugs possible in Auth controller
- Safe modification: Add test for each controller (spec 02 — test-coverage ongoing). Auth controller is highest priority (critical for security).
- Test coverage: 18 test files added in recent PRs, but 27 controllers remain untested

**Polymorphic MediaItem Hierarchy:**
- Files: `mouseion/src/Mouseion.Core/MediaItems/` — base MediaItem class with subclasses (Movie, Series, Book, Comic, Manga, Podcast, Audiobook, Music)
- Why fragile: Adding new media type requires:
  1. New entity subclass
  2. New repository implementation
  3. New controller endpoint(s)
  4. Database migration
  5. Smart list support updates
  6. Webhook payload updates
  7. Tests for all the above
- Safe modification: Centralize media-type registration in DI (currently scattered); create code-generation template for new types
- Test coverage: All subclasses tested, but integration across polymorphic hierarchy incomplete

**Migration Backward Compatibility:**
- Files: `mouseion/src/Mouseion.Infrastructure/Data/Migrations/` — FluentMigrator numbered migrations
- Why fragile: SQLite has limited ALTER TABLE support; some schema changes required data copying
- Safe modification: Always test migrations against real data dumps before deploying; maintain migration rollback procedures
- Test coverage: Unit tests for migrations (rerun migrations forward/back), but no integration tests against production schema variants

**OPDS Feed Generation:**
- Files: `mouseion/src/Mouseion.Api/Controllers/OPDSController.cs`
- Why fragile: OPDS is complex XML format with strict spec; missing auth was critical bug (now fixed in PR #215)
- Safe modification: Add OPDS feed validation tests (fetch feed, parse XML, validate against OPDS schema)
- Test coverage: OPDSControllerTests added in PR #215, but incomplete

## Scaling Limits

**In-Memory Caching Unbounded Growth:**
- Current capacity: IMemoryCache with no size limit
- Limit: MemoryCache will grow unbounded; high-volume metadata fetches (10k+ items) risk memory exhaustion
- Scaling path: Implement size-limited cache with LRU eviction (MemoryCache options), or switch to Redis for distributed deployments

**SQLite Concurrent Write Limits:**
- Current capacity: Single SQLite database file, WAL mode enabled
- Limit: SQLite WAL mode supports multiple readers, but only one writer at a time; high-concurrency deployments (>50 concurrent users updating media) will see lock contention
- Scaling path: Spec 09 (containerization) should evaluate PostgreSQL for production; keep SQLite for single-user deployments

**Webhook Queue Processing:**
- Current capacity: No queue system; webhooks processed synchronously in controller handler
- Limit: Slow external webhook handlers block API responses
- Scaling path: Implement background queue (e.g., Hangfire) to decouple webhook ingestion from processing

## Dependencies at Risk

**Radarr Fork Maintenance:**
- Risk: Mouseion is forked from upstream Radarr. Many interfaces and patterns inherited without full review.
- Impact: Dead code (13 unused interfaces) may accumulate; upstream bug fixes must be manually merged
- Migration plan: Document which interfaces/patterns are Mouseion-specific vs. upstream legacy; sunset legacy code on schedule (quarterly cleanup)

**Polly Resilience Policy Consistency:**
- Risk: Multiple external metadata providers use different retry/timeout policies
- Impact: Inconsistent behavior; some providers may timeout quickly while others retry aggressively
- Migration plan: Centralize retry policies in named configuration (Polly.Registry), share across all providers

## Missing Critical Features

**Quality Profile Support:**
- Problem: MovieSpecification has hardcoded minimum quality; needs QualityProfiles feature
- Files: `mouseion/src/Mouseion.Core/Movies/Import/Specifications/MinimumQualitySpecification.cs:49` (TODO comment)
- Blocks: Cannot implement user-configurable quality profiles for acquisitions

**albumId Support in Music Tracks:**
- Problem: Track model lacks albumId field; needed for proper album navigation in player
- Files: `akroasis/android/app/src/main/java/app/akroasis/ui/player/PlayerViewModel.kt` (TODO comments)
- Blocks: Android player cannot reliably associate tracks with albums; Web player likely same issue

**Web Audio playbackRate Control:**
- Problem: Podcast player has playback speed control in UI, not wired to Web Audio
- Files: `akroasis/web/src/components/PodcastPlayer.tsx:49` (TODO comment)
- Blocks: Web podcast player cannot adjust playback speed

**Android Auto Artwork Authentication:**
- Problem: AndroidAutoService loads artwork URLs without auth headers; may fail if artwork URLs require authentication
- Files: `akroasis/android/app/src/main/java/app/akroasis/auto/AndroidAutoService.kt:272` (TODO comment)
- Blocks: Protected artwork may not render in Android Auto interface; needs auth header passthrough

**Metadata Service Independence:**
- Problem: Mouseion still references upstream Radarr APIs for metadata (radarr.servarr.com, api.radarr.video)
- Files: `mouseion/src/Mouseion.Core/Metadata/CloudRequestBuilders/MouseionCloudRequestBuilder.cs`
- Blocks: Cannot fully self-host metadata; depends on upstream service availability
- Roadmap: Spec 05 (competitive-analysis) and downstream features require this

---

*Concerns audit: 2026-03-01*
