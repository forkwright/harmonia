# Spec 03: Discovery & Intelligence

**Status:** Draft
**Priority:** Medium
**Issues:** #66, #67, #69, #75, #76, #77, #78, #81, #84, #86

## Goal

Make the library intelligent. Go beyond browse/search into discovery: radio mode from Last.fm similar tracks, listening DNA analysis, historical visualizations, lyrics display, and "rediscover forgotten favorites" features. This is what differentiates Akroasis from a basic Mouseion client.

## Phases

### Phase 1: Radio & recommendations
- [ ] Radio mode via Last.fm similar tracks API (#75)
- [ ] "New For You" recommendations dashboard (#76)
- [ ] "Rediscover" forgotten favorites (#78)
- [ ] "On This Day" historical playback (#77)

### Phase 2: Visualization & stats
- [ ] Session tracking with daily/weekly/monthly stats (#66)
- [ ] Listening heatmap visualization (#67)
- [ ] Year in Review / historical visualization (#86)

### Phase 3: Rich metadata
- [ ] Synchronized lyrics display — LRC format (#81)
- [ ] Credits browsing — producer, engineer, musician (#69)
- [ ] Magazine-style artist/album detail pages (#68)

### Phase 4: Analysis
- [ ] Listening DNA / taste analysis (#84)
- [ ] High-res artwork zoom viewer (#70)

## Dependencies

- Most visualization features depend on Mouseion session/stats APIs (#66, #67 blocked)
- Credits data depends on Mouseion metadata enrichment (#69 blocked)
- Last.fm similar tracks API is client-callable (no backend dependency)

## Notes

- Radio mode (Phase 1) is highest-value with lowest dependency — can ship independently.
- Lyrics can be client-side (fetch from LRCLIB or embedded tags).
- Taste analysis is aspirational — needs significant listening history before it's meaningful.
