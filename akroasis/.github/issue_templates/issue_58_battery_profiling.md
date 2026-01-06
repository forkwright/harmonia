---
name: Battery Impact Profiling
about: Profile battery impact of DSP configurations on target hardware
title: '[Android] Profile battery impact of DSP configurations'
labels: 'enhancement, android, infrastructure, m'
assignees: ''
---

## Context

Current battery estimates in `BatteryAwarePlaybackManager` are rough calculations based on assumptions. Need real device testing to validate battery impact of EQ/crossfeed/headroom DSP and provide accurate estimates to users.

**Current estimates (unvalidated):**
- EQ enabled: +5% battery drain
- Crossfeed enabled: +3% battery drain
- USB DAC connected: +10% battery drain

**Goal:** Replace assumptions with real profiling data.

## Scope

### Test Configurations

Profile battery drain for these configurations (baseline + combinations):

1. **Baseline**: Playback only (no DSP, internal speaker)
2. **EQ only**: 5-band parametric EQ active
3. **Crossfeed only**: Medium crossfeed setting
4. **EQ + Crossfeed**: Both active simultaneously
5. **Full DSP**: EQ + crossfeed + headroom management
6. **USB DAC**: Baseline + USB DAC connected (bit-perfect)
7. **Bluetooth**: Baseline + Bluetooth headphones

### Test Procedure

#### Environment
- **Device**: Sony Walkman (primary target) or Pixel device
- **Format**: FLAC 16/44.1 (baseline format, consistent)
- **Screen**: Off during test (playback only)
- **Network**: Disabled (local playback)
- **Volume**: 50% (consistent across tests)

#### Test Protocol
1. Charge device to 100%
2. Disconnect charging cable
3. Start 4-hour playback test with configuration
4. Record battery percentage every 30 minutes
5. Log battery drain rate (mAh/hour)
6. Repeat test 2-3 times per configuration for consistency

#### Data Collection
- Battery percentage at 30-minute intervals
- Total playback time to battery depletion
- Average drain rate (mAh/hour)
- Device temperature (if excessive heat detected)
- Background tasks verified minimal

### Deliverables

1. **Battery Drain Table**
   - Configuration vs. drain rate (mAh/hour)
   - Relative drain vs. baseline (percentage increase)
   - Estimated playback time at 100% battery

2. **Updated Estimation Formula**
   - Replace hardcoded percentages with profiled data
   - Update `BatteryAwarePlaybackManager.calculateBatteryImpact()`
   - Add confidence intervals if variance detected

3. **Recommendations**
   - Low-power mode settings (disable EQ below 20% battery)
   - Optimal DSP settings for battery life
   - Document in user guide

4. **Documentation**
   - Add results to `docs/BATTERY_PROFILING.md`
   - Update ROADMAP with validated estimates
   - User-facing tips in app settings

## Acceptance Criteria

- [ ] Minimum 5 configurations tested (baseline + 4 DSP combinations)
- [ ] Each configuration tested for 4+ hours (or to depletion)
- [ ] Data collected at 30-minute intervals
- [ ] Battery drain rate documented (mAh/hour)
- [ ] Estimation formula updated in `BatteryAwarePlaybackManager`
- [ ] Results documented in `docs/BATTERY_PROFILING.md`
- [ ] Variance < 10% between test runs (consistency check)
- [ ] Recommendations added to user settings/guide

## Dependencies

- Sony Walkman device or Pixel device with sufficient battery capacity
- Battery monitoring tools (Android Settings or Accubattery app)
- Test audio library (FLAC 16/44.1 files, 4+ hours playback)
- Stopwatch/timer for interval tracking

## Out of Scope

- Multi-device testing (focus on Walkman primary target)
- Automated battery testing framework (manual testing acceptable)
- Screen-on battery testing (focus on background playback)
- Streaming battery impact (local playback only)
- Video playback battery testing (audio-only app)

## Testing Notes

### Battery Measurement Tools

- **Android Settings**: Battery usage stats (built-in)
- **Accubattery**: Detailed mAh tracking (recommended)
- **ADB battery stats**: `adb shell dumpsys batterystats` for detailed logs

### Expected Results

Based on similar apps:
- EQ: +3-8% drain (lightweight Android Equalizer API)
- Crossfeed: +2-5% drain (minimal processing)
- USB DAC: +5-15% drain (hardware dependent)
- Full DSP: +8-20% drain combined

If actual results deviate significantly, investigate:
- Background tasks interfering
- Device-specific optimizations
- Android audio framework overhead

## Platform(s)

Android

## Size Estimate

**m** (4-8 hours testing + 2 hours analysis)

**Breakdown:**
- Test execution: 4-6 hours (multiple 4-hour runs)
- Data collection and analysis: 2 hours
- Code updates (estimation formula): 1 hour
- Documentation: 1 hour

**Note:** Wall-clock time is 2-3 days due to 4-hour test runs, but active effort is 6-8 hours.
