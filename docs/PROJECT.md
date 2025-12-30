# Harmonia — Project Overview

## Vision

One self-hosted media platform replacing the fragmented *arr ecosystem. Mouseion manages all media types (movies, TV, music, books, audiobooks, podcasts, manga, comics, news). Akroasis plays them across Android, Web, and Desktop.

## Architecture

```
┌─────────────────────────────────────────┐
│              Akroasis                   │
│  ┌───────────┐ ┌──────┐ ┌───────────┐  │
│  │  Android   │ │ Web  │ │  Desktop  │  │
│  │  Kotlin    │ │React │ │  Tauri 2  │  │
│  └─────┬─────┘ └──┬───┘ └─────┬─────┘  │
│        └──────────┼──────────┘          │
│              Rust Audio Core            │
│         (FLAC, gapless, ReplayGain)     │
└────────────────┬────────────────────────┘
                 │ REST API v3
┌────────────────┴────────────────────────┐
│              Mouseion                   │
│  Media management, metadata, indexers,  │
│  quality profiles, download clients     │
│  (.NET 10 → Rust rewrite planned)       │
└─────────────────────────────────────────┘
```

## Open Issues

- **mouseion#225** — Rust rewrite evaluation. Decision: proceed. Single static binary (Tokio, Axum, embedded DB). Same toolchain as Aletheia. Eliminates multi-process coordination overhead and ~4-8GB runtime footprint.

## Why Monorepo

Mouseion and Akroasis are one product with two deployment targets. Separate repos created coordination overhead for API changes, shared specs, and release timing. Harmonia unifies them while preserving independent build/CI pipelines.
