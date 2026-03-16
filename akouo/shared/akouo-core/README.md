# Akouo core - Rust audio library

Pure Rust audio decoding and processing library for bit-perfect playback.

## Features

- **FLAC decoding**: Pure Rust FLAC decoder using `claxon`
- **Bit-perfect playback**: Native 16/24/32-bit PCM preservation
- **Sample rate conversion**: Using `rubato` (future)
- **ReplayGain processing**: Audio normalization (future)
- **Cross-platform**: Android (JNI), Desktop (FFI), Web (WASM future)

## Building

### Standard build

```bash
cargo build --release
cargo test
```

### Android build

Requires Android NDK 21+. Install via Android Studio SDK Manager or standalone.

**Setup NDK environment:**

```bash
export ANDROID_NDK_HOME=/path/to/ndk
export PATH=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH
```

**Add Rust Android targets:**

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

**Build for Android:**

```bash
./build-android.sh
```

This builds all Android architectures and copies `.so` files to:
- `../../android/app/src/main/jniLibs/arm64-v8a/`
- `../../android/app/src/main/jniLibs/armeabi-v7a/`
- `../../android/app/src/main/jniLibs/x86_64/`
- `../../android/app/src/main/jniLibs/x86/`

### Desktop build

```bash
cargo build --release
```

Output: `target/release/libakouo_core.so` (Linux) or `.dylib` (macOS)

## Architecture

```
akouo-core/
├── src/
│   ├── lib.rs          # Public API exports
│   ├── decoder.rs      # FLAC decoder implementation
│   ├── buffer.rs       # Gapless playback buffer
│   ├── replaygain.rs   # Audio normalization
│   ├── error.rs        # Error types
│   └── jni.rs          # Android JNI bindings (feature-gated)
├── Cargo.toml
├── .cargo/
│   └── config.toml     # Android NDK linker configuration
└── build-android.sh    # Android build script
```

## Features

### Cargo features

- `android`: Enable JNI bindings for Android (requires `jni` dependency)

**Usage:**

```bash
cargo build --features android
```

### JNI API (Android)

**Kotlin usage:**

```kotlin
val decoder = FlacDecoder()
val audioData = File("track.flac").readBytes()
val decoded = decoder.decode(audioData)

decoded?.let {
    println("Sample rate: ${it.sampleRate}")
    println("Channels: ${it.channels}")
    println("Duration: ${it.duration}ms")
    // it.samples is ByteArray of 16-bit PCM samples
}

decoder.close()
```

**Native methods:**

- `createFlacDecoder() -> Long`: Create decoder instance
- `destroyFlacDecoder(ptr: Long)`: Free decoder
- `decodeFlac(ptr: Long, data: ByteArray) -> ByteArray?`: Decode FLAC
- `getSampleRate(ptr: Long) -> Int`: Get sample rate
- `getChannels(ptr: Long) -> Int`: Get channel count
- `getBitDepth(ptr: Long) -> Int`: Get bit depth

## Testing

```bash
cargo test
cargo test --features android
```

## Benchmarks (future)

```bash
cargo bench
```

## Dependencies

- `claxon` 0.4 - Pure Rust FLAC decoder
- `rubato` 0.15 - Sample rate conversion
- `dasp` 0.11 - Digital audio signal processing
- `thiserror` 2.0 - Error derivation
- `tracing` 0.1 - Logging
- `jni` 0.21 - Android JNI bindings (optional)

## Performance notes

- Bit-perfect output (no dithering or resampling unless requested)
- Zero-copy where possible
- Optimized release builds with LTO
- SIMD acceleration (future - via `dasp`)

## Known limitations

- FLAC only (MP3, AAC, Opus, DSD planned)
- No streaming decoder (loads entire file to memory)
- No seek support within file
- ReplayGain not yet implemented

## License

See main project LICENSE.
