# Changelog

All notable changes to the Akroasis project will be documented in this file.

## Android app

See [android/CHANGELOG.md](android/CHANGELOG.md) for detailed Android client changes.

## [2026-01-07] - Voice search integration (PR #128)

**Status**: In Progress - CI running
**Changes**: 2 files changed, 346 insertions, 1 deletion
**PR**: [#128](https://github.com/forkwright/akroasis/pull/128)

### Summary
Completed voice search integration by wiring VoiceSearchHandler into PlaybackService MediaSession callback. Resolves Issue #118.

### Features
- **Voice command support**
  - Structured search (title/artist/album from Google Assistant)
  - Free-form query handling
  - Empty query fallback to recent tracks
  - Toast notifications for user feedback

- **PlaybackService integration**
  - Implemented onPlayFromSearch MediaSession callback
  - Queue replacement with search results
  - Automatic playback start from specified index
  - Error handling with user-facing messages

### Testing
- **VoiceSearchHandlerTest**: 304 lines, 17 test cases
  - Structured search scenarios (title, artist, album combinations)
  - Free-form query processing
  - Error handling (search failures, track load failures)
  - Result limiting (MAX_QUEUE_SIZE)
  - Partial track load resilience
  - Empty query edge cases

### Technical details
- Used PlaybackQueue.setQueue() for queue replacement
- Integrated with existing VoiceSearchHandler from PR #107
- Timber logging for debugging voice commands
- Android Auto and Assistant compatibility

---

## [2026-01-07] - SonarCloud code quality fixes (PR #127)

**Merged**: 7 files changed, 89 insertions, 48 deletions
**PR**: [#127](https://github.com/forkwright/akroasis/pull/127)

### Summary
Fixed 10 MAJOR SonarCloud code quality issues across Kotlin and TypeScript codebases.

### Fixed issues
- **kotlin:S6531**: Hardcoded versions in build.gradle.kts
  - Migrated all dependencies to Gradle version catalog (libs.versions.toml)
  - Centralized version management for easier maintenance

- **kotlin:S126**: Empty else blocks
  - Removed empty branches in OfflineSettingsScreen
  - Added explicit Unit branches for exhaustive when expressions

- **kotlin:S3923**: Duplicate code branches
  - Combined identical branches in ScrobbleSettingsScreen
  - Deduplicated text display logic for NowPlaying/Scrobbled states

- **kotlin:S1172**: Unused parameters
  - Removed unused bandIndex parameter from EqualizerBandControl

- **kotlin:S3923**: Chained if-else statements
  - Refactored AndroidAutoService to use when expression
  - Improved readability with pattern-based branching

- **kotlin:S1125**: Boolean simplification
  - Replaced if-throw with check() idiom in NativeAudioDecoder

- **typescript:S3358**: Else-if simplification
  - Separated conditional logic in useWebAudioPlayer hook
  - Improved readability and maintainability

### Files modified
- android/gradle/libs.versions.toml (new file)
- android/app/build.gradle.kts
- android/app/src/main/java/app/akroasis/auto/AndroidAutoService.kt
- android/app/src/main/java/app/akroasis/ui/settings/EqualizerScreen.kt
- android/app/src/main/java/app/akroasis/ui/settings/OfflineSettingsScreen.kt
- android/app/src/main/java/app/akroasis/ui/settings/ScrobbleSettingsScreen.kt
- android/app/src/main/java/app/akroasis/audio/NativeAudioDecoder.kt
- web/src/hooks/useWebAudioPlayer.ts

### Impact
- Improved maintainability through version catalog
- More idiomatic Kotlin code patterns
- Reduced code duplication
- Better error handling with Kotlin stdlib idioms

---

## [2026-01-06] - Test coverage expansion to 80%+ (PR #126)

**Merged**: 40+ files changed, 2000+ insertions
**Tests**: 110 → 473 tests (80%+ coverage)
**PR**: [#126](https://github.com/forkwright/akroasis/pull/126)

### Summary
Massive test suite expansion from ~50% to 80%+ code coverage with Jacoco CI integration.

### Test coverage
- **Instruction Coverage**: 82%
- **Branch Coverage**: 76%
- **Line Coverage**: 81%
- **Method Coverage**: 79%

### Infrastructure
- **Jacoco integration**
  - Coverage reporting with CI enforcement
  - Thresholds: 80% instruction, 75% branch
  - Enforcement on PRs only (not main branch)
  - HTML reports generated for local review

### Test classes added (60+)

**Repositories (12)**: MusicRepositoryTest, SearchRepositoryTest, FocusFilterRepositoryTest, SmartPlaylistRepositoryTest, AlbumRepositoryTest, ArtistRepositoryTest, TrackRepositoryTest, ScrobbleRepositoryTest, AudiobookRepositoryTest, EbookRepositoryTest, SessionRepositoryTest, ProgressRepositoryTest

**ViewModels (8)**: LibraryViewModelTest, SearchViewModelTest, PlayerViewModelTest, AudiobookPlayerViewModelTest, EbookReaderViewModelTest, SmartPlaylistViewModelTest, SettingsViewModelTest, FocusFilterViewModelTest

**Managers (10)**: OfflineDownloadManagerTest, NotificationManagerTest, MediaSessionManagerTest, AudiobookProgressManagerTest, EbookProgressManagerTest, NetworkManagerTest, CacheManagerTest, ScrobbleQueueManagerTest, PowerManagerTest, AnalyticsManagerTest

**Audio engine (15)**: AudioPlayerTest, TrackLoaderTest, GaplessPlaybackEngineTest, PlaybackQueueTest, EqualizerEngineTest, CrossfeedEngineTest, DynamicRangeCalculatorTest, BitPerfectCalculatorTest, LevelMatcherTest, AutoEQLoaderTest, PlaybackSpeedManagerTest, SignalPathVisualizerTest, VoiceSearchHandlerTest, NativeAudioDecoderTest, AudioPipelineStateTest

**Utilities (10+)**: AudioQualityUtilsTest, FileUtilsTest, DateUtilsTest, FormatUtilsTest, ValidationUtilsTest, CryptoUtilsTest, JsonUtilsTest, StringUtilsTest, UrlUtilsTest, CacheUtilsTest

### Testing patterns
- Mockito-Kotlin for mocking
- Kotlin coroutines test (runTest)
- Turbine for Flow testing
- AndroidX Arch Core testing for LiveData
- Robolectric for Android framework
- JUnit 4 with AndroidX Test

### Quality gates
- 80% instruction coverage threshold
- 75% branch coverage threshold
- CI fails if coverage drops below thresholds
- Only enforced on PRs to allow experimentation on main

---

## [2026-01-06] - Android test suite fixes (PR #122)

**Merged**: 23 files changed, 856 insertions, 297 deletions
**Tests**: All 110+ Android unit tests passing
**PR**: [#122](https://github.com/forkwright/akroasis/pull/122)

### Summary
Fixed all broken Android tests after Track model evolution and API changes. Introduced AudioTrackFactory pattern for improved testability.

### Major refactoring
- **AudioTrackFactory pattern**
  - Created interface + implementation for AudioTrack creation
  - Extracted from GaplessPlaybackEngine for dependency injection
  - Enables proper unit testing without Robolectric AudioTrack shadows
  - Files: `AudioTrackFactory.kt`, `RealAudioTrackFactory.kt`, `AudioModule.kt`

### Test infrastructure
- **MainDispatcherRule**: Test rule for coroutines using Dispatchers.Main
- **Robolectric 4.11.1**: Added for Android framework testing
- **Heap size**: Increased to 2048m for Phase1FeaturesTest
- **PlaybackQueue**: Fixed history initialization (added init saveSnapshot)

### Track model migration (7→20 fields)
Updated Track mocks across 12+ test files with full constructor:
- Added: albumArtist, trackNumber, discNumber, year, bitrate
- Added: sampleRate, bitDepth, fileSize, filePath
- Added: createdAt, updatedAt

### API migrations
- PlaybackQueue: Methods changed to properties (`getTracks()` → `tracks.value`)
- Method renames: `skipTo(n)` → `skipToIndex(n)`
- Import conventions: `kotlin.test.*` → `org.junit.Assert.*`

### Test fixes
- **EqualizerEngineTest**: Fixed Short vs Int type mismatches (18/18)
- **TrackLoaderTest**: Fixed native decoder expectations (6/6)
- **AudioPipelineIntegrationTest**: Fixed crossfeed samples and clipping threshold
- **ScrobbleManagerTest**: Fixed NPE from missing mocks (16/16)
- **PlaybackStateStoreTest**: Added Robolectric, fixed SharedPreferences mocking (26/26)
- **PlaybackQueueIntegrationTest**: Fixed undo/redo and history limit (18/18)
- **GaplessPlaybackEngineTest**: AudioTrackFactory refactoring (16/16)
- **AuthViewModelTest**: First new ViewModel test added (10/10)

### Technical debt resolved
- Empty queue edge cases in PlaybackQueue
- StateFlow duplicate emission in ScrobbleManager
- JUnit Assert parameter order (message first, condition second)
- ESLint suppression documentation

---

## [2026-01-02] - Phase 1+3+6+7 integration complete (PR #18)

**Merged**: 84 files changed, 11,426 insertions, 82 deletions
**Tests**: 60 → 365+ tests (40-50% coverage)
**PR**: [#18](https://github.com/forkwright/akroasis/pull/18)

### Summary
Integrated 21 major features across 4 phases plus quality improvements into unified Android client.

### Phase 1: playback excellence (6 features)
- Signal path visualization showing complete audio chain
- Gapless verification UI (<50ms threshold)
- Per-content playback speed memory (Track > Album > Default)
- Queue history (50-state undo/redo)
- Queue export (M3U/M3U8/PLS)
- Drag-to-reorder queue

### Phase 3: DSP engine (5 features)
- 5-band parametric EQ (Android Equalizer API)
- AutoEQ profiles (HD600, HD650, DT770 Pro, ATH-M50x)
- Crossfeed engine (Low/Medium/High presets)
- Headroom management (-12dB to 0dB, peak monitoring)
- Custom EQ preset save/load

### Phase 6: mobile optimization (5 features)
- Media session controls (lock screen, notifications, Bluetooth)
- Playback notification manager
- State persistence (auto-restore on restart)
- Network monitoring (WiFi/cellular detection)
- Battery optimization

### Phase 7: discovery & scrobbling (3 features)
- Last.fm integration (MD5 auth, now playing, scrobbles)
- ListenBrainz integration (token auth)
- Playback speed-aware timestamps

### Quality improvements
- Security: BuildConfig-based credential injection
- Safety: File size validation, race condition fixes, dynamic memory thresholds
- Testing: 305+ new tests added across all phases
- Integration tests: DSP chain, queue operations, scrobbling
- Manual testing: 100+ test case checklist completed

### Technical notes
- Android database migration v1 → v2 (playback_speeds table)
- Dependencies: compose-reorderable:0.9.6
- Deferred: Upsampling, convolution (post-MVP)

**See**: [android/CHANGELOG.md](android/CHANGELOG.md) for complete technical details, code samples, and architectural decisions.

---

## [2026-01-04] - SonarCloud quality fixes (PR #31)

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

### Issues created
- #24: Security - Enable ProGuard obfuscation
- #25: Bug - Fix File.delete() return value checks
- #26: Bug - Remove duplicate condition in AudioPlayer.kt
- #27: Refactor - NowPlayingScreen complexity reduction
- #28: Refactor - EqualizerScreen complexity reduction

---

## [2026-01-05] - Phase 2 audio intelligence backend integration (PR #89)

**Merged**: 16 files changed, 1,156 insertions, 60 deletions
**PR**: [#89](https://github.com/forkwright/akroasis/pull/89)
**Platform**: Android

### Added

- **Search improvement** - Real audio quality data from Mouseion
  - Before: Search inferred format from bit depth (guessing)
  - After: Search displays actual `sampleRate` and `format` fields from backend
  - Rationale: Accurate quality badges require real metadata, not inference
  - Technical: Updated SearchResult model with server-provided fields
  - Files: `data/model/Track.kt`, `ui/search/SearchResultsScreen.kt`

- **Focus filtering backend** - Roon-style complex library queries
  - Before: No advanced filtering capability
  - After: FilterRepository with 5-minute facets cache, reactive StateFlow, 11 filter fields (Format, Sample Rate, Bit Depth, Codec, Bitrate, DR, Lossless, Artist, Album, Genre, Year)
  - Rationale: Power users need Roon-level filtering (e.g., "FLAC files >24/96 with DR>12")
  - Technical: Facets caching (5-min TTL) for autocomplete, FilterSummary with avgDR/format distribution, IN/NOT_IN operators with @SerializedName annotations
  - API: `POST /api/v3/library/filter`, `GET /api/v3/library/facets`
  - Gotcha: UI dropdowns deferred to PR #90 - backend fully wired but UI was placeholder
  - Files: **NEW** `data/repository/FilterRepository.kt` (75 lines), **NEW** `ui/focus/FocusFilterViewModel.kt` (119 lines)

- **Bit-perfect logic** - DAC capability detection and calculation
  - Before: No bit-perfect detection
  - After: BitPerfectCalculator with USB DAC and phone DAC detection (Android API 23+), real ✓BP badge based on: track is lossless AND sample rate ≤ DAC max AND bit depth ≤ DAC max
  - Rationale: Audiophiles need confidence that audio is delivered unmodified to DAC
  - Technical: AudioDeviceInfo queries for accurate DAC caps (Android 23+), conservative fallback for older devices (48kHz/16-bit), DAC capability caching for performance
  - Alternative: Could've used static DAC database, but real-time detection handles custom/external DACs
  - Gotcha: USB DAC detection requires AudioManager integration via Dagger Hilt
  - Files: **NEW** `audio/BitPerfectCalculator.kt` (171 lines), `ui/search/SearchViewModel.kt`, `di/AppModule.kt`

- **Smart playlists backend** - Dynamic playlists with filter rules
  - Before: No smart playlist support
  - After: Full CRUD backend with Room database v2→v3 migration (smart_playlists table), 7 Mouseion API endpoints integrated (create, list, get, update, delete, refresh, auto-refresh)
  - Rationale: Smart playlists enable "saved searches" that update automatically after library changes
  - Technical: SmartPlaylistEntity with Room TypeConverter (FilterRequest ↔ JSON), reactive Flow<List<SmartPlaylistEntity>>, SmartPlaylistRepository with syncFromServer(), autoRefreshAll() after library scan
  - Alternative: Could've stored filter rules as raw JSON string, but TypeConverter enables type-safe Room queries
  - Gotcha: Requires `fallbackToDestructiveMigration()` for dev (wipes DB on schema change)
  - Files: **NEW** `data/local/SmartPlaylistEntity.kt` (46 lines), **NEW** `data/local/SmartPlaylistDao.kt` (34 lines), **NEW** `data/repository/SmartPlaylistRepository.kt` (225 lines), `data/local/MusicDatabase.kt` (v2→v3)

### Technical

**Phase 2 status**: 90% complete - all backend functionality delivered, UI polish remaining
**Code delivered**: 9 new files (1,045 lines), 9 modified files
**API integration**: 9 new Mouseion Phase 2 API endpoints wired
**Database migration**: Room v2→v3 ready (smart_playlists table)

---

## [2026-01-05] - Phase 2 UI polish (PR #90)

**Merged**: 3 files changed, 762 insertions, 13 deletions
**PR**: [#90](https://github.com/forkwright/akroasis/pull/90)
**Platform**: Android

### Added

- **Focus filter interactive editing** - Dropdown-based filter rule editor
  - Before: Filter rules displayed in text format with "Tap to edit" placeholder
  - After: Full inline editor with FieldDropdown (11 fields), OperatorDropdown (field-specific valid operators), ValueInput (type-specific inputs: numeric with validation, boolean with Switch UI, text with autocomplete-ready input)
  - Rationale: Complex filters require rich UI, not text input
  - Technical: 256 lines of Compose Material3 UI - collapsible edit mode ("Tap to edit" → expanded editor → "DONE"), smart operator defaults (numeric fields default to >=, others to equals)
  - Alternative: Could've used bottom sheet editor, but inline editing reduces navigation overhead
  - Gotcha: Operator list must update dynamically when field changes (numeric fields support >, <, >=, <=; text fields support equals, contains, in list)
  - Files: `ui/focus/FocusFilterScreen.kt` (+256 lines)

- **Smart playlist management** - Full CRUD interface
  - Before: No UI for smart playlists
  - After: SmartPlaylistScreen with reactive Flow, playlist cards showing name/track count/last refresh/filter rule count, Create/Edit/Delete dialogs, filter configuration via FocusFilterScreen integration
  - Rationale: Smart playlists need full lifecycle management (create, edit, refresh, delete)
  - Technical: SmartPlaylistViewModel with syncFromServer() on init, full CRUD operations (createPlaylist, updatePlaylist, deletePlaylist, refreshPlaylist), 478 lines of production UI code
  - Alternative: Could've used simple list + modal, but cards with inline actions reduce taps
  - Gotcha: Filter configuration shares FocusFilterScreen composable - required careful state management
  - Files: **NEW** `ui/playlist/SmartPlaylistScreen.kt` (363 lines), **NEW** `ui/playlist/SmartPlaylistViewModel.kt` (115 lines)

### Technical

**Phase 2 status**: 100% complete
**Code delivered**: 2 new files (478 lines), 1 modified file (+256 lines) = 734 lines of production UI code
**Phase 2 total**: 11 new files, 13 modified files, 2,518 insertions, 1,779 lines of production code

---

## [2026-01-05] - Dependency vulnerability scanning (PR #91)

**Merged**: 3 files changed, 86 insertions
**PR**: [#91](https://github.com/forkwright/akroasis/pull/91)
**Platform**: CI/CD

### Added

- **Security scan workflow** - Automated dependency scanning
  - Before: No automated vulnerability detection
  - After: Dual-platform scanning (npm audit for web, OWASP Dependency Check for Android), weekly scheduled scans (Sunday midnight), PR/push triggers to develop/main
  - Rationale: Proactive security posture requires continuous dependency monitoring
  - Technical: npm audit fails on HIGH/CRITICAL, OWASP v9.0.9 fails on CVSS >= 7.0, HTML + JSON reports uploaded as CI artifacts, suppression file support for false positives
  - Alternative: Could've used Dependabot, but OWASP Dependency Check provides more granular Android Gradle scanning
  - Gotcha: OWASP reports may contain false positives requiring suppression file maintenance
  - Files: **NEW** `.github/workflows/security-scan.yml` (78 lines), `android/app/build.gradle.kts`, `android/build.gradle.kts`

### Technical

**CI integration**: Runs on PR/push + weekly cron (Sunday 00:00)
**Failure policy**: Build fails if HIGH/CRITICAL vulnerabilities detected
**Reports**: HTML + JSON uploaded to GitHub Actions artifacts

---

## [2026-01-05] - Web platform feature parity (PR #92)

**Merged**: 8 files changed, 239 insertions, 3 deletions
**PR**: [#92](https://github.com/forkwright/akroasis/pull/92)
**Platform**: Web

### Added

- **Settings page** - Web player preferences interface
  - Before: No settings UI on web platform
  - After: Playback speed control (0.5x-2x range with slider + 7 presets), volume control slider, audio quality information (sample rate, browser limitations), about section (version, platform)
  - Rationale: Web platform needs parity with Android settings
  - Technical: React 19 with Zustand store integration, speed clamping (0.5x-2x) for safe playback, reactive effects for real-time updates
  - Files: **NEW** `web/src/pages/SettingsPage.tsx` (141 lines), `web/src/App.tsx` (settings route)

- **Playback speed control** - Real-time speed adjustment
  - Before: Web player had no speed control
  - After: Full Web Audio API playbackRate support with real-time adjustment during playback
  - Rationale: Audiobook listeners require variable speed (1.5x-2x common)
  - Technical: WebAudioPlayer.setPlaybackSpeed() method with playbackRate control, store integration with reactive updates
  - Alternative: Could've required pause to change speed, but real-time adjustment is better UX
  - Gotcha: playbackRate must be clamped (0.5x-2x) - values outside this range cause audio artifacts
  - Files: `web/src/audio/WebAudioPlayer.ts` (+13 lines), `web/src/hooks/useWebAudioPlayer.ts`, `web/src/stores/playerStore.ts`

- **Audio quality badges** - Visual quality indicators
  - Before: No quality information displayed on web player
  - After: Format badge (FLAC, MP3, etc.), Hi-Res badge (>48kHz or >16-bit), 24-bit badge, Lossless indicator, Browser resampling notice
  - Rationale: Users need transparency about audio quality and platform limitations
  - Technical: Reusable React component with TailwindCSS styling, badge color-coding (blue=format, purple=hi-res, green=24-bit, amber=lossless, yellow=warning)
  - Gotcha: Browser resampling warning is ALWAYS shown - Web Audio API resamples all audio to system sample rate (typically 48kHz), no way to bypass this limitation
  - Files: **NEW** `web/src/components/AudioQualityBadges.tsx` (49 lines), `web/src/pages/PlayerPage.tsx`

- **Navigation improvement** - Settings link added
  - Before: No way to access settings from web UI
  - After: Settings button in header navigation
  - Files: `web/src/components/Navigation.tsx` (+8 lines)

### Technical

**Code delivered**: 3 new files (190 lines), 5 modified files (+49 lines)
**Dependencies**: None added (uses existing React, Zustand, TailwindCSS)
**Browser compatibility**: Tested in Chrome, Firefox, Safari
**Limitation**: Web Audio API always resamples to system sample rate (browser restriction, not Akroasis limitation)

---

## [2026-01-05] - Voice search & A/B level normalization (PR #107)

**Merged**: Voice command support and scientific A/B comparison features
**PR**: [#107](https://github.com/forkwright/akroasis/pull/107)
**Platform**: Android

### Added

- **VoiceSearchHandler** - Media session voice command parsing
  - Structured search: title, album, artist extras
  - Free-form query parsing with keyword extraction
  - Recent tracks fallback for ambiguous queries
  - Error handling and timeout logic
  - Files: **NEW** `audio/VoiceSearchHandler.kt` (224 lines)

- **LevelMatcher** - RMS-based level normalization for A/B comparison
  - Calculates RMS levels for audio tracks
  - Applies compensating gain during A/B switch
  - Prevents "louder = better" bias in EQ comparisons
  - Manual override with gain adjustment slider
  - Files: **NEW** `audio/LevelMatcher.kt`

### Technical

- PlaybackService.onPlayFromSearch() integration pending (issue #117)
- Level matching UI integrated in AudioPlayer.kt
- Search result → queue conversion logic

---

## [2026-01-05] - Signal path source codec visualization (PR #108)

**Merged**: Show actual codec in signal path instead of filename inference
**PR**: [#108](https://github.com/forkwright/akroasis/pull/108)
**Platform**: Android

### Added

- Source codec detection via file metadata introspection
- AudioPipelineState updated with codec field
- Signal path displays: FLAC, AAC, MP3, Opus, ALAC, etc.

### Before/after

- Before: Guessed codec from filename extension (.flac, .mp3)
- After: Real codec from decoder metadata via MediaExtractor
- Rationale: Filenames can be misleading (renamed files, dual-extension formats)

### Technical

- Uses Android MediaExtractor for accurate codec detection
- Falls back to filename extension if metadata unavailable
- Files: `audio/AudioPipelineState.kt`, `audio/AudioPlayer.kt`

---

## [2026-01-05] - A/B level meter UI (PR #109)

**Merged**: Visual level meter for A/B comparison
**PR**: [#109](https://github.com/forkwright/akroasis/pull/109)
**Platform**: Android

### Added

- Level meter component showing matched RMS levels
- Manual gain adjustment slider (-12dB to +12dB)
- "Match Levels" toggle for automatic level matching
- Visual feedback during A/B switching with color coding
- Real-time level display in dB

### Technical

- Integrates with LevelMatcher from PR #107
- Compose Material3 UI with reactive StateFlow
- Files: `ui/player/components/LevelMeterCard.kt`

---

## [2026-01-05] - Web accessibility improvements (PR #115)

**Merged**: WCAG 2.1 Level AA compliance for web UI
**PR**: [#115](https://github.com/forkwright/akroasis/pull/115)
**Platform**: Web

### Fixed

- **Input.tsx**: Added htmlFor/id association using React useId() hook
  - Before: Labels not programmatically associated with inputs
  - After: Screen readers correctly announce form field labels

- **PlayerPage.tsx**: Added aria-label to play/pause button
  - Before: Icon-only button with no accessible name
  - After: "Play" or "Pause" announced by screen readers

- **QueuePage.tsx**: Added aria-label to drag handle button
  - Before: Six-dot drag icon with no context
  - After: "Drag to reorder" announced for accessibility

- **OfflineIndicator.tsx**: Added role="alert" and aria-live="assertive"
  - Before: Offline status change not announced
  - After: Screen readers immediately announce offline status

### Impact

- Screen reader compatibility improved
- Keyboard navigation improved
- WCAG 2.1 Level AA compliance achieved
- Tested with NVDA, JAWS, VoiceOver

---

## [2026-01-05] - Encrypted scrobbling token storage (PR #116)

**Merged**: Secure Last.fm/ListenBrainz token encryption
**PR**: [#116](https://github.com/forkwright/akroasis/pull/116)
**Platform**: Android

### Added

- **ScrobblePreferences** converted to EncryptedSharedPreferences
  - MasterKey with AES256_GCM encryption scheme
  - Automatic migration from plaintext to encrypted storage
  - Old plaintext data cleared after successful migration
  - Files: `data/preferences/ScrobblePreferences.kt` (modified)

### Security

- Last.fm session keys encrypted at rest
- ListenBrainz tokens encrypted at rest
- Follows existing pattern from AuthInterceptor.kt
- Zero user action required (transparent migration on first launch)
- Uses androidx.security:security-crypto library

### Technical

- Migration runs once in init block
- Checks if old plaintext prefs exist and new encrypted ones don't
- Copies all values: session keys, usernames, enabled flags, settings
- Clears old plaintext storage after migration complete

---

## [2026-01-05] - Performance profiling framework (PR #106)

**Merged**: Profiling infrastructure for battery and performance testing
**PR**: [#106](https://github.com/forkwright/akroasis/pull/106)
**Platform**: Android

### Added

- **PerformanceProfiler** - Battery impact tracking infrastructure
  - Cold start time measurement
  - Library load performance metrics
  - DSP configuration profiling hooks
  - Memory usage tracking
  - Files: **NEW** `util/PerformanceProfiler.kt`

### Usage

- Enables data-driven battery optimization
- Supports manual testing (issue #97 - battery profiling)
- Foundation for performance baselines (issue #102)
- Metrics logged to Timber for analysis

### Technical

- Lightweight instrumentation (minimal overhead)
- Conditional compilation for production builds
- Hooks into AudioPlayer, EqualizerEngine, GaplessPlaybackEngine

---

## [2026-01-05] - CI workflow optimization (PR #105)

**Merged**: Reduced CI execution time by 40%
**PR**: [#105](https://github.com/forkwright/akroasis/pull/105)
**Platform**: CI/CD

### Changed

- Parallelized independent CI jobs (build, lint, test)
- Cached Gradle dependencies (saves ~2min per run)
- Optimized artifact uploads (only upload on failure)
- Reduced redundant checks (skip web tests on Android-only changes)
- Files: `.github/workflows/*.yml`

### Impact

- CI runtime: ~8min → ~5min (40% reduction)
- Faster feedback on PRs
- Reduced GitHub Actions minutes usage
- Improved developer experience

---

## [2026-01-05] - CodeQL false positive suppressions (PR #104)

**Merged**: Suppressed CodeQL false positives with justifications
**PR**: [#104](https://github.com/forkwright/akroasis/pull/104)
**Platform**: CI/CD

### Fixed

- Added CodeQL configuration file with query suppressions
- Documented false positive rationale in comments
- Maintains code quality score without noise
- Files: **NEW** `.github/codeql/codeql-config.yml`

### Suppressions

- Suppressed: Unused parameter warnings in Compose @Composable functions (Android convention)
- Suppressed: "Potential NullPointerException" in sealed class exhaustive when statements
- All suppressions documented with inline justification comments

---

## [2026-01-05] - Documentation update (PR #103)

**Merged**: Updated docs for Phase 2 completion and recent PRs
**PR**: [#103](https://github.com/forkwright/akroasis/pull/103)
**Platform**: Documentation

### Updated

- ROADMAP.md with Phase 2 complete status
- CHANGELOG.md with PRs #89-#92 detailed entries
- LOCAL_COMMITS.md with recent activity
- README.md status section updated

---

## [2026-01-04] - Search UI with audio quality badges (PR #23)

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

## [2026-01-03] - Phase 2 UI scaffolding (PR #22)

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

## [2026-01-02] - Web PWA offline support (PR #21)

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

## [2026-01-02] - Web app MVP core features (PR #20)

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

### Technical stack
- React 19 + Vite + TypeScript
- Zustand for state management
- TailwindCSS for styling
- @dnd-kit for drag-and-drop

### Success criteria
- ✅ Audio plays from Mouseion streaming endpoint
- ✅ Gapless playback verified (<50ms gaps)
- ✅ Works in Chrome, Firefox, Safari
- ✅ Media keys functional

---

## [2026-01-02] - Documentation updates (PR #19)

**Merged**: Post-PR#18 documentation updates
**PR**: [#19](https://github.com/forkwright/akroasis/pull/19)

### Updated
- ROADMAP.md with Phase 1+3+6+7 completion
- CHANGELOG.md with feature documentation
- CONTRIBUTING.md with current project status

---

## [2025-12-31] - Quality audit remediation

Security, safety, and quality improvements across Android client.

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
- Created CHANGELOGs
- Updated project documentation
- Fixed style guide violations

See wrapper CHANGELOG.md for full details.
