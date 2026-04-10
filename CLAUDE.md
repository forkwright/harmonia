# CLAUDE.md: Harmonia monorepo

## Repository

Harmonia: unified self-hosted media platform. Rust monorepo containing backend crates, audio core, and desktop app.

```
harmonia/
├── crates/             # Rust workspace crates (backend subsystems)
│   ├── harmonia-common/    # Shared types, IDs, domain primitives
│   ├── harmonia-db/        # SQLite storage layer (sqlx)
│   ├── harmonia-host/      # Axum HTTP server and binary entry point
│   ├── horismos/           # Configuration (figment)
│   ├── exousia/            # Authentication and authorization (JWT, argon2)
│   ├── paroche/            # HTTP streaming and media serving
│   ├── taxis/              # File import, renaming, directory structure
│   ├── epignosis/          # Metadata enrichment (MusicBrainz, TMDB, etc.)
│   ├── kritike/            # Library quality and integrity verification
│   ├── komide/             # Library scanner and file watcher
│   ├── zetesis/            # Indexer search (Torznab/Newznab)
│   ├── ergasia/            # Download execution and archive extraction
│   ├── syndesmos/          # External service integration (Plex, Last.fm, Tidal)
│   ├── aitesis/            # Household media request management
│   └── syntaxis/           # Download queue orchestration and post-processing
├── akouo/
│   └── shared/
│       └── akouo-core/     # Rust audio engine (decode, DSP, output) — excluded from workspace
├── desktop/            # Tauri 2 desktop app (Rust + webview)
├── standards/          # Universal coding standards
├── docs/               # Cross-cutting documentation
│   ├── lexicon.md          # Project name registry
│   ├── LESSONS.md          # Operational rules (earned through failure)
│   ├── CLAUDE_CODE.md      # Claude Code dispatch protocol
│   ├── WORKING-AGREEMENT.md
│   └── policy/             # Agent contribution, versioning, git history
└── CLAUDE.md           # This file
```

Component-specific guidelines: `akouo/CLAUDE.md`.

## Standards

Universal: [standards/STANDARDS.md](standards/STANDARDS.md)
Rust: [standards/RUST.md](standards/RUST.md)
SQL: [standards/SQL.md](standards/SQL.md)
Shell: [standards/SHELL.md](standards/SHELL.md)
Writing: [standards/WRITING.md](standards/WRITING.md)

## Documentation

- `standards/GNOMON.md`: Greek naming methodology
- `docs/lexicon.md`: project name registry with layer tests
- `docs/LESSONS.md`: operational rules derived from real failures
- `docs/CLAUDE_CODE.md`: Claude Code prompt template and dispatch protocol
- `docs/WORKING-AGREEMENT.md`: Syn + Cody collaboration protocol
- `docs/policy/`: agent contribution, versioning, git history policies

## Branch strategy

- **Single branch:** `main`. No develop branch.
- PRs target `main`. Squash merge.
- Branch naming: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `cleanup/`

## Commit format

`category(scope): description`

Categories: feat, fix, docs, refactor, test, chore, style
Scopes: crate name (`syntaxis`, `exousia`, etc.), `desktop`, `docs`, `infra`

## Build & test

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Targeted tests during development: `cargo test -p <crate>`

## Architecture

Harmonia is a Rust platform. Single static binary (Tokio, Axum, SQLite) replacing the *arr ecosystem. 15 workspace crates covering the full media lifecycle: discovery, search, download, extraction, import, organization, metadata enrichment, quality management, serving, and household requests.

The audio engine (`akouo-core`) is built independently and shared via FFI with the Tauri desktop app. It handles bit-perfect decode, DSP (EQ, crossfeed, ReplayGain), and native audio output.

## CI

GitHub Actions workflows:
- `rust.yml`: format, clippy, test, MSRV check, rustdoc, coverage (triggers on `crates/`, `Cargo.toml`, `Cargo.lock`)
- `security.yml`: cargo-audit, cargo-deny, gitleaks, TruffleHog (triggers on all pushes/PRs + weekly schedule)
- `desktop.yml`: desktop app build pipeline

## What not to do

- Don't add dependencies without justification
- Don't modify CI workflows without understanding the full pipeline
- No AI attribution, no "Co-authored-by: Claude", no emoji indicators
- No filler words: comprehensive, robust, leverage, streamline, modernize, strategic, enhance
