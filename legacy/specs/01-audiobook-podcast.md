# Spec 01: audiobook & podcast completion

**Status:** Active
**Priority:** High
**Issues:** #49, #65, #99

## Goal

Complete the audiobook experience and add podcast support. Audiobooks are the primary Mouseion use case; the web and Android players both have foundations but need polish: cross-device position sync and unified Continue feed. Podcasts are the logical next media type after audiobooks.

## Phases

### Phase 1: audiobook polish (DONE)
- [x] Sleep timer (15/30/45/60 min, end-of-chapter): PR #143
- [x] Per-audiobook playback speed persistence: PR #143
- [x] Bookmark/clip support (save position + optional note): PR #143
- [x] Cross-device position sync via Mouseion progress API: PR #174 (syncService + sessionManager)
- [x] Unified Continue feed across media types (#65): PR #174 (continueStore + ContinueListening)

### Phase 2: podcast player (**UNBLOCKED**, PodcastController + PodcastEpisodesController exist)
- [x] Podcast library (subscribe, unsubscribe, episode list): PR #175 (podcastStore subscribe/unsubscribe + PodcastsPage)
- [x] Episode playback with position tracking: PR #159 (podcastStore) + PR #174 (session tracking)
- [ ] Chapter markers (if present in feed)
- [ ] Download for offline listening
- [x] Auto-cleanup of played episodes: PR #177 (auto-mark played + episode filtering)

### Phase 3: unified navigation
- [x] Single nav across music/audiobooks/podcasts (#49): PR #177
- [x] Cross-media search results: searchStore + SearchDropdown in Navigation
- [x] Continue feed that spans all media types: PR #174 (ContinueListening component)

## Dependencies

- ~~Mouseion podcast API (not yet built)~~ **AVAILABLE**: PodcastController, PodcastEpisodesController
- ~~Queue persistence (#99, blocked-mouseion)~~ **AVAILABLE**: QueueController with cross-device sync

## Notes

- Web audiobook support shipped in PR #140. Android audiobook in PR #123/#124.
- Sleep timer, speed, bookmarks shipped in PR #143.
- Mouseion now has progress, continue, queue, and podcast APIs; all phases unblocked.
