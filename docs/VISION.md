# Vision

## What Harmonia Is

Harmonia is a unified, self-hosted media operations platform — a single Rust binary that replaces the entire *arr ecosystem, torrent client, indexers, and media servers. It manages, downloads, organizes, and serves all media types: music, audiobooks, ebooks, podcasts, manga, news, movies, and TV shows. Video playback stays with Plex; everything else plays through Harmonia's own clients (see [GLOSSARY.md](GLOSSARY.md) for platform name definitions).

Music and audiobooks are the priority. They carry the highest quality bar — bit-perfect playback, proper metadata, gapless transitions, ReplayGain — where the *arr tools have historically made compromises. Every other media type is in scope, but music and audiobooks define the quality floor.

The system owns the full media lifecycle: discovery, search, download, extraction, import, organization, metadata enrichment, serving, and playback. No coordination overhead between a dozen specialized tools. No drift between what one tool thinks the library looks like and what another tool sees. One process, one consistent state.

## What It Replaces

The tools below are absorbed into Harmonia. Each one's function is carried natively — not proxied, not wrapped.

### Acquisition

| Tool | Function | Harmonia Equivalent |
|------|----------|---------------------|
| Sonarr | TV lifecycle (monitor, search, download, organize) | Media lifecycle management |
| Radarr | Movie lifecycle | Media lifecycle management |
| Lidarr | Music lifecycle | Media lifecycle management |
| Readarr | Book and audiobook lifecycle | Media lifecycle management |
| Prowlarr / Jackett | Indexer aggregation, Torznab/Newznab proxy | Indexer protocol (built-in Torznab/Newznab) |
| qBittorrent | Torrent downloads | Download engine |
| Unpackerr | Archive extraction after download | Download engine |
| FlareSolverr (and successors) | Cloudflare bypass for private indexers | Indexer access layer |
| Pulsarr / Autopulse | Download orchestration, post-import triggers | Download orchestration |

### Metadata and Organization

| Tool | Function | Harmonia Equivalent |
|------|----------|---------------------|
| Audiobookshelf | Audiobook serving and playback | Media serving + Akroasis |
| Kavita | Book and comic serving, reading | Media serving + Akroasis |
| Bazarr | Subtitle management | Media management |
| Tdarr | Media transcoding | Media processing |

### Library Maintenance

| Tool | Function | Harmonia Equivalent |
|------|----------|---------------------|
| Cleanarr / Decluttarr | Download queue hygiene | Download engine |
| Checkrr | File integrity verification | Library maintenance |
| Maintainerr | Auto-cleanup rules | Library maintenance |
| Excludarr / Labelarr | Monitoring rules, tagging | Library curation |
| Quasarr | Quality assessment and upgrades | Quality management |
| Kometa | Plex collection management | Plex integration |
| Wrapperr | Plex viewing statistics | Plex integration |

### Requests

| Tool | Function | Harmonia Equivalent |
|------|----------|---------------------|
| Overseerr | Media request workflow for household members | Request management |

## What We Integrate (API, Not Replace)

These services are connected via API — Harmonia uses them, not replaces them.

| Service | Purpose |
|---------|---------|
| Last.fm | Scrobbling, music discovery, artist metadata |
| Tidal | Discovery engine, want-list sync for unowned music |
| Plex | Notify on new media, collection management, viewing stats |

## What's Out of Scope

- **Video playback** — Plex handles this; Harmonia manages video files only
- **Streaming-first architecture** — self-hosted library is the source of truth, not a cache
- **Cloud dependencies** — no SaaS, no cloud storage, runs entirely on user hardware
- **Multi-tenant deployment** — single user or household; Overseerr-style requests are household members, not general users
- **Aletheia integration** — entirely separate project with no planned coupling

## Design Principles

- **Rust-native** — Rust for the backend, always, unless research proves otherwise for a specific component
- **Quality over pragmatism** — optimal solution for every domain; no shortcuts that create technical debt
- **Greek-named subsystems** — gnomon-style naming (see [gnomon.md](gnomon.md)); clean domain boundaries, separate directories
- **Nothing sacred** — existing tooling, features, and setup are all subject to revision
- **Single binary** — one process replaces the multi-app coordination overhead of the current setup
- **All media types** — music and audiobooks are priority, but every media type is in scope
- **Full lifecycle** — discovery through playback; no handoff gaps between external tools
- **Movies and TV managed, not played** — full Sonarr/Radarr replacement; Plex handles viewing
- **Parameterize, don't duplicate** — single source of truth for all config values; nothing updated in two places
- **Consolidated config** — one subsystem for all configuration and private data; simplifies `.gitignore`
- **Hierarchical structure** — applies to docs, code organization, and configuration

## Architecture Decisions

### D3: Mobile — Native Android + UniFFI

**Decision:** Tauri Mobile rejected. Mobile app is native Android (Kotlin + Jetpack Compose)
with Rust audio core exposed via UniFFI/JNI.

**Rationale (R1 research):**
- Background playback, lock screen controls, and audio focus have no Tauri Mobile solution
- Zero confirmed Tauri Mobile music apps exist anywhere
- Custom native app is required regardless — covers all 8 media types (audiobooks, podcasts,
  manga, ebooks, etc.), not just music
- Existing akroasis codebase has 29K lines of Kotlin validating this direction
- Same Rust audio core (akroasis-core), different delivery mechanism

**Secondary:** OpenSubsonic API (R11, ~25 endpoints) provides Symfonium/Ultrasonic
compatibility for music-only mobile clients as a bonus, not the primary strategy.
