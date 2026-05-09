# Harmonia

> Ἁρμονία (Harmonia): "the fitting together of disparate parts"

Unified self-hosted media platform. Rust monorepo with a server binary intended
to replace the *arr stack as a full media lifecycle manager.

## Architecture

Single Tokio/Axum/SQLite server with 20 workspace crates under `crates/`.
`crates/theatron/desktop` is an excluded Dioxus desktop package; canonical
STATE.md tracks that desktop port as the remaining Phase 3.5 scope.

| Layer | Crates | Purpose |
|-------|--------|---------|
| **Core** | themelion, apotheke, horismos | Shared types, SQLite storage, configuration |
| **Auth** | exousia | JWT authentication, argon2 password hashing |
| **Media ops** | kathodos, komide, epignosis, kritike | Import/rename, library scanning, metadata enrichment, quality verification |
| **Acquisition** | zetesis, ergasia, syntaxis, aitesis | Torznab search, download execution, queue orchestration, household requests |
| **Serving** | paroche, syndesmos, syndesis, prostheke | HTTP streaming, external integrations (Plex, Last.fm, Tidal), QUIC renderer transport, subtitles |
| **Audio** | akouo-core | Bit-perfect decode, DSP (EQ, crossfeed, ReplayGain), native audio output |
| **UI** | theatron-core | Dioxus desktop types and API client |
| **Binary** | archon | `harmonia` CLI for `serve`, `db`, `play`, `render`, and `migrate` modes |
| **Convert** | harmonia-convert | Ebook format conversion (Calibre, kepubify, pandoc) |

### Capability status

- **Shipped / wired to routes:** auth, library scan/import (kathodos), feed scheduler (komide), torrent download engine (ergasia), queue orchestration (syntaxis), HTTP/OpenSubsonic API (paroche), external integrations (syndesmos), QUIC renderer transport (syndesis), audio pipeline (akouo-core).
- **Initialized, adapter-backed or fallback-only at HTTP layer:** the live `serve` path wires adapter structs for search, download execution, queue management, requests, external integrations, subtitles, and renderer registry. It still uses 2 null placeholders for metadata resolution and curation. The fallback/test `AppState::with_stubs` path defines 9 `Null*` service implementations.
- **Stubbed:** syntaxis post-download import pipeline (`StubImportService` - downloads complete but are not auto-imported).

For current planning, blockers, and phase status, see the canonical project state maintained with the internal planning records.

## Build

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentation

- `<standards-doc>/STANDARDS.md`: Coding standards
- `<standards-doc>/GNOMON.md`: Greek naming methodology
- [docs/lexicon.md](docs/lexicon.md): Project name registry

## License

AGPL-3.0-or-later. See [NOTICE](NOTICE) for supplemental terms.
