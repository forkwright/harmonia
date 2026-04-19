# AGENTS.md - Harmonia

Cross-tool agent guidance (agents.md standard). Claude Code, Cursor, Codex, Copilot, and others read this file. Keep under 100 lines; claims that change weekly belong in a tool, not here.

## Commands

- Build: `cargo build --workspace`
- Build (release): `cargo build --workspace --release`
- Test: `cargo test --workspace`
- Format (CI gate): `cargo fmt --all -- --check`
- Lint (CI gate): `cargo clippy --workspace --all-targets -- -D warnings`
- Deps audit: `cargo deny check` (licenses + advisories)
- Targeted: `cargo test -p <crate>` - do this during development; workspace tests are slow.
- Desktop build (excluded from workspace): `cargo check --manifest-path crates/theatron/desktop/Cargo.toml`

## Rules

- **snafu for all errors, one enum per crate.** Use `.context(VariantSnafu { ... })?`; never bare `?`. `source:` for internal/well-known errors (chain walker continues); `error: String` for opaque external failures (chain stops). See `docs/architecture/errors.md`. WHY: location tracking + predictable chain walking for API responses.
- **No `unwrap()` / `expect()` / `dbg!()` / `todo!()` in library crates.** Workspace lints set these to `deny`. Use `#[expect(lint, reason = "...")]` over `#[allow]`. WHY: `#[expect]` warns when the suppression becomes stale.
- **No `thiserror`, `anyhow`, `Box<dyn Error>` in library crates.** `anyhow` is acceptable only in `archon` startup. WHY: typed errors are part of the public contract.
- **Newtypes for domain IDs.** `MediaId`, `UserId`, `DownloadId` - not raw `String`/`u64`. All defined in `themelion`.
- **Cross-subsystem type sharing goes through `themelion`.** Subsystem crates never import another subsystem's internal types. WHY: prevents cycles; the DAG in `docs/architecture/cargo.md` is law.
- **Direct call vs event rule:** if the caller needs the result to continue, direct trait call. If announcing a past-tense fact, emit an `Aggelia` event (`themelion::HarmoniaEvent` via broadcast channel). See `docs/architecture/subsystems.md`.
- **Log errors where HANDLED, not where they occur.** One log entry per error chain, at the site that decides retry/propagate/abort.
- **No AI attribution.** No `Co-authored-by: Claude`, no emoji markers. Commits use `forkwright <noreply@forkwright.dev>`.
- **No AI-trope words** in prose or comments. The kanon `WRITING/ai-trope` lint enforces the banned list; run `kanon lint . --summary` before committing.

## Architecture

- 19 crates under `crates/`. Layers flow downward (foundation -> storage -> auth -> media-ops -> acquisition -> curation -> serving -> runtime). See `_llm/architecture.toml` for the full map.
- Single binary `harmonia` (built from `archon`) with four modes selected via Clap subcommand: `serve`, `desktop`, `render`, `play`. See `docs/architecture/binary-modes.md`.
- Config cascade: `harmonia.toml` (committed defaults) -> `secrets.toml` (gitignored) -> `HARMONIA__{subsystem}__{key}` env vars. Via figment.
- Database: SQLite via sqlx; migrations owned by `apotheke`. WAL mode; dual pool isolation.

## Where to add things

| Task | Location | Registration |
|------|----------|-------------|
| CLI subcommand / mode | `crates/archon/src/cli.rs` | Clap derive in `cli.rs`; mode handler in `serve.rs`/`play.rs`/etc. |
| HTTP route | `crates/paroche/src/routes/<domain>.rs` | Router builder at the bottom of the file; mounted in `lib.rs` |
| OpenSubsonic endpoint | `crates/paroche/src/subsonic/` | Route table in `subsonic/mod.rs` |
| Error variant | `crates/<crate>/src/error.rs` | Add to the crate's single `Snafu` enum |
| Shared type / event | `crates/themelion/src/` | Add variant to `HarmoniaEvent` with past-tense name |
| Config field | `crates/horismos/src/` | Add to subsystem config struct + `harmonia.toml` default |
| External API client | `crates/syndesmos/src/` | New module; credentials via horismos |
| Metadata provider | `crates/epignosis/src/` | Register in provider registry |
| Indexer protocol | `crates/zetesis/src/` | Torznab/Newznab trait impl |
| Audio DSP stage | `crates/akouo-core/src/dsp/` | Add to pipeline; feature-gate if it needs C deps |

Per-crate `CLAUDE.md` files do not yet exist for all 19 crates - use `_llm/architecture.toml` + source for orientation. Tracking issue: #193 follow-up.

## Before submitting

1. `cargo test -p <affected-crate>` passes.
2. `cargo fmt --all -- --check` clean.
3. `cargo clippy --workspace --all-targets -- -D warnings` clean (no new suppressions without `#[expect(..., reason = "...")]`).
4. Branch name: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `chore/`.
5. Conventional commits: `type(scope): description`; scope is the crate name.
6. Squash merge only. No merge commits on `main`.

## Boundaries

- **Ask first:** workflow changes under `.github/`, root `Cargo.toml` edition/resolver changes, new workspace dependencies.
- **Never:** force-push `main`, bypass CI, commit secrets (gitleaks runs in CI), use `openssl-sys` anywhere (rustls only).
