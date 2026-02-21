# Spec 01: Audiobook & Podcast Completion

**Status:** Active
**Priority:** High
**Issues:** #49, #65, #99

## Goal

Complete the audiobook experience and add podcast support. Audiobooks are the primary Mouseion use case — the web and Android players both have foundations but need polish: cross-device position sync and unified Continue feed. Podcasts are the logical next media type after audiobooks.

## Phases

### Phase 1: Audiobook polish (DONE)
- [x] Sleep timer (15/30/45/60 min, end-of-chapter) — PR #143
- [x] Per-audiobook playback speed persistence — PR #143
- [x] Bookmark/clip support (save position + optional note) — PR #143
- [ ] Cross-device position sync via Mouseion progress API — **UNBLOCKED** (ProgressController exists)
- [ ] Unified Continue feed across media types (#65) — **UNBLOCKED** (ContinueWatchingController exists)

### Phase 2: Podcast player — **UNBLOCKED** (PodcastController + PodcastEpisodesController exist)
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

- ~~Mouseion podcast API (not yet built)~~ **AVAILABLE** — PodcastController, PodcastEpisodesController
- ~~Queue persistence (#99, blocked-mouseion)~~ **AVAILABLE** — QueueController with cross-device sync

## Notes

- Web audiobook support shipped in PR #140. Android audiobook in PR #123/#124.
- Sleep timer, speed, bookmarks shipped in PR #143.
- Mouseion now has progress, continue, queue, and podcast APIs — all phases unblocked.
