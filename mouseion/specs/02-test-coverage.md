# Spec 02: Test Coverage Expansion

**Status:** Active (58 new tests this sprint)
**Priority:** High
**Issues:** #90

## Goal

Current test coverage is estimated at ~5%. Expand to meaningful coverage of critical paths: API controllers, services, repositories, and metadata providers. Issue #90 identified this as the highest-priority infrastructure need.

## Phases

### Phase 1: Unit test foundation
- [x] Progress API controller tests (ProgressController, ContinueWatchingController, SessionsController) — 29 tests
- [x] Streaming controller tests (mime types, file not found, missing DB record) — 3 tests + 14 theory cases
- [x] Search controller tests (query validation, limit clamping, result mapping) — 7 tests
- [x] Health controller tests (health check mapping, empty state) — 2 tests
- [x] MediaProgress repository tests (upsert, get-in-progress, delete, user filtering) — 10 tests
- [x] PlaybackSession repository tests (insert, active sessions, end session, delete) — 9 tests
- [ ] Remaining controller tests (Library, Notifications, Tags, etc.)
- [ ] Service tests for business logic (add, search, import workflows)
- [ ] Quality parser tests (already strong — 131 tests)

### Phase 2: Integration tests
- [ ] API integration tests (TestServer + real database)
- [ ] Metadata provider tests (mock HTTP, verify parsing)
- [ ] Download client integration tests
- [ ] SignalR hub tests

### Phase 3: Coverage infrastructure
- [ ] Coverage reporting in CI (OpenCover → SonarCloud)
- [ ] Coverage gate: fail PR if coverage drops on changed files
- [ ] Test data builders for common entities

## Dependencies

- None — can start immediately

## Notes

- Quality parser tests (131) are the strongest area. Controllers and services are nearly untested.
- OpenTelemetry integration (#157) added recently — needs test coverage too.
- LoggerMessage migration (#154) was partial — remaining migration could be tested as completed.
