# Changelog

All notable changes to the Akroasis project will be documented in this file.

## Android App

See [android/CHANGELOG.md](android/CHANGELOG.md) for detailed Android client changes.

## [2026-01-02] - Phase 1+3+6+7 Integration Complete (PR #18)

**Merged**: 84 files changed, 11,426 insertions, 82 deletions
**Tests**: 60 → 365+ tests (40-50% coverage)
**PR**: [#18](https://github.com/forkwright/akroasis/pull/18)

### Summary
Integrated 21 major features across 4 phases plus comprehensive quality improvements into unified Android client.

### Phase 1: Playback Excellence (6 features)
- Signal path visualization showing complete audio chain
- Gapless verification UI (<50ms threshold)
- Per-content playback speed memory (Track > Album > Default)
- Queue history (50-state undo/redo)
- Queue export (M3U/M3U8/PLS)
- Drag-to-reorder queue

### Phase 3: DSP Engine (5 features)
- 5-band parametric EQ (Android Equalizer API)
- AutoEQ profiles (HD600, HD650, DT770 Pro, ATH-M50x)
- Crossfeed engine (Low/Medium/High presets)
- Headroom management (-12dB to 0dB, peak monitoring)
- Custom EQ preset save/load

### Phase 6: Mobile Optimization (5 features)
- Media session controls (lock screen, notifications, Bluetooth)
- Playback notification manager
- State persistence (auto-restore on restart)
- Network monitoring (WiFi/cellular detection)
- Battery optimization

### Phase 7: Discovery & Scrobbling (3 features)
- Last.fm integration (MD5 auth, now playing, scrobbles)
- ListenBrainz integration (token auth)
- Playback speed-aware timestamps

### Quality Improvements
- Security: BuildConfig-based credential injection
- Safety: File size validation, race condition fixes, dynamic memory thresholds
- Testing: 305+ new tests added (comprehensive coverage across all phases)
- Integration tests: DSP chain, queue operations, scrobbling
- Manual testing: 100+ test case checklist completed

### Technical Notes
- Android database migration v1 → v2 (playback_speeds table)
- Dependencies: compose-reorderable:0.9.6
- Deferred: Upsampling, convolution (post-MVP)

**See**: [android/CHANGELOG.md](android/CHANGELOG.md) for complete technical details, code samples, and architectural decisions.

---

## [2026-01-04] - SonarCloud Quality Fixes (PR #31)

**Merged**: 350 issues fixed across 34 files
**PR**: [#31](https://github.com/forkwright/akroasis/pull/31)

### Fixed
- Removed 25 unused imports (Kotlin)
- Fixed 14 window→globalThis replacements (TypeScript)
- Added comments to 3 empty else blocks
- Fixed parseFloat→Number.parseFloat (2 occurrences)
- Removed zero fractions from numbers
- Added readonly to interface props (3 files)
- Merged nested if-else statement

### Issues Created
- #24: Security - Enable ProGuard obfuscation
- #25: Bug - Fix File.delete() return value checks
- #26: Bug - Remove duplicate condition in AudioPlayer.kt
- #27: Refactor - NowPlayingScreen complexity reduction
- #28: Refactor - EqualizerScreen complexity reduction
- #29: Refactor - SignalPathView complexity reduction
- #30: Refactor - Extract duplicated string constants

---

## [2026-01-04] - Search UI with Audio Quality Badges (PR #23)

**Merged**: Full-text search with DR/format/bit-perfect indicators
**PR**: [#23](https://github.com/forkwright/akroasis/pull/23)
**Platforms**: Android, Web

### Added
- **SearchBar**: Debounced search (300ms, 2-char minimum)
- **AudioQualityBadges**: Format [FLAC], Hi-Res [24/96], DR [DR14], Bit-Perfect [✓BP]
- **QuickFilterChips**: FLAC, Hi-Res, 24-bit, Bit-Perfect, DR>12
- **Badge color-coding**: DR green ≥14, yellow 10-13, orange 7-9, red <7

### Technical
- Real-time search results (<500ms for 10,000+ tracks)
- Result sorting: Relevance, Quality (DR), Sample Rate, Bit Depth
- Grouped results: Artists, Albums, Tracks

---

## [2026-01-03] - Phase 2 UI Scaffolding (PR #22)

**Merged**: Phase 2 UI components with mock data
**PR**: [#22](https://github.com/forkwright/akroasis/pull/22)
**Platform**: Android

### Added
- **DynamicRangeCard**: DR visualization with color coding
- **BitPerfectBadge**: Bit-perfect capability indicator
- **FocusFilterScreen**: Filter UI shell (placeholder dropdowns)
- **FilterRule data model**: Field types, operators, values
- **AudioAnalysis data model**: DR, format, lossless flag

### Status
- ✅ UI scaffolding complete
- ⏸️ Full features blocked on Mouseion API (search, filter, smart playlists)

---

## [2026-01-02] - Web PWA Offline Support (PR #21)

**Merged**: PWA features with offline caching
**PR**: [#21](https://github.com/forkwright/akroasis/pull/21)
**Platform**: Web

### Added
- **PWA manifest**: Icons, theme, standalone mode
- **Service worker**: Workbox-based offline caching
- **Cache strategies**: Cache-first for artwork, network-first for API
- **Install prompt**: Manual PWA installation trigger
- **Update notification**: New version detection

### Technical
- vite-plugin-pwa for service worker generation
- Offline caching for static assets (HTML, CSS, JS)
- Progressive artwork caching

---

## [2026-01-02] - Web App MVP Core Features (PR #20)

**Merged**: React-based web player with gapless playback
**PR**: [#20](https://github.com/forkwright/akroasis/pull/20)
**Platform**: Web

### Added
- **WebAudioPlayer**: Core playback engine using Web Audio API
- **Gapless playback**: Track preloading with <50ms transitions
- **Library browsing**: Artists/Albums/Tracks with artwork
- **Queue management**: Drag-to-reorder, add/remove tracks
- **Keyboard shortcuts**: 20+ commands (space, arrows, /, M, N, P, Q, L)
- **Media Session API**: Media keys and desktop notifications

### Technical Stack
- React 19 + Vite + TypeScript
- Zustand for state management
- TailwindCSS for styling
- @dnd-kit for drag-and-drop

### Success Criteria
- ✅ Audio plays from Mouseion streaming endpoint
- ✅ Gapless playback verified (<50ms gaps)
- ✅ Works in Chrome, Firefox, Safari
- ✅ Media keys functional

---

## [2026-01-02] - Documentation Updates (PR #19)

**Merged**: Post-PR#18 documentation updates
**PR**: [#19](https://github.com/forkwright/akroasis/pull/19)

### Updated
- ROADMAP.md with Phase 1+3+6+7 completion
- CHANGELOG.md with comprehensive feature documentation
- CONTRIBUTING.md with current project status

---

## [2025-12-31] - Quality Audit Remediation

Comprehensive security, safety, and quality improvements across Android client.

### Security
- Removed hardcoded API credentials (Last.fm)
- Implemented BuildConfig-based secret injection

### Safety
- Added file size validation (500MB limit)
- Fixed FlacDecoder race condition
- Dynamic memory thresholds per device

### Infrastructure
- Timber logging framework integration
- Typed error handling (sealed classes)

### Documentation
- Created comprehensive CHANGELOGs
- Updated project documentation
- Fixed style guide violations

See wrapper CHANGELOG.md for full details.
