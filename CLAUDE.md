<!--
scope: harmonia repo conventions (archon binary, media-platform crates, Dioxus desktop)
defers_to: README.md for project overview; AGENTS.md for agent conventions
tightens: per-crate conventions live beside each crate in crates/*/
-->

# CLAUDE.md: Harmonia

Unified self-hosted media platform. Rust workspace, single binary (`harmonia`)
replacing the *arr ecosystem. Entry point: `crates/archon/src/main.rs`.

See [AGENTS.md](AGENTS.md) for cross-tool agent rules (build commands,
conventions, where-to-add table). Load [`_llm/architecture.toml`](_llm/architecture.toml)
for the layered crate map and [`_llm/decisions.toml`](_llm/decisions.toml) for
technology choices.

## Standards

Canonical standards live under `~/dev/kanon/crates/basanos/standards/`:
STANDARDS.md, RUST.md, SQL.md, SHELL.md, WRITING.md, and AGENT-DOCS.md.

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
- **IDs:** newtypes in `aggelmata` (`MediaId`, `UserId`, `DownloadId`). Never
  raw `String`/`u64`.
- **Event bus:** Aggelia (`aggelmata::HarmoniaEvent` via `tokio::sync::broadcast`).
  Fire-and-forget past-tense facts; direct trait calls when a return value is
  needed. See `docs/architecture/subsystems.md`.
- **Lint suppressions:** `#[expect(lint, reason = "...")]` over `#[allow]`;
  every suppression carries a WHY.
- **Cross-crate type sharing goes through `aggelmata`.** Never import another
  subsystem's internal types directly.

## Branch and commit

- Branch from `main`; squash merge only; delete merged branches.
- Names: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `chore/`.
- Conventional commits: `type(scope): description`. Scope is the crate name or
  `docs`/`infra`.
- No AI attribution (no "Co-authored-by: Claude", no emoji).
- No AI-trope words; the `WRITING/ai-trope` lint enforces the banned list.

## CI

- `ci.yml`: Format, Check, Clippy, Test (nextest + doctests). Toolchain comes
  from `rust-toolchain.toml`, never workflow inputs.
- `security.yml`: cargo deny, cargo audit, osv-scanner, gitleaks — on PRs,
  pushes to `main`, and a daily schedule.
- `gate-attestation.yml`: requires a `Gate-Passed:` trailer in a PR commit and
  rejects AI-attribution markers in PR title/body/commits.
- `pii-scan.yml`: scans the tree against `.github/pii-patterns.txt`.
- `release.yml`: on `v*` tags — test, build 3-target binaries, SBOM +
  provenance attestations, upload release assets.
- `release-please.yml`: version PRs; attests the release source archive.
- `stale.yml`: weekly issue/PR staleness sweep.
- `dependabot-auto-merge.yml`: auto-merges patch and dev-minor dependabot PRs
  after required checks pass.

## Boundaries

- Ask first: workflow changes under `.github/`, workspace edition/resolver
  changes, new `[workspace.dependencies]` entries.
- Never: force-push `main`, bypass CI, commit secrets, introduce `openssl-sys`
  (rustls only).
