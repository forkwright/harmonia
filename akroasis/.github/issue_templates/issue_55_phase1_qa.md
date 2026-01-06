---
name: Complete Phase 1 QA Testing
about: Execute 160+ manual test cases from PHASE1_QA.md
title: '[Android] Complete Phase 1 QA manual testing (160 test cases)'
labels: 'enhancement, android, l'
assignees: ''
---

## Context

Phase 1 features are implemented but not systematically tested. `android/PHASE1_QA.md` contains 160+ manual test cases covering all Phase 1 features. Need comprehensive testing before declaring Phase 1 complete.

## Scope

### Test Categories (from PHASE1_QA.md)

1. **Playback controls** (15 tests) - play/pause, skip, seek, repeat, shuffle
2. **Signal path visualization** (12 tests) - source format, decode, process, output, bit-perfect indicator
3. **Gapless verification** (10 tests) - gapless status, transitions, edge cases
4. **Queue operations** (20 tests) - add, remove, reorder, clear, shuffle
5. **Playback speed memory** (18 tests) - track/album/default hierarchy, persistence
6. **Queue export** (8 tests) - M3U/M3U8/PLS formats, encoding, relative paths
7. **Drag-to-reorder** (12 tests) - visual feedback, persistence, undo/redo
8. **DSP** (25 tests) - EQ, crossfeed, headroom, battery impact
9. **Scrobbling** (15 tests) - Last.fm, ListenBrainz, network handling
10. **Edge cases** (12 categories, 25 tests) - variable speed extremes, sleep timer, battery, A/B mode

### Known Limitations to Address

These issues may be discovered during testing and should be filed separately:

1. **Drag-to-reorder**: Visual only, needs backend persistence (#61)
2. **A/B mode level matching**: Needs RMS/LUFS normalization (#57)
3. **Battery estimates**: Rough calculation, needs real profiling (#58)
4. **Signal path source format**: Filename inference only, needs file introspection (#62)

## Acceptance Criteria

- [ ] All 160 test cases executed and documented in test results file
- [ ] Pass rate ≥95% for Phase 1 completion
- [ ] Bugs filed for failing tests with severity labels
- [ ] Known limitations documented in ROADMAP.md deferred features
- [ ] Test results committed to `android/PHASE1_QA_RESULTS.md`
- [ ] Regression test suite created for critical paths (future automation)

## Testing Environment

### Required Hardware
- Android device(s) for testing (target: Pixel, Sony Walkman)
- Various audio file formats (FLAC 16/44.1, 24/96, DSD, high-res PCM)
- USB DAC for bit-perfect testing
- Bluetooth headset for media controls testing

### Test Data
- Sample library with:
  - Gapless albums (Pink Floyd, concept albums)
  - Various formats (FLAC, ALAC, WAV, DSD)
  - High-res and standard resolution files
  - Albums with multiple disc sets
  - Tracks with metadata edge cases

## Dependencies

- Android device(s) with USB OTG support
- USB DAC (e.g., Fiio Q3, Topping D10s)
- Test audio library (minimum 100 tracks, 10 albums)

## Out of Scope

- Automated UI testing (separate issue #63)
- Integration testing (separate issue #63)
- Performance profiling (separate issue #64)
- iOS testing (not a platform target)

## Platform(s)

Android

## Size Estimate

**l** (1-2 days manual testing + documentation)

**Breakdown:**
- Test execution: 8-12 hours
- Bug filing: 2-4 hours
- Documentation: 2 hours
