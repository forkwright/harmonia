# Coding Conventions

**Analysis Date:** 2026-03-01

## Naming Patterns

**Files (C# Backend):**
- Controllers: `{Entity}Controller.cs` (e.g., `AlbumController.cs`, `ArtistController.cs`)
- Services: `{Entity}Service.cs` (e.g., `AudiobookService.cs`)
- Validators: `{Entity}ResourceValidator.cs` (e.g., `AlbumResourceValidator.cs`)
- Tests: `{Entity}Tests.cs` or `{Entity}ControllerTests.cs`
- Resources (DTOs): `{Entity}Resource.cs` (e.g., `AlbumResource.cs`)

**Files (TypeScript Web):**
- Components: `{Component}.tsx` (PascalCase, e.g., `ArtworkViewer.tsx`, `HeartButton.tsx`)
- Hooks: `use{Name}.ts` (e.g., `useDebounce.ts`, `useWebAudioPlayer.ts`)
- Tests: `{fileName}.test.ts` or `{fileName}.test.tsx`
- Stores: `{name}Store.ts` (e.g., `authStore.ts`, `audiobookStore.ts`)
- Services: `{name}Service.ts` (e.g., `scrobbleQueue.ts`, `syncService.ts`)

**Functions (C#):**
- PascalCase: `GetAlbums()`, `CreateArtist()`, `ValidateInput()`
- Async methods end with `Async`: `GetArtistAsync()`, `UpdateAlbumAsync()`

**Functions (TypeScript):**
- camelCase: `getAlbums()`, `createArtist()`, `validateInput()`
- React hooks follow `use` prefix: `useDebounce()`, `useWebAudioPlayer()`

**Variables (C#):**
- PascalCase for properties: `public string Name { get; set; }`
- camelCase for local variables: `var albumCount = 10;`

**Variables (TypeScript):**
- camelCase throughout: `const albumCount = 10`, `let isLoading = false`

**Types/Interfaces (C#):**
- PascalCase: `class AlbumService`, `interface IMediaRepository`, `record AlbumDto`

**Types/Interfaces (TypeScript):**
- PascalCase: `interface Track`, `type Album`, `enum MediaType`

## Code Style

**Formatting (C#):**
- Enforced by `dotnet format` (runs in CI)
- C# formatting rules configured via `.editorconfig` or `Directory.Build.props`
- Must pass: `dotnet format --verify-no-changes`

**Formatting (TypeScript/Web):**
- Enforced by Prettier (implicit in build pipeline)
- Config: `vite.config.ts` (no explicit prettier config file)

**Linting (C#):**
- StyleCop analyzers via Roslyn
- Warnings treated as errors in Release configuration

**Linting (TypeScript):**
- ESLint with configs: `eslint.config.js`
- Recommended rules for TypeScript, React, React Hooks, React Refresh
- Runs on all `.ts` and `.tsx` files
- Must pass: `npm run lint`

## Import Organization

**C# Imports:**
- System namespaces first
- Third-party namespaces (Dapper, Polly, FluentValidation)
- Application namespaces last
- Within groups: alphabetical order
- Example from codebase:
```csharp
using System;
using System.Collections.Generic;
using System.Net.Http.Json;
using Mouseion.Api.Albums;
using Mouseion.Api.Common;
```

**TypeScript Imports:**
- External dependencies first (React, zustand, react-router)
- Type imports using `import type`
- Local imports with `@/` alias for `src/`
- Barrel files used in `src/pages/`, `src/stores/`
- Example:
```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { apiClient } from './client'
import type { Track, Album } from '../types'
```

**Path Aliases:**
- TypeScript web: `@/` resolves to `src/`
- Configured in `tsconfig.json` and `vite.config.ts`

## Error Handling

**C# Patterns:**
- Custom exception classes inherit from `ApplicationException`
- Controllers return `ApiProblemDetails` via `GlobalExceptionHandlerMiddleware`
- Validation errors: FluentValidation rules raise `ValidationException`
- HTTP status codes mapped in middleware: 400 (bad request), 404 (not found), 500 (server error)
- Example from `ApiProblemDetails.cs`: structured problem responses per RFC 7807

**TypeScript Patterns:**
- Try-catch blocks for async operations
- Error objects contain `status` and `message` properties
- API client returns typed responses or throws
- Stores handle errors via state: `error: string | null`
- Example from `client.test.ts`: mock fetch errors, validate error handling

## Logging

**Framework (C#):**
- Built-in `ILogger<T>` (Microsoft.Extensions.Logging)
- Configured in `Mouseion.Host` startup

**Framework (TypeScript):**
- `console.*` methods (console.log, console.error, console.warn)
- No structured logging framework detected

**Patterns (C#):**
- Log at service level for business operations
- Log at controller level for HTTP activity (via middleware)
- Debug logs for detailed tracing
- Error logs with exception details

**Patterns (TypeScript):**
- Log API calls and responses in `client.ts`
- Log store updates at critical points
- Minimal logging in components (prefer DevTools)

## Comments

**When to Comment (C#):**
- Complex business logic requiring explanation
- Non-obvious algorithm decisions
- Public API documentation (XML comments)
- Example headers in test files cite original projects (Radarr, etc.)

**When to Comment (TypeScript):**
- Complex calculations or algorithms
- Non-obvious React/state management patterns
- Browser compatibility workarounds

**JSDoc/TSDoc:**
- Not used extensively in web codebase
- C# uses XML doc comments on public types/methods

## Function Design

**Size (C#):**
- Controller actions: 5-20 lines (delegate to services)
- Service methods: 10-50 lines (business logic concentration)
- Async throughout: all service methods are `async Task<T>`

**Size (TypeScript):**
- React components: 50-200 lines (break into smaller components if larger)
- Hooks: 20-80 lines (focused single concern)
- Test suites: descriptive nested `describe()` blocks with 3-5 focused `it()` blocks per suite

**Parameters:**
- C# services accept domain objects or records, not primitives
- TypeScript functions prefer objects over multiple parameters
- Use `CancellationToken` as last parameter in all C# async methods

**Return Values:**
- C# async methods return `Task<T>` or `Task`
- TypeScript async functions return `Promise<T>`
- Zustand stores return state snapshots

## Module Design

**Exports (C#):**
- Explicit public API surface
- Controllers and Services marked `public`
- Internal implementation marked `internal`
- Resources (DTOs) exported for API contracts

**Exports (TypeScript):**
- Named exports preferred over default
- Barrel files in `stores/`, `pages/` re-export publicly
- API client exports singleton instance: `export const apiClient = ...`
- Example from `client.ts`: exports `apiClient`, `getStreamUrl()`, `getCoverArtUrl()`

**Barrel Files:**
- Used in `src/stores/` for centralized store exports
- Used in `src/pages/` for route-level components
- Not used in `src/components/` (import directly)

---

*Convention analysis: 2026-03-01*
