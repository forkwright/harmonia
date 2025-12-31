# Mouseion ↔ Akroasis Coordination

**Last Updated**: 2025-12-31

## Current Status

**Mouseion**: Week 1 Day 1 complete - Security audit finished, CI/CD infrastructure established
**Akroasis**: Awaiting Media3 migration (your side) - no blockers for Mouseion
**Next Sync**: After Mouseion Phase 1 complete (Week 2)

---

## Timeline Update (from Mouseion ROADMAP.md)

Mouseion is entering 16-20 week migration phase:
- **Week 6**: Books/Audiobooks API ready (Phase 2 complete) ⭐
- **Week 10**: Music API ready (Phase 3 complete)
- **Week 14**: Podcast API ready (Phase 5 complete)

---

## Integration Points

### Audiobooks (Week 6 - February 2026)
**Endpoints**:
- `/api/v3/audiobooks` - Full CRUD
- `/api/v3/stream/{id}` - Already implemented
- `/api/v3/chapters/{id}` - Already implemented
- `/api/v3/progress/{id}` - New endpoint

**Authentication**: API key via X-Api-Key header (implemented)

**Security**: Path traversal protection, all endpoints authenticated

### Music (Week 10 - March 2026)
**Endpoints**:
- `/api/v3/artists`, `/api/v3/albums`, `/api/v3/tracks`
- Same streaming infrastructure as audiobooks

### Podcasts (Week 14 - April 2026)
**Endpoints**:
- `/api/v3/podcasts`, `/api/v3/podcast-episodes`
- RSS feed support

---

## Lessons Learned (Relevant to Akroasis)

### Security Best Practices

**1. API Authentication**
✅ **Implemented in Mouseion**: X-Api-Key header authentication
- Simple, effective for self-hosted apps
- Works across all clients (web, mobile, CLI)
- No session management complexity

**Recommendation for Akroasis**: Consider same approach for consistency

**2. Path Traversal Protection**
✅ **Implemented**: PathValidator service
```csharp
var normalizedBase = Path.GetFullPath(baseDirectory);
var normalizedPath = Path.GetFullPath(Path.Combine(baseDirectory, userPath));
if (!normalizedPath.StartsWith(normalizedBase)) throw SecurityException;
```

**Critical for**:
- File streaming endpoints
- Chapter info endpoints
- Any user-provided file paths

**3. No Hardcoded Secrets**
❌ **Mistake Made**: Initially tried to keep TMDB token as "safe default"
✅ **Fixed**: Removed entirely, required environment variable

**Lesson**: Even "public" API tokens should be externalized
- Prevents accidental commits
- Allows rotation without code changes
- Security scanners won't flag

**Recommendation**: Audit Akroasis for hardcoded tokens/keys

### Performance Patterns

**4. Avoid Blocking Async**
🔍 **Found in Mouseion**: `.GetAwaiter().GetResult()` in HttpClient
⚠️ **Impact**: Thread pool starvation under load

**Check Akroasis for**:
- Blocking calls in async methods
- Task.Wait() or .Result on Tasks
- Thread.Sleep instead of Task.Delay

**5. N+1 Query Prevention**
🔍 **Found in Mouseion**: Loop of individual inserts/updates
✅ **Solution**: Batch operations

**Recommendation**: Review Akroasis database access patterns

### CI/CD

**6. GitHub Actions Security**
✅ **Implemented**: Pinned actions to commit SHAs
```yaml
uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5 # v4
```

**Why**: Version tags can be updated to malicious versions

**7. CI Optimization Strategy**
**Current Mouseion CI**: 4+ minutes (full multi-platform)
**Proposed**: 1.5 minutes (PR optimizations)

**Approach**:
- Skip Docker builds on PRs (only on merge)
- Optional Windows builds for PR approval
- Single DB for PR tests, full matrix on merge

**Benefit**: Fast feedback loop without sacrificing quality

**Recommendation**: Apply same optimizations to Akroasis CI

### Code Quality

**8. SonarCloud Integration**
✅ **Value**: Found 9 CRITICAL vulnerabilities + 6 security hotspots
⚠️ **Gotcha**: Automatic analysis conflicts with CI-based scanner
✅ **Solution**: Choose ONE approach, not both

**Recommendation**: If Akroasis uses SonarCloud, verify no conflicts

**9. Command Injection in Process Execution**
🔍 **Found**: PowerShell and cmd.exe argument concatenation
✅ **Fixed**: Proper escaping functions

**Check Akroasis for**:
- Process.Start with user-provided arguments
- Shell command construction
- Script execution

**10. Cryptographic Hash Migration**
✅ **Upgraded**: MD5/SHA1 → SHA256
**Reason**: Even non-security uses should avoid weak primitives

**Recommendation**: Audit Akroasis for MD5/SHA1 usage

---

## API Contract Discussion

### Media Metadata Format
**Question for next sync**: Should we align on metadata structures?
- Artist/Author naming conventions
- Quality/format enums
- Progress tracking format

**Benefit**: Easier for clients consuming both APIs

### Shared Libraries?
**Consideration**: Could extract common code to shared library
- Authentication patterns
- Path validation
- Streaming logic

**Decision needed**: After Phase 2 complete (Week 6)

---

## Blocking Issues

**From Mouseion Side**: None - proceed with Media3 migration in parallel

**From Akroasis Side**: TBD - awaiting update on Media3 timeline

---

## Next Steps

### Mouseion
1. ✅ Complete security audit (DONE)
2. ✅ Establish CI/CD infrastructure (DONE - pending secrets)
3. 🔄 Phase 1: Quality system (Weeks 1-2)
4. 📅 Phase 2: Books/Audiobooks (Weeks 3-6)

### Coordination
1. Add TMDB_API_TOKEN GitHub secret (user action required)
2. Share LESSONS_LEARNED.md with peer
3. Sync after Phase 1 complete (Week 2) to coordinate audiobook metadata format

---

## Communication Protocol

**Update Frequency**: End of each phase (bi-weekly)
**Urgent Issues**: Flag immediately in shared notes
**Format**: Update this file + timestamp

---

## Notes

- Mouseion security audit revealed importance of automated scanning
- CI optimization discussion - recommend 1.5min PR checks vs 4min full builds
- Hardcoded secrets lesson learned - even "safe defaults" should be removed
- Path traversal protection is critical for file streaming APIs
