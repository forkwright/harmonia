# Spec 05: Competitive Analysis

**Status:** Active
**Priority:** Medium

## Goal

Map the competitive landscape of unified self-hosted media managers to validate Mouseion's positioning, identify feature gaps worth closing, and surface architectural patterns worth adopting. The *arr unification trend is accelerating — three major projects launched in the past year — and Mouseion needs to understand where it leads and where it can learn.

## Landscape Overview

| Project | Category | Stack | Stars | Media Types | Activity |
|---------|----------|-------|-------|-------------|----------|
| **Cinephage** | Acquisition | TS/Svelte/Node | 611 | 3 (movies, TV, IPTV) | Very active, 3 months old |
| **MediaManager** | Acquisition | Python/FastAPI/Svelte | 3,099 | 2 (movies, TV) | Mature, v1.12.3 |
| **Nefarious** | Acquisition | Python/Django/Angular | 1,221 | 2 (movies, TV) | Maintenance mode |
| **Reiverr** | Discovery + Streaming | TS/Svelte/NestJS | 2,106 | 2 (movies, TV) | Active, v2 rewrite |
| **Huntarr** | Companion → Platform | Python/Flask | 3,154 | 5 (via *arr) | Very active, expanding |
| **Yamtrack** | Tracking | Python/Django | 2,102 | 8 types | Active, stable |
| **Stump** | Reading Server | Rust/Axum/React | 1,935 | 3 (comics, manga, ebooks) | Active, pre-v1.0 |
| **Mouseion** | Unified management | C#/.NET 10 | — | 10+ | In development |

## Competitor Profiles

### Cinephage

**Repo:** github.com/MoldyTaint/Cinephage | **License:** GPL-3.0

**Architecture:** Monolithic Node.js + Svelte 5 frontend. Single unified database. 474 commits, 9 contributors. Solo primary developer.

**Key features:**
- Quality scoring with 50+ factors sourced from Dictionarry (external quality DB, 100+ format attributes)
- `.strm` file generation — plays via Jellyfin/Emby/Kodi without downloading
- Usenet NZB streaming with adaptive prefetching and segment caching
- Built-in Cloudflare bypass via Camoufox (C++-level fingerprint spoofing, no FlareSolverr needed)
- 10 streaming providers with circuit breaker failover
- Live TV/IPTV with Stalker portal, EPG, HLS
- Built-in indexers + Newznab + custom YAML definitions
- Smart Lists (IMDb/Trakt/TMDb auto-add)
- 4 built-in quality profiles: Best, Efficient, Micro, Streaming

**Strengths:** Most feature-dense per media type. Streaming paradigm is genuinely novel. Built-in Cloudflare bypass eliminates a container. Quality scoring depth exceeds TRaSH Guides tooling.

**Weaknesses:** Only 3 media types. Solo developer (bus factor = 1). No authentication. Node.js single-threaded performance ceiling. Custom quality profiles marked unstable. Legal grey area (Stalker portal MAC discovery).

**What Mouseion learns:**
- `.strm` file generation as first-class feature for movies/TV — browse and play without downloading
- Dictionarry-style external quality database reduces maintenance vs hand-curated definitions
- Delay profiles (wait for better quality releases) are high-value for quality-conscious users
- Smart Lists (dynamic queries against external sources with auto-add) generalizable across all media types

---

### MediaManager

**Repo:** github.com/maxdorninger/MediaManager | **License:** AGPL-3.0

**Architecture:** Python backend (FastAPI, Alembic/SQLAlchemy), Svelte frontend. Single container since v1.12.0. 1,154 commits, 26+ contributors. DigitalOcean sponsored.

**Key features:**
- Full OIDC/OAuth 2.0 (Google, Azure AD, Keycloak, any provider)
- Multi-language metadata support
- Prowlarr + Jackett + direct indexer integration
- qBittorrent with category support
- TMDB + TVDB metadata
- Adult content filtering
- Monthly release cadence (v1.10 → v1.12 in 3 months)

