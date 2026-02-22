# Spec 08: Playback Engine

**Status:** Active
**Priority:** High

## Goal

Add core playback features that close competitive gaps against Symfonium, Roon, and Plexamp: repeat/shuffle modes, crossfade (simple + intelligent), loudness normalization (ReplayGain/EBU R128), and skip silence. These are table-stakes features for a serious audio player — currently Akroasis has none of them.

## Greek Names

| Feature | Name | Meaning |
|---------|------|---------|
| Crossfade | **Metaxis** (me-TAX-is) | The space between — Platonic transitional space where one track ends and the next begins |
| ReplayGain | (no Greek name) | Industry standard term — clarity over poetry |
| Skip silence | (no Greek name) | Feature name is self-describing |

## Phases

### Phase 1: Repeat modes
- [ ] Add `repeatMode` state to `playerStore.ts` (off / all / one / shuffle-repeat)
- [ ] Add `originalQueue` for shuffle backup + Fisher-Yates shuffle
- [ ] Add `replay()` method to `WebAudioPlayer.ts`
- [ ] Update `onPlaybackEnd` in `useWebAudioPlayer.ts` for all 4 modes
- [ ] Create `RepeatButton.tsx` (cycle through 4 states)
- [ ] Place in PlayerPage transport + QueuePage header
- [ ] Tests for store, hook behavior, component

### Phase 2: ReplayGain / ReplayGain
- [ ] Extend `Track` type with ReplayGain/R128 fields
- [ ] Create `replayGainStore.ts` (mode, targetLufs, limiter, analysisCache)
- [ ] Insert `replayGainNode` (GainNode) in WebAudioPlayer after compressor
- [ ] Add dedicated limiter node (DynamicsCompressor as brick-wall)
- [ ] Create `audio/loudnessMeasure.ts` (simplified ITU-R BS.1770-4)
- [ ] Implement `getEffectiveGain()`: tags → cache → real-time scan
- [ ] Update `SignalPath.tsx` with ReplayGain/Limiter chips
- [ ] Settings UI in SettingsPage
- [ ] Tests for store, LUFS measurement, gain computation

### Phase 3: Simple crossfade / Metaxis
- [ ] Create `metaxisStore.ts` (mode, duration, curve, respectAlbumTransitions)
- [ ] Refactor WebAudioPlayer to dual-source architecture (primaryGain + secondaryGain)
- [ ] Implement `startCrossfade()` with AudioParam gain scheduling
- [ ] Add crossfade trigger timer in `useWebAudioPlayer.ts` position update
- [ ] Album transition detection (skip crossfade for same-album consecutive tracks)
- [ ] Crossfade replaces gapless when enabled
- [ ] Update `SignalPath.tsx` with Crossfade chip
- [ ] Settings UI in SettingsPage (mode, duration slider, curve, album toggle)
- [ ] Tests for store, gain scheduling, album detection

### Phase 4: Skip silence
- [ ] Create `silenceSkipStore.ts` (per-type config: music/audiobook/podcast)
- [ ] Create `audio/silenceDetector.ts` (region detection, adaptive threshold, buffer modification)
- [ ] Modify `loadTrack()` to scan and remove silence regions when enabled
- [ ] Implement seek bar time mapping for shortened buffers
- [ ] Feed silence detection into crossfade trigger (start at silence boundary)
- [ ] Settings UI per media type
- [ ] Tests for silence detection with synthetic AudioBuffers

### Phase 5: Intelligent crossfade
- [ ] Create `audio/loudnessAnalyzer.ts` (RMS profiling, fade/entry point detection)
- [ ] Implement dynamic overlap calculation (loudness contour analysis)
- [ ] Curve selection based on outro/intro loudness shapes
- [ ] Clamp to configurable min/max overlap range
- [ ] Tests with synthetic audio data showing natural fades

## Dependencies

- ReplayGain tag data requires Mouseion to include RG/R128 fields in Track metadata
- Android Track model already has `replayGainTrackGain`/`replayGainAlbumGain` — web needs parity
- No backend work needed for repeat, crossfade, or skip silence (pure frontend)

## Notes

- Pipeline evolution order matters: ReplayGain introduces per-source gain concepts, skip silence introduces buffer analysis, crossfade combines both with dual-source mixing. Build in this order.
- During crossfade, ReplayGain must be applied per-source (primaryGain * RG_A, secondaryGain * RG_B) — different tracks may have different gains.
- Repeat-one disables crossfade (track replays from start without overlap).
- Skip silence runs on pre-ReplayGain buffer (raw decoded audio) — silence detection uses absolute dB thresholds.
- `loadJson` utility should be extracted to shared `utils/storage.ts` before creating new stores (currently duplicated across 4 stores).
