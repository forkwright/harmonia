# Harmonia: project overview

## Vision

One self-hosted media platform replacing the fragmented *arr ecosystem. A single Rust binary manages all media types (movies, TV, music, books, audiobooks, podcasts, manga, comics, news). Akroasis plays them across desktop (Tauri) and eventually Android and web. Video playback stays with Plex; everything else plays through Harmonia's own clients.

## Architecture

```
┌─────────────────────────────────────────┐
│              Akroasis                   │
│  ┌───────────┐ ┌───────────┐           │
│  │  Desktop  │ │  Android  │           │
│  │  Tauri 2  │ │  (future) │           │
│  └─────┬─────┘ └─────┬─────┘           │
│        └──────┬───────┘                 │
│         akroasis-core                   │
│    (decode, DSP, ReplayGain)            │
└────────────────┬────────────────────────┘
                 │ REST API
┌────────────────┴────────────────────────┐
│         Harmonia backend                │
│  15 Rust crates: media management,      │
│  metadata, indexers, quality profiles,  │
│  downloads, serving, requests           │
│  (Tokio, Axum, SQLite)                  │
└─────────────────────────────────────────┘
```

## Current state

Phase 3 in progress. 15 workspace crates, 543 tests passing.

Completed in Phase 3:
- Download execution and archive extraction (ergasia, P3-02)
- Queue orchestration and post-processing (syntaxis, P3-03)
- Request management (aitesis, P3-05)
- External service integration (syndesmos, P3-06)
- Desktop: now playing, audiobook player, podcast player, EQ/DSP, media management UI, MPRIS/tray (P3-11 through P3-16)

Foundation (pre-Phase 3): harmonia-common, harmonia-db, harmonia-host, horismos, exousia, paroche, taxis, epignosis, kritike, komide, zetesis.

## Why monorepo

Backend crates and the desktop app are one product. Separate repos would create coordination overhead for API changes, shared types, and release timing. Independent CI pipelines per path keep builds fast.
