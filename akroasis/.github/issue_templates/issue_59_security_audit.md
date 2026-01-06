---
name: Security Audit
about: Conduct security audit of API client and file handling
title: '[Infrastructure] Conduct security audit of API client and file handling'
labels: 'enhancement, infrastructure, android, web, l'
assignees: ''
---

## Context

Security audit was deferred from Phase 0 to focus on core features. Before expanding public-facing APIs in Phase 2+, need comprehensive security review of input validation, path traversal, CORS, rate limiting, and secrets management.

**Inherited from audiobookshelf:** Akroasis fork carries over authentication and API patterns from upstream. Review required to validate security posture.

## Scope

### Audit Areas

#### 1. Input Validation
- **Scope**: User-supplied data sanitization (search queries, filenames, paths)
- **Concerns**: Injection attacks, malformed data crashes
- **Files to review**:
  - API client request builders (Retrofit)
  - File path construction (local storage, cache)
  - Search query handling
  - Playlist name validation

#### 2. Path Traversal
- **Scope**: File access restrictions, directory traversal prevention
- **Concerns**: Unauthorized file access, escaping allowed directories
- **Files to review**:
  - Local file access (downloads, cache)
  - Artwork loading
  - M3U/PLS playlist export (relative vs absolute paths)

#### 3. API Authentication
- **Scope**: Token handling, storage, rotation
- **Concerns**: Token leakage, insecure storage, session hijacking
- **Files to review**:
  - Token storage (SharedPreferences encryption)
  - API key handling (Last.fm, ListenBrainz)
  - Mouseion authentication flow

#### 4. CORS Configuration
- **Scope**: Mouseion API access from web client
- **Concerns**: Cross-origin attacks, unauthorized access
- **Files to review**:
  - Web client API calls (fetch/axios)
  - Allowed origins configuration (Mouseion backend)

#### 5. Rate Limiting
- **Scope**: API request throttling, abuse prevention
- **Concerns**: DoS attacks, excessive requests
- **Review**:
  - Client-side rate limiting (Last.fm, ListenBrainz)
  - Mouseion backend rate limits (if any)
  - Retry logic and backoff strategies

#### 6. SQL Injection
- **Scope**: Room query parameterization
- **Concerns**: Malicious SQL in queries
- **Files to review**:
  - All Room DAO queries
  - Dynamic query construction (if any)

#### 7. XSS (Cross-Site Scripting)
- **Scope**: Web UI user-generated content
- **Concerns**: Malicious scripts in playlist names, track titles
- **Files to review**:
  - Web client rendering (React components)
  - Playlist name display
  - Search results rendering

#### 8. Secrets Management
- **Scope**: API keys, tokens, credentials
- **Concerns**: Hardcoded secrets, leakage in logs/errors
- **Files to review**:
  - `local.properties` (Last.fm keys)
  - Environment variable usage
  - BuildConfig secrets
  - Git history for accidental commits

### Deliverables

1. **Security Audit Report** (`docs/SECURITY_AUDIT.md`)
   - Findings summary table (severity, area, description)
   - CVSS scores for vulnerabilities (if applicable)
   - Risk assessment matrix

2. **Vulnerability Findings**
   - Detailed description per finding
   - Severity: Critical, High, Medium, Low
   - Exploitation scenario
   - Affected files/components

3. **Remediation Recommendations**
   - Specific fixes for each finding
   - Priority order (critical first)
   - Estimated effort per fix

4. **Security Best Practices Document** (`docs/SECURITY_BEST_PRACTICES.md`)
   - Secure coding guidelines
   - Input validation patterns
   - Secrets management workflow
   - Security testing checklist

## Acceptance Criteria

- [ ] All 8 audit areas reviewed and documented
- [ ] Vulnerabilities documented with CVSS scores (if applicable)
- [ ] High/critical issues have remediation plan with timeline
- [ ] Security best practices documented for future development
- [ ] Audit report committed to `docs/SECURITY_AUDIT.md`
- [ ] Findings triaged and GitHub issues created for fixes
- [ ] No hardcoded secrets found in codebase or git history

## Dependencies

- Access to complete codebase (Android + Web + Mouseion backend)
- Understanding of Mouseion API authentication flow
- OWASP Top 10 familiarity for risk assessment

## Out of Scope

- **Penetration testing**: Manual audit only, no active exploitation
- **Automated security scanning**: Separate issue (#60 - dependency scanning)
- **Third-party library audits**: Focus on first-party code (dependencies covered by #60)
- **Network security**: HTTPS enforcement assumed (deployment concern)
- **Social engineering**: Code-level security only

## Methodology

### Review Process

1. **Automated scan** (optional): Run SAST tool (e.g., SonarCloud security rules)
2. **Manual code review**: Check each audit area systematically
3. **Threat modeling**: Identify attack vectors per component
4. **Documentation**: Record findings with severity and remediation

### Severity Scoring (CVSS-inspired)

- **Critical**: Remote code execution, full system compromise
- **High**: Authentication bypass, sensitive data exposure
- **Medium**: Limited data exposure, low-impact DoS
- **Low**: Information disclosure, minor issues

### Example Finding Format

```markdown
### Finding #1: Unvalidated File Path in Playlist Export

**Severity**: High
**CVSS**: 7.5 (CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N)
**Area**: Path Traversal (Audit Area 2)

**Description**: M3U playlist export does not validate file paths, allowing directory traversal via malicious track paths.

**Affected Files**: `PlaylistExporter.kt:142`

**Exploitation**: Attacker provides track with path `../../../../etc/passwd`, exported playlist references sensitive file.

**Remediation**: Validate paths against allowed directories, use relative paths only, sanitize input.

**Priority**: High (fix before Phase 2 release)
```

## Platform(s)

All (Android, Web, Infrastructure)

## Size Estimate

**l** (1-2 days)

**Breakdown:**
- Setup and automated scan: 2 hours
- Manual code review (8 areas): 8-10 hours
- Threat modeling: 2 hours
- Documentation and reporting: 2-3 hours
- Remediation planning: 1 hour
