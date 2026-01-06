# Akroasis Android App - Security Audit Report

**Date:** 2026-01-05
**Auditor:** Claude Sonnet 4.5
**Codebase Version:** commit 6f5c3f3 (master branch)

## Executive Summary

Comprehensive security audit of the Akroasis Android application identified **10 security findings** across multiple severity levels. The codebase shows evidence of prior security hardening (commits d89a619, 144fff5, 3b2eff0), but several vulnerabilities remain that require attention.

**Severity Breakdown:**
- **Critical:** 2 findings
- **High:** 3 findings
- **Medium:** 3 findings
- **Low:** 2 findings

---

## Critical Severity Findings

### CRIT-001: Hardcoded API Credentials in Version Control
**CVSS Score:** 9.1 (Critical)
**Location:** `/android/local.properties:6-7`

**Issue:**
Last.fm API credentials are hardcoded and committed to version control.

**Impact:**
- API keys exposed in git history
- Enables unauthorized scrobbling, rate limit exhaustion, API quota theft
- Last.fm may revoke keys if detected, breaking functionality for all users

**Remediation:**
1. **Immediate:** Revoke compromised keys at https://www.last.fm/api/account/create
2. Generate new API keys
3. Remove credentials from all git history
4. Update `.gitignore` to exclude `local.properties` permanently
5. Document in README.md that users must supply their own credentials

**Best Practice Violation:** CWE-798: Use of Hard-coded Credentials

---

### CRIT-002: No Certificate Pinning for Mouseion API
**CVSS Score:** 7.4 (Critical)
**Location:** `/android/app/src/main/java/app/akroasis/di/AppModule.kt:64-73`

**Issue:**
OkHttpClient configured without certificate pinning for user-supplied server URLs.

**Impact:**
- Vulnerable to MITM attacks on self-hosted Mouseion servers
- JWT tokens, credentials, and media streams can be intercepted
- Attacker on network can impersonate Mouseion server

**Remediation:**
1. Implement certificate pinning for known Mouseion instances
2. Allow users to add custom certificates for self-hosted deployments
3. Add network security config XML
4. Update AndroidManifest.xml to reference config

**Best Practice Violation:** CWE-295: Improper Certificate Validation

---

## High Severity Findings

### HIGH-001: Server URL Input Validation Missing
**CVSS Score:** 7.3 (High)
**Location:** `/android/app/src/main/java/app/akroasis/ui/auth/AuthViewModel.kt:37-41`

**Issue:**
Server URL validation only checks for blank input, not URL structure.

**Impact:**
- Malicious URLs can be injected
- SSRF potential
- App crash on malformed URLs

**Remediation:**
Add URL validation before saving with scheme and host checks.

**Best Practice Violation:** CWE-20: Improper Input Validation

---

### HIGH-002: Scrobble Credentials Stored in Unencrypted SharedPreferences
**CVSS Score:** 6.8 (High)
**Location:** `/android/app/src/main/java/app/akroasis/data/preferences/ScrobblePreferences.kt:14-17`

**Issue:**
Last.fm session keys and ListenBrainz tokens stored in plain SharedPreferences.

**Impact:**
- Session tokens readable by attackers with device access
- Root/ADB access exposes credentials
- Cloud backups leak long-lived tokens

**Remediation:**
Migrate to EncryptedSharedPreferences (like AuthInterceptor).

**Best Practice Violation:** CWE-311: Missing Encryption of Sensitive Data

---

### HIGH-003: Path Traversal Vulnerability in Offline Downloads
**CVSS Score:** 6.5 (High)
**Location:** `/android/app/src/main/java/app/akroasis/data/download/OfflineDownloadManager.kt:104-118`

**Issue:**
File operations use unsanitized `trackId` directly in paths.

**Impact:**
- Malicious `trackId` like `../../sensitive_file` could delete arbitrary files
- Data loss potential
- Privilege escalation if writable system files accessed

**Remediation:**
Sanitize trackId before file operations and verify canonical paths.

**Best Practice Violation:** CWE-22: Improper Limitation of a Pathname to a Restricted Directory

---

## Medium Severity Findings

### MED-001: No Rate Limiting on API Endpoints
**CVSS Score:** 5.3 (Medium)
**Location:** `/android/app/src/main/java/app/akroasis/di/AppModule.kt:64-73`

**Issue:**
OkHttpClient configured without request rate limiting.

**Impact:**
- DoS against self-hosted Mouseion servers
- Excessive battery drain
- No protection against retry storms

**Remediation:**
Add rate limiting interceptor with configurable thresholds.

**Best Practice Violation:** CWE-770: Allocation of Resources Without Limits or Throttling

