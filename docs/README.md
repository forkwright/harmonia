# Harmonia Documentation

> Start here. All Harmonia docs are linked below, including planned docs for upcoming phases.

## Foundation

- [VISION.md](VISION.md) — What Harmonia is, what it replaces, and why
- [GLOSSARY.md](GLOSSARY.md) — Subsystem names, shared paths, and values registry
- [gnomon.md](gnomon.md) — Greek naming system and layer test
- [STANDARDS.md](STANDARDS.md) — Code standards for all languages
- [CLAUDE_CODE.md](CLAUDE_CODE.md) — Claude Code dispatch protocol
- [LESSONS.md](LESSONS.md) — Operational rules from real failures
- [WORKING-AGREEMENT.md](WORKING-AGREEMENT.md) — Syn + Cody collaboration protocol
- [PROJECT.md](PROJECT.md) — Project definition and milestone context

## Naming

- [naming/registry.md](naming/registry.md) — All subsystem names with layer test tables
- [naming/topology.md](naming/topology.md) — Subsystem name relationships and hierarchy

## Architecture

- [architecture/binary-modes.md](architecture/binary-modes.md) — Binary execution modes: serve, desktop, render, play
- [architecture/subsystems.md](architecture/subsystems.md) — Subsystem map, domain boundaries, dependency graph
- [architecture/communication.md](architecture/communication.md) — Event bus and internal messaging patterns
- [architecture/configuration.md](architecture/configuration.md) — Configuration architecture and merge strategy
- [architecture/errors.md](architecture/errors.md) — Error handling strategy across subsystem boundaries
- [architecture/cargo.md](architecture/cargo.md) — Cargo workspace layout and crate map
- [architecture/auth.md](architecture/auth.md) — Authentication architecture

## Data

- [data/media-schemas.md](data/media-schemas.md) — Per-media-type table schemas
- [data/want-release.md](data/want-release.md) — Want vs Release concept design
- [data/quality-profiles.md](data/quality-profiles.md) — Quality profile system
- [data/storage.md](data/storage.md) — SQLite WAL architecture and migration strategy
- [data/entity-registry.md](data/entity-registry.md) — Shared entity and UUID cross-reference design

## Download & Acquisition

- [download/torrent.md](download/torrent.md) — librqbit integration architecture
- [download/indexer-protocol.md](download/indexer-protocol.md) — Torznab/Newznab direct implementation
- [download/orchestration.md](download/orchestration.md) — Queue, post-processing, import pipeline
- [download/cloudflare.md](download/cloudflare.md) — Cloudflare bypass architecture
- [download/archive.md](download/archive.md) — Archive extraction pipeline
- [download/usenet.md](download/usenet.md) — Usenet feasibility and approach
- [download/irc.md](download/irc.md) — IRC announce integration

## Media Lifecycle & Metadata

- [media/lifecycle.md](media/lifecycle.md) — Per-type lifecycle state machines
- [media/metadata-providers.md](media/metadata-providers.md) — Provider strategy and rate limiting
- [media/scanner.md](media/scanner.md) — Library scanner design
- [media/import-rename.md](media/import-rename.md) — Import and rename pipeline
- [media/music.md](media/music.md) — Music-specific design: MusicBrainz, ReplayGain
- [media/audiobooks.md](media/audiobooks.md) — Audiobook-specific design: M4B, chapters, position
- [media/subtitles.md](media/subtitles.md) — Subtitle management

## Serving & Integrations

- [serving/streaming.md](serving/streaming.md) — HTTP media streaming and transcoding *(planned — Phase 7)*
- [serving/opds.md](serving/opds.md) — OPDS 2.0 feed design *(planned — Phase 7)*
- [serving/plex.md](serving/plex.md) — Plex integration design *(planned — Phase 7)*
- [serving/lastfm.md](serving/lastfm.md) — Last.fm scrobbling and discovery *(planned — Phase 7)*
- [serving/tidal.md](serving/tidal.md) — Tidal discovery integration *(planned — Phase 7)*
- [serving/transcoding.md](serving/transcoding.md) — FFmpeg transcoding pipeline *(planned — Phase 7)*
- [serving/requests.md](serving/requests.md) — Household request management *(planned — Phase 7)*

## Tech Stack & Roadmap

- [roadmap/tech-stack.md](roadmap/tech-stack.md) — Crate selections, versions, rationale *(planned — Phase 8)*
- [roadmap/feature-flags.md](roadmap/feature-flags.md) — Compile-time optional capabilities *(planned — Phase 8)*
- [roadmap/implementation.md](roadmap/implementation.md) — Code milestone ordering and dependencies *(planned — Phase 8)*
- [roadmap/mvp.md](roadmap/mvp.md) — Minimum vertical slice definition *(planned — Phase 8)*
- [roadmap/migration.md](roadmap/migration.md) — Migration from C# backend *(planned — Phase 8)*

## Policy

- [policy/agent-contribution.md](policy/agent-contribution.md) — Agent PR and commit rules
- [policy/git-history.md](policy/git-history.md) — Git history conventions
- [policy/versioning.md](policy/versioning.md) — Versioning policy
