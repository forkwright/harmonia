# Akroasis Roadmap

## What's Done

| Area | Status | Notes |
|------|--------|-------|
| Rust audio core | ✅ | Bit-perfect pipeline, FLAC decode, ReplayGain, JNI + FFI |
| Android playback | ✅ | Gapless, queue history, speed memory, signal path viz |
| Android DSP | ✅ | 5-band parametric EQ, AutoEQ profiles, crossfeed, headroom |
| Android mobile | ✅ | Media session, notifications, state persistence, network monitoring |
| Android scrobbling | ✅ | Last.fm + ListenBrainz, playback-speed-aware timestamps |
| Android search/filter | ✅ | Audio quality badges, focus filters, smart playlists |
| Android audiobook | ✅ | Chapter playback, ebook reader, continue feed |
| Android Auto | ✅ | Browse hierarchy, artwork, search, genres, error handling |
| Web app | ✅ | Playback, library, queue, search, PWA, keyboard shortcuts |
| Web audiobook | ✅ | Library, player, chapters, progress, sleep timer, speed, bookmarks |
| Web EQ | ✅ | 10-band parametric EQ, presets, signal path visualization |
| Web discovery | ✅ | Synchronized lyrics (LRCLIB), Last.fm radio, offline scrobble queue |
| Web viewer | ✅ | Artwork zoom lightbox with pinch/scroll zoom and pan |
| Desktop shell | ✅ | Tauri 2.3, system tray, close-to-tray, platform detection |
| CI/CD | ✅ | Android build, web build, CodeQL, dependabot, release workflow |
| Test coverage | ✅ | 473+ Android tests, 264+ web tests |

## What's Next

Development is spec-driven. See `specs/` for active work. All GitHub issues are closed — specs are the source of truth.

| Spec | Title | Priority | Status |
|------|-------|----------|--------|
| 01 | Audiobook & podcast completion | High | Phase 1 done, Phases 2-3 unblocked |
| 02 | Desktop app (Tauri) | High | Phase 1 done, Phase 2-4 pending |
| 03 | Discovery & intelligence | Medium | Phase 1 partial, Phase 2 unblocked |
| 04 | Platform polish & offline | Medium | Phase 3 partial (Auto done) |
| 05 | Infrastructure & CI | Medium | Phase 1 done, Phase 2 partial |
| 06 | Content acquisition pipeline | Low | Partially unblocked |
| 07 | Competitive analysis | Medium | Research done |

## Mouseion Backend Status

Mouseion has shipped significant APIs that unblock most client-side work:
- Progress tracking, sessions, continue feed, queue sync, playback transfer
- Streaming with transcode endpoint (passthrough, FFmpeg later)
- Auth/JWT with multi-user support
- Smart playlists, history, library statistics
- Podcasts, download clients, import lists (Trakt, MAL, AniList)
