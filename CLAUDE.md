# CLAUDE.md: Harmonia

Unified self-hosted media platform. Rust workspace, single binary (`harmonia`)
replacing the *arr ecosystem. Entry point: `crates/archon/src/main.rs`.

See [AGENTS.md](AGENTS.md) for cross-tool agent rules (build commands,
conventions, where-to-add table). Load [`_llm/architecture.toml`](_llm/architecture.toml)
for the layered crate map and [`_llm/decisions.toml`](_llm/decisions.toml) for
technology choices.

## Standards

Kanon-synced standards live under [standards/](standards/). Universal:
[STANDARDS.md](standards/STANDARDS.md). Per-language: RUST.md, SQL.md, SHELL.md,
WRITING.md, AGENT-DOCS.md.

## Build and test

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p <crate>                          # targeted during development
cargo check --manifest-path crates/theatron/desktop/Cargo.toml   # excluded crate
```

## Key conventions

- **Errors:** snafu 0.8, one enum per crate, `.context(VariantSnafu { ... })?`.
  Location tracking via `#[snafu(implicit)]`. See `docs/architecture/errors.md`.
- **IDs:** newtypes in `themelion` (`MediaId`, `UserId`, `DownloadId`). Never
  raw `String`/`u64`.
- **Event bus:** Aggelia (`themelion::HarmoniaEvent` via `tokio::sync::broadcast`).
  Fire-and-forget past-tense facts; direct trait calls when a return value is
  needed. See `docs/architecture/subsystems.md`.
- **Lint suppressions:** `#[expect(lint, reason = "...")]` over `#[allow]`;
  every suppression carries a WHY.
- **Cross-crate type sharing goes through `themelion`.** Never import another
  subsystem's internal types directly.

## Branch and commit

- Branch from `main`; squash merge only; delete merged branches.
- Names: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `chore/`.
- Conventional commits: `type(scope): description`. Scope is the crate name or
  `docs`/`infra`.
- No AI attribution (no "Co-authored-by: Claude", no emoji).
- No AI-trope words; the `WRITING/ai-trope` lint enforces the banned list.

## CI

- `rust.yml`: format, clippy, test, MSRV, rustdoc, coverage
- `security.yml`: cargo-audit, cargo-deny, gitleaks, TruffleHog

## Boundaries

- Ask first: workflow changes under `.github/`, workspace edition/resolver
  changes, new `[workspace.dependencies]` entries.
- Never: force-push `main`, bypass CI, commit secrets, introduce `openssl-sys`
  (rustls only).
