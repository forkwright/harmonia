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

**Phase 0**: Fresh repository setup (December 2025)

This is a ground-up rewrite based on the [Radarr](https://github.com/Radarr/Radarr) architecture. See [NOTICE.md](NOTICE.md) for attribution.

## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | .NET 8.0, C# |
| Database | SQLite (default), PostgreSQL (optional) |
| ORM | Dapper |
| Logging | Serilog |
| Telemetry | OpenTelemetry |
| Real-time | SignalR |
| Frontend | React 19, TypeScript, TanStack Query, Zustand, Vite |
| Styling | Tailwind CSS |

## Project Structure

```
mouseion/
├── src/
│   ├── Mouseion.Common/     # Shared utilities, DI, HTTP client
│   ├── Mouseion.Core/       # Business logic, entities, services
│   ├── Mouseion.Api/        # REST API, controllers, middleware
│   ├── Mouseion.SignalR/    # Real-time messaging
│   └── Mouseion.Host/       # Application entry point
├── frontend/                # React frontend
├── LICENSE                  # GPL-3.0
├── NOTICE.md               # Radarr attribution
└── Mouseion.sln
```

## Media Types

| Type | Status | Hierarchy |
|------|--------|-----------|
| Movies | Planned | Movie |
| Books | Planned | Author → Series? → Book |
| Audiobooks | Planned | Author → Series? → Audiobook |
| Music | Planned | Artist → Album → Track |
| Podcasts | Planned | Show → Episode |
| TV Shows | Planned | Series → Season → Episode |

## Building

```bash
# Backend
dotnet build Mouseion.sln

# Frontend (when ready)
cd frontend && npm install && npm run dev
```

## Running

```bash
dotnet run --project src/Mouseion.Host
```

Default port: 7878

## API

Base URL: `http://localhost:7878/api/v3/`

Authentication: API key via `X-Api-Key` header

Key endpoints:
- `/media` - Unified media listing
- `/stream/{id}` - Media file streaming (HTTP 206)
- `/progress/{type}/{id}` - Playback progress
- `/history/listen` - Listening history

## License

GPL-3.0 - See [LICENSE](LICENSE)

This project is a derivative work of Radarr. See [NOTICE.md](NOTICE.md) for details.
