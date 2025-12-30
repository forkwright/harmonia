# CLAUDE.md — Harmonia Monorepo

## Repository

Harmonia — unified self-hosted media platform. Monorepo containing backend and player.

```
harmonia/
├── mouseion/    # Media management backend (.NET 10/C#, future Rust)
├── akroasis/    # Media player (Android/Kotlin, Web/React, Rust audio core)
└── docs/        # Cross-cutting documentation
```

Component-specific guidelines live in `mouseion/CLAUDE.md` and `akroasis/CLAUDE.md`.

## Branch Strategy

- **Single branch:** `main`. No develop branch.
- PRs target `main`. Squash merge.
- Branch naming: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `cleanup/`

## Commit Format

`category(scope): description`

Categories: feat, fix, docs, refactor, test, chore, style
Scopes: `mouseion`, `akroasis`, `docs`, `infra`

## Build & Test

```bash
# Mouseion (backend)
cd mouseion && dotnet build Mouseion.sln --configuration Release
cd mouseion && dotnet test --configuration Release --verbosity minimal

# Akroasis web
cd akroasis/web && npm ci && npm run lint && npm run build && npx vitest run

# Akroasis android
cd akroasis/android && ./gradlew build && ./gradlew test
```

## Architecture Direction

**Backend (Mouseion):** Currently .NET 10/C#. Planned Rust rewrite — single static binary, Tokio, Axum, embedded DB. Eliminates multi-process *arr coordination overhead. See mouseion#225.

**Player (Akroasis):** Kotlin + Jetpack Compose (Android), React 19 + TypeScript (Web), Tauri 2 (Desktop, planned). Rust audio core shared via JNI/FFI — bit-perfect FLAC, gapless playback, ReplayGain.

## Code Standards

- Self-documenting code. No inline comments except truly complex logic.
- No AI attribution. No "Co-authored-by: Claude", no emoji indicators.
- No filler words: comprehensive, robust, leverage, streamline, modernize, strategic, enhance.
- Test new features.
- Greek naming for modules/systems, English for code variables/functions.

## CI

Path-based triggers:
- `mouseion/` changes → backend CI (dotnet build/test/format)
- `akroasis/` changes → player CI (android build, web lint/test)

## What NOT to Do

- Don't mix mouseion/ and akroasis/ changes in the same PR unless they're tightly coupled
- Don't add dependencies without justification
- Don't modify CI workflows without understanding the full pipeline
