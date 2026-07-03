use std::path::{Path, PathBuf};

use horismos::ErgasiaConfig;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::error::{
    DecompressionRatioExceededSnafu, DiskSpaceQuerySnafu, ErgasiaError, ExtractionJoinSnafu,
    InsufficientDiskSpaceSnafu, NestingDepthExceededSnafu,
};
use crate::extract::detect::{ArchiveFormat, detect_archive_format, detect_by_magic_bytes};
use crate::extract::rar::{extract_rar, find_rar_first_volume};
use crate::extract::seven_zip::extract_7z;
use crate::extract::zip_extract::extract_zip;
use crate::extract::{fs_walk, rar, seven_zip, zip_extract};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub extracted_path: PathBuf,
    pub files: Vec<ExtractedFile>,
    pub archive_format: ArchiveFormat,
    pub nested_levels: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFile {
    pub path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ExtractionLimits {
    pub max_depth: u8,
    pub max_decompression_ratio: f64,
}

impl From<&ErgasiaConfig> for ExtractionLimits {
    fn from(config: &ErgasiaConfig) -> Self {
        Self {
            max_depth: config.max_extraction_depth,
            max_decompression_ratio: config.max_decompression_ratio,
        }
    }
}

// WHY: the whole pipeline (magic-byte reads, archive decompression, directory
// walks) is blocking I/O; one spawn_blocking boundary here keeps every format
// backend off the async executor instead of sprinkling wrappers per call site.
pub async fn extract_archives(
    download_path: &Path,
    output_dir: &Path,
    limits: ExtractionLimits,
) -> Result<Option<ExtractionResult>, ErgasiaError> {
    let download_path = download_path.to_path_buf();
    let output_dir = output_dir.to_path_buf();

    tokio::task::spawn_blocking(move || {
        extract_archives_blocking(&download_path, &output_dir, limits)
    })
    .await
    .context(ExtractionJoinSnafu)?
}

fn extract_archives_blocking(
    download_path: &Path,
    output_dir: &Path,
    limits: ExtractionLimits,
) -> Result<Option<ExtractionResult>, ErgasiaError> {
    let archives = find_archives_in_dir(download_path);
    if archives.is_empty() {
        return Ok(None);
    }

    std::fs::create_dir_all(output_dir).map_err(|e| {
        crate::error::ExtractFileSnafu {
            path: output_dir.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    preflight_archives(&archives, output_dir, limits.max_decompression_ratio)?;

    let Some((_, first_format)) = archives.first() else {
        return Ok(None);
    };
    let first_format = *first_format;
    let mut all_files = Vec::new();

    for (archive_path, format) in &archives {
        let files = extract_single(
            archive_path,
            output_dir,
            *format,
            limits.max_decompression_ratio,
        )?;
        all_files.extend(files);
    }

    let nested_levels = handle_nested(output_dir, 1, limits, &mut all_files)?;

    Ok(Some(ExtractionResult {
        extracted_path: output_dir.to_path_buf(),
        files: all_files,
        archive_format: first_format,
        nested_levels,
    }))
}

fn find_archives_in_dir(dir: &Path) -> Vec<(PathBuf, ArchiveFormat)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut archives = Vec::new();
    let mut seen_rar = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(format) = detect_archive_format(&path) {
            match format {
                ArchiveFormat::Rar => {
                    if !seen_rar && let Some(first_vol) = find_rar_first_volume(dir) {
                        archives.push((first_vol, ArchiveFormat::Rar));
                        seen_rar = true;
                    }
                }
                _ => {
                    archives.push((path, format));
                }
            }
        }
    }

    archives
}

// WHY: inventory is diffed against a pre-extraction snapshot so reported paths
// always match the sanitized on-disk write locations, and files from earlier
// archives sharing the output dir are never double-counted.
fn extract_single(
    archive_path: &Path,
    output_dir: &Path,
    format: ArchiveFormat,
    max_ratio: f64,
) -> Result<Vec<ExtractedFile>, ErgasiaError> {
    let before = fs_walk::snapshot_paths(output_dir);

    match format {
        ArchiveFormat::Rar => extract_rar(archive_path, output_dir)?,
        ArchiveFormat::Zip => extract_zip(archive_path, output_dir)?,
        ArchiveFormat::SevenZip => extract_7z(archive_path, output_dir, max_ratio)?,
    }

    let mut files = Vec::new();
    fs_walk::collect_files_excluding(output_dir, &before, &mut files);

    // WHY: unrar controls its own writes, so RAR cannot use the streaming byte
    // cap the zip/7z backends enforce. Instead the real bytes this archive
    // produced are checked post-hoc against the same declared×ratio cap and
    // rolled back if a header/payload-mismatch bomb slipped past the pre-flight
    // ratio guard. Best effort: the bytes touch disk transiently before removal.
    if format == ArchiveFormat::Rar {
        let declared = rar::declared_uncompressed_size(archive_path)?;
        let cap = extraction_byte_cap(declared, max_ratio);
        let produced: u64 = files.iter().map(|f| f.size_bytes).sum();
        if produced > cap {
            for file in &files {
                if let Err(err) = std::fs::remove_file(&file.path) {
                    tracing::warn!(
                        path = %file.path.display(),
                        %err,
                        "failed to remove RAR extraction output during bomb rollback"
                    );
                }
            }
            return Err(DecompressionRatioExceededSnafu {
                archive: archive_path.to_path_buf(),
                compressed: rar::volume_set_size(archive_path),
                declared_uncompressed: produced,
                max_ratio,
            }
            .build());
        }
    }

    Ok(files)
}

// WHY: a header/payload-mismatch bomb declares a small uncompressed size but
// streams far more; capping real output at declared×ratio bounds the damage to
// the same policy the pre-flight ratio guard enforces on declared sizes.
pub(crate) fn extraction_byte_cap(declared_uncompressed: u64, max_ratio: f64) -> u64 {
    if !max_ratio.is_finite() || max_ratio <= 0.0 {
        return 0;
    }
    let scaled = declared_uncompressed as f64 * max_ratio;
    if !scaled.is_finite() || scaled >= u64::MAX as f64 {
        return u64::MAX;
    }
    scaled.ceil() as u64
}

fn handle_nested(
    dir: &Path,
    current_depth: u8,
    limits: ExtractionLimits,
    all_files: &mut Vec<ExtractedFile>,
) -> Result<u8, ErgasiaError> {
    let nested_archives = find_nested_archives(dir);
    if nested_archives.is_empty() {
        return Ok(current_depth.saturating_sub(1));
    }

    ensure!(
        current_depth < limits.max_depth,
        NestingDepthExceededSnafu {
            depth: current_depth,
            max: limits.max_depth,
        }
    );

    let nested_output = dir.join(".nested");
    std::fs::create_dir_all(&nested_output).map_err(|e| {
        crate::error::ExtractFileSnafu {
            path: nested_output.clone(),
            error: e.to_string(),
        }
        .build()
    })?;

    preflight_archives(
        &nested_archives,
        &nested_output,
        limits.max_decompression_ratio,
    )?;

    for (archive_path, format) in &nested_archives {
        let files = extract_single(
            archive_path,
            &nested_output,
            *format,
            limits.max_decompression_ratio,
        )?;
        all_files.extend(files);
    }

    handle_nested(&nested_output, current_depth + 1, limits, all_files)
}

fn find_nested_archives(dir: &Path) -> Vec<(PathBuf, ArchiveFormat)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut archives = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        // SAFETY: file_type() does not follow symlinks (unlike Path::is_file /
        // is_dir), so a reified attacker symlink is classified as neither a file
        // nor a directory and cannot redirect recursion outside output_dir.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() {
            if let Some(format) = detect_by_magic_bytes(&path) {
                archives.push((path, format));
            }
        } else if file_type.is_dir() && path.file_name().map(|n| n != ".nested").unwrap_or(true) {
            archives.extend(find_nested_archives(&path));
        }
    }

    archives
}

