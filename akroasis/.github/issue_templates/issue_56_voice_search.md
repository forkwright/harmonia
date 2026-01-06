---
name: Voice Search Infrastructure
about: Implement voice search for media controls
title: '[Android] Implement voice search for media controls'
labels: 'enhancement, android, ui, m'
assignees: ''
---

## Context

`MediaSessionManager.kt` and `PlaybackService.kt` have TODO comments for voice search implementation. Android media session API supports voice commands via ACTION_PREPARE_FROM_SEARCH, enabling hands-free control via headset buttons and Assistant.

**Related TODOs:**
- `PlaybackService.kt:306`: "TODO: Implement voice search handler"
- `MediaSessionManager.kt:67`: "TODO: Add voice search action to media session"

## Scope

### Components to Implement

1. **PlaybackService.kt** - Voice search handler
   - Implement `onPrepareFromSearch(query: String?, extras: Bundle?)`
   - Parse search query into artist/album/track intent
   - Call PlayerViewModel search API
   - Handle results: auto-play top result or show results UI

2. **MediaSessionManager.kt** - Voice action registration
   - Add `ACTION_PREPARE_FROM_SEARCH` to supported actions
   - Add `ACTION_PLAY_FROM_SEARCH` for immediate playback

3. **PlayerViewModel** - Search API
   - Add `searchMedia(query: String): Flow<SearchResults>`
   - Parse natural language: "Play Pink Floyd", "Search jazz"
   - Return ranked results (artist > album > track)

4. **Voice Command Parsing**
   - Simple keyword matching (avoid complex NLP)
   - Support patterns:
     - "Play [artist name]"
     - "Play [album name]"
     - "Play [track name]"
     - "Search for [query]"
   - Extract artist/album/track from Android extras if provided

### Features

- Voice search via media session (headset button long-press)
- Natural language parsing for common patterns
- Search results handling: auto-play top result or show results screen
- Error handling: "No results found" feedback via toast/notification
- Integration with Android Assistant

## Acceptance Criteria

- [ ] Voice commands trigger search via media session
- [ ] Search queries parsed correctly (keyword matching)
- [ ] Results auto-play top match or display results screen
- [ ] Works with Bluetooth headset voice button (long-press)
- [ ] Works with "Ok Google, play [artist]" on Akroasis
- [ ] Graceful error handling with user feedback
- [ ] Voice search works while app is backgrounded
- [ ] Search results respect current library scope (not web search)

## Dependencies

### Prerequisites
- **Phase 2 search API** (#39) - Backend search endpoint with audio metadata
- Android RecognizerIntent or SpeechRecognizer API
- MediaSession framework (already implemented)

### Optional Enhancements
- ListenBrainz similar artist search (deferred)
- Last.fm artist radio ("play similar to [artist]") - deferred to Phase 7

## Out of Scope

- Advanced NLP (use simple keyword matching for MVP)
- Custom voice training or wake word detection
- Multi-language support (English only initially)
- Voice commands for DSP settings ("turn on equalizer") - future enhancement
- Web search or streaming service integration
- Offline speech recognition (requires Google services)

## Implementation Notes

### Voice Search Flow

1. User triggers voice input (headset long-press or "Ok Google")
2. Android invokes `onPrepareFromSearch(query, extras)`
3. Parse query and extras (MediaStore.INTENT_ACTION_MEDIA_PLAY_FROM_SEARCH)
4. Call backend search API
5. If single strong match: auto-play
6. If multiple matches: show results screen
7. If no matches: toast "No results found for '{query}'"

### Android Extras Handling

MediaStore provides structured extras:
- `EXTRA_MEDIA_ARTIST` - artist name
- `EXTRA_MEDIA_ALBUM` - album name
- `EXTRA_MEDIA_TITLE` - track title
- `EXTRA_MEDIA_FOCUS` - artist, album, or genre

Use extras first, fall back to raw query parsing.

## Platform(s)

Android

## Size Estimate

**m** (4-8 hours)

**Breakdown:**
- MediaSession action registration: 1 hour
- Voice search handler implementation: 2-3 hours
- Search query parsing: 2 hours
- Testing with Assistant and headset: 1-2 hours
