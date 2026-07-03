use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::error::{ErgasiaError, UnsafeArchiveEntrySnafu};

static MODERN_RAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.part(\d+)\.rar$")
        .unwrap_or_else(|e| unreachable!("regex literal is statically valid: {e}"))
});

// WHY: every genuine RAR volume — including each part of a multi-volume set —
// begins with this 6-byte signature; matching it lets the ratio denominator
// exclude junk-extension padding files that only mimic a volume's name.
const RAR_SIGNATURE: [u8; 6] = *b"Rar!\x1a\x07";

// Unix st_mode format field: entries whose type is neither regular file nor
// directory (symlink, block/char device, fifo, socket) are refused.
const S_IFMT: u32 = 0o170000;
const S_IFREG: u32 = 0o100000;
const S_IFDIR: u32 = 0o040000;
const S_IFLNK: u32 = 0o120000;

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
    // SAFETY: validate every entry across the whole volume set before any write,
    // so a hostile archive is refused atomically (mirrors zip ensure_safe_entries).
    // Providing a base to extract_with_base disables unrar's own path
    // sanitization, so this pre-validation is the sole traversal guard.
    validate_rar_entries(archive_path)?;

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

// SAFETY: lists every entry (across all volumes) and rejects the whole archive
// on the first unsafe one, before extract_rar writes anything.
fn validate_rar_entries(archive_path: &Path) -> Result<(), ErgasiaError> {
    let archive = unrar::Archive::new(archive_path)
        .open_for_listing()
        .map_err(|e| {
            crate::error::OpenArchiveSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

    for header in archive {
        let entry = header.map_err(|e| {
            crate::error::ExtractFileSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

        if let Some(reason) = rar_name_unsafe_reason(&entry.filename) {
            return Err(UnsafeArchiveEntrySnafu {
                archive: archive_path.to_path_buf(),
                entry: entry.filename.display().to_string(),
                reason: reason.to_string(),
            }
            .build());
        }

        if let Some(reason) = rar_attr_unsafe_reason(entry.file_attr, entry.is_directory()) {
            return Err(UnsafeArchiveEntrySnafu {
                archive: archive_path.to_path_buf(),
                entry: entry.filename.display().to_string(),
                reason: reason.to_string(),
            }
            .build());
        }
    }

    Ok(())
}

// NOTE: RAR entry names may use either separator and may carry a Windows
// drive-letter/backslash root, so absolute and parent-traversal checks span
// both `/` and `\` rather than relying on the host Path semantics.
fn rar_name_unsafe_reason(name: &Path) -> Option<&'static str> {
    let Some(name) = name.to_str() else {
        return Some("non-UTF-8 entry name");
    };
    if name.starts_with('/') || name.starts_with('\\') || name.get(1..2) == Some(":") {
        return Some("absolute entry path");
    }
    for segment in name.split(['/', '\\']) {
        if segment == ".." {
            return Some("parent directory traversal");
        }
    }
    None
}

// WHY: unrar 0.5.8 does not surface the RAR5 FSREDIR redirect kind, so symlinks
// are detected via the unix mode carried in file_attr (RAR3 and RAR5 both store
// S_IFLNK there for unix-host entries). Any non-regular, non-directory type is
// refused — the sanctioned fallback when the crate cannot name symlinks exactly.
fn rar_attr_unsafe_reason(file_attr: u32, is_directory: bool) -> Option<&'static str> {
    if is_directory {
        return None;
    }
    let unix_type = file_attr & S_IFMT;
    // A DOS/Windows-host entry stores FILE_ATTRIBUTE bits here, not a unix mode;
    // its format field is left unset (0), so only interpret a populated field.
    if unix_type == 0 || unix_type == S_IFREG || unix_type == S_IFDIR {
        return None;
    }
    if unix_type == S_IFLNK {
        return Some("symlink entry");
    }
    Some("non-regular file entry")
}

fn has_rar_signature(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 6];
    file.read_exact(&mut magic).is_ok() && magic == RAR_SIGNATURE
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
// ratio guard must compare against the on-disk size of every volume. The
// denominator is derived from this archive's actual naming chain (contiguous
// from the first volume) and each candidate must carry the RAR signature, so a
// junk-extension padding file cannot inflate the compressed size and smuggle a
// real bomb past the ratio guard.
pub(crate) fn volume_set_size(first_volume: &Path) -> u64 {
    let chain = rar_volume_chain(first_volume);
    if chain.is_empty() {
        return first_volume.metadata().map(|m| m.len()).unwrap_or(0);
    }
    chain
        .iter()
        .filter_map(|p| p.metadata().ok().map(|m| m.len()))
        .fold(0u64, |total, len| total.saturating_add(len))
}

// Derives the contiguous set of on-disk volumes for this archive from the first
// volume's name, verifying each carries the RAR signature. Enumeration stops at
// the first missing or non-signature-bearing candidate so unrelated files never
// join the set.
fn rar_volume_chain(first_volume: &Path) -> Vec<PathBuf> {
    let Some(dir) = first_volume.parent() else {
        return signature_filter(vec![first_volume.to_path_buf()]);
    };
    let Some(name) = first_volume.file_name().and_then(|n| n.to_str()) else {
        return signature_filter(vec![first_volume.to_path_buf()]);
    };

    // Modern scheme: <base>.partNN.rar — preserve the numeric width and count up.
    if let Some(caps) = MODERN_RAR_RE.captures(name) {
        let full = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
        let digits = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let base = name.get(..name.len() - full.len()).unwrap_or_default();
        let width = digits.len();
        let start = digits.parse::<u32>().unwrap_or(1);

        let mut chain = Vec::new();
        for n in start.. {
            let candidate = dir.join(format!("{base}.part{n:0width$}.rar"));
            if candidate.is_file() && has_rar_signature(&candidate) {
                chain.push(candidate);
            } else {
                break;
            }
        }
        return chain;
    }

    // Legacy scheme: <base>.rar, then <base>.r00, .r01, ..., .r99, .s00, ...
    let base = first_volume
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);

    let mut chain = Vec::new();
    if first_volume.is_file() && has_rar_signature(first_volume) {
        chain.push(first_volume.to_path_buf());
    }
    for i in 0u32.. {
        let block = (i / 100) as u8;
        let num = i % 100;
        let letter = (b'r' + block) as char;
        let candidate = dir.join(format!("{base}.{letter}{num:02}"));
        if candidate.is_file() && has_rar_signature(&candidate) {
            chain.push(candidate);
        } else {
            break;
        }
    }
    chain
}

fn signature_filter(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    candidates
        .into_iter()
        .filter(|p| has_rar_signature(p))
        .collect()
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

    #[test]
    fn rar_name_validation_accepts_safe_and_rejects_traversal() {
        assert!(rar_name_unsafe_reason(Path::new("a/b/c.txt")).is_none());
        assert!(rar_name_unsafe_reason(Path::new("dir/file")).is_none());

        assert!(rar_name_unsafe_reason(Path::new("../escape")).is_some());
        assert!(rar_name_unsafe_reason(Path::new("a/../../escape")).is_some());
        assert!(rar_name_unsafe_reason(Path::new("/etc/passwd")).is_some());
        // Backslash separator and drive-letter roots (Windows-origin entries).
        assert!(rar_name_unsafe_reason(Path::new(r"..\..\escape")).is_some());
        assert!(rar_name_unsafe_reason(Path::new(r"C:\Windows\evil")).is_some());
        assert!(rar_name_unsafe_reason(Path::new(r"\\server\share")).is_some());
    }

    #[test]
    fn rar_attr_validation_rejects_non_regular() {
        // Regular file and directory pass.
        assert!(rar_attr_unsafe_reason(0o100644, false).is_none());
        assert!(rar_attr_unsafe_reason(0o040755, true).is_none());
        // DOS/Windows attribute bits carry no unix mode field.
        assert!(rar_attr_unsafe_reason(0x20, false).is_none());
        // Symlink and other special files are refused.
        assert_eq!(
            rar_attr_unsafe_reason(0o120777, false),
            Some("symlink entry")
        );
        assert!(rar_attr_unsafe_reason(0o060644, false).is_some());
        assert!(rar_attr_unsafe_reason(0o010644, false).is_some());
    }

    fn signed_volume(extra: usize) -> Vec<u8> {
        let mut bytes = RAR_SIGNATURE.to_vec();
        bytes.resize(RAR_SIGNATURE.len() + extra, 0);
        bytes
    }

    #[test]
    fn volume_set_size_counts_only_signed_modern_chain() {
        let dir = tempfile::tempdir().unwrap();
        let vol = signed_volume(10); // 16 bytes each
        fs::write(dir.path().join("movie.part1.rar"), &vol).unwrap();
        fs::write(dir.path().join("movie.part2.rar"), &vol).unwrap();
        fs::write(dir.path().join("movie.part3.rar"), &vol).unwrap();
        // Junk padding: right extension, no RAR signature, huge — must be
        // excluded so it cannot inflate the compressed denominator.
        fs::write(dir.path().join("movie.part4.rar"), vec![0u8; 1_000_000]).unwrap();
        fs::write(dir.path().join("padding.rar"), vec![0u8; 1_000_000]).unwrap();

        assert_eq!(volume_set_size(&dir.path().join("movie.part1.rar")), 48);
    }

    #[test]
    fn volume_set_size_counts_only_signed_legacy_chain() {
        let dir = tempfile::tempdir().unwrap();
        let vol = signed_volume(4); // 10 bytes each
        fs::write(dir.path().join("archive.rar"), &vol).unwrap();
        fs::write(dir.path().join("archive.r00"), &vol).unwrap();
        fs::write(dir.path().join("archive.r01"), &vol).unwrap();
        // Unsigned decoy breaks the contiguous chain.
        fs::write(dir.path().join("archive.r02"), vec![0u8; 1_000_000]).unwrap();

        assert_eq!(volume_set_size(&dir.path().join("archive.rar")), 30);
    }
}
