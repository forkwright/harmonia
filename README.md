# Harmonia

> Ἁρμονία (Harmonia): "the fitting together of disparate parts"

Unified self-hosted media platform. Rust monorepo — single static binary replacing the *arr ecosystem.

## Architecture

Single Tokio/Axum/SQLite server covering the full media lifecycle: discovery, search, download, import, organization, metadata enrichment, quality management, and streaming. 19 workspace crates under `crates/`.

| Layer | Crates | Purpose |
|-------|--------|---------|
| **Core** | themelion, apotheke, horismos | Shared types, SQLite storage, configuration |
| **Auth** | exousia | JWT authentication, argon2 password hashing |
| **Media ops** | kathodos, komide, epignosis, kritike | Import/rename, library scanning, metadata enrichment, quality verification |
| **Acquisition** | zetesis, ergasia, syntaxis, aitesis | Torznab search, download execution, queue orchestration, household requests |
| **Serving** | paroche, syndesmos, syndesis, prostheke | HTTP streaming, external integrations (Plex, Last.fm, Tidal), discovery |
| **Audio** | akouo-core | Bit-perfect decode, DSP (EQ, crossfeed, ReplayGain), native audio output |
| **UI** | theatron | Dioxus desktop app (proskenion) |
| **Binary** | archon | Axum server entry point |

## Build

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentation

- [standards/STANDARDS.md](standards/STANDARDS.md): Coding standards
- [docs/gnomon.md](docs/gnomon.md): Greek naming methodology
- [docs/lexicon.md](docs/lexicon.md): Project name registry

## License

AGPL-3.0-or-later. See [NOTICE](NOTICE) for supplemental terms.
