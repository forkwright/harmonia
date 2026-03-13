# Legacy code reference

Preserved for reference during Rust rewrite. Delete when no longer needed.

## Akroasis-core/ (Rust prototype)

Old audio core using claxon and i16/i32 fixed-point. Replaced by the new Symphonia-based
f64 pipeline in `akroasis/shared/akroasis-core/`.

- `replaygain.rs`: ReplayGain formula: `gain_linear = 10^(gain_db / 20.0)` (i16 precision)
- `decoder.rs`: Claxon FLAC-only decode (replaced by Symphonia multi-codec)
- `jni.rs`: Raw JNI FFI bindings (replaced by UniFFI in Phase 6)
- `buffer.rs`: Vec<i16> ring buffer (replaced by lock-free SPSC in ring_buffer.rs)

## Web/ (TypeScript + Tauri)

React 19 + Tauri 2 web player prototype.

Key files for reference:

- `src/audio/headphoneProfiles.ts`: ~30 curated AutoEQ parametric EQ profiles
- `src/audio/autoEqConverter.ts`: Parametric → 10-band ISO EQ conversion algorithm
- `src/audio/loudnessMeasure.ts`: ITU-R BS.1770-4 loudness measurement
- `src/audio/EqualizerProcessor.ts`: 10-band parametric EQ (ISO freqs, Q=1.414)
- `src/services/scrobbleQueue.ts`: Offline-first scrobble queue with retry pattern
- `src/types/index.ts`: TypeScript types for Track, Album, Artist, Audiobook, Podcast
- `src/audio/WebAudioPlayer.ts`: Token refresh retry on 401/network errors

## Android/ (Kotlin + Jetpack Compose)

Android player prototype. MVVM + Hilt + StateFlow architecture.

## Mouseion/ (C# .NET 10 backend)

Original C# server. Dapper, DryIoc, FluentValidation, Polly. Being superseded by the Rust
rewrite (harmonia-host + crates/). Source in `src/` with 7 projects:

- `Mouseion.Core`: entities, services, business logic
- `Mouseion.Api`: controllers, middleware, API surface
- `Mouseion.Common`: shared utilities, HTTP client, DI
- `Mouseion.SignalR`: real-time messaging
- `Mouseion.Host`: entry point, configuration

## Specs/ (original design specs)

17 spec files (01–16 + TEMPLATE.md).

- Specs 08 (playback engine) and 12 (signal path) are superseded by R6 research.
- Others contain feature-level design for Phase 2+ (audiobooks, podcasts, discovery, UI).
