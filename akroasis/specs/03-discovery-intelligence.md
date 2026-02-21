# Spec 03: Discovery & Intelligence

**Status:** Active
**Priority:** Medium
**Issues:** #66, #67, #69, #76, #77, #78, #84, #86

## Goal

Make the library intelligent. Go beyond browse/search into discovery: radio mode from Last.fm similar tracks, listening stats, historical visualizations, and "rediscover forgotten favorites" features. This is what differentiates Akroasis from a basic Mouseion client.

## Phases

### Phase 1: Radio & recommendations (PARTIAL)
- [x] Radio mode via Last.fm similar tracks API (#75) — PR #144
- [ ] "New For You" recommendations dashboard (#76) — partially client-side
- [ ] "Rediscover" forgotten favorites (#78) — **UNBLOCKED** (HistoryController exists)
- [ ] "On This Day" historical playback (#77) — **UNBLOCKED** (HistoryController.GetSince)

### Phase 2: Visualization & stats — **UNBLOCKED** (SessionsController + HistoryController exist)
- [ ] Session tracking with daily/weekly/monthly stats (#66)
- [ ] Listening heatmap visualization (#67)
- [ ] Year in Review / historical visualization (#86)

### Phase 3: Rich metadata
- [x] Synchronized lyrics display — LRC format (#81) — PR #144
- [ ] Credits browsing — producer, engineer, musician (#69) — needs Mouseion metadata enrichment
- [ ] Magazine-style artist/album detail pages (#68) — needs rich metadata

### Phase 4: Analysis
- [ ] Listening DNA / taste analysis (#84)
- [x] High-res artwork zoom viewer (#70) — in progress

## Dependencies

- ~~Most visualization features depend on Mouseion session/stats APIs~~ **AVAILABLE** — SessionsController, HistoryController
- Credits data still depends on Mouseion metadata enrichment (#69)

## Notes

- Radio mode and lyrics shipped in PR #144.
- Mouseion now has HistoryController (paged, by date, by media item) and SessionsController — all history-based features are unblocked.
- "On This Day" and "Rediscover" are quick wins now that history API exists.
