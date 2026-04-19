# _llm/ - on-demand reference for AI agents

CLAUDE.md and AGENTS.md are instructions (always loaded, short). This directory is reference (loaded on demand, structured).

## Why this exists

A 19-crate workspace is too large to orient through by reading every `CLAUDE.md`. These TOML files compress the canonical docs into token-efficient views: crate tree, layer boundaries, tech decisions. Load the file that matches your task; drill into `docs/` or source only when needed.

## Files

| File | Contents | Canonical source |
|------|----------|------------------|
| `architecture.toml` | 19 crates: layer, purpose, dependency direction | `docs/architecture/cargo.md` + `docs/architecture/subsystems.md` |
| `decisions.toml` | Technology choices with rationale | `docs/VISION.md` + `docs/architecture/*.md` |

These are compressed views, not replacements. If a TOML file drifts from the source, the source wins; update the TOML.

## Loading order

1. **Cold start / first interaction:** `architecture.toml` (which crate owns what, layer boundaries).
2. **Tech/library question:** `decisions.toml` (why sqlx, why snafu, why quinn).
3. **Crate-level work:** source under `crates/<crate>/src/`. Per-crate `CLAUDE.md` files are on the roadmap (see issue #193 follow-up).
4. **Subsystem communication:** `docs/architecture/subsystems.md` (domain ownership + direct-call vs event classification).
5. **Error patterns:** `docs/architecture/errors.md` (snafu one-enum-per-crate convention).

## Format

TOML `[[array_of_tables]]` for registry-style data. Machine-parseable, diff-friendly, cheaper than markdown tables for an equivalent amount of structured content.

## Not (yet) present

- `api.toml`: routes are discoverable via `grep -rn '\.route(' crates/paroche/src`. Adding the file is cheap but the registry is unstable while paroche expands.
- `observability.toml`: no standardized metric/span surface yet.
- `L1-workspace.md` / `L2-crate-summaries/`: deferred - the multi-resolution scheme used by Aletheia lands here in a follow-up pass once per-crate `CLAUDE.md` files exist.

See [`standards/AGENT-DOCS.md`](../standards/AGENT-DOCS.md) (kanon-synced) for the full `_llm/` standard.
