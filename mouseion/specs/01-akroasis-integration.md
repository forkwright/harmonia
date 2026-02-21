# Spec 01: Akroasis Integration

**Status:** Active
**Priority:** High
**Issues:** #58

## Goal

Complete the API surface that Akroasis needs for full playback experience. Progress tracking, session management, cross-device sync, and adaptive streaming endpoints. This is the bridge between Mouseion's library management and Akroasis's playback.

## Phases

### Phase 1: Progress & sessions
- [ ] POST /api/v3/progress/{mediaId} — save playback position
- [ ] GET /api/v3/progress/{mediaId} — restore position
- [ ] GET /api/v3/continue — in-progress items across media types
- [ ] Session tracking (start/stop/duration per playback)

### Phase 2: Cross-device sync
- [ ] Queue state persistence (server-side queue for multi-device)
- [ ] Playback transfer endpoint (hand off between devices)
- [ ] Conflict resolution for concurrent position updates

### Phase 3: Streaming enhancements
- [ ] Adaptive transcoding endpoint (lossless → opus/aac by client preference)
- [ ] Bandwidth estimation hints in stream response headers
- [ ] Cover art resize endpoint (thumbnails for mobile)

## Dependencies

- Akroasis audiobook support shipped (PR #140) — client-side ready
- Session tracking needs new DB table + migration

## Notes

- Progress API partially scaffolded in Phase 2 but never completed.
- Akroasis currently uses mock data for continue-listening; wiring to real API is blocked on this spec.
