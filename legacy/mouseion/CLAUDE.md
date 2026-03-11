# CLAUDE.md — AI Coding Guidelines

## Repository

Mouseion — unified self-hosted media manager (.NET 10, C#). Backend API server. Client is Akroasis (separate repo).

## Branch Strategy

- **Single branch:** `main`. No develop branch.
- PRs target `main`. Squash merge.
- Branch naming: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `cleanup/`

## Build & Test

```bash
dotnet build Mouseion.sln --configuration Release
dotnet test --configuration Release --verbosity minimal
dotnet format --verify-no-changes
```

## Code Standards

- **Conventional commits.** `feat(scope): description`. Scopes: api, audiobook, metadata, database, indexer, quality, migration, comic, manga, news, tv, music.
- **No AI attribution.** No "Co-authored-by: Claude", no 🤖, no "AI-generated".
- **No filler words.** Don't use: comprehensive, robust, leverage, streamline, modernize, strategic, enhance.
- **Test new features.** Every new service/controller gets tests.
- **Async/await throughout.** Use CancellationToken on all async methods.
- **Dapper for data access.** No EF Core.
- **DryIoc for DI.** Register in appropriate module.

## Architecture

- `src/Mouseion.Core/` — entities, services, business logic
- `src/Mouseion.Api/` — controllers, middleware, API surface
- `src/Mouseion.Common/` — shared utilities, HTTP client, DI
- `src/Mouseion.SignalR/` — real-time messaging
- `src/Mouseion.Host/` — entry point, configuration
- `tests/` — unit and integration tests
- `specs/` — development specifications

## Patterns

- Repository pattern with Dapper (type-safe, generic base)
- Polly resilience for external metadata providers
- FluentValidation for request validation
- IMemoryCache (15-min TTL) for metadata responses
- Polymorphic MediaItem with media-type-specific subclasses

## What NOT to Do

- Don't add EF Core or change ORM strategy
- Don't modify CI workflows without understanding the full pipeline
- Don't add dependencies without justification
- Don't use `.GetAwaiter().GetResult()` — always async
- Don't hardcode API keys or secrets
