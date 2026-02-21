# Mouseion Roadmap

## What's Done

| Phase | Focus | Status |
|-------|-------|--------|
| 0: Foundation | MediaItems table, quality system (103 definitions) | ✅ |
| 1: Quality | Parsers, polymorphic types, 131 tests | ✅ |
| 2: Books/Audiobooks | CRUD, OpenLibrary/Audnexus metadata, MyAnonamouse | ✅ |
| 3: Music | MusicBrainz, AcoustID fingerprinting, 54 quality defs | ✅ |
| 4: Movies | TMDb metadata, calendar, import engine | ✅ |
| 5: TV/Podcasts | TVDB/TMDb, RSS feeds, air date monitoring | ✅ |
| 6: Infrastructure | Download clients, notifications, health checks | ✅ |
| 7: File Scanning | TagLib, spectral analysis, covers, history | ✅ |
| 8: Polish | Movie org, subtitles, auto-tagging, import pipeline | ✅ |
| 9A: Manga | MangaDex + AniList APIs, 38 tests | ✅ |
| 9B: News/RSS | Feed parsing, read/starred state | ✅ |
| 9C: Comics | ComicVine API, 22 tests | ✅ |
| 9D: Integration | Health checks, unified stats | 🚧 |

## What's Next

Spec-driven development. See `specs/` for active work.

| Spec | Title | Priority |
|------|-------|----------|
| 01 | Akroasis integration (progress, sessions, sync, webhooks, OPDS) | High |
| 02 | Test coverage expansion | High |
| 03 | Advanced features (smart lists, delay profiles, transcription, analytics) | Medium |
| 04 | Infrastructure polish | Medium |
| 05 | Competitive analysis | Reference |
| 06 | Authentication & multi-user (OIDC/OAuth, per-user state) | High |
| 07 | Tracker import pipeline (Trakt, MAL, AniList, Goodreads, Last.fm) | High |
| 08 | Acquisition intelligence (client expansion, rate limits, dedup, .strm) | Medium |

## Execution Order

Dependency-driven sequencing:

1. ~~**Spec 06 Phase 1-2** (user model + local auth)~~ ✅ PR #171
2. ~~**Spec 01 Phase 1-3** (progress, sync, streaming)~~ ✅ PR #172
3. ~~**Spec 07 Phase 1** (Trakt import)~~ ✅ PR #172
4. ~~**Spec 07 Phase 2** (MAL/AniList imports)~~ ✅ PR #173 — anime/manga migration path
5. **Spec 02** (test coverage) — continuous, parallel with feature work
6. ~~**Spec 08 Phase 1-2** (download clients + rate limiting)~~ ✅ PR #173 — responsible acquisition
7. **Spec 01 Phase 4-5** (webhooks + OPDS) — external client integration
8. **Spec 03 Phase 2-3** (smart lists + delay profiles) — intelligence layer
9. **Spec 06 Phase 3-5** (OIDC + permissions) — advanced auth
10. **Spec 07 Phase 3-5** (remaining imports + export) — full migration coverage
11. **Spec 08 Phase 3-5** (dedup + .strm + orchestration) — advanced acquisition

## Open Issues

5 open issues on GitHub — all absorbed into specs.
