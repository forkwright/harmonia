# Binary modes

Harmonia ships as a single binary (`harmonia-host`) with four execution modes,
selected via subcommand. Each mode activates a subset of the system's subsystems.

## Modes

### `Harmonia serve`

The server. Runs on the NAS or primary machine. Manages the library, API,
acquisition, and streaming.

**Active subsystems:** All backend crates: harmonia-db, horismos, exousia,
taxis, epignosis, zetesis, ergasia, syntaxis, kritike, paroche, episkope,
aitesis, syndesmos, prostheke, syndesis (QUIC server endpoint).
**Inactive:** akouo-core (server does not play audio locally).
**Listens on:** HTTP (paroche, default :8096), QUIC (syndesis, default :7472).

### `Harmonia desktop`

The Tauri desktop client. Full UI, local audio playback, connects to a serve
instance for library and acquisition.

**Active subsystems:** akouo-core (local audio engine), horismos (local config).
**Connects to:** A `harmonia serve` instance via HTTP API + QUIC audio stream.
**Does NOT run:** Library management, acquisition, metadata enrichment; all
delegated to the serve instance.

### `Harmonia render`

Headless audio renderer. Runs on Pi or dedicated audio endpoints. Receives
audio over QUIC from a serve instance and outputs to local hardware.

**Active subsystems:** akouo-core (output backend only, no local decode),
horismos (local config: output device, DSP settings).
**Connects to:** A `harmonia serve` instance via QUIC (syndesis).
**Does NOT run:** Library, API, acquisition, decode (server decodes and
streams FLAC frames).
**Local DSP:** Renderer applies its own EQ, crossfeed, volume settings
after receiving the stream.

### `Harmonia play`

CLI standalone player. No server, no network. Plays local files directly.

**Active subsystems:** akouo-core (full pipeline: decode → DSP → output).
**Does NOT run:** Library management, API, acquisition, streaming.
**Purpose:** Validates the audio engine end-to-end. Useful for quick playback
and testing. No persistent state.

## Mode selection

Mode is selected at startup via Clap subcommand:

    harmonia serve [--config path]
    harmonia desktop [--server url]
    harmonia render --server url [--device hw:1]
    harmonia play <file|directory|playlist>

## Cargo features

Each mode can be compiled independently via cargo features to produce
smaller binaries for constrained targets:

    cargo build -p harmonia-host --features serve    # server-only
    cargo build -p harmonia-host --features render   # renderer-only (Pi)
    cargo build -p harmonia-host --features play     # CLI player only

The default build includes all modes.
