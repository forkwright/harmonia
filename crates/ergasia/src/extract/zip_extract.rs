use std::fs::File;
use std::path::Path;

use snafu::ensure;
use zip::ZipArchive;

use crate::error::{ErgasiaError, UnsafeArchiveEntrySnafu};

const S_IFMT: u32 = 0o170000;
const S_IFLNK: u32 = 0o120000;

pub fn extract_zip(archive_path: &Path, output_dir: &Path) -> Result<(), ErgasiaError> {
    let mut archive = open_zip(archive_path)?;

    ensure_safe_entries(&mut archive, archive_path)?;

    archive.extract(output_dir).map_err(|e| {
        crate::error::ExtractFileSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    Ok(())
}

pub(crate) fn declared_uncompressed_size(archive_path: &Path) -> Result<u64, ErgasiaError> {
    let mut archive = open_zip(archive_path)?;

    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(|e| {
            crate::error::OpenArchiveSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;
        total = total.saturating_add(entry.size());
    }
    Ok(total)
}

// NOTE: entry names use zip's cross-platform path semantics, so both Unix and
// Windows-style roots (including drive-letter prefixes) count as absolute.
fn is_absolute_entry_name(name: &str) -> bool {
    name.starts_with('/') || name.starts_with('\\') || name.get(1..2) == Some(":")
}

fn open_zip(archive_path: &Path) -> Result<ZipArchive<File>, ErgasiaError> {
    let file = File::open(archive_path).map_err(|e| {
        crate::error::OpenArchiveSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    ZipArchive::new(file).map_err(|e| {
        crate::error::OpenArchiveSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })
}

// SAFETY: rejects the whole archive before any write occurs, so a hostile
// archive is refused atomically instead of partially extracted.
fn ensure_safe_entries(
    archive: &mut ZipArchive<File>,
    archive_path: &Path,
) -> Result<(), ErgasiaError> {
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(|e| {
            crate::error::OpenArchiveSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

        let is_symlink = entry
            .unix_mode()
            .map(|mode| mode & S_IFMT == S_IFLNK)
            .unwrap_or(false);
        ensure!(
            !is_symlink,
            UnsafeArchiveEntrySnafu {
                archive: archive_path.to_path_buf(),
                entry: entry.name().to_string(),
                reason: "symlink entry".to_string(),
            }
        );

        // WHY: zip's enclosed_name() neutralizes a leading root by stripping it
        // rather than rejecting the entry, so absolute names need an explicit
        // check to refuse the archive outright.
        ensure!(
            !is_absolute_entry_name(entry.name()),
            UnsafeArchiveEntrySnafu {
                archive: archive_path.to_path_buf(),
                entry: entry.name().to_string(),
                reason: "absolute entry name".to_string(),
            }
        );

        ensure!(
            entry.enclosed_name().is_some(),
            UnsafeArchiveEntrySnafu {
                archive: archive_path.to_path_buf(),
                entry: entry.name().to_string(),
                reason: "path escapes the extraction root".to_string(),
            }
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn zip_options() -> zip::write::SimpleFileOptions {
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
    }

    fn replace_bytes(haystack: &mut [u8], needle: &[u8], replacement: &[u8]) {
        assert_eq!(needle.len(), replacement.len());
        let mut replaced = false;
        let mut i = 0;
        while i + needle.len() <= haystack.len() {
            if &haystack[i..i + needle.len()] == needle {
                haystack[i..i + needle.len()].copy_from_slice(replacement);
                replaced = true;
                i += needle.len();
            } else {
                i += 1;
            }
        }
        assert!(replaced, "needle not found in archive bytes");
    }

    fn assert_no_entries(dir: &Path) {
        let entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
        assert!(
            entries.is_empty(),
            "expected empty output dir, found {entries:?}"
        );
    }

    #[test]
    fn extract_zip_archive() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer.start_file("hello.txt", zip_options()).unwrap();
            writer.write_all(b"Hello, World!").unwrap();
            writer
                .start_file("subdir/nested.txt", zip_options())
                .unwrap();
            writer.write_all(b"Nested content").unwrap();
            writer.finish().unwrap();
        }

        extract_zip(&zip_path, &output_dir).unwrap();

        let extracted_hello = output_dir.join("hello.txt");
        assert!(extracted_hello.exists());
        assert_eq!(
            std::fs::read_to_string(&extracted_hello).unwrap(),
            "Hello, World!"
        );
        assert!(output_dir.join("subdir/nested.txt").exists());
    }

    #[test]
    fn declared_size_sums_entries() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");

        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer.start_file("a.txt", zip_options()).unwrap();
            writer.write_all(&[0u8; 100]).unwrap();
            writer.start_file("b.txt", zip_options()).unwrap();
            writer.write_all(&[0u8; 50]).unwrap();
            writer.finish().unwrap();
        }

        assert_eq!(declared_uncompressed_size(&zip_path).unwrap(), 150);
    }

    #[test]
    fn reject_symlink_entry() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer.start_file("benign.txt", zip_options()).unwrap();
            writer.write_all(b"decoy").unwrap();
            writer
                .add_symlink("link", "../../../etc/passwd", zip_options())
                .unwrap();
            writer.finish().unwrap();
        }

        let err = extract_zip(&zip_path, &output_dir).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::UnsafeArchiveEntry { .. }),
            "expected UnsafeArchiveEntry, got: {err}"
        );
        assert_no_entries(&output_dir);
    }

    #[test]
    fn reject_absolute_path_entry() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer.start_file("Xetc/evil.txt", zip_options()).unwrap();
            writer.write_all(b"absolute").unwrap();
            writer.finish().unwrap();
        }

        // The zip writer sanitizes names, so patch the placeholder into a
        // genuinely absolute entry name (same byte length keeps offsets valid).
        let mut bytes = std::fs::read(&zip_path).unwrap();
        replace_bytes(&mut bytes, b"Xetc/evil.txt", b"/etc/evil.txt");
        std::fs::write(&zip_path, &bytes).unwrap();

        let err = extract_zip(&zip_path, &output_dir).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::UnsafeArchiveEntry { .. }),
            "expected UnsafeArchiveEntry, got: {err}"
        );
        assert_no_entries(&output_dir);
    }

    #[test]
    fn reject_parent_traversal_entry() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer.start_file("../escape.txt", zip_options()).unwrap();
            writer.write_all(b"traversal").unwrap();
            writer.finish().unwrap();
        }

        let err = extract_zip(&zip_path, &output_dir).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::UnsafeArchiveEntry { .. }),
            "expected UnsafeArchiveEntry, got: {err}"
        );
        assert_no_entries(&output_dir);
        assert!(!dir.path().join("escape.txt").exists());
    }
}