---

### MED-002: Insufficiently Protected Backup Data
**CVSS Score:** 5.0 (Medium)
**Location:** `/android/app/src/main/AndroidManifest.xml:21`

**Issue:**
`allowBackup="true"` permits automatic cloud backups without encryption.

**Impact:**
- JWT tokens, session keys, cached metadata exposed in backups
- Google Drive backups readable by attackers
- Privacy violation for self-hosted deployments

**Remediation:**
Disable automatic backups or create backup exclusion rules.

**Best Practice Violation:** CWE-359: Exposure of Private Information

---

### MED-003: Insufficient Input Sanitization in Playlist Export
**CVSS Score:** 4.8 (Medium)
**Location:** `/android/app/src/main/java/app/akroasis/ui/queue/QueueExporter.kt:45-91`

**Issue:**
Track metadata written to playlist files without sanitization.

**Impact:**
- Special characters corrupt playlist format
- Injection attacks via crafted filePaths
- Media player crashes

**Remediation:**
Sanitize metadata before writing to playlist files.

**Best Practice Violation:** CWE-116: Improper Encoding or Escaping of Output

---

## Low Severity Findings

### LOW-001: Timber Logging May Leak Sensitive Data in Production
**CVSS Score:** 3.7 (Low)

**Issue:**
Timber.d/Timber.i logs contain potentially sensitive information.

**Impact:**
- Listening history, usernames logged to Logcat
- Privacy violation

**Remediation:**
Configure Timber to disable debug logs in release builds.

**Best Practice Violation:** CWE-532: Insertion of Sensitive Information into Log File

---

### LOW-002: Missing ProGuard Obfuscation in Release Build
**CVSS Score:** 3.1 (Low)
**Location:** `/android/app/build.gradle.kts:46-47`

**Issue:**
ProGuard enabled but minification disabled.

**Impact:**
- APK reverse engineering easier
- API endpoint discovery simplified

**Remediation:**
Enable minification for release builds.

**Best Practice Violation:** CWE-656: Reliance on Security Through Obscurity

---

## Positive Security Practices Observed

1. **EncryptedSharedPreferences for JWT tokens** - Properly uses AES256-GCM
2. **SQL injection protection** - All Room queries use parameterized @Query annotations
3. **No WebView usage** - Eliminates XSS/JavaScript injection risks
4. **Dependency vulnerability scanning** - OWASP Dependency Check configured
5. **Secure password input** - PasswordVisualTransformation in use
6. **HTTPS-only external APIs** - Last.fm and ListenBrainz use hardcoded HTTPS URLs
7. **JNI pointer validation** - Added in commit 144fff5

---

## Remediation Priority Matrix

| Finding | Severity | Effort | Priority |
|---------|----------|--------|----------|
| CRIT-001: Hardcoded API Keys | Critical | Low | **P0 - Immediate** |
| CRIT-002: No Certificate Pinning | Critical | Medium | **P0 - Immediate** |
| HIGH-001: URL Validation Missing | High | Low | **P1 - Sprint 1** |
| HIGH-002: Unencrypted Scrobble Tokens | High | Low | **P1 - Sprint 1** |
| HIGH-003: Path Traversal | High | Medium | **P1 - Sprint 1** |
| MED-001: No Rate Limiting | Medium | Medium | **P2 - Sprint 2** |
| MED-002: Backup Exposure | Medium | Low | **P2 - Sprint 2** |
| MED-003: Playlist Injection | Medium | Low | **P2 - Sprint 2** |
| LOW-001: Logging PII | Low | Low | **P3 - Backlog** |
| LOW-002: ProGuard Disabled | Low | Medium | **P3 - Backlog** |

---

## Testing Recommendations

1. **Penetration Testing:**
   - MITM proxy to test certificate pinning
   - Fuzz server URL input
   - Path traversal testing with `../../` payloads

2. **Static Analysis:**
   - Android Lint security checks: `./gradlew lint`
   - Enable StrictMode in debug builds

3. **Dynamic Analysis:**
   - Monitor ADB logcat for log exposure
   - Filesystem monitoring
   - Network traffic capture

---

## Conclusion

The Akroasis Android app demonstrates **strong foundational security** with encrypted token storage, parameterized SQL queries, and no XSS vectors. However, **two critical issues require immediate attention:**

1. **Hardcoded Last.fm credentials** expose API keys
2. **Missing certificate pinning** leaves Mouseion vulnerable to MITM

Addressing P0 and P1 findings will bring the app to production-ready security posture.

**Overall Security Grade:** B (Good, with critical gaps to address)
