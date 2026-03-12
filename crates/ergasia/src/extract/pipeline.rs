use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::error::{ErgasiaError, InsufficientDiskSpaceSnafu, NestingDepthExceededSnafu};
use crate::extract::detect::{ArchiveFormat, detect_archive_format, detect_by_magic_bytes};
use crate::extract::rar::{extract_rar, find_rar_first_volume};
use crate::extract::seven_zip::extract_7z;
use crate::extract::zip_extract::extract_zip;

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

pub fn extract_archives(
    download_path: &Path,
    output_dir: &Path,
    max_depth: u8,
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

    check_disk_space(download_path, output_dir)?;

    let first_format = archives[0].1;
    let mut all_files = Vec::new();

    for (archive_path, format) in &archives {
        let files = extract_single(archive_path, output_dir, *format)?;
        all_files.extend(files);
    }

    let nested_levels = handle_nested(output_dir, 1, max_depth, &mut all_files)?;

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

fn extract_single(
    archive_path: &Path,
    output_dir: &Path,
    format: ArchiveFormat,
) -> Result<Vec<ExtractedFile>, ErgasiaError> {
    match format {
        ArchiveFormat::Rar => extract_rar(archive_path, output_dir),
        ArchiveFormat::Zip => extract_zip(archive_path, output_dir),
        ArchiveFormat::SevenZip => extract_7z(archive_path, output_dir),
    }
}

fn handle_nested(
    dir: &Path,
    current_depth: u8,
    max_depth: u8,
    all_files: &mut Vec<ExtractedFile>,
) -> Result<u8, ErgasiaError> {
    let nested_archives = find_nested_archives(dir);
    if nested_archives.is_empty() {
        return Ok(current_depth.saturating_sub(1));
    }

    ensure!(
        current_depth < max_depth,
        NestingDepthExceededSnafu {
            depth: current_depth,
            max: max_depth,
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

    for (archive_path, format) in &nested_archives {
        let files = extract_single(archive_path, &nested_output, *format)?;
        all_files.extend(files);
    }

    handle_nested(&nested_output, current_depth + 1, max_depth, all_files)
}

fn find_nested_archives(dir: &Path) -> Vec<(PathBuf, ArchiveFormat)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut archives = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(format) = detect_by_magic_bytes(&path) {
                archives.push((path, format));
            }
        } else if path.is_dir() && path.file_name().map(|n| n != ".nested").unwrap_or(true) {
            archives.extend(find_nested_archives(&path));
        }
    }

    archives
}

fn check_disk_space(download_path: &Path, output_dir: &Path) -> Result<(), ErgasiaError> {
    let archive_size = calculate_archive_size(download_path);
    let needed = (archive_size as f64 * 1.1) as u64;

    let available = get_available_space(output_dir);

    ensure!(
        available >= needed,
        InsufficientDiskSpaceSnafu { needed, available }
    );

    Ok(())
}

fn calculate_archive_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };

    entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.is_file() && detect_archive_format(&path).is_some() {
                path.metadata().ok().map(|m| m.len())
            } else {
                None
            }
        })
        .sum()
}

fn get_available_space(path: &Path) -> u64 {
    let output = std::process::Command::new("df")
        .arg("--output=avail")
        .arg("-B1")
        .arg(path)
        .output()
        .ok();

    output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines()
                .nth(1)
                .and_then(|line| line.trim().parse::<u64>().ok())
        })
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::InsufficientDiskSpaceSnafu;
    use std::io::Write;

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

    #[test]
    fn extract_zip_archive_via_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        create_test_zip(
            &download_dir,
            "test.zip",
            &[("hello.txt", b"Hello!"), ("world.txt", b"World!")],
        );

        let result = extract_archives(&download_dir, &output_dir, 3)
            .unwrap()
            .unwrap();
        assert_eq!(result.archive_format, ArchiveFormat::Zip);
        assert_eq!(result.files.len(), 2);
        assert!(output_dir.join("hello.txt").exists());
        assert!(output_dir.join("world.txt").exists());
    }

    #[test]
    fn no_archives_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();
        std::fs::write(download_dir.join("readme.txt"), b"just a text file").unwrap();

        let result = extract_archives(&download_dir, &output_dir, 3).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn nested_zip_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let download_dir = dir.path().join("download");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&download_dir).unwrap();

        let inner_dir = dir.path().join("inner_staging");
        std::fs::create_dir_all(&inner_dir).unwrap();
        let inner_zip = create_test_zip(
            &inner_dir,
            "inner.zip",
            &[("nested_file.txt", b"I am nested")],
        );
        let inner_bytes = std::fs::read(&inner_zip).unwrap();

        let outer_path = download_dir.join("outer.zip");
        {
            let file = std::fs::File::create(&outer_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("inner.zip", options).unwrap();
            writer.write_all(&inner_bytes).unwrap();
            writer.finish().unwrap();
        }

        let result = extract_archives(&download_dir, &output_dir, 3)
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

    #[test]
    fn nesting_depth_exceeded() {
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

        let result = extract_archives(&download_dir, &output_dir, 2);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("depth"),
            "expected nesting depth error, got: {err}"
        );
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
}
