# Spec 13: Intelligence Engine

**Status:** Draft
**Priority:** Medium

## Goal

Build the intelligence layer that makes Akroasis understand your listening habits: full Last.fm bidirectional sync, a context engine that knows when and what you listen to, and a Spotify Wrapped-tier year-in-review experience. This extends the existing discovery features (Spec 03) from static statistics into active intelligence that enhances every surface in the app.

## Greek Names

| Feature | Name | Meaning |
|---------|------|---------|
| Last.fm sync | **Syndesmos** (SIN-des-mos) | The binding — connective tissue between local and external listening data |
| Context engine | (no Greek name) | Practical feature — context-aware defaults based on listening patterns |
| Wrapped | **Anaskopos** (ah-NAS-koh-pos) | Looking back — retrospection as a mode of attention |

## Phases

### Phase 1: Last.fm full sync / Syndesmos
- [ ] Extend `api/lastfm.ts` with 10+ new endpoint wrappers (user.getTopTracks, artist.getSimilar, user.getLovedTracks, etc.)
- [ ] Create `lastfmSyncStore.ts` (config, sync state, cached enrichment data)
- [ ] Create `services/lastfmSync.ts` (sync service following scrobbleQueue pattern)
- [ ] Implement favorites sync: Last.fm loved ↔ local Thymesis (Spec 08)
- [ ] Implement incremental history import via `from` timestamp
- [ ] Implement per-artist tag caching (7-day TTL)
- [ ] Implement top-50 artist similarity caching
- [ ] Periodic background sync (configurable interval, rate-limited 5 req/s)
- [ ] Create `LastfmSyncSettings.tsx` in SettingsPage
- [ ] Tests for API wrappers, sync logic, rate limiting, conflict resolution

### Phase 2: Context engine
- [ ] Create `utils/contextEngine.ts` (pure functions — signals, patterns, matching, recommendations)
- [ ] Create `contextStore.ts` (patterns, recommendations, build on start + periodic)
- [ ] Implement signal computation (time-of-day, day-of-week, season, session state)
- [ ] Implement behavioral pattern builder (group sessions by context, compute dominant preferences)
- [ ] Implement pattern matching (find patterns matching current signals)
- [ ] Implement recommendation generator (from matched patterns to concrete tracks)
- [ ] Create `ContextSuggestionsSection.tsx` for DiscoveryPage
- [ ] Integrate with queue empty state (show context-based suggestions)
- [ ] Tests for pattern building, matching, recommendations with synthetic temporal data

### Phase 3: Wrapped / Anaskopos
- [ ] Extend `utils/discoveryStats.ts` with wrapped computation functions
- [ ] Create `wrappedStore.ts` (period selection, data, slide navigation)
- [ ] Implement: genre distribution, listening streak, new discoveries, hourly/daily distributions
- [ ] Merge local history + Last.fm data for complete picture
- [ ] Create `WrappedPage.tsx` (full-viewport slide presentation)
- [ ] Create wrapped slide components (stat, top lists, patterns, DNA, summary)
- [ ] Support year/quarter/month periods
- [ ] Add route `/wrapped` to App.tsx
- [ ] Slide navigation: swipe, arrow keys, dots, auto-advance 8s
- [ ] Tests for all computation functions, store, period selection

## Dependencies

- Syndesmos depends on Last.fm API key already being stored (existing feature)
- Syndesmos favorites sync depends on Thymesis (Spec 09 Phase 1)
- context engine depends on session data from Mouseion (already available via SessionsController)
- context engine enrichment depends on Syndesmos tag data (Phase 1)
- Anaskopos depends on Syndesmos for Last.fm enrichment (optional — works without)
- Anaskopos extends existing `computeYearInReview` in `discoveryStats.ts`

## Notes

- Build order matters: Syndesmos first (produces data), then Kanon/context engine (consume data), then Anaskopos (consumes everything).
- context engine is deliberately invisible — it enhances existing surfaces (discovery, queue, search) rather than adding a new page. Users should feel it, not see it.
- Conflict resolution for Last.fm sync: Last.fm is source of truth for historical data, local is source of truth for real-time. Bidirectional with last-write-wins per item.
- Wrapped slides use animated transitions on the bronze palette. Full-viewport, immersive. Pattern follows competitive Spotify Wrapped with self-hosted data sovereignty.
- All context engine computation is client-side. No ML, no external API — statistical pattern extraction from session timestamps and track metadata.
- Behavioral patterns need minimum 5 sessions per context group for confidence > 0.5.
