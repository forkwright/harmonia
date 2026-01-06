---
name: A/B Mode Level Normalization
about: Implement RMS/LUFS level normalization for fair A/B comparison
title: '[Android] Implement RMS/LUFS level normalization for A/B comparison'
labels: 'enhancement, android, audio, m'
assignees: ''
---

## Context

Current A/B mode for EQ comparison doesn't normalize levels between A (no EQ) and B (EQ enabled) states. This creates unfair comparisons because louder often sounds subjectively better, even if audio quality is unchanged. Need level matching to ensure comparisons are based on tonal changes only.

**Current behavior:** A/B switch applies/removes EQ, but output levels may differ.

**Desired behavior:** A and B states matched to same perceived loudness (RMS or LUFS).

## Scope

### Audio Analysis

- Calculate RMS or LUFS levels for current track
- Store level data in track metadata cache (avoid recalculating)
- Real-time level matching during A/B playback
- Apply compensating gain to match loudness between states

### UI Components

- **Level meter**: Displays matched levels for A and B
- **Manual gain adjustment slider**: Override automatic matching (±6dB)
- **"Match Levels" toggle**: Enable/disable automatic level matching
- **Status indicator**: Show "Levels Matched" when active

### Implementation

1. **Audio Analysis**
   - Use Android Loudness API (API 28+) or custom RMS calculation
   - Calculate integrated loudness (LUFS) or RMS per track
   - Cache results in Room database

2. **Level Matching**
   - Measure A state output level (no EQ)
   - Measure B state output level (with EQ)
   - Calculate gain compensation: `compensationGain = levelA - levelB`
   - Apply gain to B state during playback

3. **Gain Application**
   - Use AudioTrack volume control (preferred, sample-accurate)
   - Or apply digital gain via DSP chain (may introduce latency)
   - Ensure no clipping: limit max gain to prevent >0dBFS

4. **A/B Switching**
   - Seamless transition with level matching active
   - Preserve user EQ settings
   - Visual feedback showing matched levels

## Acceptance Criteria

- [ ] A/B mode matches perceived loudness between states
- [ ] Level meter displays matched RMS/LUFS values
- [ ] Manual gain override slider functional (±6dB range)
- [ ] "Match Levels" toggle works correctly
- [ ] No clipping during level matching (peak limiter if needed)
- [ ] Works with various track loudnesses (-6dB to -20dB average)
- [ ] Level matching calculation < 100ms (no perceptible delay)
- [ ] Level cache persists across app restarts

## Dependencies

- Android Loudness API (API 28+) or fallback to RMS calculation
- Access to audio buffer for level measurement
- Room database for level cache storage

## Out of Scope

- Advanced LUFS calculation (use RMS for MVP, upgrade later)
- Per-track loudness normalization (ReplayGain is separate feature)
- Dynamic range preservation (focus on matching average levels)
- Multi-band level matching (analyze full spectrum, match single value)
- Background level analysis job (calculate on-demand only for MVP)

## Technical Notes

### RMS vs LUFS

- **RMS (Root Mean Square)**: Simple, fast, good approximation
- **LUFS (Loudness Units Full Scale)**: Perceptually accurate, ITU-R BS.1770 standard
- **Recommendation**: Start with RMS for MVP, consider LUFS upgrade if needed

### Level Calculation Window

- Analyze first 30 seconds of track (representative sample)
- Or analyze full track if <2 minutes
- Skip intro silence (first 3 seconds)

### Gain Compensation Range

- Typical EQ changes: ±3dB
- Safe range for auto-matching: ±6dB
- Warn user if exceeds range ("Level difference too large for matching")

## Platform(s)

Android

## Size Estimate

**m** (4-8 hours)

**Breakdown:**
- RMS/LUFS calculation: 2-3 hours
- Level matching logic: 2 hours
- UI components (meter, slider, toggle): 2 hours
- Testing with various tracks: 1-2 hours
