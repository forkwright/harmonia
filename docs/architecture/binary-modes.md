# Binary modes

Harmonia ships its wired backend and audio entry points through the `archon`
binary. Mode is selected via Clap subcommand, and each mode activates a subset
of the system's subsystems. The Dioxus desktop client is a separate package
under `crates/theatron/desktop/`; it is not an `archon` subcommand today.

## Modes

### `harmonia serve`

The server. Runs on the NAS or primary machine. Manages the library, API,
acquisition, and streaming.

**Active subsystems:** All backend crates: apotheke, horismos, exousia,
kathodos, epignosis, eksetasis, ergasia, syntaxis, kritike, paroche, aitesis,
syndesmos, prostheke, syndesis (QUIC server endpoint).
**Inactive:** akouo-core (server does not play audio locally).
**Listens on:** HTTP (paroche, default :8096), QUIC (syndesis, default :7472).

### `harmonia db migrate`

Database maintenance. Runs configured SQLite migrations against the databases
defined in `harmonia.toml`.

**Active subsystems:** horismos (configuration), apotheke (SQLite pools and
migrations).
**Does NOT run:** HTTP API, acquisition, playback, renderer transport.

### `harmonia render`

Headless audio renderer. Runs on Pi or dedicated audio endpoints. Receives
audio over QUIC from a serve instance and outputs to local hardware.

**Active subsystems:** akouo-core (output backend only, no local decode),
horismos (local config: output device, DSP settings).
**Connects to:** A `harmonia serve` instance via QUIC (syndesis).
**Does NOT run:** Library, API, acquisition, decode (server decodes and
streams FLAC frames).
**Local DSP:** Renderer applies its own EQ, crossfeed, volume settings
after receiving the stream.

### `harmonia play`

CLI standalone player. No server, no network. Plays local files directly.

**Active subsystems:** akouo-core (full pipeline: decode → DSP → output).
**Does NOT run:** Library management, API, acquisition, streaming.
**Purpose:** Validates the audio engine end-to-end. Useful for quick playback
and testing. No persistent state.

### `harmonia migrate`

Legacy library migration. Converts an existing media tree into Harmonia's
canonical storage layout.

**Active subsystems:** migration planner and filesystem operations.
**Does NOT run:** Server routes, acquisition, playback, renderer transport.

### `harmonia mcp`

Local MCP stdio server (rmcp) for agent-driven maintenance operations. It
exposes machine-callable tools for command modes that are intentionally local
or long-running.

Offline tools, run in-process with no server required:

- `harmonia_db_migrate`
- `harmonia_migrate_library`
- The playback and renderer lifecycle trios: `harmonia_play_file_start` /
  `harmonia_play_file_status` / `harmonia_play_file_stop` and
  `harmonia_render_start` / `harmonia_render_status` / `harmonia_render_stop`
- Their blocking one-call forms `harmonia_play_file` and `harmonia_render`,
  kept for short work a caller wants to await in a single call

plus four acquisition tools (`harmonia_search_releases`,
`harmonia_enqueue_download`, `harmonia_list_downloads`,
`harmonia_cancel_download`) that forward to a running `harmonia serve` over
its local Unix-socket acquisition bridge; if the server is not running, those
calls return a tool-level error naming the socket.

The server speaks MCP over newline-delimited JSON-RPC stdio via the `rmcp`
crate: each request runs as its own task (a `ping` is answered while a long
call is in flight), the protocol version is negotiated rather than echoed,
and each tool's `inputSchema` is generated from its typed parameter struct.

Playback and rendering are long-lived, so the lifecycle trios represent them
as state rather than an indefinitely open RPC (#652): `*_start` spawns the
work and returns an op id immediately, `*_status` reports running / exited
(with the exit summary) / idle, and `*_stop` cancels the op's token and
awaits its teardown. The registry holds one op per kind per server process —
a start while one runs is refused, never replaced, and op ids (`playback-1`,
`renderer-2`, …) are monotonic per process. The blocking forms do not join
the registry: they cancel through the MCP `notifications/cancelled` path and
are invisible to `*_status`.

The HTTP API served by `paroche` remains the canonical remote service surface
for library, acquisition, request, renderer, and user-facing operations. The MCP
mode exists so automation can invoke offline command paths without parsing CLI
output.

**Active subsystems:** only the subsystem required by the invoked tool.
**Does NOT run by default:** HTTP API, acquisition scheduler, library watchers,
or renderer transport unless the caller invokes a renderer tool.

## Mode selection

Mode is selected at startup via Clap subcommand:

    harmonia serve [--config path]
    harmonia db migrate [--config path]
    harmonia render [--server addr] [--config path]
    harmonia play <file> [--device name]
    harmonia migrate --source path --target path --media-type music|books|audiobooks|podcasts
    harmonia mcp

## Desktop client

The desktop client is the standalone Dioxus package
`crates/theatron/desktop/`, sharing API types through `theatron-core`. It
connects to a `harmonia serve` instance for library and acquisition behavior and
handles local UI and playback concerns. The package is intentionally excluded
from the workspace while the Phase 3.5 desktop port remains in progress; build
it directly when working on that track:

    cargo check --manifest-path crates/theatron/desktop/Cargo.toml
    cargo build --release --manifest-path crates/theatron/desktop/Cargo.toml

## Cargo features

`archon` exposes no per-mode Cargo features. Build the full CLI
binary with:

    cargo build -p archon
