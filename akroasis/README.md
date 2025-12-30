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

**Focus**: Portable media consumption. TV shows and movies are handled by Plex.

## Architecture

- **Backend**: [Mouseion](https://github.com/forkwright/mouseion) - Production-ready REST API for audiobooks, music, ebooks
- **Frontend**: Client-only applications (no server management UI)
  - **Android** (Kotlin + Jetpack Compose) - Priority platform
  - **Desktop** (Tauri + React) - Linux native app
  - **Web** (React PWA) - Browser-based player

## Project Status

**Phase 0: Research & Foundation** - Complete

Repository structure initialized. Next: Phase 2 implementation.

See [ROADMAP.md](ROADMAP.md) for detailed phase breakdown.

## Repository Structure

```
akroasis/
├── android/              # Android app (Kotlin + Jetpack Compose)
├── web/                  # Web/Desktop app (Tauri + React)
├── shared/
│   └── akroasis-core/    # Rust audio core (FLAC, gapless, ReplayGain)
├── docs/                 # Documentation and design specs
├── .github/workflows/    # CI/CD (Rust, Web builds)
├── CLAUDE.md             # Development rules and workflow
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

Development is currently in research phase. See [CLAUDE.md](CLAUDE.md) for development guidelines and workflow.

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

## License

TBD

## Contributing

Project is in early development. Contribution guidelines will be established in Phase 1.

---

Solo project with AI pair programming. All AI-generated code is reviewed before merging.
