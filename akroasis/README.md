# Akroasis

> Ἀκρόασις (akroasis) - "a hearing", from ἀκροάομαι (akroaomai) - "to listen"

Unified media player for music, audiobooks, podcasts, and ebooks. Designed for audiophiles who demand bit-perfect playback alongside seamless integration of portable media.

**Scope**: Audio (music, audiobooks, podcasts) + reading (ebooks). TV shows and movies are handled by Plex.

## Project Vision

**No-compromise** unified player with:
- **Bit-perfect audio**: High-res playback (24/96, 24/192, DSD), gapless, exclusive mode
- **Unified interface**: One app for music, audiobooks, podcasts, ebooks - no compromise
- **Sony Walkman optimized**: Priority platform for portable audiophile playback
- **Self-hosted first**: Full control via [Mouseion](https://github.com/forkwright/mouseion) backend
- **Privacy-first**: No telemetry, no tracking, local-first data storage

**Focus**: Portable media consumption. TV shows and movies are handled by Plex.

## Architecture

- **Backend**: [Mouseion](https://github.com/forkwright/mouseion) - REST API for audiobooks, music, ebooks
- **Frontend**: Client-only applications (no server management UI)
  - **Android** (Kotlin + Jetpack Compose) - Priority platform
  - **Desktop** (Tauri + React) - Linux native app
  - **Web** (React PWA) - Browser-based player

## Project Status

**Active Development** - Multiple phases complete

- ✅ Phase 0: Foundation (Rust audio core, JNI integration, APK builds)
- ✅ Phase 1: Playback Excellence (signal path visualization, gapless, queue history, playback speed memory)
- ✅ Phase 3: DSP Engine (5-band parametric EQ, AutoEQ profiles, crossfeed, headroom management)
- ✅ Phase 6: Mobile Optimization (media session, playback notifications, state persistence, network monitoring)
- ✅ Phase 7: Discovery & Scrobbling (Last.fm and ListenBrainz integration)
- 🚧 Web App: MVP in progress
- ⏸️  Phase 2: Audio Intelligence - Waiting for Mouseion APIs (Week 7-8)

**Recent Achievements**: PR #18 merged 21 features across 4 phases, 84 files changed, 11,426 insertions, 365+ tests (40-50% coverage).

See [ROADMAP.md](ROADMAP.md) for detailed phase breakdown.
See [android/CHANGELOG.md](android/CHANGELOG.md) for comprehensive recent changes.

## Repository Structure

```
akroasis/
├── android/              # Android app (Kotlin + Jetpack Compose)
├── web/                  # Web/Desktop app (Tauri + React)
├── shared/
│   └── akroasis-core/    # Rust audio core (FLAC, gapless, ReplayGain)
├── docs/                 # Documentation and design specs
├── .github/workflows/    # CI/CD (Rust, Web builds)
└── ROADMAP.md            # Phase-by-phase implementation plan
```

## Technology Stack

### Android
- Kotlin + Jetpack Compose
- Rust audio core (akroasis-core via JNI)
- Retrofit + OkHttp (Mouseion API)
- Room (offline cache)
- Hilt (DI)

### Desktop (Tauri)
- Tauri 2 (Rust backend)
- React + TypeScript
- Rust audio core (akroasis-core via FFI)
- Tailwind CSS

### Web (PWA)
- React + TypeScript
- Web Audio API
- Tailwind CSS
- Shared frontend with Tauri

### Audio Core (Rust)
- FLAC decoder (claxon)
- Gapless playback buffer
- ReplayGain processing
- Cross-platform (JNI for Android, FFI for Desktop)

### Backend (External)
- Mouseion (C# .NET 8.0)
- SQLite/PostgreSQL database
- REST API with API key auth

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines and workflow.

## Goals

1. **Music player** (Primary): Bit-perfect playback, gapless, high-res audio, EQ, ReplayGain, offline sync
2. **Audiobook player**: Chapter navigation, position tracking, narrator info, sleep timer, speed control
3. **Podcast player**: Episode management, position sync, speed control, chapter markers
4. **eBook reader** (Secondary): EPUB support, position sync, basic annotations
5. **Unified UX**: Consistent interface across all media types
6. **Quality focus**: No compromise on audio quality

## Inspiration

Drawing from the best in class:
- **Music**: [Plexamp](https://plexamp.com), [Symfonium](https://symfonium.app), [USB Audio Player PRO](https://www.extreamsd.com/index.php/products/usb-audio-player-pro), Sony Walkman Native App
- **Podcasts**: [PocketCasts](https://pocketcasts.com) - Clean UI, smart sync, chapter markers
- **eBooks**: [Bookfusion](https://www.bookfusion.com) - EPUB reader, sync, annotations

## Privacy

**Core principle: Your data, your control.**

- **No telemetry** - Zero analytics, no phone-home
- **No tracking** - No usage metrics, no user profiling
- **Local-first** - All data stored on your device or self-hosted server
- **Self-hosted** - Full control via your own Mouseion instance
- **No third-party services** - No cloud sync, no external dependencies without explicit opt-in

Your media, your metadata, your listening history - all yours.

## License

GPL-3.0

## Contributing

Project is in early development. Contribution guidelines will be established in Phase 1.

## Support

This project is free and always will be.
Support development: [GitHub Sponsors](https://github.com/sponsors/forkwright)

---

Solo project with AI pair programming. All AI-generated code is reviewed before merging.