**Strengths:** Largest community (3k stars in one year). Best auth story (OIDC). Regular releases. DigitalOcean sponsorship. Growing contributor base (7 new in v1.12.2).

**Weaknesses:** Only 2 media types. No quality scoring. No streaming. No subtitle management. No built-in indexers. No real-time push (no SignalR/WebSocket). qBittorrent only.

**What Mouseion learns:**
- OIDC/OAuth 2.0 is table stakes for multi-user households and shared servers
- Multi-language metadata is essential for a global media types (manga, music, international film)
- Single-container deployment eliminates CORS/reverse proxy pain — ship as one container from day one
- ID-based indexer searching (IMDB/TMDB/TVDB IDs instead of title strings) improves accuracy

---

### Nefarious

**Repo:** github.com/lardbit/nefarious | **License:** GPL-3.0

**Architecture:** Python/Django + Angular + Celery task queue. Bundled Jackett + Transmission in Docker Compose. 954 commits, 11 contributors. 7 years old, effectively maintenance mode.

**Key features:**
- Movie + TV discovery via TMDB
- Quality profiles (resolution filtering)
- Subtitle auto-download via OpenSubtitles
- Spam/fake content detection with blacklisting
- Stuck torrent identification with auto-blacklist
- VPN integration as first-class concern
- Multi-user with basic permission tiers

**Strengths:** Oldest unified attempt still alive. VPN integration. Fake content detection.

**Weaknesses:** 2 media types after 7 years. Solo maintainer. Hardcoded Transmission dependency (no download client abstraction). Django + Angular contribution barrier. No API docs, no plugin system, no webhooks.

**What Mouseion learns (cautionary tale):**
- Expand media types early or die — a 2-type manager will always lose to specialized tools
- Abstract integration points (download clients, indexers) or they become architecture debt
- Fake content detection and blacklisting is genuinely useful and often overlooked
- VPN-awareness matters for self-hosters

---

### Reiverr

**Repo:** github.com/aleksilassila/reiverr | **License:** AGPL-3.0

**Architecture:** NestJS backend, Svelte frontend, TypeORM/SQLite. Plugin system via npm packages. v2.0 ground-up rewrite. 2,106 stars.

**Key features:**
- Plugin architecture: `PluginProvider` → `SourceProvider` abstraction
- Built-in plugins: Jellyfin (library streaming), Torrent-Stream (via Jackett)
- TV-first 10-foot UI with custom focus management (remote/keyboard navigation)
- Stack-based routing (TV UX pattern, not URL-based SPA)
- Decoupled metadata (TMDB) from content delivery (plugins)
- Independent playback tracking
- Multi-user support

**Plugin pattern (most architecturally interesting):**
```
PluginProvider (abstract)
  └── SourceProvider
      ├── getMovieCatalogue() / getEpisodeCatalogue()
      ├── getMovieStreams() / getEpisodeStreams()
      ├── proxyHandler()
      └── settingsManager
```
Plugins discovered by `.plugin` directory extension. No config file needed.

**Strengths:** Cleanest architecture. Genuine plugin system with proper abstractions. TV-first UI is unique. Torrent streaming compelling.

**Weaknesses:** Only 2 media types. v2 missing v1 features. Plugin scope too narrow (streaming only, no management). Plugin ecosystem is effectively zero. Security caveats acknowledged.

**What Mouseion learns:**
- The plugin pattern is right but scope is wrong — extend to: metadata providers, download clients, notification channels, import sources, media type definitions
- Decouple metadata from content delivery — TMDB-for-discovery / plugins-for-streaming split is elegant
- TV/10-foot UIs are an underserved niche worth considering for Akroasis
- npm-package-as-plugin (installable, versionable, own dependencies) is good DX

---

### Huntarr

**Repo:** github.com/plexguide/Huntarr.io | **License:** GPL-3.0

