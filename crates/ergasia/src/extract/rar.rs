use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::error::ErgasiaError;

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

pub fn extract_rar(archive_path: &Path, output_dir: &Path) -> Result<(), ErgasiaError> {
    let archive = unrar::Archive::new(archive_path)
        .open_for_processing()
        .map_err(|e| {
            crate::error::OpenArchiveSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

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

        let next = header.extract_with_base(output_dir).map_err(|e| {
            crate::error::ExtractFileSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

        cursor = next;
    }

    Ok(())
}

pub(crate) fn declared_uncompressed_size(archive_path: &Path) -> Result<u64, ErgasiaError> {
    let archive = unrar::Archive::new(archive_path)
        .open_for_listing()
        .map_err(|e| {
            crate::error::OpenArchiveSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

    let mut total: u64 = 0;
    for header in archive {
        let entry = header.map_err(|e| {
            crate::error::OpenArchiveSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;
        total = total.saturating_add(entry.unpacked_size);
    }
    Ok(total)
}

// WHY: a multi-volume RAR declares the unpacked size of the whole set, so the
// ratio guard must compare against the on-disk size of every volume, not just
// the first one.
pub(crate) fn volume_set_size(first_volume: &Path) -> u64 {
    let Some(dir) = first_volume.parent() else {
        return first_volume.metadata().map(|m| m.len()).unwrap_or(0);
    };

    let Ok(entries) = std::fs::read_dir(dir) else {
        return first_volume.metadata().map(|m| m.len()).unwrap_or(0);
    };

    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_rar_volume(p))
        .filter_map(|p| p.metadata().ok().map(|m| m.len()))
        .fold(0u64, |total, len| total.saturating_add(len))
}

fn is_rar_volume(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            ext == "rar"
                || (ext.len() == 3
                    && ext.starts_with('r')
                    && ext.chars().skip(1).all(|c| c.is_ascii_digit()))
        })
        .unwrap_or(false)
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

    // NOTE: extract_rar's success path has no test — RAR compression is
    // proprietary (unrar is extract-only, no OSS encoder exists), so a fixture
    // archive cannot be generated at test time. Error paths are covered below.

    #[test]
    fn extract_rar_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let err = extract_rar(&dir.path().join("nonexistent.rar"), &output_dir).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::OpenArchive { .. }),
            "expected OpenArchive for a missing file, got: {err}"
        );
    }

    #[test]
    fn extract_rar_corrupt_archive_errors() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();
        let rar_path = dir.path().join("corrupt.rar");
        fs::write(&rar_path, b"Rar!\x1a\x07\x00 not actually a valid archive").unwrap();

        let err = extract_rar(&rar_path, &output_dir).unwrap_err();
        assert!(
            matches!(
                err,
                ErgasiaError::OpenArchive { .. } | ErgasiaError::ExtractFile { .. }
            ),
            "expected OpenArchive or ExtractFile for a corrupt archive, got: {err}"
        );
        let leftovers: Vec<_> = fs::read_dir(&output_dir).unwrap().flatten().collect();
        assert!(
            leftovers.is_empty(),
            "corrupt archive must not produce output: {leftovers:?}"
        );
    }

    #[test]
    fn declared_uncompressed_size_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = declared_uncompressed_size(&dir.path().join("nonexistent.rar")).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::OpenArchive { .. }),
            "expected OpenArchive, got: {err}"
        );
    }
}
