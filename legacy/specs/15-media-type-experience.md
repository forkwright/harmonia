# Spec 15: per-media-type experience

**Status:** Draft
**Priority:** Medium
**Depends On:** Spec 09 (design system), Spec 12 (signal path for music player)

## Goal

Music, audiobooks, and podcasts are different media with different needs. A music player prioritizes album art, signal quality, and quick track switching. An audiobook player prioritizes position in the book, sleep timer, and playback speed. A podcast player prioritizes episode management, show notes, and completion tracking. Akroasis should feel like the best player for whichever media type you're currently using, not like a music player that also grudgingly handles audiobooks.

Each media type gets a player experience tuned to its needs. The shared infrastructure (audio pipeline, transport controls, queue) stays unified. The surface, what you see and interact with, adapts to what you're listening to.

## Design philosophy

**Context-appropriate controls.** A music player doesn't need a sleep timer front and center. An audiobook player doesn't need a signal path visualization. Show what matters, hide what doesn't, not by removing controls, but by adjusting their prominence and position.

**Seamless switching.** You pause your audiobook, play a song, then return to the book. The audiobook remembers where you were. The transition should feel natural; the UI shifts its posture, not its identity.

**Media-type detection, not user declaration.** The player knows it's playing an audiobook because the backend says it is. No "switch to audiobook mode" setting. The UI adapts automatically based on what's loaded.

## Phases

### Phase 1: music player (refinement)

The music player is the default and most developed. Refinements for best-in-class:

- [ ] **Album art dominance.** Art is the largest element on the player page. Full-width on mobile, 400px+ on desktop. Click to zoom (existing ArtworkViewer).
- [ ] **Signal path always visible** (from Spec 12). The defining music feature: every stage of your audio pipeline, quality-coded.
- [ ] **Format badge.** FLAC/WAV/ALAC in one color tier, MP3/AAC in another. At-a-glance quality assessment.
- [ ] **Gapless indicator.** When the next track in the queue is from the same album and gapless is active, show a subtle "gapless →" indicator.
- [ ] **Quick album access.** Tap album name on player → navigate to album in library. Tap artist → navigate to artist.
- [ ] **Queue preview.** Show next 2-3 tracks below transport controls. Collapsible. Click to skip to.
- [ ] **Radio mode indicator.** When radio mode (Last.fm similar tracks) is active, show pulsing radio icon and "Radio from: [seed track]."

### Phase 2: audiobook player

A dedicated player surface when audiobook content is loaded.

```
┌─────────────────────────────────────────┐
│                                         │
│        [Book Cover — Square]            │
│                                         │
│   The Name of the Wind                  │
│   Patrick Rothfuss · Read by Nick Podehl│
│                                         │
│   Chapter 7: The Half-Turned World      │
│   ━━━━━━━━━━━━━━━━━━━━━○──── 2:34:17   │
│   Overall: ━━━━━━━━━○──────── 47%       │
│                                         │
│         ⏪30   ▶   30⏩                  │
│                                         │
│   [1.25x]  [Sleep: 45min]  [Bookmark]   │
│                                         │
│   Chapters ▾                            │
│   ├ Ch 5: The Tar Baby                  │
│   ├ Ch 6: The University  ✓             │
│   ├ Ch 7: The Half-Turned World  ◀ NOW  │
│   ├ Ch 8: The Draccus                   │
│   └ Ch 9: Portico                       │
│                                         │
└─────────────────────────────────────────┘
```

- [ ] **Dual progress bars.** Chapter progress (primary) and book overall progress (secondary, smaller). User always knows where they are in both scales.
- [ ] **Skip 30s buttons** replace previous/next track. Configurable: 10s, 15s, 30s, 60s.
- [ ] **Speed control prominent.** Audiobook users change speed constantly. Button with current speed, tap to cycle presets, long-press for fine adjustment.
- [ ] **Sleep timer prominent.** Quick presets (15, 30, 45, 60 min, end of chapter). Active timer shown with countdown.
- [ ] **Chapter list.** Collapsible, shows all chapters with completion checkmarks. Current chapter highlighted. Tap to jump.
- [ ] **Bookmark button.** One-tap to save current position with optional note. Bookmarks listed in expandable section.
- [ ] **No signal path.** Audiobook listeners don't care about FLAC vs MP3 in the same way. The format badge can exist but the signal chain doesn't need player-page real estate.
- [ ] **No album art zoom.** Book covers are informational, not art. Keep them prominent but don't optimize for fullscreen viewing.
- [ ] **Position persistence.** Cross-device via Mouseion progress API. Resume exactly where you left off.

### Phase 3: podcast player

Podcast listeners want to manage episodes and shows, not admire album art.

