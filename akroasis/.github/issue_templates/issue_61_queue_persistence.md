---
name: Queue Reordering Backend Persistence
about: Implement backend queue state persistence for drag-reorder
title: '[Android] Implement backend queue state persistence for drag-reorder'
labels: 'enhancement, android, ui, blocked-mouseion, s'
assignees: ''
---

## Context

Current drag-to-reorder queue management is visual-only (local state). Queue order is not persisted to backend, so reordering is lost on app restart or across devices. Need server-side queue/playlist API for persistent state.

**Current behavior:**
- User drags queue item to new position
- UI updates immediately (smooth UX)
- Order not saved to backend
- Lost on app restart or device switch

**Desired behavior:**
- Drag-to-reorder persists to backend
- Queue state syncs across devices
- Survives app restart

## Scope

### Backend API Requirements (Mouseion)

Need queue management endpoints:

1. **Update queue order**
   - `PUT /api/v3/queue/reorder`
   - Body: `{ "trackIds": [uuid1, uuid2, uuid3, ...] }`
   - Response: Updated queue state

2. **Get current queue**
   - `GET /api/v3/queue`
   - Response: `{ "tracks": [...], "currentIndex": 0 }`

3. **Persist queue on changes**
   - Auto-save queue state on add/remove/reorder
   - Associate queue with user session

### Android Client Changes

1. **QueueViewModel**
   - Add `syncQueueToBackend()` method
   - Call after drag-reorder completes
   - Debounce rapid reorders (wait 1s after last change)

2. **Drag-to-reorder callback**
   - On drop event, call `syncQueueToBackend()`
   - Show sync status (syncing/synced/error)

3. **Queue restoration**
   - On app start, fetch queue from backend
   - Merge with local state if conflict

## Acceptance Criteria

- [ ] Backend API endpoints implemented (Mouseion)
- [ ] Android client syncs queue order to backend
- [ ] Drag-to-reorder persists across app restarts
- [ ] Queue state syncs across devices (Android + Web)
- [ ] Debouncing prevents excessive API calls (max 1 call/second)
- [ ] Sync status indicator shows progress
- [ ] Conflict resolution: Server state wins on startup
- [ ] Works offline: Queue locally, sync when online

## Dependencies

**Mouseion Backend:** API endpoints must be implemented first.

**Android:** Client changes estimated 2-3 hours after backend ready.

## Out of Scope

- Undo/redo for backend-synced changes (local undo/redo remains)
- Multi-device conflict resolution (server wins for MVP)
- Queue versioning (optimistic concurrency control deferred)
- Collaborative queue editing (multi-user feature, post-MVP)

## Implementation Notes

### Sync Strategy

- **Optimistic UI updates**: Update UI immediately, sync in background
- **Debouncing**: Wait 1 second after last reorder before syncing
- **Error handling**: If sync fails, show error toast, retry on next network
- **Offline mode**: Queue locally, sync when network available

### Conflict Resolution (MVP)

- On app start: Server state overwrites local state
- On sync conflict: Last write wins (server timestamp)
- Future: Operational Transform or CRDT for collaborative editing

## Platform(s)

Android (client), Backend (Mouseion)

## Size Estimate

**s** (1-4 hours Android, 2-3 hours Backend)

**Breakdown:**
- Backend API: 2-3 hours (endpoints + persistence)
- Android sync logic: 1-2 hours
- Testing and conflict scenarios: 1-2 hours
