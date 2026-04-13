# Usenet / NNTP: feasibility verdict and native download pipeline design

Cross-references: [architecture/cargo.md](../architecture/cargo.md) (usenet feature flag), [architecture/subsystems.md](../architecture/subsystems.md) (Ergasia ownership), [download/archive.md](archive.md) (extraction after download)

---

## Feasibility verdict: GO

Harmonia implements native Usenet download. No SABnzbd sidecar. The single-binary philosophy extends to download protocols.

The crate ecosystem provides enough to build a production-grade Usenet client with one identified gap (PAR2 repair) that has a clear v1 solution.

**Feature flag:** All Usenet code paths are gated behind the `usenet` feature in `archon/Cargo.toml`. Builds without `--features usenet` produce a binary with no Usenet dependency weight.

---

## Crate assessment

### Nntp-rs (jvz-devx)

The primary Usenet download library. Assessed capabilities:

| Capability | Status |
|-----------|--------|
| Async NNTP client | Yes (Tokio-native) |
| TLS 1.2 / 1.3 | Yes (rustls) |
| Connection pooling | Yes (bb8) |
| RFC 8054 DEFLATE compression | Yes |
| yEnc decoding | Yes (built-in) |
| CRC32 verification per segment | Yes |
| PAR2 packet parsing | Yes (extracts checksums and file list) |
| PAR2 Reed-Solomon repair | **No** (parsing only, not repair) |
| SASL PLAIN authentication | Yes |
| Test coverage | ~1400 tests |
| Unsafe code | Zero |

**Crates.io publication status:** Needs verification at implementation time.

- If published: `nntp-rs = "0.x"` (verify exact version)
- If not published: git dependency, `nntp-rs = { git = "https://github.com/jvz-devx/nntp-rs" }`
- If the crate becomes unmaintainable: build a minimal NNTP client on `tokio::net::TcpStream` + `tokio-rustls`. RFC 3977 is straightforward; connection pooling (bb8) is the primary value-add from nntp-rs.

### Nzb-rs

NZB XML file parser. Uses `roxmltree` internally. Provides `Nzb::parse()` entry point. Spec-compliant with the NZB 1.1 format. No known gaps; this is the right tool for NZB parsing.

### Identified gaps

| Gap | Impact | v1 Solution |
|-----|--------|-------------|
| PAR2 Reed-Solomon repair | Corrupt/missing segments cannot be repaired natively | `par2cmdline` subprocess |
| Multi-server failover | nntp-rs manages a single server connection pool | Ergasia implements failover at the download level |
| Rate limiting | No built-in bandwidth throttle | Ergasia implements token bucket around the connection pool |

---

## NZB-to-import pipeline

```
NZB file received (from Newznab indexer search result)
    |
Parse NZB (nzb-rs): extract server list, groups, segment Message-IDs per file
    |
For each file in NZB (parallel across files, sequential within file):
    Fetch all segments from NNTP server (nntp-rs connection pool)
    Decode yEnc per segment (nntp-rs built-in)
    Verify CRC32 per segment — mismatch emits SegmentCrcFailed, fetch retried or failed
    |
Reassemble file from decoded segments (ordered by segment number attribute in NZB)
    |
PAR2 verification:
    Parse .par2 files (nntp-rs) — extract checksums and expected file list
    Verify each reconstructed file hash against PAR2 checksums
    |
    +-- All files verified: OK
    |     Proceed to extraction/import
    |
    +-- Verification failed: corruption or missing segments
          PAR2 repair via par2cmdline subprocess
          |
          +-- Repair succeeded: proceed to extraction/import
          |
          +-- Repair failed: emit DownloadFailed
    |
Extract archives (if applicable — see archive.md)
    |
Feed import pipeline (Syntaxis -> Kathodos)
```

---

## Segment download

NZB format: `<file>` elements, each containing `<segment>` elements with Message-IDs and size.

For each segment:

1. Send `ARTICLE <message-id>` to NNTP server via connection pool
2. NNTP server responds: article headers + yEnc-encoded binary body
3. nntp-rs decodes yEnc, outputs raw bytes + CRC32 checksum
4. Verify CRC32 against the yEnc trailer `=yend` checksum
5. On mismatch: retry from next available provider

Segments ordered by `number` attribute in NZB. Reassembly: concatenate decoded segment bytes in ascending `number` order to reconstruct the original file.

**Connection pool:** `bb8` pool per provider, pool size = `config.ergasia.usenet_connections` (default: 10). Multiple segments fetched concurrently; segment-level concurrency within each file, file-level concurrency across files.

---

## Multi-server support

nntp-rs manages a single server's connection pool. Ergasia owns multi-server failover.

**Provider configuration:** `[ergasia.usenet_providers]` array, each entry:

```toml
[[ergasia.usenet_providers]]
host = "news.provider-a.com"
port = 563
tls = true
username = "user@provider-a.com"
password = "secret"
connections = 10
priority = 1
max_retention_days = 3650   # optional — skip this provider for older articles
```

**Failover logic (Ergasia-owned):**

