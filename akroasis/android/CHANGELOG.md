# Changelog

## [Unreleased] - Phase 0 Settings UI

### Added
- **SettingsScreen**: Main settings navigation
- **AudioSettingsScreen**: Audio quality controls
  - Equalizer toggle and preset selector
  - Playback speed slider (0.5x-2.0x)
  - Gapless playback toggle
  - Crossfade duration picker (0-10s)
  - USB DAC selector with capability display
- **NowPlayingScreen**: Settings button in top bar

### Changed
- **NowPlayingScreen**: Added Scaffold with TopAppBar

---

## [2025-12-30] - Phase 0 Advanced Audio

### Added
- **EqualizerEngine**: Parametric EQ using Android Equalizer API
  - 6 presets: Flat, Rock, Jazz, Classical, Pop, Bass Boost
  - Per-band level control
  - Session-based attachment
- **GaplessPlaybackEngine**: Dual AudioTrack architecture
  - Primary/secondary track switching
  - Preloading for < 50ms gaps
  - MODE_STATIC for small files, MODE_STREAM for large
- **CrossfadeEngine**: Volume ramping between tracks
  - Equal-power crossfade curve
  - Configurable duration (0-10s)
  - Fade in/out support
- **UsbDacDetector**: USB audio device detection
  - Capability reporting (sample rates, bit depth, channels)
  - High-res audio detection (96kHz+)
  - DSD format support detection
  - Preferred DAC selection
- **AudioPreferences**: Persistent audio settings storage

### Changed
- **AudioPlayer**: Integrated equalizer with session attachment
- **PlayerViewModel**: Exposed all audio controls to UI
- **AppModule**: Updated DI wiring for new engines

**PR**: #9

---

## [2025-12-30] - Audio Quality Features

### Added
- **Playback speed control**: 0.5x-2.0x range
- **Sleep timer**: Auto-stop with countdown (15/30/45/60 minutes)
- **ReplayGainProcessor**: Loudness normalization (track/album modes)
- **Audio format display**: Sample rate, bit depth, channels

### Changed
- **AudioPlayer**: Speed control via PlaybackParams (Android M+)
- **PlayerViewModel**: Sleep timer integration
- **NowPlayingScreen**: Audio format and speed display

**PR**: #8

---

## [2025-12-30] - Performance & Caching

### Changed
- **PlaybackQueue**: Thread safety with Mutex
- **MusicRepository**: Response caching with Room database (1-hour TTL)
- **MusicRepository**: Retry logic with exponential backoff
- **AudioPlayer**: Memory efficiency (MODE_STREAM for large files)

### Fixed
- Thread safety in queue operations
- Network resilience with automatic retries
- Memory usage for large audio files

**PR**: #7

---

## [2025-12-30] - Error Handling & UI

### Changed
- **Error handling**: Better error messages and recovery
- **Caching**: Response caching for offline usage
- **UI**: Loading states and error displays

**PR**: #6

---

## [2025-12-30] - Stability Fixes

### Fixed
- Thread safety in PlaybackQueue
- Memory leaks in audio playback
- Crash on rapid play/pause

**PR**: #5

---

## [2025-12-30] - Phase 2 Queue & Background

### Added
- **PlaybackQueue**: Queue management with shuffle/repeat
- **PlaybackService**: Background playback with MediaSession
- **Media controls**: Android notification controls

**PR**: #4

---

## [2025-12-30] - Phase 1 Authentication & Library

### Added
- **Authentication**: Login with Mouseion API
- **Library browsing**: Artist → Album → Track navigation
- **Album art**: Cover art display with Coil
- **Position tracking**: Resume from last position
- **Seek control**: Seekbar in Now Playing screen

**PR**: #3

---

## [2025-12-30] - Phase 0 Audio Pipeline

### Added
- **AudioPlayer**: Custom AudioTrack implementation
  - Bit-perfect mode support (Android 14+)
  - MODE_STATIC and MODE_STREAM support
- **TrackLoader**: Audio decoding with caching
- **NativeAudioDecoder**: FLAC/MP3/AAC decoding
- **NowPlayingScreen**: Basic playback UI
- **Library screens**: Artist/Album browsing

**PR**: #2

---

## [2025-12-30] - Repository Initialization

### Added
- **Project structure**: Android app with Jetpack Compose
- **Dependencies**: Hilt, Retrofit, Room, Coil
- **Basic navigation**: MainActivity and routing
- **Material3 theme**: Dark theme support

**PR**: #1
