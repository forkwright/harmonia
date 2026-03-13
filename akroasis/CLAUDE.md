# CLAUDE.md: AI coding guidelines

## Repository

Akroasis is a unified media player (music, audiobooks, podcasts). Three platforms: Android (Kotlin), Web (React/TS), Desktop (Tauri). Rust audio core shared via JNI/FFI. Backend is Mouseion (separate repo).

## Branch strategy

- **Single branch:** `main`. No develop branch.
- PRs target `main`. Squash merge.
- Branch naming: `feat/`, `fix/`, `docs/`, `refactor/`, `perf/`, `test/`, `cleanup/`

## Build & test

```bash
# Web
cd web && npm ci && npm run lint && npm run build && npx vitest run

# Android
cd android && ./gradlew build && ./gradlew test
```

## Code standards

- **Conventional commits.** `feat(scope): description`. Scopes: audiobook, web, android, audio, ui, api, playback, infrastructure.
- **No AI attribution.** No "Co-authored-by: Claude", no 🤖, no "AI-generated".
- **No filler words.** Don't use: comprehensive, robust, leverage, streamline, modernize, strategic, enhance.
- **Test new features.** Every new module gets tests.
- **TypeScript strict mode.** Zero `any` in new code.
- **Zustand for state** (web). MVVM + StateFlow (Android).

## PR rules

- **Minimum density:** Don't open PRs for trivial single-line changes; batch with related work.
- **Large PRs (>1000 lines):** Acceptable when cohesive. Don't artificially split related changes.
- **PR body:** Explain what and why. Table of changes if >5 files.
- **No placeholder code.** Ship working features or don't ship.

## Architecture

- `web/src/api/client.ts`: all Mouseion API calls
- `web/src/stores/`: Zustand stores (playerStore, authStore, audiobookStore)
- `web/src/mocks/`: MSW handlers + mock data for dev mode
- `web/src/pages/`: route-level components
- `android/app/src/main/java/app/akroasis/`: Android app root
- `shared/akroasis-core/`: Rust audio core
- `specs/`: development specifications

## Specs

Development is spec-driven. See `specs/` directory. When implementing a feature:
1. Check if a spec exists
2. Reference spec number in PR title if applicable
3. Update spec phase checkboxes when completing work

## What NOT to do

- Don't modify CI workflows without understanding the full pipeline
- Don't add new dependencies without justification
- Don't rename or restructure without a spec
- Don't touch android/ code from web PRs or vice versa (keep PRs platform-scoped)
