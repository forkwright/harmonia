# Changelog

All notable changes to Mouseion will be documented in this file.

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
