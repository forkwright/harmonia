use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use sevenz_rust2::ArchiveEntry;

use crate::error::{
    DecompressionRatioExceededSnafu, ErgasiaError, ExtractFileSnafu, UnsafeArchiveEntrySnafu,
};
use crate::extract::pipeline::extraction_byte_cap;

// Windows file attributes carried by 7z entries.
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const FILE_ATTRIBUTE_UNIX_EXTENSION: u32 = 0x8000;

// Unix st_mode format field (present in the high 16 bits when the unix
// extension flag is set).
const S_IFMT: u32 = 0o170000;
const S_IFREG: u32 = 0o100000;
const S_IFDIR: u32 = 0o040000;
const S_IFLNK: u32 = 0o120000;

pub fn extract_7z(
    archive_path: &Path,
    output_dir: &Path,
    max_ratio: f64,
) -> Result<(), ErgasiaError> {
    // SAFETY: reject the whole archive before any write if any entry has an
    // absolute or parent-traversal name or is a symlink/special file — the
    // sevenz_rust2 default extractor does dest.join(name) + File::create with no
    // such check. Mirrors zip ensure_safe_entries.
    let listing = sevenz_rust2::Archive::open(archive_path).map_err(|e| {
        crate::error::OpenArchiveSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;
    for entry in &listing.files {
        if let Some(reason) = unsafe_entry_reason(entry) {
            return Err(UnsafeArchiveEntrySnafu {
                archive: archive_path.to_path_buf(),
                entry: entry.name().to_string(),
                reason: reason.to_string(),
            }
            .build());
        }
    }

    let declared = declared_uncompressed_size(archive_path)?;
    let compressed = archive_path.metadata().map(|m| m.len()).unwrap_or(0);
    let mut guard = ExtractGuard {
        archive_path,
        output_dir,
        cap: extraction_byte_cap(declared, max_ratio),
        compressed,
        max_ratio,
        written_total: 0,
        created: Vec::new(),
    };

    // WHY: the closure returns sevenz_rust2::Error, so the real ErgasiaError is
    // captured out-of-band and a sentinel is returned to abort the stream.
    let mut captured: Option<ErgasiaError> = None;
    let result = sevenz_rust2::decompress_file_with_extract_fn(
        archive_path,
        output_dir,
        |entry, reader, _dest| match guard.write_entry(entry, reader) {
            Ok(keep_going) => Ok(keep_going),
            Err(err) => {
                captured = Some(err);
                Err(sevenz_rust2::Error::from(std::io::Error::other(
                    "extraction aborted",
                )))
            }
        },
    );

    if let Some(err) = captured {
        guard.cleanup();
        return Err(err);
    }
    result.map_err(|e| {
        guard.cleanup();
        ExtractFileSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    Ok(())
}

// Tracks per-archive extraction state so a byte-cap breach mid-stream can roll
// back exactly the files this extraction created.
struct ExtractGuard<'a> {
    archive_path: &'a Path,
    output_dir: &'a Path,
    cap: u64,
    compressed: u64,
    max_ratio: f64,
    written_total: u64,
    created: Vec<PathBuf>,
}

impl ExtractGuard<'_> {
    fn write_entry(
        &mut self,
        entry: &ArchiveEntry,
        reader: &mut dyn Read,
    ) -> Result<bool, ErgasiaError> {
        // Defense in depth: the closure's dest is attacker-influenced, so
        // re-validate and rebuild the path from the vetted output root.
        if let Some(reason) = unsafe_entry_reason(entry) {
            return Err(UnsafeArchiveEntrySnafu {
                archive: self.archive_path.to_path_buf(),
                entry: entry.name().to_string(),
                reason: reason.to_string(),
            }
            .build());
        }

        let dest = self.output_dir.join(entry.name());
        if !dest.starts_with(self.output_dir) {
            return Err(UnsafeArchiveEntrySnafu {
                archive: self.archive_path.to_path_buf(),
                entry: entry.name().to_string(),
                reason: "path escapes the extraction root".to_string(),
            }
            .build());
        }

        if entry.is_directory() {
            std::fs::create_dir_all(&dest).map_err(|e| self.extract_err(&dest, e))?;
            return Ok(true);
        }

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| self.extract_err(parent, e))?;
        }

        let file = File::create(&dest).map_err(|e| self.extract_err(&dest, e))?;
        self.created.push(dest.clone());
        let mut writer = BufWriter::new(file);

        // WHY: cap the real bytes this entry may write to the remaining cap
        // headroom (+1 so an over-long stream lands exactly one byte past the
        // cap and is detected), catching a header/payload-mismatch bomb before
        // it fully materializes on disk.
        let remaining = self.cap.saturating_sub(self.written_total);
        let mut limited = reader.take(remaining.saturating_add(1));
        let copied =
            std::io::copy(&mut limited, &mut writer).map_err(|e| self.extract_err(&dest, e))?;
        self.written_total = self.written_total.saturating_add(copied);
        if self.written_total > self.cap {
            return Err(DecompressionRatioExceededSnafu {
                archive: self.archive_path.to_path_buf(),
                compressed: self.compressed,
                declared_uncompressed: self.written_total,
                max_ratio: self.max_ratio,
            }
            .build());
        }
        writer.flush().map_err(|e| self.extract_err(&dest, e))?;
        Ok(true)
    }

    fn extract_err(&self, path: &Path, error: std::io::Error) -> ErgasiaError {
        ExtractFileSnafu {
            path: path.to_path_buf(),
            error: error.to_string(),
        }
        .build()
    }

    // Best-effort rollback of files written before an abort.
    fn cleanup(&self) {
        for path in &self.created {
            if let Err(err) = std::fs::remove_file(path) {
                tracing::warn!(
                    path = %path.display(),
                    %err,
                    "failed to remove partial 7z extraction output during rollback"
                );
            }
        }
    }
}

