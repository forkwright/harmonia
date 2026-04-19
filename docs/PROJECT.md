# Harmonia: project overview

## Vision

One self-hosted media platform replacing the fragmented *arr ecosystem. A single Rust binary (built from `archon`) manages all media types: movies, TV, music, books, audiobooks, podcasts, manga, comics, news. The Akouo player family handles playback across a Dioxus desktop app (proskenion), with Android and web clients planned. Video playback stays with Plex; everything else plays through Harmonia's own clients.

## Architecture

```
┌─────────────────────────────────────────┐
│              Akouo                      │
│  ┌───────────┐ ┌───────────┐           │
│  │  Desktop  │ │  Android  │           │
│  │  Dioxus   │ │  (future) │           │
│  └─────┬─────┘ └─────┬─────┘           │
│        └──────┬───────┘                 │
│         akouo-core                      │
│    (decode, DSP, ReplayGain)            │
└────────────────┬────────────────────────┘
                 │ REST + WebSocket + QUIC
┌────────────────┴────────────────────────┐
│         Harmonia backend (archon)       │
│  Rust workspace crates: media mgmt,     │
│  metadata, indexers, quality profiles,  │
│  downloads, serving, requests           │
│  (Tokio, Axum, SQLite, Quinn)           │
└─────────────────────────────────────────┘
```

## Current state

Backend crate map: see [`_llm/architecture.toml`](../_llm/architecture.toml).
Workspace member count, test count, and merged-PR history are derivable
from the repository; do not embed them here. Run:

```bash
cargo metadata --format-version 1 | jq '.workspace_members | length'
cargo test --workspace 2>&1 | tail -5
gh pr list --state merged --limit 200
```

Completed phases:
- **Phase 1-2:** foundation (themelion, apotheke, horismos, exousia, paroche,
  kathodos, epignosis, kritike, komide, zetesis, archon)
- **Phase 3:** acquisition + requests (ergasia, syntaxis, aitesis, syndesmos);
  desktop UI (theatron) reached feature parity for music + audiobook playback
- **Phase 3.5+ (in progress):** QUIC streaming (syndesis), subtitles (prostheke),
  consolidated theatron workspace

## Why monorepo

Backend crates and the desktop app are one product. Separate repos would create coordination overhead for API changes, shared types, and release timing. Independent CI pipelines per path keep builds fast.
