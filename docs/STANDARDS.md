# Harmonia Code Standards

> Single source of truth for all languages. `.claude/rules/*.md` files are excerpts of this document.
> Each rule: what / why / compliant / non-compliant.

---

## Philosophy

Code is the documentation. Names, types, and structure carry meaning. Comments explain *why*, never *what*. If code needs a comment to explain what it does, the code needs refactoring.

Fail fast, fail loud. Crash on invariant violations. No defensive fallbacks for impossible states. Sentinel values and silent degradation are bugs. Surface errors at the point of origin with full context.

Parse, don't validate. Invalid data cannot exist. Newtypes with validation constructors enforce invariants at construction time. Once a value is constructed, its validity is a type-level guarantee.

Minimize surface area. `pub(crate)` by default (Rust), `internal` by default (C#), unexported by default (Kotlin/TS). Every public item is a commitment. Expose the smallest API that serves the need.

---

## Universal Rules

These apply regardless of language.

### Naming

#### Gnomon System (Persistent Names)

Module directories, subsystems, and major features follow the [gnomon naming system](gnomon.md). Names identify essential natures, not implementations. Pass the layer test (L1-L4). If no Greek word fits naturally, the essential nature isn't clear yet — wait.

Applies to: modules, subsystems, features that persist.
Does not apply to: variables, functions, test fixtures, temporary branches.

#### Code Names

| Context | Convention | Example |
|---------|-----------|---------|
| Files | `kebab-case` (Rust/TS), `PascalCase` (C#), `camelCase` (Kotlin) | `session-store.rs`, `SessionStore.cs`, `sessionStore.kt` |
| Types / Traits / Classes | `PascalCase` | `SessionStore`, `MediaProvider` |
| Functions / Methods | `snake_case` (Rust), `PascalCase` (C#), `camelCase` (Kotlin/TS) | `load_config` / `LoadConfig` / `loadConfig` |
| Constants | `UPPER_SNAKE` | `MAX_TURNS`, `DEFAULT_PORT` |

Verb-first for functions: `load_config`, `CreateSession`, `parseInput`. Drop `get` prefix on getters.

### Error Handling (Universal)

Every error must:
1. Carry context — what operation failed, with what inputs
2. Be typed — callers can match on error kind
3. Propagate — chain errors with `.context()` or equivalent, never swallow
4. Surface — log at the point of handling, not the point of throwing

Fail fast:
- Panic on programmer bugs (violated invariants, impossible states)
- `Result` / exceptions for anything the caller could handle or report
- Never panic/crash in library code for recoverable errors

No silent catch:
- Every catch/match block must log, propagate, return a meaningful value, or explain why it's discarded
- `/* intentional: reason */` for deliberate discard — never empty catch

### Documentation

Zero-comment default:
- No inline comments except genuinely non-obvious *why* explanations
- No creation dates, author info
- No AI generation indicators
- File headers: one line describing purpose

Doc comments only on:
- Public API items that cross module boundaries
- `unsafe` functions (mandatory `# Safety` section)
- Functions that can panic (mandatory `# Panics` section)
- Functions returning errors with non-obvious conditions

### Testing (Universal)

Behavior over implementation:
- Test what the code does, not how it does it
- One logical assertion per test
- Descriptive names: `returns_empty_when_session_has_no_turns`, not `test_add` or `it_works`

Property-based testing:
- Serialization round-trips, algebraic properties, state machine invariants
- Example tests document expected behavior; property tests catch the unexpected

No coverage targets. Coverage is a vanity metric. Test the behavior that matters.

### Module Boundaries

Imports flow from higher layers to lower layers only. Never create cycles. Adding a cross-module import requires verifying the dependency graph.

Each module declares its public surface explicitly. Consumers import from the public API, not internal files.

### Git & Workflow

#### Commits

Conventional commits with component scope:
```
feat(mouseion): add audiobook chapter detection
fix(akroasis): correct gapless playback gap on FLAC
refactor(mouseion): extract metadata provider interface
test(akroasis): add property tests for ReplayGain calculation
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `perf`
Scopes: `mouseion`, `akroasis`, `docs`, `infra`

Rules:
- Present tense, imperative mood: "add X" not "added X"
- First line ≤72 characters
- Body wraps at 80 characters
- One logical change per commit
- Squash merge on PR

#### Authorship

All commits use the operator's identity. Agents are tooling, not contributors.

#### Branches

| Type | Pattern | Example |
|------|---------|---------|
| Feature | `feat/<description>` | `feat/audiobook-chapters` |
| Bug fix | `fix/<description>` | `fix/gapless-gap` |
| Chore/docs | `chore/<description>` | `chore/update-deps` |

Always branch from `main`. Always rebase before pushing. Never commit directly to `main`.

---

## Rust

For future mouseion rewrite + akroasis audio core. See `.claude/rules/rust.md` for full rules.

### Edition & Toolchain

- Edition: **2024** (latest stable)
- Async: **Tokio** runtime, native `async fn` in traits (no `async-trait` crate)

### Error Handling (Rust)

Per-crate error types via `snafu` (not thiserror):

```rust
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum ConfigError {
    #[snafu(display("failed to read config from {path}"))]
    ReadConfig {
        path: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
```

Layering:
- `snafu` in library crates — typed, matchable, with Location tracking
- `anyhow` only in `main()`, CLI entry points, and tests
- Convention: `source` field = internal error (walk chain), `error` field = external (stop walking)
- Log errors where HANDLED, not where they occur

### Types & Patterns

- Newtypes for domain identifiers (no raw `String`/`u64`)
- Typestate for state machines
- `#[non_exhaustive]` on public enums that may grow
- `pub(crate)` by default

### Allocation Awareness

| Situation | Use | Avoid |
|-----------|-----|-------|
| Read-only string input | `&str` | `String` |
| Usually borrowed, sometimes owned | `Cow<'_, str>` | `String` |
| Must own | `String` | — |
| Compile-time known | `&'static str` | `String` |

### Lint Suppression

- `#[expect(lint)]` over `#[allow(lint)]`
- Every suppression requires inline reason

---

## C# / .NET

Mouseion backend. See `.claude/rules/dotnet.md` for full rules.

### Framework & Version

- **.NET 10**, C# latest
- **Dapper** for data access (no EF Core)
- **DryIoc** for dependency injection
- **FluentValidation** for request validation
- **Polly** for resilience

### Async/Await

- `CancellationToken` on all async method signatures
- Never `.GetAwaiter().GetResult()` — always async
- `ConfigureAwait(false)` in library code

### Error Handling (C#)

- Custom exception types per domain with meaningful messages
- Never `catch (Exception)` without re-throw or logging
- Use `IResult` / `Results.Problem()` for API error responses
- Log at handling point with structured context

### Patterns

- Repository pattern with Dapper (type-safe, generic base)
- `IMemoryCache` with explicit TTL for metadata responses
- Polymorphic `MediaItem` with media-type-specific subclasses
- Register services in appropriate DryIoc module

### Naming (C#)

| Element | Convention |
|---------|-----------|
| Classes, methods, properties | `PascalCase` |
| Local variables, parameters | `camelCase` |
| Private fields | `_camelCase` |
| Constants | `PascalCase` |
| Interfaces | `IPascalCase` |
| Async methods | suffix `Async` |

### Testing (C#)

- xUnit with `FluentAssertions`
- Test naming: `MethodName_Condition_ExpectedResult`
- Mock external services, not internal logic
- In-memory SQLite for data access tests

---

## Kotlin

Akroasis Android. See `.claude/rules/kotlin.md` for full rules.

### Framework & Libraries

- **Jetpack Compose** for UI
- **Hilt** for dependency injection
- **Room** for local persistence
- **StateFlow / SharedFlow** for reactive state
- **Coroutines** for async

### Architecture

- MVVM with `ViewModel` + `StateFlow`
- Repository pattern for data access
- Use cases for business logic
- Unidirectional data flow

### Error Handling (Kotlin)

- `Result<T>` for operations that can fail
- `sealed class` for error hierarchies
- Never bare `catch (Exception)` — catch specific types
- `runCatching { }` for concise error handling

### Naming (Kotlin)

| Element | Convention |
|---------|-----------|
| Classes, interfaces | `PascalCase` |
| Functions, properties | `camelCase` |
| Constants | `UPPER_SNAKE` or `PascalCase` |
| Packages | `lowercase` |
| Composables | `PascalCase` (per Compose convention) |

### Compose

- State hoisting — stateless composables by default
- `remember` / `rememberSaveable` for local state
- Previews for all reusable composables
- Material 3 design system

---

## TypeScript

Akroasis Web. Standards follow React 19 + TypeScript strict mode conventions.

### Framework & Libraries

- **React 19** with function components
- **TypeScript strict mode** — zero `any` in new code
- **Zustand** for state management
- **Vitest** for testing

### Error Handling (TypeScript)

- Typed error classes, never bare `throw new Error(...)`
- Every catch block must log, rethrow, or return meaningful value
- `void` prefix for intentional fire-and-forget promises
- No floating promises — every `async` call must be awaited or explicitly voided

### Naming (TypeScript)

| Element | Convention |
|---------|-----------|
| Files | `kebab-case.ts` / `kebab-case.tsx` |
| Components | `PascalCase` |
| Functions, variables | `camelCase` |
| Constants | `UPPER_SNAKE` |
| Types, interfaces | `PascalCase` |

---

## Gnomon Naming Convention

Persistent names for modules, subsystems, and major components follow the naming system documented in [gnomon.md](gnomon.md). Names identify essential natures, pass the layer test (L1-L4), and compose with the existing name topology.

The naming system is not decoration. Names that identify the right essential nature survive refactors, communicate architectural intent, and resist the drift toward generic labels.

**Process:**
1. Identify the essential nature (not the implementation)
2. Construct from Greek roots using prefix-root-suffix system
3. Run the layer test (L1 practical through L4 reflexive)
4. Check topology against existing names
5. If no Greek word fits naturally, the essential nature isn't clear yet — wait
