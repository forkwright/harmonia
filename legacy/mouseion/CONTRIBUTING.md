# Contributing to Mouseion

Mouseion is a unified media manager for movies, TV, music, books, audiobooks, podcasts, manga, comics, webcomics, and news. This guide covers local development setup and contribution workflow.

## Prerequisites

- [.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10.0) (preview)
- Git
- A code editor (VS Code, Rider, or Visual Studio)

## Quick start

```bash
# Clone
git clone https://github.com/your-org/mouseion.git
cd mouseion

# Build and test
make build
make test

# Run locally
make run
# API available at http://localhost:8989
# Swagger UI at http://localhost:8989/swagger
```

## Project structure

```
src/
├── Mouseion.Common/     # Shared utilities, extensions, options
├── Mouseion.Core/       # Domain entities, repositories, services, migrations
├── Mouseion.Api/        # Controllers, validators, middleware, security
├── Mouseion.SignalR/    # Real-time notification hub
└── Mouseion.Host/       # ASP.NET Core host, DI wiring, startup
tests/
├── Mouseion.Common.Tests/
├── Mouseion.Core.Tests/
└── Mouseion.Api.Tests/
specs/                   # Living specification documents
```

## Development workflow

1. **Branch from main**: `git checkout -b feat/your-feature main`
2. **Make changes** and ensure tests pass: `make test`
3. **Format code**: `make format`
4. **Push and open PR**: PRs require CI to pass before merge
5. **Squash merge** into main

## Running tests

```bash
# All tests
make test

# Specific test projects
make test-api      # API controller + integration tests
make test-core     # Core service + repository tests
make test-common   # Common utility tests

# Specific test file
dotnet test tests/Mouseion.Api.Tests/ --filter "FullyQualifiedName~TagControllerTests"
```

## Architecture

### Authentication
- **JWT** (primary): Bearer token in Authorization header
- **API Key** (fallback): `X-Api-Key` header for automation
- **OIDC/OAuth 2.0**: External identity providers (Keycloak, Authentik, etc.)

### Database
- **SQLite** (default) with Dapper ORM
- **FluentMigrator** for schema migrations (numbered `XXX_Description.cs`)
- PostgreSQL support available via connection string

### DI container
- **DryIoc** for production, default ASP.NET Core DI for tests
- Services registered in `Program.cs` (both paths)
- Singleton lifetime for most services (SQLite thread safety via Polly retry)

### Key patterns
- `ModelBase` → all entities inherit from this (provides `int Id`)
- `BasicRepository<T>` → generic CRUD with Dapper + Polly retry
- `IBasicRepository<T>` → async-first interface with legacy sync methods
- Controllers return DTOs (Resources), not entities directly
- FluentValidation for request validation (auto-registered)

## Specs

Living specifications in `specs/`. Each has phases with acceptance criteria. Check spec status before starting work on a feature; it may already be partially implemented.

## Code style

- C# 13 / .NET 10 features welcome
- Nullable reference types enabled (`<Nullable>enable</Nullable>`)
- `ConfigureAwait(false)` on all async calls
- Serilog for logging (`Log.ForContext<T>()`)
- Copyright header on all files

## Docker

```bash
# Local dev environment
make docker-up    # Start
make docker-down  # Stop
```

## Useful commands

```bash
make help         # List all make targets
make loc          # Lines of code count
make todos        # Find TODO/FIXME/HACK comments
make swagger      # Generate swagger.json
make format-check # CI-style format check
```
