# Performance Profiling

Baseline metrics and profiling methodology for Akroasis performance tracking.

## Key Metrics

### Android App

#### Startup Performance
- **Cold start time**: App launch to first frame (Target: <2s)
- **Warm start time**: Background resume to first frame (Target: <500ms)
- **Library load time**: Initial library scan completion (Target: <3s for 10k tracks)

#### Playback Performance
- **Track load time**: API fetch to playback start (Target: <200ms)
- **Gapless transition**: Gap between tracks (Target: <50ms, verified in Phase 1)
- **Skip responsiveness**: Skip command to playback start (Target: <100ms)

#### UI Responsiveness
- **Library scroll FPS**: Minimum frame rate during scrolling (Target: 60 FPS)
- **Search latency**: Keystroke to results displayed (Target: <300ms)
- **Queue reorder lag**: Drag operation frame rate (Target: 60 FPS)

#### Memory Usage
- **Idle memory**: App backgrounded with music paused (Target: <150 MB)
- **Playback memory**: Active playback with queue (Target: <200 MB)
- **Peak memory**: Library scan or large operation (Target: <300 MB)

#### Battery Impact
- **Baseline playback**: Music playback, no DSP (Reference baseline)
- **With EQ**: 5-band parametric EQ active (Target: <10% overhead)
- **With crossfeed**: Medium crossfeed active (Target: <5% overhead)
- **Full DSP**: EQ + crossfeed + headroom (Target: <15% overhead)

### Web App

#### Load Performance
- **Initial load**: First contentful paint (Target: <1.5s)
- **Time to interactive**: Ready for user input (Target: <3s)
- **Bundle size**: Total JS/CSS payload (Target: <500 KB gzipped)

#### Playback Performance
- **Track load time**: Fetch to playback start (Target: <300ms)
- **Gapless transition**: Gap between tracks (Target: <100ms, browser limitation)
- **Seek responsiveness**: Seek command to playback (Target: <100ms)

#### Resource Usage
- **Memory footprint**: Active playback state (Target: <100 MB)
- **CPU usage**: Idle playback (Target: <5% on modern CPU)

## Profiling Tools

### Android

**Startup Profiling:**
```bash
adb shell am start -W app.akroasis/.MainActivity
# Output: TotalTime, WaitTime, DisplayTime
```

**CPU Profiling:**
- Android Studio Profiler (CPU, Memory, Network tabs)
- System tracing: `Debug.startMethodTracing()` / `Debug.stopMethodTracing()`

**Memory Profiling:**
- LeakCanary integration (detect memory leaks)
- Android Studio Memory Profiler (heap dumps)

**Battery Profiling:**
- Battery Historian (visualize battery drain)
- `dumpsys batterystats` for detailed power usage

**Frame Rate:**
```bash
adb shell dumpsys gfxinfo app.akroasis
# Look for: Janky frames, 90th/95th/99th percentile
```

### Web

**Chrome DevTools:**
- Performance tab: Record startup/interaction
- Lighthouse: Overall performance score
- Network tab: Bundle size analysis

**Bundle Analysis:**
```bash
npm run build -- --analyze
# Generates bundle size visualization
```

**Performance Measurement API:**
```javascript
performance.mark('track-load-start');
// ... load track ...
performance.mark('track-load-end');
performance.measure('track-load', 'track-load-start', 'track-load-end');
```

## Baseline Establishment

### Test Environment

**Android:**
- Device: Sony Walkman (target device)
- OS: Android 13+
- Network: WiFi (stable connection)
- Library: 10,000 tracks (representative dataset)

**Web:**
- Browser: Chrome 120+ (primary target)
- Device: Desktop (modern CPU)
- Network: Fast 3G throttling (realistic mobile)

### Profiling Procedure

1. **Clean state**: Clear cache, fresh install
2. **Warmup**: Perform test operation 3 times (discard results)
3. **Measurement**: Run test 10 times, record median
4. **Documentation**: Record date, build version, device specs

## Performance Regression Testing

**CI Integration:**
- Track bundle size in CI (fail if >10% increase)
- Monitor Android APK size (warn if >5 MB increase)

**Regular profiling:**
- Weekly: Run full profiling suite on develop branch
- Pre-release: Complete performance audit
- Post-major feature: Profile affected areas

## Current Status

**Last profiled:** Not yet established (Issue #64)

**Known bottlenecks:**
- TBD after initial profiling

**Planned improvements:**
- TBD based on profiling results

## References

- [Android Performance](https://developer.android.com/topic/performance)
- [Web Vitals](https://web.dev/vitals/)
- [Chrome DevTools](https://developer.chrome.com/docs/devtools/)
