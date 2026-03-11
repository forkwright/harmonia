# Spec 05: Infrastructure & CI

**Status:** Active
**Priority:** Medium
**Issues tracked:** #50, #51, #52, #59, #88, #119 (closing all — spec is source of truth)

## Goal

Harden the build pipeline, improve test coverage, conduct security audit, and set up monitoring.

## Phases

### Phase 1: CI consolidation (DONE)
- [x] Merge redundant web workflows — PR #149 (removed Dependency Audit, security-scan.yml)
- [x] Fix pr-check.yml AI indicator blocklist
- [x] Update SonarCloud newCode reference (develop → main)
- [x] Eliminate develop branch, target main directly
- [x] Add dependabot for npm, gradle, cargo, github-actions — PR #149
- [x] Remove redundant Windows compile check and duplicate Rust checks — PR #149

### Phase 2: Test coverage
- [x] Expand web test coverage — now 396 web tests (up from 70), shipped across PRs #143-174
- [ ] Integration tests for API client against real Mouseion
- [ ] E2E tests for critical user flows

### Phase 3: Security
- [ ] Security audit of API client, auth flow, file handling
- [ ] Dependency audit and update policy (dependabot now handles automated checks)
- [ ] Secret rotation documentation
- [x] JWT auth integration with Mouseion AuthController — PR #159 (authStore + login/refresh/logout)

### Phase 4: Monitoring
- [ ] Homelab webhook and metrics system — partially blocked on Mouseion
- [ ] Performance profiling baseline
- [ ] Bundle size tracking in CI

### Phase 5: Documentation
- [ ] Developer guide
- [ ] API client documentation
- [ ] Contribution workflow with spec references

## Dependencies

- Mouseion now has AuthController with JWT — security audit should include JWT integration
- Mouseion has OpenTelemetry support — monitoring can hook into this

## Notes

- Web test count: 70 → 396 across PRs #143-174.
- CI simplified from 6 workflows to 5 (removed security-scan.yml, dependabot replaces it).
- Android CI trimmed: removed Windows compile check, duplicate Rust checks, dead develop branch.
