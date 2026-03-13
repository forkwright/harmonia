# Mouseion

> Μουσεῖον (Mouseion): "temple of the Muses", origin of the Library of Alexandria

Unified self-hosted media manager. Movies, books, audiobooks, music, podcasts, TV shows, manga, comics, news feeds: one application replacing the entire *arr ecosystem.

## Architecture

```
mouseion/
├── src/
│   ├── Mouseion.Common/     # Shared utilities, DI, HTTP
│   ├── Mouseion.Core/       # Business logic, entities, services
│   ├── Mouseion.Api/        # REST API, controllers, middleware
│   ├── Mouseion.SignalR/    # Real-time messaging
│   └── Mouseion.Host/       # Application entry point
├── tests/                   # Unit and integration tests
├── specs/                   # Development specifications
└── Mouseion.sln
```

## Stack

| Component | Technology |
|-----------|------------|
| Runtime | .NET 10, C# |
| Database | SQLite (default), PostgreSQL (optional) |
| ORM | Dapper |
| Logging | Serilog + OpenTelemetry |
| Real-time | SignalR |
| API | REST v3, OpenAPI at `/swagger` |

## Running

```bash
dotnet build Mouseion.sln
dotnet run --project src/Mouseion.Host
```

Default port: 7878. Auth: API key via `X-Api-Key` header.

## Media types

Movies, Books, Audiobooks, Music, Podcasts, TV Shows, Manga, Webcomics, Comics, News/RSS: all with full CRUD APIs, quality profiles, metadata providers, and monitoring.

## Client

[Akroasis](https://github.com/forkwright/akroasis): unified player for music, audiobooks, and podcasts.

## License

GPL-3.0, derivative of [Radarr](https://github.com/Radarr/Radarr). See [NOTICE.md](NOTICE.md).
