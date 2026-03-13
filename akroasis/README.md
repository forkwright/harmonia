# Akroasis

> Ἀκρόασις (akroasis): "a hearing", from ἀκροάομαι (akroaomai): "to listen"

Unified media player for music, audiobooks, and podcasts. Bit-perfect audio. Self-hosted backend.

## Architecture

```
akroasis/
├── android/              # Android app (Kotlin + Jetpack Compose)
├── web/                  # Web/Desktop app (React + TypeScript + Tauri)
├── shared/
│   └── akroasis-core/    # Rust audio core (FLAC, gapless, ReplayGain)
├── specs/                # Development specifications
└── .github/workflows/    # CI/CD
```

**Backend:** [Mouseion](https://github.com/forkwright/mouseion), a C# .NET REST API for media management.
Akroasis is client-only. No server management UI.

## Platforms

| Platform | Stack | Status |
|----------|-------|--------|
| **Android** | Kotlin, Jetpack Compose, Hilt, Room | Feature-rich (playback, DSP, scrobbling, audiobooks, ebooks) |
| **Web** | React 19, Vite, TypeScript, Tailwind, Zustand | MVP (playback, library, audiobooks, PWA) |
| **Desktop** | Tauri 2 + shared web frontend | Planned (spec 02) |

**Audio core** is Rust: bit-perfect pipeline with FLAC decoding, gapless playback, and ReplayGain. Shared via JNI (Android) and FFI (Desktop).

## Development

```bash
# Web
cd web && npm install && npm run dev

# Android
cd android && ./gradlew build
```

Web dev uses MSW (Mock Service Worker); no backend needed for development. See `web/DEVELOPMENT.md`.

## Roadmap

Spec-driven development. See `specs/` for active work and `ROADMAP.md` for high-level status.

## Privacy

No telemetry. No tracking. Local-first. Self-hosted. Your data stays yours.

## License

GPL-3.0