// WHY: both guards run before any extraction write. The ratio guard is a policy
// check distinct from disk-space sufficiency: a decompression bomb can pass the
// space check on a large disk and still be hostile.
fn preflight_archives(
    archives: &[(PathBuf, ArchiveFormat)],
    output_dir: &Path,
    max_ratio: f64,
) -> Result<(), ErgasiaError> {
    let mut total_declared: u64 = 0;
    for (archive_path, format) in archives {
        let declared = enforce_decompression_ratio(archive_path, *format, max_ratio)?;
        total_declared = total_declared.saturating_add(declared);
    }

    let needed = needed_with_headroom(total_declared);
    let available = get_available_space(output_dir)?;

    ensure!(
        available >= needed,
        InsufficientDiskSpaceSnafu { needed, available }
    );

    Ok(())
}

fn enforce_decompression_ratio(
    archive_path: &Path,
    format: ArchiveFormat,
    max_ratio: f64,
) -> Result<u64, ErgasiaError> {
    let declared = declared_uncompressed_size(archive_path, format)?;
    let compressed = compressed_size_on_disk(archive_path, format)?;

    ensure!(
        declared as f64 <= compressed as f64 * max_ratio,
        DecompressionRatioExceededSnafu {
            archive: archive_path.to_path_buf(),
            compressed,
            declared_uncompressed: declared,
            max_ratio,
        }
    );

    Ok(declared)
}

