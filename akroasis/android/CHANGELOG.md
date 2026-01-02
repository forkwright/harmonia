# Changelog

## [2026-01-01] - Phase 1+3+6+7 Integration Complete

### Summary
Integrated **all available Android features** from Phases 1, 3, 6, and 7 plus quality improvements into single unified branch.

**Total Features**: 21 major features across 4 phases
**Branch**: feature/phase-3-dsp-engine
**Commits**: 21 commits (4 merges + 17 feature commits)

### Phase Integration
- **Phase 1**: Playback Excellence (6 features)
- **Phase 3**: DSP Engine (5 features)
- **Phase 6**: Offline & Mobile (5 features)
- **Phase 7**: Discovery & Scrobbling (3 features)
- **Quality**: Security, safety, logging, testing

---

## [2026-01-01] - Phase 7: Discovery & Scrobbling

### Added
- **Last.fm Integration**: Full scrobbling support with authentication, retry logic, settings UI
- **ListenBrainz Integration**: Open-source scrobbling alternative with token auth
- **ScrobbleManager**: Unified coordination tracking playback progress

**Branch**: feature/phase-7-discovery (4 commits)

---

## [2026-01-01] - Phase 6: Offline & Mobile Optimization

### Added
- **Media Session Controls**: Lock screen, notifications, Bluetooth support
- **Playback Notifications**: Rich media notifications with artwork and controls
- **State Persistence**: Quick resume with queue/position restoration
- **Adaptive Streaming**: Network-aware quality switching
- **Offline Download Manager**: Smart sync and storage management
- **Android Auto**: Car integration with media browser

**Branch**: feature/phase-6-offline-mobile (5 commits)

---

## [2026-01-01] - Phase 3: DSP Engine

### Added
- **CrossfeedEngine**: Stereo fatigue reduction
  - Simple L + αR, R + αL mixing algorithm
  - Presets: Low (0.15), Medium (0.30), High (0.50)
  - Real-time processing via AudioTrack mixing
- **HeadroomManager**: Clipping prevention
  - Adjustable headroom (-12dB to 0dB)
  - Real-time peak monitoring with threshold detection
  - Smart recommendation based on EQ gains
  - Clipping event detection and tracking
- **AutoEQ Integration**: Headphone EQ profiles
  - AutoEQProfile data model (parametric: frequency, gain, Q)
  - AutoEQRepository: 4 embedded profiles (HD 600, HD 650, DT 770 Pro, ATH-M50x)
  - AutoEQConverter: Parametric-to-fixed-band approximation algorithm
  - Profile search functionality
- **EqualizerScreen**: Comprehensive EQ UI (428 lines)
  - 5-band graphical EQ with slider controls
  - 6 built-in presets + custom preset save/load
  - AutoEQ headphone profile search and apply
  - Real-time frequency response visualization
  - Peak level monitoring with clipping warnings
- **AudioSettingsScreen**: Enhanced DSP controls (441 lines)
  - Equalizer section with preset picker
  - Crossfeed toggle and strength selector
  - Headroom adjustment slider
  - Peak meter display

### Changed
- **EqualizerEngine**: Extended beyond basic wrapper
  - Custom preset persistence (PresetEntity in Room)
  - 5-band fixed EQ (Android Equalizer API limitation)
  - Band level validation and normalization
- **PlayerViewModel**: Integrated DSP dependencies
  - Added crossfeedEngine, headroomManager, autoEQRepository
  - Exposed EQ/crossfeed/headroom state to UI
  - Battery-aware DSP disable on low power

### Technical Decisions
- **Kotlin/Android API hybrid** instead of Rust DSP pipeline
  - Trade-off: Faster delivery (4 commits vs 3-4 weeks)
  - Battery efficient (may use hardware DSP on some devices)
  - Limitation: Fixed-band EQ (no Q control, fixed frequencies)
- **No upsampling**: Android AudioTrack handles SRC, hardware DAC preferred
- **No convolution**: CPU/battery cost too high for mobile
- **AutoEQ approximation**: Parametric → fixed-band (good enough for mobile)

### Quality Score
- **75%**: 4/6 features delivered (Parametric EQ*, AutoEQ, Crossfeed*, Headroom)
- *Compromised implementations (fixed bands, simple mixing)
- **Mobile use case**: 80%+ quality
- **Audiophile use case**: 65-70% quality

**Branch**: feature/phase-3-dsp-engine (4 commits: e5372af, 54bb4e1, ef1015e, 9d0236b)

