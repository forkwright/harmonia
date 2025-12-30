# Attribution Notice

Mouseion is a derivative work based on Radarr.

## Original Project

- **Name**: Radarr
- **Repository**: https://github.com/Radarr/Radarr
- **License**: GPL-3.0
- **Copyright**: 2010-2025 Radarr Contributors

## Derivative Work

- **Name**: Mouseion
- **Repository**: https://github.com/forkwright/mouseion
- **License**: GPL-3.0
- **Copyright**: 2024-2025 Mouseion Contributors

## What Was Derived

This project incorporates code and patterns from Radarr, including:

- Core infrastructure (HTTP client, configuration, dependency injection)
- Data access layer (Dapper-based repository pattern)
- Download client integrations (16 clients: QBittorrent, Transmission, Sabnzbd, etc.)
- Indexer framework (Newznab, Torznab, and private tracker implementations)
- Notification system (26 providers: Discord, Telegram, Plex, etc.)
- API patterns and middleware (authentication, validation, REST conventions)
- SignalR real-time messaging infrastructure
- Quality profile and custom format systems
- Scheduler and task management

## Modifications

Major modifications from the original Radarr codebase:

- **Unified media types**: Extended beyond movies to support Books, Audiobooks, Music, Podcasts, and TV Shows
- **Namespace migration**: All namespaces changed from `NzbDrone.*`/`Radarr.*` to `Mouseion.*`
- **Fresh database schema**: New schema designed for multi-media support (not backward compatible with Radarr)
- **Frontend modernization**: Complete rewrite using React 19, TanStack Query, Zustand, and Vite
- **Observability**: Added Serilog and OpenTelemetry (replacing NLog)
- **Streaming API**: Added HTTP 206 range request support for audio/video streaming

## Acknowledgments

We thank the Radarr team and all contributors for creating and maintaining the excellent foundation this project builds upon.

The *arr ecosystem (Sonarr, Radarr, Lidarr, Readarr) has pioneered self-hosted media automation, and Mouseion aims to continue that tradition with a unified approach.