1. Sort providers by `priority` (ascending; lower number = higher priority)
2. Attempt segment fetch from highest-priority provider
3. On failure (article not found, connection error, auth failure): try next provider
4. If all providers exhausted for a segment: emit `SegmentMissing`
5. If enough segments missing to make PAR2 repair impossible: emit `DownloadFailed`

**Retention routing:** For articles older than `max_retention_days`, skip that provider entirely and try the next. Avoids wasting connection attempts on providers with short retention.

---

## PAR2 verification and repair

### What nntp-rs provides

nntp-rs parses PAR2 file packets and extracts:
- Expected file list with MD5 checksums and sizes
- Block size and block count
- Recovery block data (the parity data)

It does NOT implement Reed-Solomon repair; it cannot reconstruct missing blocks from parity data.

### V1: par2cmdline subprocess

The standard Usenet PAR2 repair tool. Widely available on Linux, macOS, and Windows.

**Verification:**
```
par2 verify <path/to/file.par2>
```
Exit code 0: all files verified OK. Non-zero: repair needed or repair impossible.

**Repair:**
```
par2 repair <path/to/file.par2>
```
Exit code 0: repair succeeded. Non-zero: repair failed (insufficient parity data).

**System dependency:** `par2cmdline` (or the `par2` binary) must be in PATH. Document clearly in deployment instructions.

**Graceful degradation when par2 binary is absent:**
1. Log warning: "par2 binary not found; PAR2 repair unavailable"
2. Emit `ErgasiaError::Par2NotInstalled`
3. Attempt import anyway; if files are actually complete, import may succeed
4. If files are corrupt: import fails with a clear error distinguishing "no par2" from "repair attempted and failed"

**par2_binary_path config:** `config.ergasia.par2_binary_path` (default: `"par2"`, searches PATH). Set to absolute path if `par2` is installed in a non-standard location.

### Future: native PAR2 repair

Full native implementation would require:

1. PAR2 packet parsing (already provided by nntp-rs)
2. Reed-Solomon math over GF(2^16) (`reed-solomon-simd` crate provides this)
3. File reconstruction from recovery blocks (application-level logic on top of the math)

Estimated effort: significant. Deferred to a future milestone. The `par2cmdline` dependency is acceptable for v1 given that Usenet users universally have it installed.

---

## Usenet download state machine

Extends Ergasia's download state machine. No seeding state; Usenet has no upload obligation.

```
Queued
    |
FetchingSegments (progress: segments_fetched / total_segments)
    |
Reassembling (concatenating decoded segment bytes per file)
    |
Verifying (PAR2 checksum comparison)
    |
    +-- All files OK
    |       |
    |   Completed ──> (proceed to extraction/import)
    |
    +-- Corruption detected
            |
        Repairing (par2cmdline subprocess)
            |
            +-- Repair succeeded
            |       |
            |   Completed ──> (proceed to extraction/import)
            |
            +-- Repair failed
                    |
                Failed (reason: Par2RepairFailed)

Failed (missing segments / all servers exhausted / auth failure)
```

**Progress tracking:** `percent` = `(segments_fetched / total_segments) * 100` during `FetchingSegments`. Emitted as `DownloadProgress` events on Aggelia bus at a throttled rate (at most once per second per download).

---

## Bandwidth control

nntp-rs has no built-in rate limiting. Ergasia wraps the segment fetch loop with a token bucket rate limiter:

- `config.ergasia.usenet_bandwidth_limit_kbps`: 0 = unlimited (default)
- Limit applies across all active Usenet downloads combined (not per-download)
- Token bucket replenishes at `usenet_bandwidth_limit_kbps * 1024 / 8` bytes per second
- Each segment fetch acquires tokens equal to the expected segment size before sending the `ARTICLE` command

---

## Error handling

`ErgasiaError` variants for Usenet:

```rust
#[derive(Debug, Snafu)]
pub enum ErgasiaError {
    #[snafu(display("NZB parse failed"))]
    NzbParse {
        source: nzb_rs::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Cannot connect to NNTP server {server}"))]
    NntpConnect {
        server: String,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("NNTP authentication failed for server {server}"))]
    NntpAuth {
        server: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Segment {message_id} not found on any configured server"))]
    SegmentMissing {
        message_id: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("CRC32 mismatch for segment {message_id}"))]
    SegmentCrcFailed {
        message_id: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("PAR2 verification found corruption at {path}"))]
    Par2VerifyFailed {
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("par2cmdline repair failed for {path}"))]
    Par2RepairFailed {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("par2 binary not found in PATH — PAR2 repair unavailable"))]
    Par2NotInstalled {
        #[snafu(implicit)]
        location: Location,
    },
}
```

---

## Horismos configuration

`[ergasia]` Usenet additions in `harmonia.toml`:

```toml
[ergasia]
# Connection pool size per provider (total concurrent NNTP connections to one server).
usenet_connections = 10

# Bandwidth limit across all Usenet downloads. 0 = unlimited.
usenet_bandwidth_limit_kbps = 0

# Path to par2cmdline binary. Defaults to searching PATH.
par2_binary_path = "par2"

[[ergasia.usenet_providers]]
host = "news.example.com"
port = 563
tls = true
username = "youruser"
password = "yourpassword"
connections = 10
priority = 1
# max_retention_days = 3650  # optional
```
