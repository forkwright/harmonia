# Spec 01: Audiobook & Podcast Completion

**Status:** Active
**Priority:** High
**Issues:** #47, #49, #65, #99

## Goal

Complete the audiobook experience and add podcast support. Audiobooks are the primary Mouseion use case — the web and Android players both have foundations but need polish: sleep timer, speed per-book, bookmarks, and cross-device position sync. Podcasts are the logical next media type after audiobooks.

## Phases

### Phase 1: Audiobook polish
- [ ] Sleep timer (15/30/45/60 min, end-of-chapter)
- [ ] Per-audiobook playback speed persistence
- [ ] Bookmark/clip support (save position + optional note)
- [ ] Cross-device position sync via Mouseion progress API
- [ ] Unified Continue feed across media types (#65)

### Phase 2: Podcast player
- [ ] Podcast library (subscribe, unsubscribe, episode list)
- [ ] Episode playback with position tracking
- [ ] Chapter markers (if present in feed)
- [ ] Download for offline listening
- [ ] Auto-cleanup of played episodes

### Phase 3: Unified navigation
- [ ] Single nav across music/audiobooks/podcasts (#49)
- [ ] Cross-media search results
- [ ] Continue feed that spans all media types

## Dependencies

- Mouseion podcast API (not yet built — backend work required)
- Queue persistence (#99, blocked-mouseion)

## Notes

- Web audiobook support shipped in PR #140. Android audiobook in PR #123/#124.
- Podcast feed parsing could be client-side (RSS) to avoid backend dependency for Phase 2.
