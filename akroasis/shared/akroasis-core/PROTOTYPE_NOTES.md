# Prototype notes: akroasis-core

Documents findings from the pre-R6 prototype and decisions made during the P1-01 scaffold.

## Prototype inventory (now in `legacy/akroasis-core/`)

| File | What it did | Why removed |
|------|-------------|-------------|
| `decoder.rs` | `FlacDecoder` via `claxon`, returned `Vec<i32>` | claxon is single-codec; i32 internal format is wrong for f64 pipeline |
| `buffer.rs` | `AudioBuffer` backed by `Vec<i16>`, position-tracking read/write | Not lock-free; i16 precision ceiling; no ring semantics |
| `replaygain.rs` | Applied dB gain to `&mut [i16]` slices | Integer sample math loses precision; replaced by dsp/replaygain.rs on f64 |
| `jni.rs` | Raw JNI bindings to `FlacDecoder` via pointer juggling | UniFFI replaces raw JNI in Phase 6; decoder API has changed entirely |
| `lib.rs` | Exposed `AudioConfig`, `FlacDecoder`, `AudioError` | All superseded by new module tree |

## Key architecture decisions

### F64 internal pipeline
All decoded samples are f64 in `[-1.0, 1.0]`. This avoids precision loss during chained DSP
(EQ + ReplayGain + compressor + volume). Quantisation to i16/i24/i32 happens only at the
output stage in `dsp/volume.rs` (TPDF dither) + `output/format.rs`.

### Symphonia over claxon
symphonia handles FLAC, WAV, ALAC, AIFF, MP3, AAC, Vorbis, Opus-in-OGG, and MP4/M4A with a
single unified `FormatReader` + `Decoder` API. claxon is FLAC-only and unmaintained.

### Snafu over thiserror
Per project rules. `Location` tracking via `#[snafu(implicit)]` aids debugging of async decode
tasks where stack traces are truncated.

### Native async fn in traits
Rust 1.75+ supports native `async fn` in traits without `async-trait`. The `AudioDecoder` and
`OutputBackend` traits use this feature. No `async-trait` crate dependency.

### Cpal as optional feature
cpal requires ALSA headers on Linux. Gated behind `native-output` feature to allow the crate to
compile in CI environments without audio hardware. On-device builds enable the feature.

### Lock-free SPSC ring buffer
`ring_buffer.rs` uses `UnsafeCell<f64>` per slot with atomic read/write positions. No `Mutex`,
no allocation after `new()`. Required for the audio output callback which runs at real-time
priority and must not block.

## Dependency notes

- `opus = "0.3"` links `libopus` via `audiopus_sys`. Requires opus C library on the build host.
  Pure-Rust Opus support pending (no production-ready crate as of 2026-03).
- `lofty = "0.22"` is used for gapless metadata (LAME header, iTunSMPB). It also reads
  ReplayGain tags but the actual gain computation uses `ebur128`.
- `rubato = "1.0"`: API changed significantly from `0.15` used in the prototype. Phase 1
  stubs reference the 1.x API.

## Stub map (P1-0X targets)

| Stub | Implementing prompt |
|------|---------------------|
| `decode/symphonia.rs` | P1-02 |
| `decode/opus.rs` | P1-03 |
| `decode/probe.rs` | P1-02 |
| `dsp/silence.rs` | P1-04 |
| `dsp/eq.rs` | P1-05 |
| `dsp/crossfeed.rs` | P1-06 |
| `dsp/replaygain.rs` | P1-07 |
| `dsp/compressor.rs` | P1-08 |
| `dsp/convolution.rs` | P1-09 |
| `dsp/volume.rs` | P1-10 |
| `output/cpal.rs` | P1-11 |
| `output/format.rs` | P1-11 |
| `gapless/prebuffer.rs` | P1-10 |
| `engine.rs` (body) | P1-10 |
