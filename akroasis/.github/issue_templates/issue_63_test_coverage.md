---
name: Expand Test Coverage to 70%
about: Expand test coverage from 40-50% to 70%+ with integration and E2E tests
title: '[Infrastructure] Expand test coverage from 40-50% to 70%+'
labels: 'enhancement, infrastructure, android, web, xl'
assignees: ''
---

## Context

Current test coverage is 40-50% (365+ unit tests). Need to expand with integration tests, E2E tests, and UI tests to reach 70%+ coverage for production readiness.

**Current state:**
- 365+ unit tests (Phase 1 features)
- Good coverage of core logic (ViewModels, managers)
- Minimal integration testing
- No E2E testing
- No UI automation

**Goal:** Comprehensive test suite with 70%+ coverage across all platforms.

## Scope

### Test Categories to Add

#### 1. Integration Tests (Android)
- **API client integration**: Test Retrofit + Mouseion backend
- **Database integration**: Room DAOs with in-memory DB
- **Service integration**: PlaybackService + MediaSessionManager
- **Scrobbling integration**: Last.fm/ListenBrainz end-to-end

**Files to test:**
- `ApiClient.kt` with mock Mouseion server
- All DAOs with in-memory database
- `PlaybackService` lifecycle
- `ScrobblerManager` with mock API

**Estimated tests:** 50-75 integration tests

#### 2. E2E Tests (Android)
- **Playback flow**: Browse → Play → Pause → Skip → Stop
- **Queue management**: Add → Reorder → Remove → Export
- **DSP workflow**: Enable EQ → Apply crossfeed → Verify signal path
- **Scrobbling workflow**: Play track → Verify scrobble submitted

**Framework:** Espresso or Compose UI Testing

**Estimated tests:** 25-40 E2E tests

#### 3. UI Tests (Android)
- **Now Playing screen**: All controls functional
- **Signal Path visualization**: Updates correctly
- **EQ screen**: Frequency bands adjustable
- **Queue screen**: Drag-to-reorder works

**Framework:** Compose UI Testing (`@Test + ComposeTestRule`)

**Estimated tests:** 30-50 UI tests

#### 4. Integration Tests (Web)
- **API client**: Fetch library, playback endpoints
- **State management**: Zustand store updates
- **Audio engine**: Web Audio API playback
- **Queue management**: Add/remove/reorder

**Framework:** Vitest or Jest with React Testing Library

**Estimated tests:** 40-60 integration tests

#### 5. E2E Tests (Web)
- **Playback flow**: Library → Play → Queue → Controls
- **Search workflow**: Search → Filter → Play results
- **PWA workflow**: Install → Offline playback

**Framework:** Playwright or Cypress

**Estimated tests:** 20-30 E2E tests

#### 6. Edge Case Tests (All Platforms)
- **Network errors**: Offline, timeout, 500 errors
- **Empty states**: No library, empty queue
- **Permission errors**: Storage, network denied
- **Large datasets**: 10,000+ tracks, stress testing

**Estimated tests:** 25-40 edge case tests

### Test Infrastructure

1. **CI/CD integration**
   - Run tests on every PR
   - Fail build on test failures
   - Code coverage reporting (Codecov or similar)

2. **Test data management**
   - Mock Mouseion API responses
   - Test audio files (various formats)
   - Fixture data for library/albums/tracks

3. **Performance testing**
   - Benchmark critical paths (library load, playback start)
   - Regression detection (alert if >10% slower)

## Acceptance Criteria

- [ ] Overall test coverage ≥70% (SonarCloud or Jacoco)
- [ ] Integration tests added for API, database, services
- [ ] E2E tests cover critical user flows (playback, queue, DSP)
- [ ] UI tests validate all interactive components
- [ ] CI runs all tests on every PR
- [ ] Code coverage reports generated and tracked
- [ ] Test data fixtures and mocks documented
- [ ] Flaky tests eliminated (>99% pass rate)

## Coverage Breakdown Target

| Platform | Current | Target | New Tests Needed |
|----------|---------|--------|------------------|
| Android Core | 60% | 75% | 75-100 tests |
| Android UI | 20% | 60% | 30-50 tests |
| Web Core | 40% | 70% | 40-60 tests |
| Web UI | 10% | 50% | 20-30 tests |
| **Overall** | **45%** | **70%** | **~200 tests** |

## Dependencies

- Espresso or Compose UI Testing (Android)
- Playwright or Cypress (Web E2E)
- Mock Mouseion API (WireMock or similar)
- Test audio files (various formats)
- CI/CD pipeline (GitHub Actions already configured)

## Out of Scope

- Performance profiling (separate issue #64)
- Load testing (post-MVP)
- Accessibility testing (future enhancement)
- Cross-browser compatibility testing (focus on Chrome/Firefox/Safari manual testing)
- Security testing (separate audit #59)

## Implementation Strategy

### Phase 1: Integration Tests (Week 1)
- Android: API client, database, services
- Web: API client, state management

### Phase 2: E2E Tests (Week 1-2)
- Android: Espresso tests for critical flows
- Web: Playwright tests for core features

### Phase 3: UI Tests (Week 2)
- Android: Compose UI tests
- Web: React Testing Library

### Phase 4: Edge Cases (Week 2-3)
- Network errors, empty states, permissions
- Large datasets and stress tests

### Phase 5: CI Integration (Week 3)
- Automate test runs on PR
- Coverage reporting
- Flaky test elimination

## Platform(s)

All (Android, Web, Infrastructure)

## Size Estimate

**xl** (3+ days, 20-30 hours)

**Breakdown:**
- Integration tests: 8-10 hours
- E2E tests: 6-8 hours
- UI tests: 4-6 hours
- Edge cases: 3-4 hours
- CI integration: 2-3 hours
- Documentation: 1-2 hours
