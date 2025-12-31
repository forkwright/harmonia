# Security & Performance Audit - Fixes Tracker

**Audit Date:** 2025-12-31
**Status:** In Progress

## CRITICAL - Immediate Action Required

### 1. Dependencies (2 Critical)

- [x] **CVE-2013-5042: SignalR 1.1.0 XSS Vulnerability** ✅ FIXED (2025-12-31)
  - Removed: Microsoft.AspNetCore.SignalR.Core 1.1.0
  - Solution: Use built-in ASP.NET Core SignalR via FrameworkReference
  - File: `src/Mouseion.SignalR/Mouseion.SignalR.csproj`
  - Severity: HIGH

- [x] **.NET Version Mismatch** ✅ FIXED (2025-12-31)
  - Upgraded: .NET 8.0 → .NET 10.0 (latest)
  - All packages aligned to .NET 10.x
  - CI workflows updated to .NET 10.0.x
  - Severity: MEDIUM

### 2. Security (7 Critical/High)

- [x] **Missing Authentication on API Endpoints** ✅ FIXED (2025-12-31)
  - Solution: Added API key authentication via X-Api-Key header
  - File: `src/Mouseion.Api/Security/ApiKeyAuthenticationHandler.cs`
  - Applied [Authorize] to ChaptersController
  - Severity: CRITICAL

- [x] **Path Traversal Vulnerability** ✅ FIXED (2025-12-31)
  - Solution: Created PathValidator service with path normalization
  - File: `src/Mouseion.Common/Security/PathValidator.cs`
  - Integrated into ChaptersController
  - Severity: CRITICAL

- [x] **MD5 Cryptographic Weakness** ✅ FIXED (2025-12-31)
  - Solution: Replaced MD5.Create() with SHA256.Create()
  - File: `src/Mouseion.Common/Crypto/HashProvider.cs`
  - Severity: HIGH

- [x] **SHA1 Cryptographic Weakness** ✅ FIXED (2025-12-31)
  - Solution: Replaced SHA1.Create() with SHA256.Create()
  - File: `src/Mouseion.Common/Crypto/HashConverter.cs`
  - Severity: HIGH

- [x] **Command Injection - PowerShell** ✅ FIXED (2025-12-31)
  - Solution: Implemented EscapePowerShellArg() with single-quote escaping
  - File: `src/Mouseion.Common/Processes/ProcessProvider.cs:417-428`
  - Removed `-ExecutionPolicy Bypass` security risk
  - Severity: HIGH

- [x] **Command Injection - cmd.exe** ✅ FIXED (2025-12-31)
  - Solution: Implemented EscapeCommandLineArg() with cmd.exe metacharacter escaping
  - File: `src/Mouseion.Common/Processes/ProcessProvider.cs:387-412`
  - Escapes: &, |, <, >, ^, ", %
  - Severity: HIGH

- [x] **Missing HTTP Security Headers** ✅ FIXED (2025-12-31)
  - Solution: Created SecurityHeadersMiddleware
  - File: `src/Mouseion.Api/Security/SecurityHeadersMiddleware.cs`
  - Headers: HSTS, X-Content-Type-Options, X-Frame-Options, CSP, Referrer-Policy, Permissions-Policy
  - Severity: HIGH

### 3. Performance (5 Critical)

- [ ] **Blocking Async Calls - Thread Pool Starvation**
  - File: `src/Mouseion.Common/Http/HttpClient.cs` (lines 133, 318, 329, 343, 354, 365)
  - Fix: Remove `.GetAwaiter().GetResult()`, make callers async
  - Severity: CRITICAL

- [ ] **N+1 Database Queries**
  - File: `src/Mouseion.Core/Datastore/BasicRepository.cs:104-115`
  - Fix: Implement batch operations for InsertMany/UpdateMany
  - Severity: CRITICAL

- [ ] **Uncached Reflection in Hot Path**
  - File: `src/Mouseion.Core/Datastore/BasicRepository.cs:120-146`
  - Fix: Cache PropertyInfo and SQL statements
  - Severity: CRITICAL

- [ ] **Lock Contention on HTTP Requests**
  - File: `src/Mouseion.Common/Http/HttpClient.cs:185,244`
  - Fix: Use ConcurrentDictionary instead of lock
  - Severity: HIGH

- [ ] **Blocking Thread.Sleep in Request Path**
  - File: `src/Mouseion.Common/TPL/RateLimitService.cs:52`
  - Fix: Replace with async Task.Delay
  - Severity: HIGH

## HIGH Priority

### Dependencies
- [ ] OpenTelemetry version alignment (1.10.0 → 1.14.0)
- [ ] DryIoc preview-04 → stable 8.x when available

### Security
- [ ] Weak PRNG for OAuth (use RandomNumberGenerator)
- [ ] AllowedHosts wildcard
- [ ] Missing HTTPS enforcement
- [ ] Bare catch blocks
- [ ] ReDoS potential in regex

### Performance
- [ ] Multiple filesystem enumerations
- [ ] ToList() on property access
- [ ] Process.GetProcesses() not cached

## MEDIUM Priority
- [ ] CORS configuration
- [ ] Connection string parsing fragility
- [ ] Inefficient string operations

## Fix Strategy

1. **Week 1 Day 1**: Document issues (DONE)
2. **Phase 1**: Fix CRITICAL security (authentication, path traversal, crypto)
3. **Phase 1**: Upgrade vulnerable dependencies
4. **Phase 1**: Fix CRITICAL performance (async blocking, N+1 queries)
5. **Phase 2**: Fix HIGH severity issues
6. **Phase 3**: Fix MEDIUM severity issues during code cleanup

## Notes

- These fixes will be integrated into Phase 1 work (quality system development)
- Each fix will include tests
- Security fixes take precedence over features
- Performance fixes tracked separately from feature development
