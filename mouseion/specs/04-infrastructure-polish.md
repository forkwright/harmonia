# Spec 04: Infrastructure Polish

**Status:** Complete (Phase 1-3 complete)
**Priority:** Medium
**Issues:** —

## Goal

Harden CI, consolidate redundant workflows, automate dependency management, improve developer experience.

## Phases

### Phase 1: CI consolidation ✅
- [x] Eliminate develop branch, target main
- [x] Delete stale remote branches
- [x] Fix pr-check.yml AI indicator blocklist
- [x] Update all workflow branch references (develop → main)
- [x] Update SonarCloud newCode reference

### Phase 2: Workflow optimization ✅
- [x] Consolidate 3 CI jobs (build-test, lint, dependencies) into 1 sequential job
- [x] Remove codacy.yml (redundant with CodeQL)
- [x] Shared NuGet cache, consistent action versions across all workflows
- [x] specs/ added to paths-ignore
- [x] Bump all GitHub Actions to latest (checkout@v6, upload-artifact@v6, codeql-action@v4, semantic-pull-request@v6)
- [x] Dependabot: grouped NuGet updates (Microsoft.*, MailKit+MimeKit), increased PR limits
- [x] Auto-merge workflow for dependabot patch/minor PRs (major labeled for manual review)
- [x] Repo settings: allow_auto_merge, delete_branch_on_merge

### Phase 3: Developer experience ✅
- [x] Pre-commit hook for `dotnet format`
- [x] Makefile or justfile for common commands
- [x] Local development setup docs (README or CONTRIBUTING.md)
- [x] Docker compose for local dev (SQLite, no external deps)

## Dependencies

- None

## Notes

- CI now runs ~2 min per PR (down from ~4 min with 3 parallel jobs + 3x restore).
- Dependabot auto-merges patch/minor after CI passes. Major version bumps get `major-update` label for manual review.