fn unsafe_entry_reason(entry: &ArchiveEntry) -> Option<&'static str> {
    // NOTE: an empty name maps to the extraction root itself (7z stores a
    // root-directory entry with an empty name); it cannot traverse, so only
    // absolute, parent-traversal, and non-regular entries are refused.
    let name = entry.name();
    if name.starts_with('/') || name.starts_with('\\') || name.get(1..2) == Some(":") {
        return Some("absolute entry path");
    }
    for segment in name.split(['/', '\\']) {
        if segment == ".." {
            return Some("parent directory traversal");
        }
    }

    if !entry.is_directory() {
        let attr = entry.windows_attributes;
        if attr & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Some("reparse point / symlink entry");
        }
        if attr & FILE_ATTRIBUTE_UNIX_EXTENSION != 0 {
            let unix_type = (attr >> 16) & S_IFMT;
            if unix_type == S_IFLNK {
                return Some("symlink entry");
            }
            if unix_type != 0 && unix_type != S_IFREG && unix_type != S_IFDIR {
                return Some("non-regular file entry");
            }
        }
    }

    None
}

pub(crate) fn declared_uncompressed_size(archive_path: &Path) -> Result<u64, ErgasiaError> {
    let archive = sevenz_rust2::Archive::open(archive_path).map_err(|e| {
        crate::error::OpenArchiveSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    Ok(archive
        .files
        .iter()
        .fold(0u64, |total, entry| total.saturating_add(entry.size)))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;
    use std::path::PathBuf;

    use super::*;

    const NO_RATIO_LIMIT: f64 = 100.0;

    fn create_test_7z(root: &Path, contents: &[(&str, &[u8])]) -> PathBuf {
        let staging = root.join("staging");
        fs::create_dir_all(&staging).unwrap();
        for (name, data) in contents {
            fs::write(staging.join(name), data).unwrap();
        }
        let archive_path = root.join("test.7z");
        sevenz_rust2::compress_to_path(&staging, &archive_path).unwrap();
        archive_path
    }

    // Writes a 7z with a single verbatim entry name (bypassing the filesystem so
    // hostile names survive) and optional windows attributes.
    fn create_7z_raw_entry(
        root: &Path,
        entry_name: &str,
        data: &[u8],
        windows_attributes: Option<u32>,
    ) -> PathBuf {
        let archive_path = root.join("crafted.7z");
        let mut writer = sevenz_rust2::ArchiveWriter::create(&archive_path).unwrap();
        let mut entry = ArchiveEntry::new_file(entry_name);
        if let Some(attr) = windows_attributes {
            entry.has_windows_attributes = true;
            entry.windows_attributes = attr;
        }
        writer
            .push_archive_entry(entry, Some(Cursor::new(data.to_vec())))
            .unwrap();
        writer.finish().unwrap();
        archive_path
    }

    fn assert_empty(dir: &Path) {
        let leftovers: Vec<_> = fs::read_dir(dir).unwrap().flatten().collect();
        assert!(
            leftovers.is_empty(),
            "expected empty dir, found {leftovers:?}"
        );
    }

    #[test]
    fn extract_7z_success() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = create_test_7z(
            dir.path(),
            &[("hello.txt", b"Hello, 7z!"), ("data.bin", &[0xAB; 64])],
        );
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        extract_7z(&archive_path, &output_dir, NO_RATIO_LIMIT).unwrap();

        assert_eq!(
            fs::read_to_string(output_dir.join("hello.txt")).unwrap(),
            "Hello, 7z!"
        );
        assert_eq!(fs::read(output_dir.join("data.bin")).unwrap(), [0xAB; 64]);
    }

    #[test]
    fn extract_7z_corrupt_archive_errors() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("corrupt.7z");
        fs::write(&archive_path, b"7z\xBC\xAF\x27\x1C not actually valid").unwrap();
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let err = extract_7z(&archive_path, &output_dir, NO_RATIO_LIMIT).unwrap_err();
        assert!(
            matches!(
                err,
                ErgasiaError::OpenArchive { .. } | ErgasiaError::ExtractFile { .. }
            ),
            "expected OpenArchive or ExtractFile for a corrupt archive, got: {err}"
        );
    }

    #[test]
    fn extract_7z_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let err = extract_7z(
            &dir.path().join("nonexistent.7z"),
            &output_dir,
            NO_RATIO_LIMIT,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                ErgasiaError::OpenArchive { .. } | ErgasiaError::ExtractFile { .. }
            ),
            "expected OpenArchive or ExtractFile for a missing archive, got: {err}"
        );
    }

    #[test]
    fn declared_size_sums_entries() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path =
            create_test_7z(dir.path(), &[("a.bin", &[0u8; 100]), ("b.bin", &[0u8; 50])]);

        assert_eq!(declared_uncompressed_size(&archive_path).unwrap(), 150);
    }

    #[test]
    fn reject_parent_traversal_entry() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = create_7z_raw_entry(dir.path(), "../escape.txt", b"traversal", None);
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let err = extract_7z(&archive_path, &output_dir, NO_RATIO_LIMIT).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::UnsafeArchiveEntry { .. }),
            "expected UnsafeArchiveEntry, got: {err}"
        );
        assert_empty(&output_dir);
        assert!(!dir.path().join("escape.txt").exists());
    }

    #[test]
    fn reject_absolute_path_entry() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = create_7z_raw_entry(dir.path(), "/etc/evil.txt", b"absolute", None);
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let err = extract_7z(&archive_path, &output_dir, NO_RATIO_LIMIT).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::UnsafeArchiveEntry { .. }),
            "expected UnsafeArchiveEntry, got: {err}"
        );
        assert_empty(&output_dir);
    }

    #[test]
    fn reject_symlink_entry() {
        let dir = tempfile::tempdir().unwrap();
        // Unix-mode symlink: FILE_ATTRIBUTE_UNIX_EXTENSION with S_IFLNK in the
        // high 16 bits.
        let attr = FILE_ATTRIBUTE_UNIX_EXTENSION | (S_IFLNK << 16);
        let archive_path = create_7z_raw_entry(dir.path(), "link", b"../../etc/passwd", Some(attr));
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let err = extract_7z(&archive_path, &output_dir, NO_RATIO_LIMIT).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::UnsafeArchiveEntry { .. }),
            "expected UnsafeArchiveEntry, got: {err}"
        );
        assert_empty(&output_dir);
    }

    #[test]
    fn byte_cap_aborts_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = create_test_7z(dir.path(), &[("payload.bin", &[0u8; 4096])]);
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        // A zero ratio forces a byte cap of 0, so the first byte written trips
        // the guard and the partial output must be rolled back.
        let err = extract_7z(&archive_path, &output_dir, 0.0).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::DecompressionRatioExceeded { .. }),
            "expected DecompressionRatioExceeded, got: {err}"
        );
        assert_empty(&output_dir);
    }
}
