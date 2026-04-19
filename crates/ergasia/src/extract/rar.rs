use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::error::ErgasiaError;
use crate::extract::pipeline::ExtractedFile;

static MODERN_RAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.part(\d+)\.rar$")
        .unwrap_or_else(|e| unreachable!("regex literal is statically valid: {e}"))
});

pub fn find_rar_first_volume(dir: &Path) -> Option<PathBuf> {
    let entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("rar"))
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        return None;
    }

    let modern_first = entries
        .iter()
        .filter(|p| {
            p.to_str()
                .map(|s| MODERN_RAR_RE.is_match(s))
                .unwrap_or(false)
        })
        .min_by_key(|p| {
            p.to_str()
                .and_then(|s| MODERN_RAR_RE.captures(s))
                .and_then(|c| c.get(1))
                .and_then(|m| m.as_str().parse::<u32>().ok())
                .unwrap_or(u32::MAX)
        });

    if let Some(first) = modern_first {
        return Some(first.clone());
    }

    let first_rar = entries.iter().find(|p| {
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("rar"))
            .unwrap_or(false)
    })?;

    let stem = first_rar.file_stem()?.to_str()?;
    let r00_path = dir.join(format!("{stem}.r00"));
    if r00_path.exists() {
        return Some(first_rar.clone());
    }

    Some(first_rar.clone())
}

pub fn extract_rar(
    archive_path: &Path,
    output_dir: &Path,
) -> Result<Vec<ExtractedFile>, ErgasiaError> {
    let archive = unrar::Archive::new(archive_path)
        .open_for_processing()
        .map_err(|e| {
            crate::error::OpenArchiveSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

    let mut files = Vec::new();
    let mut cursor = archive;

    loop {
        let header_opt = cursor.read_header().map_err(|e| {
            crate::error::ExtractFileSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

        let Some(header) = header_opt else {
            break;
        };

        let entry = header.entry();
        let is_dir = entry.is_directory();
        let filename = entry.filename.clone();
        let size = entry.unpacked_size;

        let next = header.extract_with_base(output_dir).map_err(|e| {
            crate::error::ExtractFileSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

        if !is_dir {
            files.push(ExtractedFile {
                path: output_dir.join(&filename),
                size_bytes: size,
            });
        }

        cursor = next;
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn find_modern_part1_rar() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("movie.part1.rar"), b"Rar!placeholder").unwrap();
        fs::write(dir.path().join("movie.part2.rar"), b"Rar!placeholder").unwrap();
        fs::write(dir.path().join("movie.part3.rar"), b"Rar!placeholder").unwrap();

        let first = find_rar_first_volume(dir.path()).unwrap();
        assert!(
            first.to_str().unwrap().contains("part1.rar"),
            "expected part1.rar, got {:?}",
            first
        );
    }

    #[test]
    fn find_modern_part01_rar() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("album.part01.rar"), b"Rar!placeholder").unwrap();
        fs::write(dir.path().join("album.part02.rar"), b"Rar!placeholder").unwrap();

        let first = find_rar_first_volume(dir.path()).unwrap();
        assert!(
            first.to_str().unwrap().contains("part01.rar"),
            "expected part01.rar, got {:?}",
            first
        );
    }

    #[test]
    fn find_legacy_rar_with_r00() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("archive.rar"), b"Rar!placeholder").unwrap();
        fs::write(dir.path().join("archive.r00"), b"placeholder").unwrap();
        fs::write(dir.path().join("archive.r01"), b"placeholder").unwrap();

        let first = find_rar_first_volume(dir.path()).unwrap();
        assert!(
            first
                .extension()
                .unwrap()
                .to_str()
                .unwrap()
                .eq_ignore_ascii_case("rar"),
            "expected .rar extension, got {:?}",
            first
        );
    }

    #[test]
    fn find_single_rar_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("single.rar"), b"Rar!placeholder").unwrap();

        let first = find_rar_first_volume(dir.path()).unwrap();
        assert!(first.to_str().unwrap().contains("single.rar"));
    }

    #[test]
    fn no_rar_files_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), b"not an archive").unwrap();

        assert!(find_rar_first_volume(dir.path()).is_none());
    }
}
