# Spec 14: adaptive experience

**Status:** Draft
**Priority:** Medium
**Depends On:** Spec 11 (library browsing for genre/filter defaults), Spec 09 (design system)

## Goal

The system should feel like it knows you without you telling it. Not through explicit preferences or configuration screens, but through observation. The genre you play most becomes the default view. The artist you listened to yesterday rises in search. The time of day shapes what's surfaced. The features you use get emphasis; the ones you don't fade. This is what "molds to its user" means: the UI adapts its posture based on accumulated behavior, not declared settings.

No algorithms. No machine learning. No recommendation engine. Just careful observation of what you actually do, expressed through small adjustments to ordering, defaults, and emphasis. The user should never notice the system adapting; they should just feel like it works the way they'd expect.

## Design philosophy

**Observation, not inference.** The system knows you played Zach Bryan 47 times this month. It doesn't try to figure out that you're feeling nostalgic or going through something. It just makes Zach Bryan easy to find.

**Recency over frequency.** Something you played yesterday matters more than something you played 200 times six months ago. Tastes change. The system should track the change, not anchor to the past.

**Graceful degradation.** A new user with zero history gets a perfectly functional app: alphabetical ordering, no defaults, no bias. Adaptation emerges gradually. First listen, nothing changes. Tenth listen, subtle shifts. Hundredth listen, the app feels like yours.

**Reversible and invisible.** No "personalized for you" banners. No "because you listened to..." explanations. The user can always access everything; adaptation adjusts ordering and defaults, never hides content.

## Phases

### Phase 1: behavioral data collection

Build the client-side observation layer. No backend changes; this is all localStorage + sessionStorage.

```typescript
interface ListeningProfile {
  // Genre affinity: genre → { count, lastPlayed, recentCount }
  genreWeights: Record<string, {
    totalPlays: number
    lastPlayedAt: string      // ISO timestamp
    last30DaysPlays: number
  }>
  
  // Artist affinity: artistId → same shape
  artistWeights: Record<string, {
    totalPlays: number
    lastPlayedAt: string
    last30DaysPlays: number
  }>
  
  // Time-of-day patterns: hour (0-23) → genre distribution
  timePatterns: Record<number, Record<string, number>>
  
  // Feature usage: which nav items, how often
  featureUsage: Record<string, {
    uses: number
    lastUsed: string
  }>
  
  // Session metadata
  lastUpdated: string
  totalListeningMs: number
}
```

- [ ] Create `useListeningProfile` hook that updates on track play, navigation, feature use
- [ ] Persist to `localStorage` (key: `akroasis_profile`)
- [ ] Update genre/artist weights on track play completion (>30 seconds = counted)
- [ ] Track time-of-day listening patterns (hour → genre → count)
- [ ] Track feature usage (which pages visited, search used, filters used)
- [ ] Decay function: weights older than 90 days halved, older than 180 days quartered
- [ ] Maximum profile size: cap at 200 genres, 500 artists, 2KB total

### Phase 2: contextual defaults

Use the listening profile to set intelligent defaults throughout the UI.

**Library defaults:**
- [ ] Default genre filter pre-selected to most-played genre of last 30 days (clearable)
- [ ] Default sort: "Recently played" if user has history, "A-Z" if new user
- [ ] Albums view: recently played albums surfaced first (mixed into alphabetical, not a separate section)

**Search:**
- [ ] Search results weighted by listening profile: artists you've played rank higher than unknown artists with the same name
- [ ] Recent search terms shown on search focus (before typing)

**Navigation emphasis:**
- [ ] Nav items with zero usage in 30 days get `text-muted` styling (not hidden, just quieter)
- [ ] Most-used nav item gets slightly brighter text (subtle, <10% brightness delta)
- [ ] If Podcasts is never visited, its nav item is de-emphasized. If user starts using it, it normalizes within 3 visits

### Phase 3: temporal awareness

The same user at 8 AM and 11 PM has different needs.

- [ ] Time-of-day detection: morning (5-11), afternoon (12-17), evening (18-22), night (23-4)
- [ ] If user consistently plays genre X during time Y, surface genre X albums/artists when opening the app during time Y
- [ ] Implementation: "Suggested" section at top of Library, max 6 items, based on current time's genre weights. Only shown if confidence is high (>10 plays in this time slot for this genre)
- [ ] No "Good morning" greeting or clock UI; the adaptation is in content ordering, not chrome

