# Spec 05: Infrastructure & CI

**Status:** Active
**Priority:** Medium
**Issues:** #50, #51, #52, #59, #88, #119

## Goal

Harden the build pipeline, improve test coverage, conduct security audit, and set up monitoring. The repo has 8 CI workflows — some redundant, some broken. Fix the foundation before building more features.

## Phases

### Phase 1: CI consolidation (done as part of repo cleanup)
- [x] Merge redundant web workflows (web.yml + web-ci.yml → single)
- [x] Fix pr-check.yml AI indicator blocklist (blocks legitimate words)
- [x] Update SonarCloud newCode reference (develop → main)
- [x] Eliminate develop branch, target main directly

### Phase 2: Test coverage
- [ ] Expand web test coverage to match Android (currently 70 vs 473)
- [ ] Integration tests for API client against real Mouseion
- [ ] E2E tests for critical user flows (#50)

### Phase 3: Security
- [ ] Security audit of API client, auth flow, file handling (#59, #119)
- [ ] Dependency audit and update policy
- [ ] Secret rotation documentation

### Phase 4: Monitoring
- [ ] Homelab webhook and metrics system (#88)
- [ ] Performance profiling baseline (#51)
- [ ] Bundle size tracking in CI

### Phase 5: Documentation
- [ ] Developer guide (replaces deleted docs) (#52)
- [ ] API client documentation
- [ ] Contribution workflow with spec references

## Dependencies

- Homelab metrics (#88) depends on Mouseion webhook endpoints

## Notes

- SonarCloud is configured but newCode reference was pointing at deleted develop branch.
- CodeQL runs weekly — configuration is solid, 0 open alerts.
- Android CI builds but Kotlin 2.3.0 isn't supported by CodeQL extractor yet.
