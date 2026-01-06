# GitHub Issue Templates - Batch Creation Summary

Created: 2026-01-05

## Overview

10 comprehensive GitHub issue templates created for high/medium priority backlog items identified during Phase 2 completion audit. Templates follow standardized format with Context, Scope, Acceptance Criteria, Dependencies, Out of Scope, Platform, and Size Estimate.

## Templates Created

### High Priority Issues

#### #54: Track.albumId in Mouseion
- **File**: `issue_54_track_albumid.md`
- **Labels**: enhancement, blocked-mouseion, s
- **Size**: s (1-4 hours)
- **Summary**: Add albumId foreign key to Track model for reliable album associations, enabling stable playback speed memory at album level

#### #55: Phase 1 QA Testing
- **File**: `issue_55_phase1_qa.md`
- **Labels**: enhancement, android, l
- **Size**: l (1-2 days)
- **Summary**: Execute 160+ manual test cases from PHASE1_QA.md covering all Phase 1 features (playback, queue, DSP, scrobbling)

#### #56: Voice Search Infrastructure
- **File**: `issue_56_voice_search.md`
- **Labels**: enhancement, android, ui, m
- **Size**: m (4-8 hours)
- **Summary**: Implement voice command handling for media controls via MediaSession API (headset button, Assistant integration)

#### #57: A/B Mode Level Normalization
- **File**: `issue_57_ab_level_normalization.md`
- **Labels**: enhancement, android, audio, m
- **Size**: m (4-8 hours)
- **Summary**: RMS/LUFS level matching for fair A/B EQ comparison (eliminate loudness bias)

#### #58: Battery Impact Profiling
- **File**: `issue_58_battery_profiling.md`
- **Labels**: enhancement, android, infrastructure, m
- **Size**: m (4-8 hours active, 2-3 days wall-clock)
- **Summary**: Profile battery drain for DSP configurations on Sony Walkman, replace rough estimates with real data

#### #59: Security Audit
- **File**: `issue_59_security_audit.md`
- **Labels**: enhancement, infrastructure, android, web, l
- **Size**: l (1-2 days)
- **Summary**: Comprehensive security review of 8 areas (input validation, path traversal, auth, CORS, rate limiting, SQL injection, XSS, secrets)

### Medium Priority Issues

#### #61: Queue Reordering Backend Persistence
- **File**: `issue_61_queue_persistence.md`
- **Labels**: enhancement, android, ui, blocked-mouseion, s
- **Size**: s (1-4 hours Android, 2-3 hours Backend)
- **Summary**: Server-side queue state persistence for drag-reorder operations, enabling cross-device sync

#### #62: Signal Path Format Detection
- **File**: `issue_62_signal_path_format_detection.md`
- **Labels**: enhancement, android, audio, s
- **Size**: s (1-4 hours)
- **Summary**: File metadata introspection instead of filename inference for accurate format display (24/96 vs 16/44.1)

#### #63: Expand Test Coverage to 70%
- **File**: `issue_63_test_coverage.md`
- **Labels**: enhancement, infrastructure, android, web, xl
- **Size**: xl (3+ days, 20-30 hours)
- **Summary**: Integration tests, E2E tests, UI tests to increase coverage from 40-50% to 70%+

#### #64: Performance Profiling
- **File**: `issue_64_performance_profiling.md`
- **Labels**: enhancement, infrastructure, android, web, l
- **Size**: l (1-2 days)
- **Summary**: Profile and establish baselines for cold start, library load, scroll FPS, memory usage, bundle size, API latency

## Template Format Standards

All templates follow this structure:

```markdown
---
name: [Feature name]
about: [One-line description]
title: '[Platform] Short description'
labels: 'label1, label2, size'
assignees: ''
---

## Context
[Why this issue exists, current state, desired state]

## Scope
[What needs to be done - detailed bullets]

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2

## Dependencies
[Prerequisites or blockers]

## Out of Scope
[What's explicitly not included]

## Platform(s)
[Android/Web/Backend/Infrastructure]

## Size Estimate
**size** (time estimate)

**Breakdown:**
- Task 1: X hours
- Task 2: Y hours
```

## Size Estimates Guide

- **xs**: <1 hour
- **s**: 1-4 hours
- **m**: 4-8 hours
- **l**: 1-2 days
- **xl**: 3+ days

## Next Steps

1. **Create GitHub issues**: Use gh CLI or web UI to create issues from templates
2. **Assign labels**: Ensure labels match repository label scheme
3. **Prioritize**: Order issues in project board (High → Medium → Low)
4. **Link dependencies**: Reference Mouseion API dependencies in cross-repo tracking
5. **Update project docs**: Reference issues in ROADMAP.md and CHANGELOG.md

## Usage Instructions

### Using gh CLI

```bash
cd /home/ck/Projects/Akroasis-wrapper/akroasis

# Create issue from template
gh issue create --template issue_54_track_albumid.md

# Or batch create (requires manual editing per issue)
for template in .github/issue_templates/issue_*.md; do
  gh issue create --template "$template"
done
```

### Using GitHub Web UI

1. Go to https://github.com/forkwright/akroasis/issues/new
2. Click "Get started" next to desired template
3. Review and edit as needed
4. Assign labels, projects, milestones
5. Click "Submit new issue"

## Statistics

- **Total templates**: 10
- **High priority**: 6 issues
- **Medium priority**: 4 issues
- **Platforms**:
  - Android: 7 issues
  - Web: 3 issues
  - Backend (Mouseion): 2 issues (blocked)
  - Infrastructure: 3 issues
- **Estimated total effort**: 10-15 days (80-120 hours)
- **Blocked on Mouseion**: 2 issues (#54, #61)

## Related Documentation

- **Plan file**: `/home/ck/.claude/plans/functional-wandering-blum.md`
- **QA test plan**: `/home/ck/Projects/Akroasis-wrapper/akroasis/android/PHASE1_QA.md`
- **Roadmap**: `/home/ck/Projects/Akroasis-wrapper/akroasis/ROADMAP.md`
- **Existing issues**: GitHub #32-#53 (22 issues already created)
