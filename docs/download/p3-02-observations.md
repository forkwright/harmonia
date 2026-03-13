# P3-02 Ergasia: implementation observations

## Librqbit API notes

- `Session::new_with_opts` takes `PathBuf` directly, not `Into<PathBuf>`. The design doc showed `.into()` but it's unnecessary.
- `SessionPersistenceConfig::Json { folder: Some(path) }` is the correct way to enable persistence. There is no `SessionOptions::default_persistence()` helper.
- `Session::with_torrents` passes a `dyn Iterator` (not `ExactSizeIterator`), so `.len()` is unavailable; use `.count()` instead.
- `ManagedTorrent::stats()` is synchronous, not async. The design doc's `api_stats_v1` mapping is conceptual; the actual call is just `.stats()`.
- `Session::pause` takes `&Arc<ManagedTorrent>`, not a torrent ID. Must look up the handle first.
- `Session::delete` takes `TorrentIdOrHash` and a `bool` (keep files).
- librqbit errors are `anyhow::Error`, not a typed enum. Ergasia wraps them as `String` in the `error` field (per the snafu convention for external/opaque errors).

## Unrar crate API

- The unrar 0.5 crate uses a typestate pattern: `Archive::new(path).open_for_processing()` → iterate with `.read_header()` → `.extract_with_base(output_dir)`.
- There is no `.extract_to()` convenience method on `Archive` directly. The design doc's pseudocode was simplified.
- `FileHeader.unpacked_size` is `u64`, `FileHeader.filename` is `PathBuf`.

## Disk space check

- Used `df --output=avail -B1` subprocess instead of raw `libc::statvfs` FFI to avoid adding `libc` as a dependency. This is simpler and sufficient for the pre-check use case.

## Config defaults

- Design doc torrent.md specifies `max_concurrent_downloads: 5` and `seed_ratio_threshold: 1.0`. The previous horismos config had `max_concurrent_downloads: 3` and `seeding_ratio_limit: 2.0`. Updated to match design doc values.

## Test coverage

- 32 unit tests covering: state machine transitions (valid + invalid), seeding policy (ratio, time, zero-download edge case, per-tracker override resolution), progress throttling (window, delta, combined), archive format detection (magic bytes for RAR/ZIP/7z, extension filtering), first volume detection (modern/legacy/single), ZIP extraction round-trip, nested extraction, nesting depth limit, serde round-trips.
- RAR extraction tests are not included because they require actual RAR archives with valid internal structure; the unrar C library validates archive headers strictly. Integration tests with real archives should be added in a follow-up.
