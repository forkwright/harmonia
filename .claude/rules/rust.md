# Rust Rules

Rules for Rust code in Harmonia — mouseion rewrite (Tokio, Axum, embedded DB) and akroasis-core (audio processing via JNI/FFI).

---

## Error Handling

**snafu** (not thiserror) for library crate error enums. GreptimeDB pattern.

- `snafu` enums per crate with `.context()` propagation and `Location` tracking
- No `unwrap()` in library code. `anyhow` only in CLI entry points.
- Convention: `source` field = internal error (walk chain), `error` field = external (stop walking)
- Log errors where HANDLED, not where they occur

Compliant:
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

fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .context(ReadConfigSnafu { path: path.display().to_string() })?;
    Ok(config)
}
```

Non-compliant:
```rust
pub fn parse_config(input: &str) -> Config {
    serde_json::from_str(input).unwrap() // unwrap in library code
}

pub fn connect(url: &str) -> anyhow::Result<Connection> { ... } // anyhow in library

let contents = std::fs::read_to_string(path)?; // bare ? without context
```

---

## Async

All I/O is async (Tokio). No `block_on` inside async context.

### Cancellation Safety

Document cancellation safety for every public async method. In `select!`:
- Cancel-SAFE: `sleep()`, `Receiver::recv()`, `Sender::reserve()`, reads into owned buffers
- Cancel-UNSAFE: `Sender::send(msg)` (message lost), `write_all()` (partial write), mutex guard across `.await`
- All `select!` branches must be cancel-safe or use reserve-then-send pattern

### Locks Across Await

Never hold `std::sync::Mutex` guards across `.await` points. Either drop before await or use `tokio::sync::Mutex`.

### Spawned Tasks

Spawned tasks are `'static` — they outlive any reference. Move owned data in. Clone `Arc`s before spawn. Propagate tracing spans.

### No Nested Runtimes

Never call `Runtime::block_on()` from within async context.

---

## Lifetime & Borrowing

### No Clone Spam

The borrow checker is telling you the data flow is wrong. `.clone()` silences it without fixing the architecture.

### Own by Default

Start with owned types. Only add lifetimes when profiling shows the allocation matters. Config structs own their strings.

### Use Cow for Mixed Owned/Borrowed

```rust
fn normalize_path(path: &str) -> Cow<'_, str> {
    if path.starts_with('/') {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(format!("/{path}"))
    }
}
```

### Arena Over Self-Referential Structs

Never fight the borrow checker with `RefCell` or `unsafe` for graph structures. Use arena allocation with index-based references.

---

## Type System

### Newtypes for Domain Concepts

Domain IDs are newtype wrappers, not bare `String`/`u64`. Zero-cost, compile-time parameter swap safety.

### #[non_exhaustive] on Public Enums

All public enums that may grow must use `#[non_exhaustive]`.

### Typestate Pattern

Use typestate for multi-step builders and connection lifecycle. Compile-time state validation over runtime checks.

### Exhaustive Matching

Use `match` with explicit variants over wildcard arms when the enum is under your control.

---

## Concurrency

### Prefer std::sync::Mutex for Short Critical Sections

`std::sync::Mutex` is faster than `tokio::sync::Mutex` for non-contended, non-async operations. Only use `tokio::sync::Mutex` when holding across `.await`.

---

## Rust 2024 Edition

### Use Standard Library Types

```rust
use std::sync::LazyLock;
static CONFIG: LazyLock<Config> = LazyLock::new(|| load_config());
// NOT: lazy_static, once_cell
```

### #[expect(lint)] Over #[allow(lint)]

`#[expect]` warns you when the suppression is no longer needed.

### Native Async Traits

Use native `async fn` in traits (stable since 1.75). No `async-trait` crate.

### Let Chains and Async Closures

2024 edition features — use them.

---

## API Design

### Accept impl Into<String>, Return Concrete

Flexible input, concrete output.

### Ensure Send + Sync

All types used in async contexts must be `Send + Sync`.

---

## Testing

### Mock at Trait Boundaries

Don't mock internal functions. Define traits at module boundaries and inject test implementations.

### Property Tests for Serialization

Every type that implements `Serialize` + `Deserialize` gets a roundtrip property test.

---

## AI-Specific Anti-Patterns

Things Claude tends to do wrong in Rust:

1. **Over-engineering** — wrapper types with no value, trait abstractions with one impl
2. **Outdated crate choices** — `lazy_static`, `once_cell`, `async-trait`, `failure`
3. **Hallucinated APIs** — verify method signatures against docs.rs. Always `cargo check`.
4. **Incomplete trait impls** — missing `size_hint`, `source()`, Serialize edge cases
5. **Clone to satisfy borrow checker** — restructure ownership instead
6. **unwrap() in library code** — use `?` or explicit error handling
7. **std::sync::Mutex in async** — use tokio::sync::Mutex when holding across `.await`
8. **Ignoring Send+Sync** — types not Send used across thread boundaries

---

## Logging

`tracing` with structured spans. `#[instrument]` on public functions.

- Spawned tasks MUST propagate spans
- Never bare `tokio::spawn()` — always `.instrument()`
- Never hold `span.enter()` across `.await` points

---

## Dependencies

- Prefer std when adequate
- Each new dependency must justify itself
- Pin unstable crates (pre-1.0) to exact versions
- `thiserror` replaced by `snafu` for library crates
- `async-trait` unnecessary — use native async fn in trait
