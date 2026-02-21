# Spec 04: Platform Polish & Offline

**Status:** Draft
**Priority:** Medium
**Issues:** #71, #72, #73, #74, #79, #94

## Goal

Polish existing platforms. Adaptive streaming (lossless on WiFi, compressed on cellular), predictive offline sync, Android Auto, Wear OS, and cross-device playback transfer. These are the features that make daily use seamless.

## Phases

### Phase 1: Adaptive streaming
- [ ] WiFi → lossless, cellular → opus/aac transcoding (#72)
- [ ] Configurable quality preferences per network type
- [ ] Bandwidth estimation and automatic fallback

### Phase 2: Offline
- [ ] Predictive offline sync based on listening patterns (#71)
- [ ] Manual download queue
- [ ] Storage management (cache limits, cleanup)

### Phase 3: Android ecosystem
- [ ] Full Android Auto media browser (#73)
- [ ] Wear OS companion app (#74)
- [ ] Playback transfer between devices (#79)

### Phase 4: QA
- [ ] Phase 1 manual QA pass (#94, #55 — deduplicate these)
- [ ] Regression test suite for core playback

## Dependencies

- Adaptive streaming requires Mouseion transcoding endpoint (#72 blocked)
- Playback transfer requires Mouseion session API (#79 blocked)
- Wear OS needs Android app stable first

## Notes

- Android Auto (#73) is medium effort, high daily-use value.
- Wear OS (#74) is large effort, niche value — deprioritize.
- Issues #94 and #55 are duplicates (both "Phase 1 QA"). Close one.