**Architecture:** Python/Flask with blueprint routes, multi-threaded orchestrator (`ThreadPoolExecutor`), SQLite/WAL. 3,653 commits, 266 releases. Port 9705.

**Key features:**
- Retroactive library completion (walks library gaps, triggers *arr searches in batches)
- Rate limiting: hourly caps per instance (default 20/hr, max 400)
- Stateful deduplication (prevents re-searching already-processed items)
- Media Hunt: unified request UI (TMDB discovery)
- NZB Hunt: built-in Usenet downloader (120-thread NNTP, own child process)
- Requestarr: request management
- Multi-user auth with 2FA + Plex OAuth
- Auto-restart for crashed threads, 15s health checks
- Swaparr queue monitoring

**Strengths:** Solves a universal pain point (library gaps). Excellent rate limiting and API management. Explosive growth (3k stars in 11 months). Relentless release cadence. NZB Hunt is bold.

**Weaknesses:** Still fundamentally dependent on *arr APIs (orchestration layer, not replacement). Flask won't scale as features grow. No media library management. No plugin system. UI is functional but not polished.

**What Mouseion learns:**
- "Obvious gap" products grow fastest — retroactive library completion was universally wanted
- Rate limiting and indexer-friendly behavior are first-class features, not afterthoughts
- Hourly caps, queue-aware pausing, stateful deduplication prevent being banned by indexers
- Unraid/TrueNAS/Umbrel app stores are distribution channels — target early
- Thread-per-app-type is simple effective concurrency for managing multiple backends

---

### Yamtrack

**Repo:** github.com/FuzzyGrim/Yamtrack | **License:** AGPL-3.0

**Architecture:** Python/Django, Celery/Redis, SQLite or PostgreSQL. TailwindCSS frontend. 2,503 commits, 28 contributors. v0.25.0.

**Key features (widest media type coverage):**
- 8 types: movies, TV, anime, manga, video games, books, comics, board games
- Dual-entity data model: **Item** (shared metadata) + **Media** (per-user tracking)
- Abstract base class polymorphism (Media → BasicMedia, Movie, TV, Game, etc.)
- TV three-level hierarchy: Show → Season → Episode with aggregated computed properties
- Centralized `media_type_config.py` defines behavior per type
- Import from: Trakt, Simkl, MyAnimeList, AniList, Kitsu (with periodic auto-import)
- Media server auto-tracking: Jellyfin, Plex, Emby
- Calendar with iCalendar (.ics) subscription
- OIDC + 100+ social providers via django-allauth
- Collaborative lists with member management
- Per-episode tracking, rewatch counts, notes
- django `simple-history` for all field changes

**Metadata sources:** TMDB, MyAnimeList, IGDB, Steam, BoardGameGeek, OpenLibrary, AniList, MangaDex.

**Strengths:** Most media types (8). Cleanest multi-type data model. Excellent import ecosystem. Calendar with iCal. Active development. OIDC + social auth.

**Weaknesses:** Tracking only — no downloading, no file management, no content serving. Django + SQLite scaling limits. No plugin system. CSV-only export.

**What Mouseion learns (study this data model closely):**
- Item/Media split is the correct abstraction — shared metadata vs per-user tracking
- Abstract base class polymorphism scales to 8+ types without complex table inheritance
- TV must be Show → Season → Episode with aggregated computed properties at Show level
- `media_type_config` centralized registry makes adding types systematic
- Import from existing trackers (Trakt, MAL, AniList, Simkl) is migration table stakes
- Media server auto-tracking (Jellyfin/Plex/Emby marks watched content) removes friction
- Calendar + iCal subscription is high value, low effort

---

### Stump

**Repo:** github.com/stumpapp/stump | **License:** MIT

**Architecture:** Rust/Axum backend, React/TypeScript frontend, Tauri desktop, React Native mobile. Prisma ORM. Monorepo with Cargo + Yarn workspaces. 752 commits, 27 contributors.

