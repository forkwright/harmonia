---
name: Add albumId to Track model
about: Add albumId field to Track model for reliable album associations
title: '[Backend] Add albumId to Track model for reliable album associations'
labels: 'enhancement, blocked-mouseion, s'
assignees: ''
---

## Context

Current Track model uses album name strings for association, which is fragile across metadata updates. Need proper albumId foreign key to enable reliable playback speed memory at album level and prevent association breaks during metadata changes.

## Scope

- Add `albumId` field to Track model (UUID or int foreign key)
- Update `GET /api/v3/tracks` response to include albumId
- Update `GET /api/v3/albums/{id}/tracks` to use albumId joins instead of string matching
- Database migration: Link existing tracks to albums by name matching, log orphaned tracks
- Add foreign key constraint with cascading delete behavior

## Acceptance Criteria

- [ ] Track API returns albumId field in response
- [ ] Album-track associations use FK constraints
- [ ] Migration successfully links existing tracks (report match rate)
- [ ] Orphaned tracks logged for manual review
- [ ] Akroasis playback speed memory can use reliable albumId
- [ ] API documentation updated with new field

## Dependencies

**Mouseion Backend:** Database schema change required. This is a Mouseion-owned task.

**Akroasis:** Will adapt PlaybackSpeedManager to use albumId once available (estimated 1-2 hours client work).

## Out of Scope

- Client-side changes (Akroasis will adapt after backend ready)
- Multi-disc album handling (can use same albumId, separate issue if needed)
- Album deduplication (future enhancement)

## Platform(s)

Backend (Mouseion)

## Size Estimate

**s** (1-4 hours)

**Breakdown:**
- Schema change + migration: 2 hours
- API updates: 1 hour
- Testing: 1 hour
