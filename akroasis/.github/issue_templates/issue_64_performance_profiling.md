---
name: Performance Profiling
about: Profile and establish performance baselines for Android and Web
title: '[Infrastructure] Profile and establish performance baselines'
labels: 'enhancement, infrastructure, android, web, l'
assignees: ''
---

## Context

No performance baselines established. Need to profile cold start time, library load speed, scroll FPS, memory usage, and bundle size to establish targets and track regressions.

**Current state:** No formal performance metrics
**Goal:** Establish baselines and monitoring for key metrics

## Scope

### Metrics to Profile

#### 1. Cold Start Time (Android)
- **Metric**: Time from app launch to first frame
- **Target**: <2 seconds
- **Measurement**: Android Studio Profiler or `adb shell am start -W`
- **Factors**: Database init, API token load, initial UI render

#### 2. Library Load Time (Android + Web)
- **Metric**: Time to display 10,000+ tracks in library
- **Target**: <1 second for initial render, <500ms for subsequent
- **Measurement**: Log timestamps around data fetch and UI render
- **Factors**: API latency, JSON parsing, RecyclerView/list rendering

#### 3. Scroll FPS (Android + Web)
- **Metric**: Frames per second during library scroll
- **Target**: 60 FPS (16.67ms per frame)
- **Measurement**: Android GPU Profiling or Chrome DevTools Performance
- **Factors**: List recycling, image loading, layout complexity

#### 4. Memory Usage (Android + Web)
- **Metric**: Heap size during normal operation
- **Target Android**: <150MB for typical library (5,000 tracks)
- **Target Web**: <200MB for typical library
- **Measurement**: Android Studio Memory Profiler or Chrome DevTools Memory
- **Factors**: Bitmap caching, database cache, audio buffers

#### 5. Bundle Size (Web)
- **Metric**: Initial JS bundle size
- **Target**: <500KB gzipped
- **Measurement**: Vite build stats or Webpack Bundle Analyzer
- **Factors**: Dependencies, code splitting, tree shaking

#### 6. API Latency (Android + Web)
- **Metric**: Time for key API calls
- **Targets**:
  - GET library: <500ms
  - GET tracks: <200ms
  - POST scrobble: <300ms
- **Measurement**: Network profiling (Logcat or Chrome DevTools Network)

#### 7. Playback Start Time (Android + Web)
- **Metric**: Time from "Play" tap to first audio sample
- **Target**: <200ms
- **Measurement**: Log timestamps or audio callback timing
- **Factors**: Network latency, decoder init, buffer fill

### Profiling Procedure

#### Android
1. **Setup**: Release build with Proguard, USB debugging
2. **Tools**: Android Studio Profiler (CPU, Memory, Network)
3. **Scenarios**:
   - Cold start (app not in memory)
   - Warm start (app in background)
   - Library browse (10,000+ tracks)
   - Playback (FLAC, high-res)
   - Queue operations (100+ tracks)

#### Web
1. **Setup**: Production build, Chrome DevTools
2. **Tools**: Lighthouse, Performance tab, Network tab
3. **Scenarios**:
   - Initial page load
   - Library browse (large dataset)
   - Playback (gapless transitions)
   - Search (10,000+ tracks)

### Deliverables

1. **Performance Baseline Report** (`docs/PERFORMANCE_BASELINES.md`)
   - Table of metrics with baseline values
   - Profiling screenshots/graphs
   - Device/browser specs used

2. **Regression Tracking**
   - CI job to run performance tests
   - Alert if metrics degrade >10%

3. **Optimization Recommendations**
   - Hotspots identified (CPU, memory)
   - Low-hanging fruit (e.g., lazy loading, image optimization)
   - Priority order for fixes

## Acceptance Criteria

- [ ] All 7 metrics profiled on Android
- [ ] All 7 metrics profiled on Web
- [ ] Baselines documented in `docs/PERFORMANCE_BASELINES.md`
- [ ] CI job runs performance tests on PRs (optional, nice-to-have)
- [ ] Regression threshold defined (e.g., alert if >10% slower)
- [ ] Optimization recommendations documented with priorities
- [ ] At least one optimization implemented (low-hanging fruit)

## Performance Targets

| Metric | Android Target | Web Target | Priority |
|--------|----------------|------------|----------|
| Cold start | <2s | <3s (initial load) | High |
| Library load | <1s | <1s | High |
| Scroll FPS | 60 FPS | 60 FPS | High |
| Memory usage | <150MB | <200MB | Medium |
| Bundle size | N/A | <500KB gzipped | Medium |
| API latency | <500ms | <500ms | High |
| Playback start | <200ms | <200ms | High |

## Dependencies

- Android Studio Profiler (Android)
- Chrome DevTools (Web)
- Release builds for accurate profiling
- Test library with 10,000+ tracks
- Target devices: Sony Walkman, Pixel

## Out of Scope

- Battery profiling (covered in #58)
- Network profiling under poor conditions (future)
- Multi-device comparison (focus on primary targets)
- Automated performance regression testing (nice-to-have, not required)

## Testing Environment

### Android
- **Device**: Sony Walkman (primary), Pixel (secondary)
- **OS**: Android 12+
- **Build**: Release with Proguard enabled
- **Library**: 10,000+ tracks for stress testing

### Web
- **Browser**: Chrome (primary), Firefox, Safari
- **Environment**: Production build (Vite production mode)
- **Library**: 10,000+ tracks

## Platform(s)

Android, Web, Infrastructure

## Size Estimate

**l** (1-2 days)

**Breakdown:**
- Android profiling: 4-6 hours
- Web profiling: 3-4 hours
- Analysis and documentation: 3-4 hours
- CI integration (optional): 2-3 hours
- Optimization implementation: 2-4 hours
