# Desktop architecture

The desktop client is a pre-alpha Dioxus (Rust) application named
**periskopio**, living in `crates/theatron/desktop/`. Shared types and an API
client live in `crates/theatron/core/` (`skene`) for reuse by future frontends.

The routed views are stubs. Playback is not wired, and the `skene` API client is
not connected to the UI.

This crate is **excluded from the workspace** in root `Cargo.toml` to decouple
its build from backend CI. Build standalone:

```bash
cargo check --manifest-path crates/theatron/desktop/Cargo.toml
cargo build --release --manifest-path crates/theatron/desktop/Cargo.toml
```

## Current state

- Framework: Dioxus 0.7 (`desktop`, `router` features)
- Shared client: `skene` (reqwest + serde + snafu), not yet wired to the UI
- App state: development server URL, in-memory auth placeholder, and theme
- Playback: not implemented

## Prior Tauri/React design (removed)

The initial design used Tauri 2 with a React + Zustand + TanStack Query frontend.
That approach was retired in favor of Dioxus to keep the entire stack in Rust and
share types with the backend without a code-generation layer. History is in git.

## Planned communication

The planned client communicates with a `harmonia serve` instance over HTTP
(REST + WebSocket) and uses `syndesis` for audio over QUIC. Neither transport is
wired into the current desktop UI. See
[`../architecture/binary-modes.md`](../architecture/binary-modes.md) for the
current `archon` subcommands and the standalone desktop package boundary.
