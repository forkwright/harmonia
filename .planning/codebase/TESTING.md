# Testing Patterns

**Analysis Date:** 2026-03-01

## Test Framework

**C# Backend:**
- Runner: xUnit (via `dotnet test`)
- Config: Implicit (xUnit auto-discovery of `*Tests.cs` files)
- Command: `dotnet test --configuration Release --verbosity minimal`

**TypeScript Web:**
- Runner: Vitest 4.0.16
- Config: `vitest.config.ts`
- Commands:
  - `npm run test` - Run all tests once
  - `npm run test:watch` - Watch mode
  - `npm run test:ui` - UI dashboard
  - `npm run test:coverage` - Coverage report

**Assertion Library:**
- C#: xUnit's `Assert.*` methods
- TypeScript: Vitest's `expect()` (compatible with Jest)

## Test File Organization

**Location (C#):**
- Separate `tests/` directory parallel to `src/`
- Structure mirrors source: `tests/Mouseion.Api.Tests/Albums/AlbumControllerTests.cs` mirrors `src/Mouseion.Api/Albums/AlbumController.cs`
- Example paths:
  - `tests/Mouseion.Api.Tests/`
  - `tests/Mouseion.Core.Tests/`
  - `tests/Mouseion.Common.Tests/`

**Location (TypeScript):**
- Co-located with source files in `src/`
- Pattern: `{module}.test.ts` or `{component}.test.tsx` in same directory
- Examples:
  - `src/api/client.test.ts`
  - `src/components/HeartButton.test.tsx`
  - `src/stores/authStore.test.ts`
  - `src/hooks/useDebounce.test.ts`

**Naming (C#):**
- Convention: `{EntityName}Tests.cs` or `{EntityName}ControllerTests.cs`
- Examples: `AlbumControllerTests.cs`, `AudiobookServiceTests.cs`
- Namespace: `Mouseion.Api.Tests.{Feature}` mirrors production namespace

**Naming (TypeScript):**
- Convention: `{fileName}.test.ts` or `{fileName}.test.tsx`
- Examples: `client.test.ts`, `HeartButton.test.tsx`

## Test Structure

**C# Suite Organization:**
```csharp
namespace Mouseion.Api.Tests.Albums;

public class AlbumControllerTests : ControllerTestBase
{
    public AlbumControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetAlbums_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/albums");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<AlbumResource>>();
        Assert.NotNull(result);
    }
}
```

**Pattern:**
- Inherits from `ControllerTestBase` for HTTP client setup
- One test class per entity/controller
- Method naming: `{Action}_{ExpectedResult}_{Condition}()` (e.g., `GetAlbum_ReturnsAlbum_WhenExists()`)
- Use `[Fact]` for deterministic tests, `[Theory]` for parameterized
- Async/await throughout: `public async Task {TestName}()`

**TypeScript Suite Organization:**
```typescript
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { apiClient } from './client'

describe('ApiClient', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorageMock.clear()
  })

  afterEach(() => {
    localStorageMock.clear()
  })

  describe('Initialization', () => {
    it('should use singleton instance', () => {
      expect(apiClient).toBeDefined()
    })

    it('should load server URL from localStorage', () => {
      // Test body
    })
  })
})
```

**Pattern:**
- Nested `describe()` blocks for logical grouping
- Setup: `beforeEach()` for test initialization, `afterEach()` for cleanup
- Method naming: `should {expected behavior}` (readable, not technical)
- Mock setup at module level: `globalThis.fetch = vi.fn()`
- Focused assertions: 1-3 per test

## Mocking

**Framework (C#):**
- `TestWebApplicationFactory` creates in-memory HTTP test server
- Fixture setup in base class: `ControllerTestBase`
- `Client` property provides `HttpClient` for API testing

**Framework (TypeScript):**
- Vitest's `vi` for spies/mocks
- MSW (Mock Service Worker) for API mocking
- Happy-dom for DOM testing environment
- Config in `vitest.config.ts`: `environment: 'happy-dom'`

**Mocking Patterns (C#):**
```csharp
// Integration testing: full HTTP stack with in-memory database
var response = await Client.GetAsync("/api/v3/albums");
var created = await response.Content.ReadFromJsonAsync<AlbumResource>();

// No explicit mocking library used; tests use real HTTP layer
```

**Mocking Patterns (TypeScript):**
```typescript
// From client.test.ts
globalThis.fetch = vi.fn()

const localStorageMock = (() => {
  let store: Record<string, string> = {}
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => { store[key] = value },
    removeItem: (key: string) => { delete store[key] },
    clear: () => { store = {} }
  }
})()

Object.defineProperty(globalThis, 'localStorage', {
  value: localStorageMock
})
```

**What to Mock:**
- External HTTP APIs (via fetch mocks or MSW)
- Browser APIs: localStorage, sessionStorage
- Date/time via `vi.useFakeTimers()`
- Timers and delays

**What NOT to Mock:**
- Core application logic (services, stores)
- Internal API calls (test real client behavior)
- DOM rendering (test actual output)

## Fixtures and Factories

**Test Data (C#):**
- Inline object creation in tests: `new AlbumResource { Title = "...", ... }`
- Factory methods in `ControllerTestBase` or test helpers
- Example: Tests create artists first, then albums dependent on artist IDs

**Test Data (TypeScript):**
- `src/test/` contains setup files (no separate fixtures directory detected)
- Mock data defined in `src/mocks/` for MSW handlers
- Inline data creation common: `const mockTrack = { id: 1, title: '...' }`

**Location:**
- C# test helpers: `tests/` directory classes and base classes
- TypeScript: `src/test/setup.ts` (setup file), `src/mocks/` (mock handlers)

## Coverage

**Requirements:**
- Thresholds enforced in `vitest.config.ts` (web only):
  - lines: 60%
  - functions: 60%
  - branches: 60%
  - statements: 60%
- C# backend: No explicit coverage threshold detected (spec `02-test-coverage.md` suggests ongoing focus)

**View Coverage (TypeScript):**
```bash
npm run test:coverage
# Outputs: text, json, html reports
# HTML report: coverage/index.html
```

**Coverage Exclusions (TypeScript):**
- `node_modules/`
- `src/test/`
- `**/*.d.ts`
- `**/*.config.*`
- `**/mockData`
- `dist/`

## Test Types

**Unit Tests (C#):**
- Service tests: test business logic in isolation
- Example: `AudiobookServiceTests` validates service methods directly
- Use `[Theory]` for parameterized unit tests with `[InlineData]`

**Unit Tests (TypeScript):**
- Component tests: render and validate output
- Hook tests: mount and verify state changes
- Store tests: dispatch actions and verify state
- Utility function tests: call function, verify result
- Example: `useDebounce.test.ts` verifies debounce timing

**Integration Tests (C#):**
- Controller tests: HTTP layer with in-memory database
- Full request/response cycle tested
- Example: `AlbumControllerTests` creates artist, then album, validates response
- `TestWebApplicationFactory` provides real-ish environment

**Integration Tests (TypeScript):**
- API client tests: test HTTP client against mocked endpoints
- Store tests with API calls: test store dispatching API actions
- Service integration: test services calling client
- Example: `authStore.test.ts` tests auth flow through API client

**E2E Tests:**
- C#: Not detected in test structure (integration tests used instead)
- TypeScript: Not detected (focus on unit + integration via MSW)

## Common Patterns

**Async Testing (C#):**
```csharp
[Fact]
public async Task GetAlbums_ReturnsSuccessfully()
{
    var response = await Client.GetAsync("/api/v3/albums");
    response.EnsureSuccessStatusCode();
    var result = await response.Content.ReadFromJsonAsync<PagedResult<AlbumResource>>();
    Assert.NotNull(result);
}
```

**Pattern:** All test methods are `async Task`, use `await` liberally, no `.Result` blocking.

**Async Testing (TypeScript):**
```typescript
it('should fetch albums', async () => {
  globalThis.fetch = vi.fn().mockResolvedValueOnce({
    ok: true,
    json: async () => ({ items: [] })
  })

  const result = await apiClient.getAlbums()
  expect(result).toBeDefined()
})
```

**Pattern:** Mock async operations with `mockResolvedValueOnce()`, use `await` in test body.

**Error Testing (C#):**
```csharp
[Fact]
public async Task AddAlbum_ReturnsBadRequest_WithInvalidData()
{
    var invalidAlbum = new AlbumResource { /* missing required fields */ };
    var response = await Client.PostAsJsonAsync("/api/v3/albums", invalidAlbum);
    Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
}
```

**Pattern:** Assert on status code or exception type.

**Error Testing (TypeScript):**
```typescript
it('should handle fetch errors', () => {
  globalThis.fetch = vi.fn().mockRejectedValueOnce(new Error('Network error'))

  expect(() => apiClient.getAlbums()).rejects.toThrow('Network error')
})
```

**Pattern:** Mock rejection, use `rejects.toThrow()` for error assertions.

---

*Testing analysis: 2026-03-01*
