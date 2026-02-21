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
| Web app | ✅ | Playback, library, queue, search, PWA, keyboard shortcuts |
| Web audiobook | ✅ | Library, player, chapters, progress tracking |
| CI/CD | ✅ | Android build, web build, CodeQL, release workflow |
| Test coverage | ✅ | 473+ Android tests, 70 web tests |

## What's Next

Development is spec-driven. See `specs/` for active work.

| Spec | Title | Priority |
|------|-------|----------|
| 01 | Audiobook & podcast completion | High |
| 02 | Desktop app (Tauri) | High |
| 03 | Discovery & intelligence | Medium |
| 04 | Platform polish & offline | Medium |
| 05 | Infrastructure & CI | Medium |
| 06 | Content acquisition pipeline | Low |

## Open Issues

~28 open issues on GitHub. Most are absorbed into specs above.
Issues tagged `blocked-mouseion` depend on backend API work in [Mouseion](https://github.com/forkwright/mouseion).
