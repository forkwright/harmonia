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
