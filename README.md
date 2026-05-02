# Harmonia

> Ἁρμονία (Harmonia): "the fitting together of disparate parts"

Unified self-hosted media platform. Rust monorepo — single static binary replacing the *arr ecosystem.

## Architecture

Single Tokio/Axum/SQLite server with 20 workspace crates under `crates/`.

| Layer | Crates | Purpose |
|-------|--------|---------|
| **Core** | themelion, apotheke, horismos | Shared types, SQLite storage, configuration |
| **Auth** | exousia | JWT authentication, argon2 password hashing |
| **Media ops** | kathodos, komide, epignosis, kritike | Import/rename, library scanning, metadata enrichment, quality verification |
| **Acquisition** | zetesis, ergasia, syntaxis, aitesis | Torznab search, download execution, queue orchestration, household requests |
| **Serving** | paroche, syndesmos, syndesis, prostheke | HTTP streaming, external integrations (Plex, Last.fm, Tidal), QUIC renderer transport, subtitles |
| **Audio** | akouo-core | Bit-perfect decode, DSP (EQ, crossfeed, ReplayGain), native audio output |
| **UI** | theatron-core | Dioxus desktop types and API client |
| **Binary** | archon | Axum server entry point |
| **Convert** | harmonia-convert | Ebook format conversion (Calibre, kepubify, pandoc) |

### Capability status

- **Shipped / wired to routes:** auth, library scan/import (kathodos), feed scheduler (komide), torrent download engine (ergasia), queue orchestration (syntaxis), HTTP/OpenSubsonic API (paroche), external integrations (syndesmos), QUIC renderer transport (syndesis), audio pipeline (akouo-core).
- **Initialized, null-adapter at HTTP layer:** metadata resolution (epignosis), quality tracking (kritike — `register_import` is currently a no-op), indexer search (zetesis), subtitle management (prostheke), household requests (aitesis), queue manager UI.
- **Stubbed:** syntaxis post-download import pipeline (`StubImportService` — downloads complete but are not auto-imported).

For current planning, blockers, and phase status, see the canonical project state: [`kanon/projects/harmonia/STATE.md`](https://github.com/forkwright/kanon/blob/main/projects/harmonia/STATE.md).

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