### Phase 4: continue & momentum

Respect listening momentum. If you played 5 tracks from an album, you probably want the 6th.

- [ ] "Continue listening" section: albums where >25% but <100% of tracks have been played recently
- [ ] Queue awareness: if queue has items, show "Your queue (3 tracks)" prominently
- [ ] Album completion indicator: subtle progress bar on album cards showing % listened
- [ ] After finishing an album, surface similar albums by same artist or in same genre (not algorithmic, just factual relationships: "More from this artist", "Also in Country")

### Phase 5: listening streaks & habits

Light gamification through honest observation. Not badges or achievements; just mirror what the user is doing.

- [ ] Listening streak: "Played music 12 days in a row" (only shown in Discovery or Settings, not pushed)
- [ ] Weekly summary available in Discovery: top artists, total listening time, new artists discovered, format quality distribution
- [ ] This data already exists in the history API; this phase is purely UI presentation

## Anti-patterns (what not to do)

- **Don't hide content.** Adaptation reorders, it never removes. Every album, artist, genre is always accessible.
- **Don't explain the adaptation.** No "Because you listened to Zach Bryan..." cards. The UI just works.
- **Don't use the word "personalized."** Or "recommended." Or "for you." These are marketing words that erode trust.
- **Don't push.** No notifications saying "You haven't listened today!" or "New music from your favorites!" The system is reactive, not proactive.
- **Don't collect what you don't use.** If a data point doesn't drive a specific UI behavior, don't store it.
- **Don't sync this data to the server.** The listening profile is local to the device. Each device adapts independently. This is a privacy feature, not a limitation.

## Technical notes

### Storage budget

The listening profile should fit in ~2KB of localStorage. At that size:
- 100 genres × 30 bytes = 3KB
- 200 artists × 30 bytes = 6KB
- 24 hours × 20 genres × 4 bytes = ~2KB
- Feature usage: ~500 bytes

Total: ~12KB worst case. Well within localStorage limits (5-10MB typical).

### Decay algorithm

```typescript
function decayWeight(weight: GenreWeight): number {
  const daysSinceLast = daysBetween(new Date(weight.lastPlayedAt), new Date())
  
  if (daysSinceLast < 7) return weight.last30DaysPlays * 1.0    // This week: full weight
  if (daysSinceLast < 30) return weight.last30DaysPlays * 0.7   // This month: 70%
  if (daysSinceLast < 90) return weight.totalPlays * 0.3         // This quarter: 30% of total
  if (daysSinceLast < 180) return weight.totalPlays * 0.1        // This half: 10% of total
  return 0                                                        // Older: irrelevant
}
```

Decay runs lazily, calculated on access, not on a timer.

### Confidence threshold

Adaptation should only activate when there's sufficient data to be meaningful.

- Genre default: requires ≥10 plays in a genre in last 30 days
- Time-of-day: requires ≥10 plays in a time slot for a genre
- Nav de-emphasis: requires ≥30 days of app usage with zero visits to that section
- Search weighting: requires ≥5 plays of an artist in last 90 days

Below these thresholds, the UI behaves as if no profile exists. This prevents erratic behavior from small sample sizes.

## Dependencies

- Spec 11 (library browsing) for genre defaults and sort ordering
- History API for initial profile seeding (optional, can build profile from scratch via client observation)
- `playerStore` for track play events
- `useLocation` for navigation tracking

## Notes

- This spec is deliberately conservative. It's easy to add more adaptation later; it's hard to remove adaptation that users have come to rely on. Start with genre defaults and search weighting. Add temporal awareness only after validating that genre defaults feel natural.
- The "molds to the user" goal is most powerfully expressed through absence of friction, not presence of features. If the user never has to scroll past genres they don't care about, the system has adapted. No announcement needed.
- Consider: the listening profile could seed from Mouseion's history API on first app load, giving the system a head start. But local-only is simpler and more private. Decide during implementation.
- Last.fm scrobble data (if connected) is another potential seed source. User's external listening history could inform initial profile. Deferred; the system should work without external data.
- The anti-patterns section is as important as the features. The difference between "this app gets me" and "this app is tracking me" is entirely in execution and framing.
