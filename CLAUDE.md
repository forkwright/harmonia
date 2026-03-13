# CLAUDE.md: Harmonia monorepo

## Repository

Harmonia: unified self-hosted media platform. Monorepo containing backend and player.

```
harmonia/
├── mouseion/       # Media management backend (.NET 10/C#, future Rust)
├── akroasis/       # Media player (Android/Kotlin, Web/React, Rust audio core)
├── standards/      # Universal coding standards (all languages)
├── docs/           # Cross-cutting documentation
│   ├── gnomon.md           # Greek naming methodology
│   ├── lexicon.md          # Project name registry
│   ├── LESSONS.md          # Operational rules (earned through failure)
│   ├── CLAUDE_CODE.md      # Claude Code dispatch protocol
│   ├── WORKING-AGREEMENT.md
│   └── policy/             # Agent contribution, versioning, git history
└── CLAUDE.md       # This file — project conventions for CC agents
```

Component-specific guidelines live in `mouseion/CLAUDE.md` and `akroasis/CLAUDE.md`.

## Standards

Universal: [standards/STANDARDS.md](standards/STANDARDS.md)
Rust: [standards/RUST.md](standards/RUST.md)
C#/.NET: [standards/CSHARP.md](standards/CSHARP.md)
Kotlin: [standards/KOTLIN.md](standards/KOTLIN.md)
TypeScript: [standards/TYPESCRIPT.md](standards/TYPESCRIPT.md)
C++: [standards/CPP.md](standards/CPP.md)
SQL: [standards/SQL.md](standards/SQL.md)
Shell: [standards/SHELL.md](standards/SHELL.md)
Writing: [standards/WRITING.md](standards/WRITING.md)

## Documentation

- `docs/gnomon.md`: Greek naming methodology
- `docs/lexicon.md`: project name registry with layer tests
- `docs/LESSONS.md`: operational rules derived from real failures
- `docs/CLAUDE_CODE.md`: Claude Code prompt template and dispatch protocol
- `docs/WORKING-AGREEMENT.md`: Syn + Cody collaboration protocol
- `docs/policy/`: agent contribution, versioning, git history policies

## Branch strategy

- **Single branch:** `main`. No develop branch.
- PRs target `main`. Squash merge.
- Branch naming: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `cleanup/`

## Commit format

`category(scope): description`

Categories: feat, fix, docs, refactor, test, chore, style
Scopes: `mouseion`, `akroasis`, `docs`, `infra`

## Build & test

```bash
# Mouseion (backend)
cd mouseion && dotnet build Mouseion.sln --configuration Release
cd mouseion && dotnet test --configuration Release --verbosity minimal

# Akroasis web
cd akroasis/web && npm ci && npm run lint && npm run build && npx vitest run

# Akroasis android
cd akroasis/android && ./gradlew build && ./gradlew test
```

## Architecture direction

**Backend (Mouseion):** Currently .NET 10/C#. Planned Rust rewrite: single static binary, Tokio, Axum, embedded DB. Eliminates multi-process *arr coordination overhead. See mouseion#225.

**Player (Akroasis):** Kotlin + Jetpack Compose (Android), React 19 + TypeScript (Web), Tauri 2 (Desktop, planned). Rust audio core shared via JNI/FFI: bit-perfect FLAC, gapless playback, ReplayGain.

## CI

Path-based triggers:
- `mouseion/` changes: backend CI (dotnet build/test/format)
- `akroasis/` changes: player CI (android build, web lint/test)

## What not to do

- Don't mix mouseion/ and akroasis/ changes in the same PR unless tightly coupled
- Don't add dependencies without justification
- Don't modify CI workflows without understanding the full pipeline
- No AI attribution, no "Co-authored-by: Claude", no emoji indicators
- No filler words: comprehensive, robust, leverage, streamline, modernize, strategic, enhance
