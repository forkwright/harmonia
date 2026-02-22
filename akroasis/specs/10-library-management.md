# Spec 10: Library Management

**Status:** Active
**Priority:** High

## Goal

Add favorites, manual playlists, and smart playlists — the core library organization features every music player needs. Favorites is a track-only feature backed by the server API. Manual playlists support drag-to-reorder. Smart playlists evaluate rule-based criteria (metadata + listening data + Last.fm enrichment) with live or snapshot modes.

## Greek Names

| Feature | Name | Meaning |
|---------|------|---------|
| Favorites | **Thymesis** (thu-MAY-sis) | Spirited recognition — thymos marks what's worth returning to |
| Manual playlists | **Sylloges** (sil-lo-GAYS) | Deliberate gathering — curation as intentional collection |
| Smart playlists | **Kanon** (ka-NOHN) | The measuring rod — the rule that generates the collection |

## Phases

### Phase 1: Favorites / Thymesis
- [ ] Create `thymesisStore.ts` (Set<number> of IDs, optimistic toggle, loadFavorites)
- [ ] Add API client methods: POST/DELETE favorites/:trackId, GET favorites, GET favorites/ids
- [ ] Create `HeartButton.tsx` (filled/outline heart, click toggles)
- [ ] Place HeartButton in PlayerPage, LibraryPage track rows, QueuePage rows
- [ ] Add mock handlers (stateful in-memory Set)
- [ ] Add `/favorites` route or Library view tab
- [ ] Tests for store (optimistic toggle, rollback on failure) and component

### Phase 2: Manual playlists / Sylloges
- [ ] Add `Playlist` type to `types/index.ts`
- [ ] Create `playlistStore.ts` (CRUD, track add/remove/reorder)
- [ ] Add API client methods for full playlist CRUD + track management
- [ ] Create `PlaylistsPage.tsx` (list with create/delete, Favorites pinned at top)
- [ ] Create `PlaylistDetailPage.tsx` (tracks with DndKit drag-to-reorder)
- [ ] Create `AddToPlaylistMenu.tsx` (dropdown on track rows)
- [ ] Add routes `/playlists` and `/playlists/:id` to App.tsx
- [ ] Add "Playlists" to Navigation.tsx
- [ ] Add mock handlers (stateful in-memory playlists)
- [ ] Tests for store, pages, menu component

### Phase 3: Smart playlists / Kanon
- [ ] Add smart playlist types (SmartPlaylist, RuleGroup, Rule, field/operator enums)
- [ ] Create `smartPlaylistStore.ts` (client-side, localStorage persistence)
- [ ] Create `utils/smartPlaylistEngine.ts` (pure evaluation functions)
- [ ] Create `SmartPlaylistEditor.tsx` (recursive rule group builder)
- [ ] Integrate with playlistStore (smart playlists appear in same list)
- [ ] Live mode: re-evaluate on open. Snapshot mode: cache results, manual refresh
- [ ] Tests for rules engine (every field/operator combo), store, editor

### Phase 4: Last.fm enrichment for smart playlists
- [ ] Add `lastfmTag` and `similarityScore` rule fields
- [ ] Wire `EvaluationContext` to Last.fm sync data (Spec 13)
- [ ] Add `contextMood` and `contextTimeOfDay` fields (wired to context engine, Spec 13)
- [ ] Tests for enriched rule evaluation

## Dependencies

- Favorites requires Mouseion API endpoints: `POST/DELETE /api/v3/favorites/:trackId`, `GET /api/v3/favorites`, `GET /api/v3/favorites/ids`
- Playlists require Mouseion API: full CRUD on `/api/v3/playlists` with track management sub-routes
- Smart playlist enrichment depends on Spec 13 (Last.fm sync + context engine)
- Smart playlists are client-side — no backend dependency for the rules engine itself

## Notes

- Favorites is NOT a playlist — it's a separate concept with its own store and API. But it appears as a pinned virtual entry in the playlists list for discoverability.
- Music tracks only for playlists — audiobooks and podcasts are excluded.
- Smart playlist engine follows `discoveryStats.ts` pattern: pure functions, all data passed in, fully testable. The EvaluationContext bundles all external data (play records, Last.fm tags, context signals).
- Rule operators are contextual: text fields get is/contains/etc, numeric get greater/less/between, date fields get inLast/notInLast/before/after.
- Nested rule groups support AND/OR boolean logic at each level.