**Key features:**
- OPDS 1.2 (full) + OPDS 2.0 (experimental)
- Format support: EPUB, PDF, CBZ, CBR
- Folder-based library organization (directory = series)
- Granular access control with managed user accounts
- Web reader built-in
- Documented REST API
- Reading lists and collections
- API key auth in OPDS URLs for headerless clients
- Tauri desktop app (lightweight vs Electron)

**Architecture note:** Prisma in Rust is unconventional (most Rust projects use Diesel/SeaORM). Provides migration management and type safety at cost of extra runtime dependency. Shared types generated across Rust and TypeScript boundaries.

**Strengths:** Rust performance targeting RPi-class hardware. Most complete OPDS implementation. Clean monorepo with cross-language type generation. Tauri desktop. Active contributors (27).

**Weaknesses:** Pre-v1.0 with explicit stability warning. Only 3 media types. No metadata fetching yet (post-v1.0). No acquisition. Niche market (comics/manga readers).

**What Mouseion learns:**
- OPDS 1.2 support is essential for any book/comic media type — e-reader integration
- Folder-based library scanning (directory = series) is simple, predictable, worth supporting alongside DB-driven organization
- Monorepo with cross-language type generation (Rust→TS, adaptable to C#→TS) reduces type drift
- Tauri for companion desktop app is the right call over Electron

## Feature Matrix

| Capability | Cinephage | MediaManager | Nefarious | Reiverr | Huntarr | Yamtrack | Stump | **Mouseion** |
|------------|-----------|--------------|-----------|---------|---------|----------|-------|--------------|
| Media types | 3 | 2 | 2 | 2 | 5 (via *arr) | 8 | 3 | **10+** |
| Acquisition | Yes | Yes | Yes | Stream | Via *arr | No | No | **Yes** |
| File management | Basic | Basic | No | No | No | No | Yes | **Yes** |
| Tracking | No | No | No | Basic | No | **Core** | Basic | **Yes** |
| Content serving | Stream | No | No | Stream | No | No | Reader | **Planned** |
| Quality scoring | **50+ factors** | None | Resolution | No | Via *arr | No | No | **103 defs** |
| Streaming (.strm) | **Yes** | No | No | Torrent | No | No | No | No |
| Subtitles | **6 providers** | No | OpenSubs | No | No | No | No | Yes |
| OIDC/OAuth | No | **Yes** | No | No | 2FA+Plex | **Yes** | No | API key |
| Multi-user | No | **Yes** | Basic | Yes | Yes | **Yes** | Yes | No |
| Plugin system | No | No | No | **Yes** | No | No | No | No |
| Real-time push | Workers | No | No | No | No | No | No | **SignalR** |
| Cloudflare bypass | **Built-in** | No | No | No | No | No | No | No |
| Live TV/IPTV | **Yes** | No | No | No | No | No | No | No |
| OPDS | No | No | No | No | No | No | **Yes** | No |
| Import (Trakt/MAL) | No | No | No | No | No | **Yes** | No | No |
| Calendar/iCal | No | No | No | No | No | **Yes** | No | No |
| Media server sync | No | No | No | Jellyfin | No | **3 servers** | No | No |
| Rate limiting | No | No | No | No | **Yes** | No | No | No |
| Metadata sources | 1 | 2 | 1 | 1 | Via *arr | **8** | 0 | **8+** |
| File scanning | Watch | Basic | No | No | No | No | Folder | **TagLib+spectral** |
| Companion app | No | No | No | No | No | No | Tauri+mobile | **Akroasis** |

## Strategic Findings

### Mouseion's Moat

**Nobody covers all media types natively.** The widest acquisition-focused project (Huntarr) covers 5 types by delegating to existing *arr APIs. Yamtrack covers 8 types but tracking-only. Mouseion's 10+ type single-codebase with native acquisition is unprecedented.

**Full lifecycle management.** Discovery → acquisition → organization → tracking → serving. No competitor covers all phases. Most handle 1-2. Mouseion + Akroasis can own the complete pipeline.

**Architecture advantage.** C#/.NET 10 with Dapper and SignalR outperforms every competitor's stack at scale. The field is overwhelmingly Python + TypeScript. The only comparable performance choice (Stump's Rust) is niche.

**Metadata provider diversity.** 8+ providers (TMDB, TVDB, MusicBrainz, AcoustID, OpenLibrary, Audnexus, MangaDex, AniList, ComicVine) vs most competitors at 1-2.

### Market Gaps Mouseion Should Fill

1. **OIDC/OAuth authentication** — MediaManager and Yamtrack have it; Mouseion doesn't. For multi-user households this is a blocker.
2. **Import from existing trackers** — Yamtrack imports from 5 sources. Users won't migrate without their history.
3. **Media server auto-tracking** — Yamtrack's Jellyfin/Plex/Emby integration auto-marks watched content. Reduces friction to zero.
4. **OPDS support** — Stump proves this matters for books/comics. Any reading media needs OPDS 1.2 at minimum.
5. **Calendar with iCal** — Yamtrack shows high value, low implementation cost. Track upcoming releases across all media types.
6. **Rate-limited acquisition** — Huntarr's hourly caps and stateful deduplication prevent indexer bans. Essential for responsible automated searching.

### Patterns to Avoid

1. **Nefarious's trap:** Starting with 2 types and never expanding. (Mouseion already avoids this.)
2. **Reiverr's trap:** Ground-up v2 rewrite that loses feature parity for months.
3. **Scope creep without architecture:** Huntarr grows features faster than Flask can support. Clear module boundaries matter.
4. **Eternal pre-v1.0:** Stump has been WIP for years. Ship a usable subset early.
5. **Hardcoded integrations:** Nefarious locked to Transmission. Abstract download clients and indexers.

## Phases

### Phase 1: Adopt competitive patterns
- [ ] Evaluate OIDC/OAuth for multi-user auth (reference MediaManager's implementation)
- [ ] Design import pipeline for Trakt, MAL, AniList tracking data
- [ ] Add OPDS 1.2 endpoints for book/audiobook/comic media types
- [ ] Add calendar API with iCal (.ics) subscription for release monitoring
- [ ] Implement rate limiting per indexer (hourly caps, stateful dedup — Huntarr pattern)

### Phase 2: Differentiation features
- [ ] `.strm` file generation for movies/TV (Cinephage pattern — diskless streaming via Jellyfin/Emby)
- [ ] Delay profiles (configurable wait for higher quality releases before downloading)
- [ ] Smart Lists (dynamic queries against TMDB/Trakt/MusicBrainz with auto-add to library)
- [ ] Media server auto-tracking integration (Jellyfin webhook → mark watched in Mouseion)

### Phase 3: Architecture investments
- [ ] Plugin/extension system for media type definitions (NuGet packages implementing `IMediaTypeProvider`)
- [ ] Download client abstraction layer (qBit, Transmission, SABnzbd, NZBGet via adapter pattern)
- [ ] Cross-language type generation (C# → TypeScript) for Akroasis type safety
- [ ] Unraid/TrueNAS/Umbrel app store packaging for distribution

## Dependencies

- OIDC evaluation may generate its own spec
- OPDS implementation depends on Spec 01 (Akroasis integration) for serving architecture
- Import pipeline depends on external tracker API stability (Trakt, MAL rate limits)

## Notes

- Research conducted Feb 2026. Star counts and activity levels will shift.
- Cinephage and Huntarr are growing fastest — re-evaluate in 3 months.
- MediaManager's AGPL-3.0 license means studying their OIDC implementation is fine but direct code reuse requires matching license terms.
- Yamtrack's data model (Item/Media split, abstract base polymorphism) aligns with Mouseion's existing polymorphic MediaItem pattern — validate that the patterns are compatible.
- The unification trend is real: 3 major projects launched in the past year. Community demand for *arr consolidation is high. Mouseion's timing is right.
