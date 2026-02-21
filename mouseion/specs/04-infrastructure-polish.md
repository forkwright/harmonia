# Spec 04: Infrastructure Polish

**Status:** Active
**Priority:** Medium
**Issues:** (CI/workflow improvements)

## Goal

Harden CI, consolidate redundant workflows, clean up configuration. The repo had 8 CI workflows with overlapping coverage and stale branch references.

## Phases

### Phase 1: CI consolidation (done as part of repo cleanup)
- [x] Eliminate develop branch, target main
- [x] Delete 11 stale remote branches
- [x] Fix pr-check.yml AI indicator blocklist
- [x] Update all workflow branch references (develop → main)
- [x] Update SonarCloud newCode reference

### Phase 2: Workflow optimization
- [x] Consolidate ci.yml + lint.yml into single workflow (3 jobs: build-test, lint, dependencies)
- [x] Remove codacy.yml (redundant with CodeQL)
- [x] Shared NuGet cache across all jobs
- [x] specs/ added to paths-ignore
- [ ] Docker build only on tags + main (already configured, verify)

### Phase 3: Developer experience
- [ ] Pre-commit hook for `dotnet format`
- [ ] Makefile or justfile for common commands
- [ ] Local development setup docs

## Dependencies

- None

## Notes

- Codacy workflow used project token secret that was not configured. Removed in favor of CodeQL.
- SonarCloud and CodeQL provide overlapping static analysis — keeping CodeQL only for security.
- Docker workflow already correctly scoped to main + tags.