fn declared_uncompressed_size(
    archive_path: &Path,
    format: ArchiveFormat,
) -> Result<u64, ErgasiaError> {
    match format {
        ArchiveFormat::Rar => rar::declared_uncompressed_size(archive_path),
        ArchiveFormat::Zip => zip_extract::declared_uncompressed_size(archive_path),
        ArchiveFormat::SevenZip => seven_zip::declared_uncompressed_size(archive_path),
    }
}

fn compressed_size_on_disk(
    archive_path: &Path,
    format: ArchiveFormat,
) -> Result<u64, ErgasiaError> {
    if format == ArchiveFormat::Rar {
        return Ok(rar::volume_set_size(archive_path));
    }

    archive_path.metadata().map(|m| m.len()).map_err(|e| {
        crate::error::OpenArchiveSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })
}

fn needed_with_headroom(total_declared: u64) -> u64 {
    total_declared.saturating_add(total_declared / 10)
}

fn get_available_space(path: &Path) -> Result<u64, ErgasiaError> {
    let stat = rustix::fs::statvfs(path).map_err(|e| {
        DiskSpaceQuerySnafu {
            path: path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::error::InsufficientDiskSpaceSnafu;

    const TEST_LIMITS: ExtractionLimits = ExtractionLimits {
        max_depth: 3,
        max_decompression_ratio: 100.0,
    };

    fn create_test_zip(dir: &Path, name: &str, contents: &[(&str, &[u8])]) -> PathBuf {
        let zip_path = dir.join(name);
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (filename, data) in contents {
            writer.start_file(filename.to_string(), options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
        zip_path
    }

    fn create_deflated_bomb_zip(dir: &Path, name: &str, uncompressed_len: usize) -> PathBuf {
        let zip_path = dir.join(name);
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer.start_file("bomb.bin", options).unwrap();
        writer.write_all(&vec![0u8; uncompressed_len]).unwrap();
        writer.finish().unwrap();
        zip_path
    }

    #[tokio::test]
    async fn extract_zip_archive_via_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        create_test_zip(
            &download_dir,
            "test.zip",
            &[("hello.txt", b"Hello!"), ("world.txt", b"World!")],
        );

        let result = extract_archives(&download_dir, &output_dir, TEST_LIMITS)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.archive_format, ArchiveFormat::Zip);
        assert_eq!(result.files.len(), 2);
        assert!(output_dir.join("hello.txt").exists());
        assert!(output_dir.join("world.txt").exists());
    }

    #[tokio::test]
    async fn no_archives_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();
        std::fs::write(download_dir.join("readme.txt"), b"just a text file").unwrap();

        let result = extract_archives(&download_dir, &output_dir, TEST_LIMITS)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn nested_zip_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        let inner_dir = dir.path().join("inner_staging");
        std::fs::create_dir_all(&inner_dir).unwrap();
        let inner_zip = create_test_zip(
            &inner_dir,
            "INNER.zip",
            &[("nested_file.txt", b"I am nested")],
        );
        let inner_bytes = std::fs::read(&inner_zip).unwrap();

        let outer_path = download_dir.join("OUTER.zip");
        {
            let file = std::fs::File::create(&outer_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("INNER.zip", options).unwrap();
            writer.write_all(&inner_bytes).unwrap();
            writer.finish().unwrap();
        }

        let result = extract_archives(&download_dir, &output_dir, TEST_LIMITS)
            .await
            .unwrap()
            .unwrap();
        assert!(result.nested_levels >= 1);
        assert!(
            result
                .files
                .iter()
                .any(|f| f.path.to_str().unwrap().contains("nested_file.txt"))
        );
    }

    #[tokio::test]
    async fn nesting_depth_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        let inner_dir = dir.path().join("staging");
        std::fs::create_dir_all(&inner_dir).unwrap();

        let mut current_content = b"deepest content".to_vec();
        for i in 0..4 {
            let zip_path = inner_dir.join(format!("level{i}.zip"));
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let inner_name = if i == 0 {
                "data.txt".to_string()
            } else {
                format!("level{}.zip", i - 1)
            };
            writer.start_file(&inner_name, options).unwrap();
            writer.write_all(&current_content).unwrap();
            writer.finish().unwrap();
            current_content = std::fs::read(&zip_path).unwrap();
        }

        let outer_path = download_dir.join("deep.zip");
        {
            let file = std::fs::File::create(&outer_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("level3.zip", options).unwrap();
            writer.write_all(&current_content).unwrap();
            writer.finish().unwrap();
        }

        let limits = ExtractionLimits {
            max_depth: 2,
            max_decompression_ratio: 100.0,
        };
        let result = extract_archives(&download_dir, &output_dir, limits).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("depth"),
            "expected nesting depth error, got: {err}"
        );
    }

    #[tokio::test]
    async fn reject_archive_exceeding_decompression_ratio() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        // 1 MiB of zeros deflates to a few KiB: the declared/compressed ratio
        // far exceeds the 10x test limit.
        create_deflated_bomb_zip(&download_dir, "bomb.zip", 1024 * 1024);

        let limits = ExtractionLimits {
            max_depth: 3,
            max_decompression_ratio: 10.0,
        };
        let err = extract_archives(&download_dir, &output_dir, limits)
            .await
            .unwrap_err();
        assert!(
            matches!(err, ErgasiaError::DecompressionRatioExceeded { .. }),
            "expected DecompressionRatioExceeded, got: {err}"
        );
        let leftovers: Vec<_> = std::fs::read_dir(&output_dir).unwrap().flatten().collect();
        assert!(
            leftovers.is_empty(),
            "expected no extraction output, found {leftovers:?}"
        );
    }

    #[tokio::test]
    async fn reject_nested_bomb() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        // The outer zip stores the bomb uncompressed (ratio ~1x), so only the
        // nested pre-flight can catch the inner high-ratio archive.
        let staging = dir.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let bomb = create_deflated_bomb_zip(&staging, "inner.zip", 1024 * 1024);
        let bomb_bytes = std::fs::read(&bomb).unwrap();

        let outer_path = download_dir.join("outer.zip");
        {
            let file = std::fs::File::create(&outer_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("inner.zip", options).unwrap();
            writer.write_all(&bomb_bytes).unwrap();
            writer.finish().unwrap();
        }

        let limits = ExtractionLimits {
            max_depth: 3,
            max_decompression_ratio: 10.0,
        };
        let err = extract_archives(&download_dir, &output_dir, limits)
            .await
            .unwrap_err();
        assert!(
            matches!(err, ErgasiaError::DecompressionRatioExceeded { .. }),
            "expected DecompressionRatioExceeded for nested bomb, got: {err}"
        );
        assert!(
            !output_dir.join(".nested").join("bomb.bin").exists(),
            "nested bomb payload must not be extracted"
        );
    }

    #[tokio::test]
    async fn multi_archive_inventory_not_double_counted() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        create_test_zip(&download_dir, "a.zip", &[("first.txt", b"one")]);
        create_test_zip(&download_dir, "b.zip", &[("second.txt", b"two")]);

        let result = extract_archives(&download_dir, &output_dir, TEST_LIMITS)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            result.files.len(),
            2,
            "each extracted file must be inventoried exactly once: {:?}",
            result.files
        );
    }

    #[tokio::test]
    async fn inventory_paths_match_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        // Entry names with redundant components that extraction normalizes.
        create_test_zip(
            &download_dir,
            "messy.zip",
            &[("a/./b.txt", b"dot component"), ("c//d.txt", b"empty seg")],
        );

        let result = extract_archives(&download_dir, &output_dir, TEST_LIMITS)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result.files.len(), 2);
        for file in &result.files {
            let meta = file.path.symlink_metadata().unwrap_or_else(|_| {
                panic!(
                    "inventory path does not exist on disk: {}",
                    file.path.display()
                )
            });
            assert_eq!(
                meta.len(),
                file.size_bytes,
                "size mismatch for {}",
                file.path.display()
            );
        }
    }

    // WHY: with a single worker thread, a synchronous extraction inside the
    // async fn would never yield between the counter snapshot and completion,
    // freezing the ticker at zero progress; the spawn_blocking boundary yields
    // to the executor, so the ticker must advance.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn extract_does_not_block_executor() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();
        let entries: Vec<(String, Vec<u8>)> = (0..200)
            .map(|i| (format!("file_{i}.txt"), vec![b'x'; 4096]))
            .collect();
        let entry_refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(name, data)| (name.as_str(), data.as_slice()))
            .collect();
        create_test_zip(&download_dir, "many.zip", &entry_refs);

        let counter = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let ticker_counter = Arc::clone(&counter);
        let ticker_stop = Arc::clone(&stop);
        let ticker = tokio::spawn(async move {
            while !ticker_stop.load(Ordering::Relaxed) {
                ticker_counter.fetch_add(1, Ordering::Relaxed);
                tokio::task::yield_now().await;
            }
        });

        let before = counter.load(Ordering::Relaxed);
        let result = extract_archives(&download_dir, &output_dir, TEST_LIMITS).await;
        let after = counter.load(Ordering::Relaxed);
        stop.store(true, Ordering::Relaxed);
        ticker.await.unwrap();

        assert!(result.unwrap().is_some());
        assert!(
            after > before,
            "executor made no progress while extraction ran: before={before} after={after}"
        );
    }

    #[test]
    fn disk_space_query_failure_propagates() {
        let err =
            get_available_space(Path::new("/nonexistent-harmonia-test-path/child")).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::DiskSpaceQuery { .. }),
            "expected DiskSpaceQuery, got: {err}"
        );
    }

    #[test]
    fn preflight_propagates_disk_space_query_failure() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = create_test_zip(dir.path(), "test.zip", &[("a.txt", b"data")]);

        let err = preflight_archives(
            &[(zip_path, ArchiveFormat::Zip)],
            Path::new("/nonexistent-harmonia-test-path/child"),
            100.0,
        )
        .unwrap_err();
        assert!(
            matches!(err, ErgasiaError::DiskSpaceQuery { .. }),
            "expected DiskSpaceQuery, got: {err}"
        );
    }

    #[test]
    fn needed_with_headroom_saturates() {
        assert_eq!(needed_with_headroom(0), 0);
        assert_eq!(needed_with_headroom(100), 110);
        assert_eq!(needed_with_headroom(u64::MAX), u64::MAX);
    }

    #[test]
    fn insufficient_disk_space_detected() {
        let err: ErgasiaError = InsufficientDiskSpaceSnafu {
            needed: 1_000_000_000_000u64,
            available: 100u64,
        }
        .build();
        assert!(err.to_string().contains("insufficient disk space"));
    }

    #[test]
    fn extraction_result_serde_roundtrip() {
        let result = ExtractionResult {
            extracted_path: PathBuf::from("/tmp/extract"),
            files: vec![
                ExtractedFile {
                    path: PathBuf::from("/tmp/extract/file1.txt"),
                    size_bytes: 1024,
                },
                ExtractedFile {
                    path: PathBuf::from("/tmp/extract/file2.flac"),
                    size_bytes: 50_000_000,
                },
            ],
            archive_format: ArchiveFormat::Zip,
            nested_levels: 0,
        };
        let json = serde_json::to_string(&result).unwrap();
        let recovered: ExtractionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.files.len(), 2);
        assert_eq!(recovered.archive_format, ArchiveFormat::Zip);
        assert_eq!(recovered.nested_levels, 0);
    }

    #[test]
    fn extraction_byte_cap_bounds_and_saturates() {
        assert_eq!(extraction_byte_cap(0, 100.0), 0);
        assert_eq!(extraction_byte_cap(100, 10.0), 1000);
        assert_eq!(extraction_byte_cap(5, 1.0), 5);
        // Non-positive / non-finite ratios collapse to a zero cap.
        assert_eq!(extraction_byte_cap(100, 0.0), 0);
        assert_eq!(extraction_byte_cap(100, -1.0), 0);
        assert_eq!(extraction_byte_cap(100, f64::NAN), 0);
        // Overflow saturates instead of wrapping.
        assert_eq!(extraction_byte_cap(u64::MAX, 2.0), u64::MAX);
    }

    #[test]
    #[cfg(unix)]
    fn find_nested_archives_does_not_follow_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        std::fs::create_dir_all(&output).unwrap();

        // A genuine archive directly inside the walked tree is found.
        create_test_zip(&output, "real.zip", &[("a.txt", b"y")]);

        // An attacker symlink inside output pointing at an external directory
        // that holds another archive must NOT be followed.
        let external = dir.path().join("external");
        std::fs::create_dir_all(&external).unwrap();
        create_test_zip(&external, "outside.zip", &[("secret.txt", b"x")]);
        std::os::unix::fs::symlink(&external, output.join("link")).unwrap();

        let found = find_nested_archives(&output);
        assert!(
            found.iter().any(|(p, _)| p.ends_with("real.zip")),
            "real archive missing: {found:?}"
        );
        assert!(
            found.iter().all(|(p, _)| !p.starts_with(&external)),
            "symlink was followed outside the extraction root: {found:?}"
        );
    }
}
