# Spec 03: Advanced Features

**Status:** Draft
**Priority:** Medium
**Issues:** #57, #58, #60, #61

## Goal

Intelligence layer on top of the media library. Podcast transcription, smart playlists from audio analysis, taste profiles, and multi-zone playback. These are differentiators — features no single *arr tool provides.

## Phases

### Phase 1: Smart playlists
- [ ] Audio analysis-based playlist generation (#60)
- [ ] Dynamic playlists from quality/genre/era filters
- [ ] Playlist CRUD API endpoints

### Phase 2: Analytics
- [ ] Taste profile from listening history (#57)
- [ ] Listening statistics (daily/weekly/monthly)
- [ ] "On This Day" / historical playback data

### Phase 3: Transcription
- [ ] Whisper API integration for podcast transcription (#61)
- [ ] Full-text search across transcripts
- [ ] Chapter marker generation from transcripts

### Phase 4: Multi-zone
- [ ] WebSocket-based synchronized playback (#58)
- [ ] Zone management API
- [ ] Latency compensation

## Dependencies

- Smart playlists need audio analysis data (TagLib metadata exists, spectral analysis in Phase 7)
- Transcription needs Whisper API access (local or OpenAI)
- Multi-zone is complex — significant new infrastructure

## Notes

- Smart playlists (Phase 1) has lowest dependency and highest immediate value.
- Multi-zone (#58) is labeled "moon shot" — deprioritize.
- Taste profile needs substantial listening history to be meaningful.
