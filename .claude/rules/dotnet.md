# .NET / C# Rules

Rules for Mouseion — media management backend on .NET 10, C#.

---

## Build & Validate

```bash
dotnet build Mouseion.sln --configuration Release
dotnet test --configuration Release --verbosity minimal
dotnet format --verify-no-changes
```

All three must pass before any PR.

---

## Async/Await

- `CancellationToken` on ALL async method signatures — no exceptions
- Never `.GetAwaiter().GetResult()` — always async all the way
- `ConfigureAwait(false)` in library/service code (not needed in controllers)
- `Task.Run` only for CPU-bound work, never for I/O

Compliant:
```csharp
public async Task<Album> GetAlbumAsync(int id, CancellationToken ct)
{
    return await _repository.FindByIdAsync(id, ct).ConfigureAwait(false);
}
```

Non-compliant:
```csharp
public Album GetAlbum(int id)
{
    return _repository.FindByIdAsync(id, CancellationToken.None).GetAwaiter().GetResult();
}
```

---

## Data Access

**Dapper only.** No Entity Framework Core. No ORM magic.

- Generic repository base with type-safe queries
- Parameterized queries always — never string interpolation in SQL
- Use `CommandDefinition` with `CancellationToken` for cancellable queries
- Transaction scope for multi-statement operations

Compliant:
```csharp
public async Task<MediaItem?> FindByIdAsync(int id, CancellationToken ct)
{
    const string sql = "SELECT * FROM media_items WHERE id = @Id";
    return await _connection.QueryFirstOrDefaultAsync<MediaItem>(
        new CommandDefinition(sql, new { Id = id }, cancellationToken: ct));
}
```

Non-compliant:
```csharp
var sql = $"SELECT * FROM media_items WHERE id = {id}"; // SQL injection
```

---

## Dependency Injection

**DryIoc** container. Register services in the appropriate module class.

- Constructor injection only — no service locator pattern
- Interface-based registration for testability
- Scoped lifetime for request-bound services
- Singleton for stateless services and caches

---

## Validation

**FluentValidation** for request DTOs.

- One validator class per request type
- Validate at the API boundary, not in business logic
- Return structured error responses with field-level details

---

## Resilience

**Polly** for external service calls (metadata providers, external APIs).

- Retry with exponential backoff for transient failures
- Circuit breaker for cascading failure protection
- Timeout policies on all external HTTP calls
- Combine policies in a `PolicyWrap`

---

## Caching

- `IMemoryCache` for metadata responses (15-min default TTL)
- Cache keys must be deterministic and include all varying parameters
- Never cache user-specific data in shared cache
- Explicit eviction on data mutation

---

## Error Handling

- Custom exception types per domain area
- Never `catch (Exception)` without re-throw or structured logging
- Use `IResult` / `Results.Problem()` for API error responses
- Include correlation ID in error responses

---

## Architecture

```
src/Mouseion.Core/     — entities, services, business logic
src/Mouseion.Api/      — controllers, middleware, API surface
src/Mouseion.Common/   — shared utilities, HTTP client, DI
src/Mouseion.SignalR/  — real-time messaging
src/Mouseion.Host/     — entry point, configuration
tests/                 — unit and integration tests
```

- Core has no dependency on Api or Host
- Api depends on Core, never the reverse
- Common is a leaf — imported by all, imports nothing project-specific

---

## Naming

| Element | Convention | Example |
|---------|-----------|---------|
| Classes | `PascalCase` | `AudiobookService` |
| Methods | `PascalCase` | `GetChaptersAsync` |
| Properties | `PascalCase` | `TotalDuration` |
| Private fields | `_camelCase` | `_repository` |
| Local variables | `camelCase` | `albumCount` |
| Interfaces | `IPascalCase` | `IMediaRepository` |
| Async methods | suffix `Async` | `FetchMetadataAsync` |
| Constants | `PascalCase` | `DefaultCacheTtl` |

---

## What NOT to Do

- Don't add EF Core or change ORM strategy
- Don't use `.GetAwaiter().GetResult()`
- Don't hardcode API keys or secrets
- Don't add dependencies without justification
- Don't use `dynamic` or `var` when type is non-obvious
