# Desktop Architecture

Architecture decisions pending — to be resolved before P3-10.

## Open Decisions

| Decision | Options | Blocked Prompts |
|----------|---------|-----------------|
| React state management | Zustand, TanStack Query, Jotai | P3-10, P3-13, P3-14, P3-15 |
| Tauri IPC pattern | Typed RPC via serde, raw JSON invoke() | P3-10 through P3-16 |
| Desktop build matrix | Single universal binary, separate x86_64/aarch64 | P3-16 |
| Signal path visualization | WebGL spectrum, Canvas-based | P3-11 |

## Context

The desktop client is a Tauri 2 application with:
- React 19 + TypeScript frontend
- Rust backend for IPC (harmonia-desktop crate)
- HTTP communication to a running harmonia server

See `docs/architecture/binary-modes.md` for the `harmonia desktop` execution mode context.
