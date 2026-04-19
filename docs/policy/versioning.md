# Versioning policy

## Scheme

Single workspace version in `Cargo.toml` `[workspace.package] version`. Every
crate inherits via `version.workspace = true`. Release automation is driven by
[release-please](../../release-please-config.json); the manifest in
`.release-please-manifest.json` tracks the current version.

Pre-stable (`0.MINOR.PATCH`): breaking changes allowed in MINOR bumps with
a migration note. Post-`1.0.0`: breaking changes require a MAJOR bump.

## What counts as breaking

Breaking (MINOR bump pre-1.0, MAJOR bump post-1.0):

- Changing an HTTP or OpenSubsonic endpoint request or response shape
- Removing or renaming a `harmonia.toml` key
- Database schema change without an automatic sqlx migration
- Removing or renaming a CLI subcommand or flag

Non-breaking (PATCH bump):

- New endpoints, new config keys with defaults, new DB columns with defaults
- Bug fixes and performance work
- Documentation changes

## Process

release-please opens a release PR when conventional commits accumulate on `main`.
The PR is reviewed, approved, and merged by the operator - never auto-merged.
Tagging and binary artifact builds are handled by the workflow; nothing is
manual except approval.

Conventional commits in scope for the changelog: `feat`, `fix`, `perf`,
`refactor`, `docs`. Hidden: `test`, `chore`, `ci`, `style`. See
[`AGENTS.md`](../../AGENTS.md) for the full commit convention.
