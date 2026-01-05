# Mouseion

Unified media manager for Movies, Books, Audiobooks, Music, Podcasts, and TV Shows.

Greek: Μουσεῖον (Mouseion) = "temple of the Muses", origin of the Library of Alexandria.

## Overview

Mouseion is a self-hosted media automation tool that:
- Tracks your media library across multiple types
- Monitors for new releases and downloads automatically
- Manages quality profiles per media type
- Integrates with download clients and indexers
- Provides streaming API for client apps (see [Akroasis](https://github.com/forkwright/akroasis))

## Status

**Phase 7 Complete**: All Core Features Implemented (January 2026)

All 7 development phases complete: Foundation, quality system, books/audiobooks, music, movies, TV/podcasts, infrastructure, and file scanning. Archive migration finished. Phase 8 (polish & enhancements) in planning.

This is a ground-up rewrite based on the [Radarr](https://github.com/Radarr/Radarr) architecture. See [NOTICE.md](NOTICE.md) for attribution and [ROADMAP.md](ROADMAP.md) for development timeline and Phase 8 priorities.

## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | .NET 8.0, C# |
| Database | SQLite (default), PostgreSQL (optional) |
| ORM | Dapper |
| Logging | Serilog |
| Telemetry | OpenTelemetry |
| Real-time | SignalR |
| API | REST (v3), OpenAPI/Swagger |

## Project Structure

```
mouseion/
├── src/
│   ├── Mouseion.Common/     # Shared utilities, DI, HTTP client
│   ├── Mouseion.Core/       # Business logic, entities, services
│   ├── Mouseion.Api/        # REST API, controllers, middleware
│   ├── Mouseion.SignalR/    # Real-time messaging
│   └── Mouseion.Host/       # Application entry point
├── tests/                   # Unit and integration tests
├── LICENSE                  # GPL-3.0
├── NOTICE.md               # Radarr attribution
└── Mouseion.sln
```

**Note**: This is the backend API server. Frontend clients are developed separately (see [Akroasis](https://github.com/forkwright/akroasis) for reference client).

## Media Types

| Type | Status | Hierarchy |
|------|--------|-----------|
| Movies | ✅ Complete | Movie |
| Books | ✅ Complete | Author → Series? → Book |
| Audiobooks | ✅ Complete | Author → Series? → Audiobook |
| Music | ✅ Complete | Artist → Album → Track |
| Podcasts | ✅ Complete | Show → Episode |
| TV Shows | ✅ Complete | Series → Season → Episode |

## Building

```bash
dotnet build Mouseion.sln
```

## Running

```bash
dotnet run --project src/Mouseion.Host
```

Default port: 7878

## API

Base URL: `http://localhost:7878/api/v3/`

Authentication: API key via `X-Api-Key` header

Documentation: `/swagger` (OpenAPI/Swagger UI)

**Key endpoints**:
- `/search` - Full-text search with audio quality metadata
- `/library/filter` - Complex filtering (16 fields, AND/OR logic)
- `/library/facets` - Autocomplete values for filters
- `/tracks/batch` - Batch track info queries
- `/playlists/smart` - Auto-refreshing smart playlists
- `/stream/{id}` - Media file streaming (HTTP 206, range requests)
- `/chapters/{id}` - Chapter markers (M4B, MP3, cue sheets)
- `/tracks/{id}/audio-analysis` - Audio quality analysis (DR, ReplayGain, bit depth)
- `/albums/{id}/versions` - Album editions/remasters
- `/history` - Media event history

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## Support

This project is free and always will be.

Support development: [GitHub Sponsors](https://github.com/sponsors/forkwright)

## License

GPL-3.0 - See [LICENSE](LICENSE)

This project is a derivative work of Radarr. See [NOTICE.md](NOTICE.md) for details.
