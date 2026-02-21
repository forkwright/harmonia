# Spec 04: Platform Polish & Offline

**Status:** Active
**Priority:** Medium
**Issues tracked:** #72, #74, #79 (closing all — spec is source of truth)

## Goal

Polish existing platforms. Adaptive streaming (lossless on WiFi, compressed on cellular), predictive offline sync, Wear OS, and cross-device playback transfer. These are the features that make daily use seamless.

## Phases

### Phase 1: Adaptive streaming — **UNBLOCKED** (StreamingController.Transcode exists, passthrough mode)
- [ ] WiFi → lossless, cellular → opus/aac transcoding
- [ ] Configurable quality preferences per network type
- [ ] Bandwidth estimation and automatic fallback

### Phase 2: Offline
- [ ] Predictive offline sync based on listening patterns
- [ ] Manual download queue
- [ ] Storage management (cache limits, cleanup)

### Phase 3: Android ecosystem
- [x] Full Android Auto media browser — PR #147 (artwork, search, genres, error handling)
- [ ] Wear OS companion app (large effort, low priority)
- [ ] Playback transfer between devices — **UNBLOCKED** (QueueController.TransferPlayback exists)

### Phase 4: QA
- [ ] Regression test suite for core playback
- [ ] Manual QA checklist for release readiness

## Dependencies

- ~~Adaptive streaming requires Mouseion transcoding endpoint~~ **AVAILABLE** — StreamingController has passthrough transcode, FFmpeg is Phase 3+
- ~~Playback transfer requires Mouseion session API~~ **AVAILABLE** — QueueController with cross-device transfer

## Notes

- Android Auto shipped in PR #147 with artwork, search, genres, and error items.
- Mouseion transcode endpoint exists but currently passthrough-only (returns source format if match). Full FFmpeg transcoding is Mouseion-side Phase 3+.
- Wear OS is large effort for niche audience — deprioritize.
