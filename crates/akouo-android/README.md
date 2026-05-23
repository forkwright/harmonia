# akouo-android

UniFFI binding crate for Harmonia Phase 05 Android playback.

This crate keeps Android output ownership on the Kotlin side. Rust owns decode,
DSP, and buffering through `akouo-core`; Kotlin registers an `AudioCallback` and
writes the delivered interleaved `f64` frames to AudioTrack, AAudio, or Oboe.

The `RingBuffer` remains internal to Rust and is never exposed over UniFFI. The
FFI boundary receives an owned `Vec<f64>` per callback frame. With the default
callback size of 1024 interleaved samples, stereo 44.1 kHz playback crosses the
boundary about 86 times per second. Smaller 512-sample callback frames raise
that to about 172 allocations per second. This is the accepted Phase 05 tax until
UniFFI has a stable pointer/borrowed-buffer story for foreign callbacks.

Generate Kotlin bindings after building the library:

```sh
cargo build -p akouo-android
cargo install uniffi_bindgen --version 0.31.1
uniffi-bindgen generate \
  --library "${CARGO_TARGET_DIR:-target}/debug/libakouo_android.so" \
  --language kotlin \
  --config crates/akouo-android/uniffi.toml \
  --out-dir target/uniffi/akouo-android
```

The generated Kotlin package is `io.forkwright.harmonia.akouo`.
