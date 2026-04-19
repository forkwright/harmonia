# Desktop architecture

The desktop client is a Dioxus (Rust) application named **proskenion**, living in
`crates/theatron/desktop/`. It shares types and an API client with future TUI
and web frontends via `crates/theatron/core/` (`theatron-core`).

This crate is **excluded from the workspace** in root `Cargo.toml` to decouple
its build from backend CI. Build standalone:

```bash
cargo check --manifest-path crates/theatron/desktop/Cargo.toml
cargo build --release --manifest-path crates/theatron/desktop/Cargo.toml
```

## Current state

- Framework: Dioxus 0.7 (`desktop`, `router` features)
- Shared client: `theatron-core` (reqwest + serde + snafu)
- Config persistence: `dirs` + `toml` for local settings (server URL, token)
- Auth: Bearer JWT issued by `exousia`; stored locally, refreshed on the
  server via `paroche`

## Prior Tauri/React design (removed)

The initial design used Tauri 2 with a React + Zustand + TanStack Query frontend.
That approach was retired in favor of Dioxus to keep the entire stack in Rust and
share types with the backend without a code-generation layer. History is in git.

## Communication

The desktop talks to a `harmonia serve` instance over HTTP (REST + WebSocket)
and, for audio, over QUIC via `syndesis`. See
[`../architecture/binary-modes.md`](../architecture/binary-modes.md) for the
`harmonia desktop` execution mode context.