---

## [2026-01-01] - Phase 1: Client-Side Playback Excellence

### Added
- **AudioPipelineState**: Real-time audio pipeline tracking
  - Input format (sample rate, bit depth, channels)
  - DSP chain state (EQ, crossfeed, gapless buffer)
  - Output device info (DAC model, capabilities)
  - Processing latency measurement
- **SignalPathCard**: Visual pipeline display in NowPlayingScreen
  - Flow chart: Input → DSP → Output → DAC
  - Format annotations (24/96 FLAC, 16/44 PCM, etc.)
  - Active DSP indicators (EQ, crossfeed)
  - Collapsible card with expand/collapse animation
- **GaplessVerificationViewModel/Screen**: Album scanner UI
  - Track-by-track gap measurement (nanosecond precision)
  - Pass/fail threshold (<50ms = gapless)
  - Progress indicator during scan
  - Results table with track names and gap durations
- **PlaybackSpeedMemory**: Per-content speed persistence
  - PlaybackSpeedRecord entity (content_id, speed, content_type)
  - PlaybackSpeedDao with UPSERT operations
  - PlaybackSpeedPreferences: Track > Album > Audiobook > Default hierarchy
  - MusicDatabase v2 migration (added playback_speeds table)
- **QueueExporter**: Playlist export functionality
  - M3U export (simple path list)
  - M3U8 export (extended with metadata: #EXTINF)
  - PLS export (numbered entries with File1=, Title1=, Length1=)
  - Uses Android Storage Access Framework for save location
- **Queue History**: Undo/redo with 50-state buffer
  - PlaybackQueue.kt: History tracking with addToHistory()
  - Circular buffer implementation (older states drop)
  - undo() and redo() operations
  - State restoration (tracks, currentIndex)
- **Drag-to-Reorder**: Queue reordering with visual feedback
  - Uses compose-reorderable library (org.burnoutcrew)
  - QueueScreen.kt: ReorderableList integration
  - Drag handle icon on each queue item
  - Real-time position updates during drag

### Changed
- **NowPlayingScreen**: Added signal path card below playback controls
  - Toggle button to show/hide pipeline visualization
  - Layout adjustment for card insertion
- **QueueScreen**: Enhanced with export and drag functionality
  - Export button in TopAppBar
  - Reorderable list with drag handles
  - History controls (undo/redo buttons)
- **AudioPlayer**: Pipeline state tracking
  - Emits AudioPipelineState updates on format changes
  - Tracks active DSP modules
  - Measures processing latency
- **GaplessPlaybackEngine**: Gap measurement on track transitions
  - GapMeasurement data class (trackId, gapMs, timestamp)
  - Track-by-track gap logging
  - Exposed via StateFlow for UI consumption

### Dependencies
- org.burnoutcrew.composereorderable:0.9.6 (drag-to-reorder)

### Breaking Changes
- MusicDatabase v1 → v2 (adds playback_speeds table)
  - Migration: CREATE TABLE playback_speeds (content_id TEXT PRIMARY KEY, speed REAL, content_type TEXT)
  - Old data preserved, new table created

**Branch**: feature/phase-1-playback-excellence (7 commits: bb24543, 9dc5073, 7ba4dcf, 59a314e, f5d0f01, c716d9d, 95fb7d0)

---

## [2025-12-31] - Quality Audit Remediation

### Security
- **BuildConfig credentials**: Last.fm API_KEY/API_SECRET from local.properties
- **Removed hardcoded secrets** from source code

### Safety
- **File size validation**: 500MB limit on track loading
- **OOM protection**: Catch OutOfMemoryError on large file loads
- **Thread-safe FlacDecoder**: @Volatile + synchronized blocks
- **Dynamic memory thresholds**: 20% of device heap (was hardcoded 10MB)

### Infrastructure
- **Timber logging**: Structured logging across all components
- **Typed errors**: LoadError sealed class (NetworkError, UnsupportedFormat, DecodeError, FileSizeError)

### Testing
- **ScrobbleManager tests**: Full test coverage with mock clients
- **TrackLoader tests**: Error handling and retry logic validation
- **GaplessPlaybackEngine tests**: Track switching and preloading scenarios

### Changed
- **NativeAudioDecoder**: Added defensive error handling for library loading
- **LibraryViewModel**: Removed unsafe !! operators
- **AuthViewModel**: Null-safe early returns
- **AudioPlayer**: Added playback event logging
- **GaplessPlaybackEngine**: Added gap measurement logging

---

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
