# Spec 03: Discovery & Intelligence

**Status:** Active
**Priority:** Medium
**Issues:** #66, #67, #69, #76, #77, #78, #84, #86

## Goal

Make the library intelligent. Go beyond browse/search into discovery: radio mode from Last.fm similar tracks, listening stats, historical visualizations, and "rediscover forgotten favorites" features. This is what differentiates Akroasis from a basic Mouseion client.

## Phases

### Phase 1: Radio & recommendations (PARTIAL)
- [x] Radio mode via Last.fm similar tracks API (#75) — PR #144
- [x] "New For You" recommendations dashboard (#76) — computeNewForYou + NewForYouSection on DiscoveryPage
- [x] "Rediscover" forgotten favorites (#78) — PR #174 (RediscoverSection + computeRediscoverCandidates)
- [x] "On This Day" historical playback (#77) — PR #174 (OnThisDaySection + computeOnThisDay)

### Phase 2: Visualization & stats — **UNBLOCKED** (SessionsController + HistoryController exist)
- [x] Session tracking with daily/weekly/monthly stats (#66) — PR #174 (ListeningStatsSection + computeListeningStats)
- [x] Listening heatmap visualization (#67) — PR #174 (ListeningHeatmap + computeDailyActivity)
- [x] Year in Review / historical visualization (#86) — PR #175 (YearInReviewSection + computeYearInReview)

### Phase 3: Rich metadata
- [x] Synchronized lyrics display — LRC format (#81) — PR #144
- [ ] Credits browsing — producer, engineer, musician (#69) — needs Mouseion metadata enrichment
- [ ] Magazine-style artist/album detail pages (#68) — needs rich metadata

### Phase 4: Analysis
- [x] Listening DNA / taste analysis (#84) — PR #177
- [x] High-res artwork zoom viewer (#70) — PR #148 (ArtworkViewer + artworkViewerStore)

## Dependencies

- ~~Most visualization features depend on Mouseion session/stats APIs~~ **AVAILABLE** — SessionsController, HistoryController
- Credits data still depends on Mouseion metadata enrichment (#69)

## Notes

- Radio mode and lyrics shipped in PR #144.
- Mouseion now has HistoryController (paged, by date, by media item) and SessionsController — all history-based features are unblocked.
- "On This Day" and "Rediscover" are quick wins now that history API exists.
