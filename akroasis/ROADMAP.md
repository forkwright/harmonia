# Akroasis Development Roadmap

Phased implementation plan for unified media player (audiobooks, ebooks, music) with bit-perfect audio.

**Estimated total timeline**: 6-8 months to production release

---

## 🎯 Current Status (2026-01-02)

**Completed Phases**: 0, 1, 3, 6, 7 (21 major features, 365+ tests, 40-50% coverage)

### ✅ Phase 0: Foundation - COMPLETE (2025-12-31)
- Rust audio core with bit-perfect pipeline (16/24/32-bit native preservation)
- Android build environment and JNI integration
- APK builds with error handling and diagnostics

### ✅ Phase 1: Playback Excellence - COMPLETE (2026-01-01)
- Signal path visualization showing complete audio chain
- Gapless playback verification UI (<50ms threshold)
- Per-content playback speed memory (Track > Album > Default hierarchy)
- Queue history with 50-state undo/redo
- Queue export (M3U/M3U8/PLS formats)
- Drag-to-reorder queue management

### ✅ Phase 3: DSP Engine - COMPLETE (2026-01-01)
- 5-band parametric EQ using Android Equalizer API
- AutoEQ profiles (HD600, HD650, DT770 Pro, ATH-M50x)
- Crossfeed engine with Low/Medium/High presets
- Headroom management (-12dB to 0dB, peak monitoring, clipping detection)
- Custom EQ preset save/load functionality
- ⏸️ Deferred: Upsampling, Convolution (post-MVP)

### ✅ Phase 6: Mobile Optimization - COMPLETE (2026-01-01)
- Media session integration (notification/lock screen controls)
- Playback notification manager
- State persistence (auto-restore playback on restart)
- Network monitoring (WiFi/cellular detection with adaptive streaming)
- Battery optimization

### ✅ Phase 7: Discovery & Scrobbling - COMPLETE (2026-01-01)
- Last.fm integration (MD5 auth, now playing, scrobbles)
- ListenBrainz integration (token auth, concurrent submission)
- Playback speed-aware timestamp calculation
- Scrobble settings UI

### ⏸️ Phase 2: Audio Intelligence - UI Complete, Features Blocked

