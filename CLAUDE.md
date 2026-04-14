# CLAUDE.md: Harmonia monorepo

## Repository

Harmonia: unified self-hosted media platform. Rust monorepo  -  single static binary replacing the *arr ecosystem.

```
harmonia/
├── crates/             # Rust workspace crates
│   ├── themelion/          # Shared types, IDs, domain primitives
│   ├── apotheke/           # SQLite storage layer (sqlx)
│   ├── archon/             # Axum HTTP server and binary entry point
│   ├── horismos/           # Configuration (figment)
│   ├── exousia/            # Authentication and authorization (JWT, argon2)
│   ├── paroche/            # HTTP streaming and media serving
│   ├── kathodos/           # File import, renaming, directory structure, canonical storage
│   ├── epignosis/          # Metadata enrichment (MusicBrainz, Audnexus, TMDB)
│   ├── kritike/            # Library quality, integrity, format scoring
│   ├── komide/             # Library scanner and file watcher
│   ├── zetesis/            # Indexer search (Torznab/Newznab)
│   ├── ergasia/            # Download execution and archive extraction
│   ├── syndesmos/          # External service integration (Plex, Last.fm, Tidal)
│   ├── aitesis/            # Household media request management
│   ├── syntaxis/           # Download queue orchestration and post-processing
│   ├── akouo-core/         # Rust audio engine (decode, DSP, output)
│   ├── theatron/           # UI crates (core + desktop)
│   ├── prostheke/          # Service discovery (mDNS)
│   └── syndesis/           # Integration glue
├── standards/          # Coding standards (kanon-synced)
├── docs/               # Documentation
│   ├── data/               # Storage layout, schemas, quality profiles
│   ├── lexicon.md          # Project name registry
│   └── LESSONS.md          # Operational rules (earned through failure)
└── CLAUDE.md           # This file
```

## Standards

Universal: [standards/STANDARDS.md](standards/STANDARDS.md)
Rust: [standards/RUST.md](standards/RUST.md)
SQL: [standards/SQL.md](standards/SQL.md)
Shell: [standards/SHELL.md](standards/SHELL.md)
Writing: [standards/WRITING.md](standards/WRITING.md)

## Build & test

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Targeted tests during development: `cargo test -p <crate>`

## Architecture

19 workspace crates covering the full media lifecycle: discovery, search, download, extraction, import, organization, metadata enrichment, quality management, serving, and household requests. Audio engine (akouo-core) handles bit-perfect decode, DSP (EQ, crossfeed, ReplayGain), and native audio output.

## Branch strategy

- **Single branch:** `main`. No develop branch.
- PRs target `main`. Squash merge.
- Branch naming: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `cleanup/`

## Commit format

`category(scope): description`

Categories: feat, fix, docs, refactor, test, chore, style
Scopes: crate name (`syntaxis`, `exousia`, `kathodos`, etc.), `docs`, `infra`

## CI

GitHub Actions workflows:
- `rust.yml`: format, clippy, test, MSRV check, rustdoc, coverage
- `security.yml`: cargo-audit, cargo-deny, gitleaks, TruffleHog

## What not to do

- Don't add dependencies without justification
- Don't modify CI workflows without understanding the full pipeline
- No AI attribution, no "Co-authored-by: Claude", no emoji indicators
- No filler words: comprehensive, robust, leverage, streamline, modernize, strategic, enhance
