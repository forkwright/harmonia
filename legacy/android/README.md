# Akroasis Android

Native Android music player with bit-perfect audio playback.

## Features

- **Bit-Perfect Audio**: Custom AudioTrack implementation with BIT_PERFECT mode (Android 14+)
- **Advanced Audio**:
  - Parametric EQ with 6 presets (Flat, Rock, Jazz, Classical, Pop, Bass Boost)
  - Gapless playback (< 50ms gap)
  - Crossfade (0-10s configurable)
  - ReplayGain loudness normalization
  - Playback speed (0.5x-2.0x)
- **USB DAC Support**: Detection and bit-perfect routing
- **Background Playback**: MediaSession integration
- **Queue Management**: Shuffle, repeat modes
- **Sleep Timer**: Auto-stop with countdown

## Requirements

- Android Studio Ladybug | 2024.2.1 or later
- JDK 17
- Android SDK 35
- Minimum: Android 10 (API 29)
- Target: Android 15 (API 35)
- Recommended: Android 14+ for bit-perfect mode

## Building

```bash
./gradlew build
```

## Running

```bash
./gradlew installDebug
```

## Architecture

- **UI**: Jetpack Compose Material3
- **DI**: Hilt
- **Networking**: Retrofit + OkHttp
- **Database**: Room (track caching)
- **Audio**: Custom AudioTrack with advanced features
- **Background**: PlaybackService (MediaSession)
- **State**: Kotlin Coroutines + Flow

## Project structure

```
app/src/main/java/app/akroasis/
├── audio/              # Audio playback engine
│   ├── AudioPlayer.kt
│   ├── EqualizerEngine.kt
│   ├── GaplessPlaybackEngine.kt
│   ├── CrossfadeEngine.kt
│   ├── UsbDacDetector.kt
│   └── ReplayGainProcessor.kt
├── data/               # Data layer
│   ├── api/            # Mouseion API client
│   ├── model/          # Data models
│   └── repository/     # Repositories
├── ui/                 # UI layer
│   ├── player/         # Now playing screen
│   ├── library/        # Library browsing
│   ├── settings/       # Settings screens
│   └── theme/          # Material3 theme
├── service/            # Background services
└── di/                 # Dependency injection
```

## Development status

**Phase 0**: ✅ Complete
- Audio foundation with advanced features
- Settings UI
- Mouseion API integration

**Next**: Waiting for Mouseion backend (Week 3-6)