**UI Status:** ✅ COMPLETE (PR #22, #23)
**Full Features:** ⏸️ BLOCKED on Mouseion APIs

**Completed UI Components:**
- ✅ Search bar with audio quality badges (PR #23)
- ✅ DynamicRangeCard with color coding
- ✅ BitPerfectBadge component
- ✅ FocusFilterScreen shell
- ✅ QuickFilterChips (FLAC, Hi-Res, 24-bit, etc.)

**Blocked on Mouseion APIs:**
- `/api/v3/search?q={query}` - Enhanced search with audio metadata
- `/api/v3/library/filter` - Complex filtering
- `/api/v3/tracks/{id}/audio-analysis` - DR values, format details
- `/api/v3/playlists/smart` - Smart playlist CRUD

**GitHub Issues:** #39 (Search), #40 (Filters), #41 (Smart Playlists), #42 (Bit-Perfect Logic)

### ✅ Web App MVP - COMPLETE (2026-01-02)

**Status:** ✅ COMPLETE
**Stack:** React 19 + Vite + TypeScript + TailwindCSS
**Completed PRs:** #20 (Core), #21 (PWA), #23 (Search)

**Completed Features:**
- ✅ Web Audio API playback engine with gapless transitions
- ✅ Library browsing (artists/albums/tracks)
- ✅ Queue management with drag-to-reorder (@dnd-kit)
- ✅ PWA with offline support (Workbox service worker)
- ✅ Media Session API (media keys, desktop notifications)
- ✅ Full-text search with audio quality badges
- ✅ Keyboard shortcuts (space, arrows, M, N, P, /, Q, L)
- ✅ Zustand state management
- ✅ Responsive design (desktop + mobile web)

**Browser Limitations Accepted:**
- Resampling may occur (not bit-perfect)
- Format support browser-dependent (FLAC, AAC, MP3, Opus)

**Recent Achievement**: PR #18 merged 21 features across 4 phases (84 files changed, 11,426 insertions, 82 deletions)

---

## Phase 0: Research & Foundation

**Duration**: 2-3 weeks
**Status**: ✅ COMPLETE (2025-12-31)

### Goals
- Validate Sony Walkman API access for bit-perfect audio
- Design audio pipeline architecture
- Document Mouseion API completely
- Finalize technology stack
- Initialize project repositories

### Key Tasks
- [ ] Research Sony Developer Program (API for dedicated audio route)
- [ ] Audit UAPP bit-perfect implementation approach
- [ ] Research Android 14 BIT_PERFECT mode details
- [ ] Evaluate ExoPlayer vs. custom AudioTrack
- [ ] Document all Mouseion REST endpoints
- [ ] Create OpenAPI spec for Mouseion API
- [ ] Decide: Vue.js vs. React for web
- [ ] Set up Android project (Kotlin + Jetpack Compose)
- [ ] Set up Web project

### Success Criteria
-  Sony Walkman constraints documented (API access yes/no)
-  Audio pipeline architecture decided and documented
-  Mouseion API fully specified with OpenAPI
-  Technology stack selected and justified
-  Both Android and Web projects initialized

---

## Phase 1: Mouseion Backend Preparation

**Duration**: 1-2 weeks
**Status**: ⏸️ BLOCKED - Waiting for Mouseion Phase 1 completion (Week 7-8)

**Note**: Original high-level phases were reorganized during implementation. See **Current Status** section above for detailed breakdown of actual phases completed (0, 1, 3, 6, 7).

### Goals
- Ensure Mouseion provides all APIs needed by Akroasis
- Add streaming, progress tracking, playlist endpoints
- Validate authentication flow

### Key Tasks
- [ ] Audit Mouseion API completeness
- [ ] Add `/api/v3/stream/{mediaId}` if missing
- [ ] Add audiobook progress tracking endpoint
- [ ] Add playlist management endpoints
- [ ] Verify HTTP 206 range request support
- [ ] Test authentication from external client
- [ ] Update OpenAPI spec

### Success Criteria
-  All Akroasis requirements satisfied by Mouseion API
-  Streaming works with range requests (seeking)
-  Authentication flow validated from client

---

## Phase 2: Android App Foundation

**Duration**: 4-6 weeks
**Status**: ✅ SUBSTANTIALLY COMPLETE - Core features implemented across actual Phases 0, 1, 3, 6, 7

**Note**: This original high-level phase was broken down into multiple focused implementation phases during development. See **Current Status** section above for what's been completed. Remaining work (Audio Intelligence features) blocked on Mouseion APIs.

### Goals
- Build core Android infrastructure
- Implement bit-perfect audio pipeline
- Launch music player (first media type)
- Basic offline sync

### Key Tasks
- [ ] Initialize Kotlin Android project (Jetpack Compose, Hilt, Retrofit, Room)
- [ ] Implement bit-perfect audio (based on Phase 0 decision)
  - Option A: Sony API integration
  - Option B: Android 14 BIT_PERFECT mode + AudioTrack
- [ ] Implement gapless playback
- [ ] Support FLAC, ALAC, WAV, DSD, high-res PCM
- [ ] Music library browsing (Artist → Album → Track)
- [ ] Playback controls + queue management
- [ ] Now playing UI with artwork
- [ ] Background playback (MediaSession API)
- [ ] Android Auto / Bluetooth controls
- [ ] Basic download manager and offline playback

### Success Criteria
-  Bit-perfect playback verified on target devices
-  Music playback functional (browse, play, queue)
-  Gapless playback < 50ms
-  Background playback with media controls
-  Offline sync working

---

## Phase 3: Web App Foundation

**Duration**: 2 weeks (completed faster than estimated)
**Status**: ✅ COMPLETE (2026-01-02)

**Note**: See **Web App MVP** section in Current Status above for detailed completion info.

### Goals
- Build web-based player for desktop/browsers ✅
- Web Audio API playback (not bit-perfect, browser limitations accepted) ✅
- Desktop PWA ✅

### Key Tasks
- [x] Initialize web project (React 19 + Vite + TypeScript + Tailwind + Zustand)
- [x] Implement Web Audio API playback
- [x] Support FLAC, AAC, MP3, Opus (browser-dependent)
- [x] Gapless playback (preload next track)
- [x] Music library browsing
- [x] Playback controls + queue with drag-reorder
- [x] Now playing UI
- [x] Keyboard shortcuts (20+ commands)
- [x] Service worker for offline caching (Workbox)
- [x] PWA manifest (installable)
- [x] Media session API (desktop media keys)
- [x] Full-text search with audio quality badges (PR #23)

### Success Criteria
- ✅ Web player functional in Chrome, Firefox, Safari
- ✅ Gapless playback working (<50ms transitions)
- ✅ PWA installable on desktop
- ✅ Media keys working

**Completed PRs:** #20 (Core playback + queue), #21 (PWA), #23 (Search UI)

---

## Phase 4: Linux Native App

**Duration**: 4-5 weeks
**Status**:  Pending

### Goals
- Native Linux desktop app with bit-perfect audio
- PipeWire integration for modern audio stack
- Consistent UX with Android/Web clients

### Key Tasks

**Linux Native (C++ or Rust)**:
- [ ] Choose framework (Qt6, GTK4, or Tauri)
- [ ] Integrate libFLAC + DSD decoder (same as Android)
- [ ] Implement PipeWire audio output (bit-perfect capable)
- [ ] Fallback to PulseAudio for older systems
- [ ] Music library browsing
- [ ] Playback controls + queue management
- [ ] Gapless playback implementation
- [ ] MPRIS D-Bus integration (media keys, desktop integration)
- [ ] System tray icon and notifications
- [ ] Desktop file and app icon

**Audio Pipeline**:
- [ ] PipeWire native output (preferred, bit-perfect)
- [ ] PulseAudio fallback (transparent)
- [ ] ALSA direct output (optional, expert mode)
- [ ] Sample rate preservation (no resampling)
- [ ] DSD playback via DoP

**Packaging**:
- [ ] AppImage (universal, portable)
- [ ] Flatpak (sandboxed, Flathub distribution)
- [ ] .deb package (Debian/Ubuntu)
- [ ] AUR package (Arch Linux)

### Success Criteria
-  Bit-perfect playback verified on PipeWire
-  Music playback functional
-  Gapless < 10ms
-  MPRIS integration working
-  Packaged for major distros

---

## Phase 5: Unified Media Interface

**Duration**: 6-8 weeks
**Status**:  Pending

### Goals
- Add audiobook player (Android + Web + Linux)
- Add ebook reader (Android + Web + Linux)
- Unified navigation across all three media types

### Key Tasks

**Audiobook (Android)**:
- [ ] Library browsing (Author → Series → Book)
- [ ] Chapter navigation
- [ ] Position tracking and resume
- [ ] Sleep timer
- [ ] Playback speed (0.5x - 3x)
- [ ] Bookmarks

**Audiobook (Web)**:
- [ ] Library browsing
- [ ] Playback with position tracking
- [ ] Chapter navigation and bookmarks

**eBook (Android)**:
- [ ] Integrate EPUB reader (FolioReader-Android)
- [ ] Library browsing
- [ ] Reading position sync
- [ ] Highlights and notes
- [ ] Font/theme customization

**eBook (Web)**:
- [ ] Integrate EPUB.js
- [ ] Library browsing
- [ ] Reading position sync
- [ ] Highlights and notes

**Audiobook (Linux)**:
- [ ] Library browsing
- [ ] Playback with position tracking
- [ ] Chapter navigation and bookmarks
- [ ] Sleep timer, playback speed

**eBook (Linux)**:
- [ ] Integrate EPUB reader library (foliate, ePub.js via WebView)
- [ ] Library browsing
- [ ] Reading position sync
- [ ] Highlights and notes

**Unified Navigation**:
- [ ] Bottom nav / sidebar (Music, Audiobooks, eBooks)
- [ ] Unified search across all types
- [ ] Unified recent/favorites
- [ ] Consistent UI patterns

### Success Criteria
-  All three media types playable/readable
-  Unified navigation and search
-  Position tracking syncs across devices
-  Consistent UX across types

---

## Phase 6: Advanced Features

**Duration**: 4-5 weeks
**Status**:  Pending

### Goals
- Advanced audio features (EQ, ReplayGain, crossfade)
- Playlists and collections
- Advanced sync capabilities
- Discovery and recommendations
- Last.fm integration (scrobbling, stats)

### Key Tasks

**Audio Features**:
- [ ] Parametric EQ (AutoEQ database)
- [ ] ReplayGain support
- [ ] Crossfade between tracks
- [ ] Loudness normalization
- [ ] Audio effects (reverb, bass boost)

**Library & Playlists**:
- [ ] Playlist creation and management
- [ ] Smart playlists (auto-generated)
- [ ] Collections (group books by theme)
- [ ] Genre/mood browsing

**Sync & Storage**:
- [ ] Selective sync (choose downloads)
- [ ] Background sync with notifications
- [ ] Storage management

**Discovery & Insights**:
- [ ] Recently added, recommendations
- [ ] Listening statistics and insights
- [ ] Last.fm OAuth authentication
- [ ] Last.fm scrobbling (now playing + completed tracks)
- [ ] Last.fm stats display (top artists/albums/tracks)
- [ ] Loved tracks integration

### Success Criteria
-  EQ and audio effects working
-  Playlists sync across devices
-  Advanced sync with storage management
-  Discovery features engaging
-  Last.fm scrobbles reliably (>99% success)

---

## Phase 7: Polish & Release

**Duration**: 3-4 weeks
**Status**:  Pending

### Goals
- Performance optimization complete
- Extensive testing on all platforms (Walkman, Pixel, Linux desktop)
- Complete documentation
- Public release

### Key Tasks

**Performance Optimization**:
- [ ] Profile and optimize Android (memory, CPU, battery)
- [ ] Profile and optimize Linux (memory, CPU)
- [ ] Optimize web bundle size (lazy loading, code splitting)

**Platform Testing**:
- [ ] Extensive Sony Walkman testing (audio quality, battery life)
- [ ] Pixel 10 XL testing (bit-perfect USB DAC verification)
- [ ] Linux desktop testing (PipeWire, PulseAudio, multiple distros)
- [ ] Verify audio quality across all platforms (spectral analysis)
- [ ] Cross-platform position sync verification

**Quality Assurance**:
- [ ] Unit tests (core logic, all platforms)
- [ ] Integration tests (API client)
- [ ] UI tests (critical flows)
- [ ] Audio quality verification (spectral analysis)

**Documentation**:
- [ ] User guide (setup, features, all platforms)
- [ ] Developer documentation
- [ ] Contribution guidelines
- [ ] Platform-specific installation guides

**Release Packaging**:
- [ ] Google Play Store listing (Android)
- [ ] Web app deployment (hosting)
- [ ] Linux packages (AppImage, Flatpak, .deb, AUR)
- [ ] GitHub release with binaries for all platforms

### Success Criteria
-  App performs well on all platforms
-  Audio quality verified (Walkman transparent, Pixel/Linux bit-perfect)
-  Cross-platform sync working reliably
-  Critical bugs resolved
-  Documentation complete
-  Public release published for all platforms

---

## Phase 8: Last.fm Smart Discovery

**Duration**: 3-4 weeks
**Status**:  Future
**Dependencies**: Requires Mouseion backend support

### Goals
- Smart wishlists (automated library expansion based on Last.fm)
- Intelligent playlist generation combining local + Last.fm data
- Enhanced music discovery features

### Key Tasks

**Mouseion Backend** (coordinate with Mouseion agent):
- [ ] Last.fm API proxy/cache layer
- [ ] Import lists (smart wishlists)
  - Top artists albums (weekly/monthly/yearly)
  - Similar artists to favorites
  - Recommendations based on listening history
- [ ] Smart playlist generation engine
- [ ] Background jobs for wishlist sync
- [ ] MusicBrainz metadata matching

**Akroasis Client**:
- [ ] Import list configuration UI
- [ ] Smart wishlist management
- [ ] Smart playlist display and playback
- [ ] Discovery UI (similar artists, recommendations)
- [ ] Listening insights dashboard
- [ ] Library growth analytics

**Integration**:
- [ ] Automatic album discovery workflow
- [ ] Wishlist → Download → Import → Sync → Play
- [ ] Playlist refresh triggers
- [ ] Cache management for Last.fm data

### Success Criteria
-  Smart wishlists discover 10+ albums monthly
-  Smart playlists generate in <2 seconds
-  Discovery features increase engagement 20%
-  Automated library growth working end-to-end

---

## Post-Release: AudioPi Replacement

**Timeline**: Post-Phase 8
**Status**:  Future

### Goal
Replace AudioPi as dedicated bit-perfect audio endpoint in homelab.

### Requirements
- **Headless mode**: Run without GUI (systemd service)
- **Network control**: Full API control via Mouseion backend
- **Bit-perfect output**: PipeWire passthrough to high-end DAC
- **Low resource usage**: Efficient on Pi/SBC hardware
- **Auto-start**: Boot directly into playback service
- **Remote control**: Control via Android/Web clients
- **Zone support**: Multi-room audio (Phase 8+)

### Implementation
- [ ] Headless service mode (no X11/Wayland required)
- [ ] Systemd unit file for auto-start
- [ ] MPD protocol compatibility (optional, for existing clients)
- [ ] Low-power optimization for SBC hardware
- [ ] Integration with existing homelab infrastructure (see anamnesis docs)

---

## Success Metrics (Project-Wide)

### Audio Quality (Primary Goal)
- [ ] Bit-perfect playback on Android 14+ devices with USB DAC
- [ ] Sony Walkman playback quality meets user expectations
- [ ] Gapless playback < 50ms gap (ideally 0ms)
- [ ] High-res audio (24/96, 24/192) at native sample rates
- [ ] DSD playback functional (if hardware supports)

### Performance
- [ ] Android battery life within 10% of native Sony app
- [ ] App cold start < 2 seconds
- [ ] Library browsing smooth (60fps) with 10,000+ items
- [ ] Web app loads < 3 seconds (initial)

### User Experience
- [ ] All media types accessible within 2 taps
- [ ] Search returns results < 500ms (local cache)
- [ ] Offline sync reliable without conflicts
- [ ] UI consistent and intuitive across types

### Integration
- [ ] Mouseion API calls succeed > 99.9%
- [ ] Position sync across devices < 5 seconds
- [ ] Playlists sync reliably

---

## Key Risks & Mitigation

### Risk 1: Sony Walkman API Unavailable
**Impact**: High - Cannot achieve true bit-perfect on primary target
**Mitigation**: Optimize for 192kHz/32-bit pipeline, focus on gapless/EQ/features

### Risk 2: Android BIT_PERFECT Insufficient
**Impact**: Medium - May not work as expected
**Mitigation**: Research UAPP implementation, consider alternatives

### Risk 3: ExoPlayer Limitations
**Impact**: Medium - May need custom audio implementation
**Mitigation**: Budget time for custom AudioTrack in Phase 2

### Risk 4: Mouseion API Changes
**Impact**: Low-Medium - Backend changes could break client
**Mitigation**: Version API, use OpenAPI spec for contract testing

---

## Technology Stack Summary

### Android
- Kotlin, Jetpack Compose
- Custom AudioTrack + BIT_PERFECT mode (or Sony API)
- Retrofit + OkHttp, Room, Hilt
- MVVM with Coroutines + Flow

### Web
- Vue.js (user familiar) OR React
- Web Audio API, EPUB.js, Tailwind CSS
- Pinia (Vue) or Zustand (React)

### Linux Native
- Framework: Qt6, GTK4, or Tauri (TBD Phase 4)
- Audio: PipeWire (primary), PulseAudio (fallback), ALSA (optional)
- Decoders: libFLAC, DSD decoder (shared with Android)
- Desktop Integration: MPRIS D-Bus, system tray
- Packaging: AppImage, Flatpak, .deb, AUR

### Backend (External)
- Mouseion: C# .NET 8.0 REST API
- Enhancements: Streaming, progress tracking, playlists

---

## Deferred Features (Post-MVP)

These features were considered during development but deferred due to complexity, resource constraints, or platform limitations:

### Audio Processing
- **Upsampling**: Android AudioTrack handles sample rate conversion; hardware DAC preferred for quality
- **Convolution Engine**: CPU/battery cost too high for mobile; deferred to desktop/server-side
- **True Parametric EQ**: Android Equalizer API limited to 5-band fixed; AutoEQ profiles cover 95% of use cases

### Platform Limitations
- **Bit-Perfect on Web**: Browser audio stack resamples; accepted limitation
- **DSD Native Playback**: Limited DAC support; DoP fallback implemented

### Future Consideration
- Multi-threaded decoding (single-stream playback doesn't benefit)
- Hardware-accelerated transcoding (explicitly excluded per ROADMAP)
- Vim-style navigation (niche use case)
- Collaborative smart playlists (post-MVP)

---

## References

**Audio Technology**:
- [Android BIT_PERFECT Mode](https://source.android.com/docs/core/audio/preferred-mixer-attr)
- [ExoPlayer Discussion](https://github.com/androidx/media/issues/415)
- [UAPP](https://www.extreamsd.com/index.php/products/usb-audio-player-pro)

**Sony Walkman**:
- [WM1AM2 Optimization](https://www.head-fi.org/threads/sony-wm1am2-and-wm1zm2-android-walkman-optimization-guide.962975/)
- [NW-A300 Series](https://www.head-fi.org/threads/new-sony-walkman-nw-a300-series-android-12.966467/)

**Reference Apps**:
- [Symfonium](https://symfonium.app)
- [Plexamp](https://www.plex.tv/plexamp/)

**Backend**:
- [Mouseion](https://github.com/forkwright/mouseion)