```
┌─────────────────────────────────────────┐
│                                         │
│   [Show Art]  The Tim Ferriss Show      │
│               Episode 742               │
│               "Naval Ravikant on..."    │
│                                         │
│   ━━━━━━━━━━━━━━━━━━○──── 1:47:23      │
│                                         │
│         ⏪15   ▶   30⏩                  │
│                                         │
│   [1.5x]  [Sleep: Off]  [Mark Played]  │
│                                         │
│   Show Notes ▾                          │
│   Naval discusses wealth creation...    │
│   Links: [tim.blog/naval] [...]         │
│                                         │
│   More Episodes ▾                       │
│   ├ #741: Dr. Andrew Huberman  ✓ played │
│   ├ #740: Rick Rubin            2h 15m  │
│   └ #739: Brené Brown           1h 45m  │
│                                         │
└─────────────────────────────────────────┘
```

- [ ] **Asymmetric skip.** Back 15s, forward 30s. Podcast convention; you rewind less than you skip.
- [ ] **Show notes expandable.** Links rendered as clickable. This is where podcast value often lives.
- [ ] **Mark played button.** Prominent; podcast listeners aggressively manage completion state.
- [ ] **Episode list from same show.** Below the player, not in a separate page. Recent episodes with play state.
- [ ] **Speed persistent per show.** You might listen to one show at 1.5x and another at 1x. Speed follows the show, not global.
- [ ] **Auto-mark played.** When >90% of episode is consumed, auto-mark as played with undo option.
- [ ] **Download indicator.** For offline-capable builds (Tauri, Android): show download state, enable offline playback.

### Phase 4: player surface switching

The unified system that selects the right player UI based on content type.

- [ ] `mediaType` field on the currently-playing item determines which player surface renders
- [ ] Mini-player bar adapts: music shows signal quality dot, audiobook shows chapter progress, podcast shows episode title
- [ ] Transition between player types: if you pause an audiobook and play a song, the player UI smoothly transitions. No jarring switch.
- [ ] "Return to audiobook" persistent indicator: if an audiobook was paused and music is playing, a subtle pill shows "↩ The Name of the Wind · Ch 7 · 2:34:17"; tap to switch back
- [ ] Audio focus rules: playing music doesn't discard audiobook position. They coexist as separate streams (only one active at a time, positions preserved for both)

### Phase 5: mini-player adaptation

The persistent mini-player bar (Spec PR #179) should reflect the current media type.

| Element | Music | Audiobook | Podcast |
|---------|-------|-----------|---------|
| Art | Album cover | Book cover | Show art |
| Primary text | Track title | Book title | Episode title |
| Secondary text | Artist · Album | Chapter N | Show name |
| Quality dot | Signal quality tier | Hidden | Hidden |
| Progress | Track progress | Chapter progress | Episode progress |
| Controls | Play/Pause | Play/Pause | Play/Pause |

## Technical notes

### Media type detection

```typescript
type MediaType = 'music' | 'audiobook' | 'podcast'

function getMediaType(): MediaType {
  if (podcastStore.currentEpisode) return 'podcast'
  if (audiobookStore.currentBook) return 'audiobook'
  return 'music'
}
```

The player page component renders conditionally:

```tsx
function PlayerPage() {
  const mediaType = getMediaType()
  
  switch (mediaType) {
    case 'audiobook': return <AudiobookPlayer />
    case 'podcast': return <PodcastPlayer />
    default: return <MusicPlayer />
  }
}
```

### Shared infrastructure

All three player surfaces share:
- `useWebAudioPlayer` hook (transport controls, seek, volume)
- Audio pipeline (decode → EQ → compressor → volume → output)
- Mini-player bar (with media-type-specific rendering)
- History tracking (all plays recorded regardless of type)
- Queue system (unified queue, mixed media types)

## Dependencies

- Mouseion backend: audiobook chapters, podcast episodes, progress API: all **exist**
- Spec 08 for consistent component styling across player surfaces
- Spec 12 for signal path in music player surface
- Existing stores: `playerStore`, `podcastStore`, `useAudiobookStore`

## Notes

- The audiobook player is arguably more important than the music player for Akroasis's market position. Spec 07 (competitive analysis) found that Audiobookshelf has 11.8K stars; audiobook users are underserved and passionate. A beautiful audiobook player experience in a unified app is the #1 differentiator.
- Podcast player is third priority. Get music and audiobooks excellent first. Podcast is table stakes; it needs to work, it doesn't need to be best-in-class.
- The "return to audiobook" pill (Phase 4) is a killer feature for the person who listens to an audiobook during commute, plays music at the gym, and switches back to the audiobook after. Seamless context preservation.
- Speed persistence per show/book is important. Don't make users re-set speed every time they switch content.
- The player surface switching should be animated but fast. Crossfade the content, don't animate a page transition. It's the same page, different content.
