# CodeQL Configuration

## Overview

CodeQL security scanning runs automatically on all PRs and weekly scheduled scans. Configuration suppresses false positives while maintaining security coverage.

## Suppressed Alerts

### java/android/backup-enabled (Severity: Note)

**Location:** `android/app/src/main/AndroidManifest.xml:20`

**Alert:** "Backups are allowed in this Android application."

**Suppression Rationale:**
- Self-hosted application with user data that SHOULD be backed up
- Stores playback state, playlists, settings, queue history
- No sensitive data in app storage (API keys in BuildConfig, not backed up)
- `allowBackup=true` is intentional and desirable

**Decision:** Suppress via CodeQL config - not a security issue for this use case

### js/missing-origin-check (Severity: Warning)

**Location:** `web/public/mockServiceWorker.js`

**Alert:** "Postmessage handler has no origin check."

**Suppression Rationale:**
- Generated code from MSW (Mock Service Worker) library v2.x
- Used only in development environment, not production builds
- Production build excludes this file entirely
- Not actual application code

**Decision:** Exclude via `paths-ignore` in CodeQL config

## Configuration File

**Path:** `.github/codeql/codeql-config.yml`

```yaml
name: "CodeQL Configuration"

paths-ignore:
  - '**/node_modules/**'
  - '**/dist/**'
  - '**/build/**'
  - 'web/public/mockServiceWorker.js'  # Generated MSW code, dev-only

query-filters:
  - exclude:
      id: java/android/backup-enabled

queries:
  - uses: security-and-quality
```

## Code Quality Standards

### Exception Handling

**TypeScript/JavaScript:**
- Generic `catch (error)` is acceptable - browser APIs throw various types
- Convert unknown errors to Error type when re-throwing
- Log errors with context for debugging

**Kotlin/Android:**
- No generic `catch (Exception)` blocks found
- Kotlin's type system and sealed classes provide better error handling
- Use specific exception types where applicable

### Current Status

- **Total Alerts:** 0 open (2 suppressed)
- **Last Scan:** Weekly Monday 6am UTC
- **Languages:** JavaScript, Java (Kotlin excluded - extractor doesn't support Kotlin 2.3.0 yet)
- **Query Suite:** security-and-quality

## Updating Configuration

When adding new suppressions:

1. Document rationale in this file
2. Update `.github/codeql/codeql-config.yml`
3. Verify suppression works in next scan
4. Review annually to ensure suppressions still valid

## References

- [CodeQL Documentation](https://codeql.github.com/docs/)
- [Mouseion CodeQL Fixes](../../ai_devops_collab/inbox/for_mouseion/) - Battle-tested strategies from 200+ fixes
