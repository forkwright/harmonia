# Archive extraction: RAR/ZIP/7z pipeline with nested detection

Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Ergasia ownership), [download/torrent.md](torrent.md) (DownloadCompleted trigger)

---

## When extraction happens

Archive extraction occurs in the post-processing pipeline, triggered by the `DownloadCompleted` event:

```
DownloadCompleted (from Ergasia)
    |
Syntaxis post-processing picks up the completed download
    |
Syntaxis scans download path for archive files
    |
    +-- Archives detected? YES
    |     Syntaxis calls Ergasia.extract(download_path)
    |     Wait for ExtractionResult
    |     Pass extracted_path to Kathodos.import()
    |
    +-- Archives detected? NO
          Skip extraction
          Pass original download_path to Kathodos.import()
```

Extraction is synchronous within the post-processing pipeline; it blocks import for that download. This is intentional: import cannot begin until extraction is complete and the file set is known.

---

## Format support

Three crate selections based on research:

| Format | Extensions | Crate | Notes |
|--------|-----------|-------|-------|
| RAR | `.rar`, `.r00`–`.r99`, `.part1.rar` | `unrar` 0.5.x | C FFI, vendored unrar source (no system libunrar needed), typestate API |
| ZIP | `.zip` | `zip` 2.x | Pure Rust, `ZipArchive::extract()` |
| 7z | `.7z` | `sevenz-rust2` (latest) | Pure Rust fork of unmaintained sevenz-rust, password-protected archive support |

No other formats are supported in v1. Files with unrecognized magic bytes emit `UnsupportedFormat` and fail extraction for that download.

---

## First volume detection

Multi-part RARs span multiple files. Only the first volume is passed to the extraction API. The `unrar` crate then follows continuation volumes automatically.

### Naming conventions

Two naming schemes exist in the wild:

- **Modern (WinRAR 3.x+):** `.part1.rar`, `.part01.rar`, `.part001.rar` (numbered suffix in the base name)
- **Legacy:** `.rar` (first), `.r00`, `.r01`, `.r02`, ... (extension changes per volume)

### Detection algorithm

```rust
fn find_rar_first_volume(dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|ext| ext == "rar").unwrap_or(false))
        .collect();

    // Modern naming: find the .partN.rar with lowest N
    if let Some(first) = candidates.iter()
        .filter(|p| is_modern_part(p))
        .min_by_key(|p| extract_part_number(p))
    {
        return Some(first.clone());
    }

    // Legacy naming: the .rar file is the first volume when .r00 also exists
    let has_r00 = dir.join(base_name).with_extension("r00").exists();
    if has_r00 {
        return candidates.into_iter().find(|p| p.extension().map(|e| e == "rar").unwrap_or(false));
    }

    // Single-file RAR (no continuation volumes)
    candidates.into_iter().next()
}
```

Priority order:
1. `.part1.rar` / `.part01.rar` / `.part001.rar` (lowest part number wins)
2. `.rar` when `.r00` also exists (legacy multi-part)
3. `.rar` alone (single-file)

**Never iterate and extract each `.rar*` file independently.** This causes double extraction, file corruption from partial reassembly, and duplicated output. Always open from the first volume only.

---

## Nested archive detection

Some downloads contain archives within archives (e.g., a `.rar` containing `.zip` files). Ergasia recurses until no archive signatures are found, up to a configurable depth limit.

### Detection method

Check both file extension and first 4 bytes (magic bytes), as extensions can be mislabeled:

| Format | Magic Bytes | Hex |
|--------|------------|-----|
| RAR | `Rar!` | `52 61 72 21` |
| ZIP | `PK\x03\x04` | `50 4B 03 04` |
| 7z | `7z\xBC\xAF` | `37 7A BC AF` |

Extension check is a fast pre-filter. Magic byte check is authoritative.

### Recursive extraction

```
extract(download_path, depth=0)
    |
    Extract all archives found at this level
    |
    Scan extracted directory for further archives
    |
    +-- depth < max_extraction_depth?
    |     extract(extracted_dir, depth+1)
    |
    +-- depth >= max_extraction_depth?
          Log error: nesting depth exceeded
          Emit DownloadFailed with NestingDepthExceeded reason
          Stop recursion
```

`max_extraction_depth` default: 3. This prevents zip bomb attacks and infinite recursion from pathological archive nesting.

---

## Extraction output

Extracted files land in a per-download temp directory:

```
{config.ergasia.extraction_temp_dir}/{download_id}/
```

`extraction_temp_dir` defaults to `{download_dir}/.extraction/`.

The `extracted_path` from `ExtractionResult` replaces the original `download_path` in the `CompletedDownload` struct passed to Kathodos for import.

**Cleanup policy:**

- After successful Kathodos import: delete `{extraction_temp_dir}/{download_id}/` immediately
- After failed import: retain for debugging. A periodic janitor task deletes directories older than `extraction_cleanup_hours` (default: 48 hours)
- Archives in the original download path are never deleted by Ergasia; that is Kathodos's responsibility after hardlink/move

---

## Disk space pre-check

Before any extraction begins:

1. Calculate `archive_size` (sum of all archive file sizes in the download directory)
2. Check available space on the extraction temp dir filesystem
3. Required space: `archive_size * 1.1` (10% overhead for filesystem metadata)
4. If insufficient: emit `ErgasiaError::InsufficientDiskSpace`, do not attempt partial extraction

For Usenet downloads: extraction may follow PAR2 repair, which also requires temporary space. The space check accounts for the post-repair file sizes, not the original segment sizes.

---

## ExtractionResult type

```rust
pub struct ExtractionResult {
    pub extracted_path: PathBuf,
    pub files: Vec<ExtractedFile>,
    pub archive_format: ArchiveFormat,
    pub nested_levels: u8,
}

pub enum ArchiveFormat {
    Rar,
    Zip,
    SevenZip,
}

pub struct ExtractedFile {
    pub path: PathBuf,
    pub size_bytes: u64,
}
```

`nested_levels` tracks how many recursion levels were required. A value of 0 means a flat archive (no nesting). Used for logging and diagnostics.

---

## Error handling

`ErgasiaError` variants for archive extraction:

```rust
#[derive(Debug, Snafu)]
pub enum ErgasiaError {
    #[snafu(display("Failed to open archive at {path}"))]
    OpenArchive {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to extract file {path}"))]
    ExtractFile {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Nested archive depth {depth} exceeds maximum {max}"))]
    NestingDepthExceeded {
        depth: u8,
        max: u8,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Insufficient disk space: need {needed} bytes, have {available} bytes"))]
    InsufficientDiskSpace {
        needed: u64,
        available: u64,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported archive format at {path}: magic bytes {magic_bytes:02X?}"))]
    UnsupportedFormat {
        path: PathBuf,
        magic_bytes: [u8; 4],
        #[snafu(implicit)]
        location: Location,
    },
}
```

---

## Horismos configuration

`[ergasia]` additions in `harmonia.toml`:

```toml
[ergasia]
# Temporary directory for archive extraction. Defaults to {download_dir}/.extraction/
# Must be on the same filesystem as download_dir for efficient cleanup.
extraction_temp_dir = "/data/downloads/.extraction"

# Maximum nested archive depth. Prevents zip bombs and infinite recursion.
max_extraction_depth = 3

# How long to retain failed extraction directories before cleanup (hours).
extraction_cleanup_hours = 48
```
