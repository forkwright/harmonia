use std::path::Path;

use crate::error::TaxisError;

/// Ensure all parent directories for the given path exist.
pub async fn ensure_parent_dirs(path: &Path) -> Result<(), TaxisError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        let parent = parent.to_path_buf();
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&parent).map_err(|e| TaxisError::FileOperation {
                operation: "create_dir_all".into(),
                source_path: parent.clone(),
                target_path: parent.clone(),
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        })
        .await
        .map_err(|e| TaxisError::BlockingTaskFailed {
            message: e.to_string(),
            location: snafu::location!(),
        })
        .and_then(|r| r)?;
    }
    Ok(())
}

/// Hardlink source to target. Falls back to copy on EXDEV (cross-device).
pub async fn hardlink_or_copy(source: &Path, target: &Path) -> Result<FileOpResult, TaxisError> {
    let source = source.to_path_buf();
    let target = target.to_path_buf();

    ensure_parent_dirs(&target).await?;

    tokio::task::spawn_blocking(move || match std::fs::hard_link(&source, &target) {
        Ok(()) => Ok(FileOpResult::Hardlinked),
        Err(e) if is_cross_device(&e) => std::fs::copy(&source, &target)
            .map(|_| FileOpResult::Copied)
            .map_err(|io_err| TaxisError::FileOperation {
                operation: "copy".into(),
                source_path: source.clone(),
                target_path: target.clone(),
                source: io_err,
                location: snafu::Location::new(file!(), line!(), column!()),
            }),
        Err(e) => Err(TaxisError::FileOperation {
            operation: "hardlink".into(),
            source_path: source.clone(),
            target_path: target.clone(),
            source: e,
            location: snafu::Location::new(file!(), line!(), column!()),
        }),
    })
    .await
    .map_err(|e| TaxisError::BlockingTaskFailed {
        message: e.to_string(),
        location: snafu::location!(),
    })
    .and_then(|r| r)
}

/// Copy source to target.
pub async fn copy_file(source: &Path, target: &Path) -> Result<FileOpResult, TaxisError> {
    let source = source.to_path_buf();
    let target = target.to_path_buf();

    ensure_parent_dirs(&target).await?;

    tokio::task::spawn_blocking(move || {
        std::fs::copy(&source, &target)
            .map(|_| FileOpResult::Copied)
            .map_err(|e| TaxisError::FileOperation {
                operation: "copy".into(),
                source_path: source.clone(),
                target_path: target.clone(),
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })
    })
    .await
    .map_err(|e| TaxisError::BlockingTaskFailed {
        message: e.to_string(),
        location: snafu::location!(),
    })
    .and_then(|r| r)
}

/// Rename (move) source to target. Uses atomic rename on same FS.
pub async fn rename_file(source: &Path, target: &Path) -> Result<FileOpResult, TaxisError> {
    let source = source.to_path_buf();
    let target = target.to_path_buf();

    ensure_parent_dirs(&target).await?;

    tokio::task::spawn_blocking(move || match std::fs::rename(&source, &target) {
        Ok(()) => Ok(FileOpResult::Renamed),
        Err(e) if is_cross_device(&e) => {
            let tmp = target.with_extension("tmp");
            std::fs::copy(&source, &tmp) // kanon:ignore PERFORMANCE/no-blocking-io-in-async -- inside spawn_blocking; synchronous std::fs calls are intentional
                .and_then(|_| std::fs::rename(&tmp, &target)) // kanon:ignore PERFORMANCE/no-blocking-io-in-async -- inside spawn_blocking
                .map(|_| {
                    // WHY: temp file cleanup failure is non-fatal; OS will reclaim on exit
                    std::fs::remove_file(&source).ok(); // kanon:ignore PERFORMANCE/no-blocking-io-in-async -- inside spawn_blocking
                    FileOpResult::Renamed
                })
                .map_err(|io_err| TaxisError::FileOperation {
                    operation: "rename".into(),
                    source_path: source.clone(),
                    target_path: target.clone(),
                    source: io_err,
                    location: snafu::Location::new(file!(), line!(), column!()),
                })
        }
        Err(e) => Err(TaxisError::FileOperation {
            operation: "rename".into(),
            source_path: source.clone(),
            target_path: target.clone(),
            source: e,
            location: snafu::Location::new(file!(), line!(), column!()),
        }),
    })
    .await
    .map_err(|e| TaxisError::BlockingTaskFailed {
        message: e.to_string(),
        location: snafu::location!(),
    })
    .and_then(|r| r)
}

fn is_cross_device(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::CrossesDevices || e.raw_os_error() == Some(18) // EXDEV on Linux
}

/// Checks whether `target` already holds the same content as `source`.
///
/// Two tiers: (1) exact same `(dev, ino)` — `target` is a hardlink of
/// `source` (the common same-filesystem `ImportOrigin::Download` case);
/// (2) for distinct inodes (an EXDEV cross-device copy), equal size AND a
/// byte-for-byte content comparison. A missing `target` is `Ok(false)`, not
/// an error — the common case (no prior import).
///
/// WHY the byte compare rather than the old same-size heuristic: size alone
/// returned `true` for any distinct file that merely matched the target's
/// length, silently dropping a genuinely different (e.g. higher-quality)
/// file as "already present" (data loss). Size alone also cannot recognize a
/// legitimate EXDEV-copy re-import, so it would suffix a duplicate on every
/// crash-replay. A content compare is correct on both axes; it runs only on
/// an equal-size collision (rare), and the archon haves short-circuit (keyed
/// on `file_path`) remains the durable idempotency layer.
pub async fn same_file(source: &Path, target: &Path) -> Result<bool, TaxisError> {
    let source = source.to_path_buf();
    let target = target.to_path_buf();

    tokio::task::spawn_blocking(move || same_file_blocking(&source, &target))
        .await
        .map_err(|e| TaxisError::BlockingTaskFailed {
            message: e.to_string(),
            location: snafu::location!(),
        })
        .and_then(|r| r)
}

fn same_file_blocking(source: &Path, target: &Path) -> Result<bool, TaxisError> {
    use std::os::unix::fs::MetadataExt;

    if !target.exists() {
        return Ok(false);
    }

    let source_meta = std::fs::metadata(source).map_err(|e| TaxisError::FileOperation {
        operation: "stat".into(),
        source_path: source.to_path_buf(),
        target_path: target.to_path_buf(),
        source: e,
        location: snafu::location!(),
    })?;
    let target_meta = std::fs::metadata(target).map_err(|e| TaxisError::FileOperation {
        operation: "stat".into(),
        source_path: source.to_path_buf(),
        target_path: target.to_path_buf(),
        source: e,
        location: snafu::location!(),
    })?;

    // Exact hardlink: the import already landed as a second link to the
    // source inode (same-filesystem `hardlink_or_copy`).
    if source_meta.dev() == target_meta.dev() && source_meta.ino() == target_meta.ino() {
        return Ok(true);
    }

    // WHY: a cross-device import copies (EXDEV fallback) to a FRESH inode, so
    // a crash-replay re-import cannot match by inode and would otherwise
    // suffix a duplicate every retry. Fall back to a content comparison —
    // gated on equal size (cheap), then a byte compare. A byte compare (NOT
    // size alone) is required: a genuinely different file of equal length
    // must not be treated as already-present (that would silently drop it).
    if source_meta.len() != target_meta.len() {
        return Ok(false);
    }
    files_have_identical_content(source, target)
}

/// Byte-for-byte comparison of two files, chunked so large media never loads
/// wholesale. Callers gate this on equal size (it assumes both files have the
/// same length).
fn files_have_identical_content(source: &Path, target: &Path) -> Result<bool, TaxisError> {
    use std::io::BufRead;

    let op_err = |operation: &str, e: std::io::Error| TaxisError::FileOperation {
        operation: operation.into(),
        source_path: source.to_path_buf(),
        target_path: target.to_path_buf(),
        source: e,
        location: snafu::location!(),
    };

    let mut sf =
        std::io::BufReader::new(std::fs::File::open(source).map_err(|e| op_err("open", e))?);
    let mut tf =
        std::io::BufReader::new(std::fs::File::open(target).map_err(|e| op_err("open", e))?);
    loop {
        // fill_buf + consume avoids fixed-buffer index slicing; the compared
        // prefix goes through `.get()`, never direct indexing.
        let consumed = {
            let sbuf = sf.fill_buf().map_err(|e| op_err("read", e))?;
            let tbuf = tf.fill_buf().map_err(|e| op_err("read", e))?;
            match (sbuf.is_empty(), tbuf.is_empty()) {
                (true, true) => return Ok(true),
                (true, false) | (false, true) => return Ok(false),
                (false, false) => {}
            }
            let len = sbuf.len().min(tbuf.len());
            if sbuf.get(..len) != tbuf.get(..len) {
                return Ok(false);
            }
            len
        };
        sf.consume(consumed);
        tf.consume(consumed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileOpResult {
    Hardlinked,
    Copied,
    Renamed,
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn hardlink_succeeds_on_same_fs() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("target.flac");
        std::fs::write(&source, b"FLAC data").unwrap();

        let result = hardlink_or_copy(&source, &target).await.unwrap();
        assert_eq!(result, FileOpResult::Hardlinked);
        assert!(target.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let src_meta = std::fs::metadata(&source).unwrap();
            let tgt_meta = std::fs::metadata(&target).unwrap();
            assert_eq!(src_meta.ino(), tgt_meta.ino());
        }
    }

    #[tokio::test]
    async fn copy_creates_independent_file() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("subdir/target.flac");
        std::fs::write(&source, b"content").unwrap();

        let result = copy_file(&source, &target).await.unwrap();
        assert_eq!(result, FileOpResult::Copied);
        assert!(target.exists());
        assert_eq!(std::fs::read(&target).unwrap(), b"content");
    }

    #[tokio::test]
    async fn rename_moves_file() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("original.flac");
        let target = dir.path().join("renamed.flac");
        std::fs::write(&source, b"data").unwrap();

        let result = rename_file(&source, &target).await.unwrap();
        assert_eq!(result, FileOpResult::Renamed);
        assert!(target.exists());
        assert!(!source.exists());
    }

    #[tokio::test]
    async fn ensure_parent_dirs_creates_nested() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a/b/c/file.flac");
        ensure_parent_dirs(&path).await.unwrap();
        assert!(dir.path().join("a/b/c").exists());
    }

    #[tokio::test]
    async fn same_file_true_for_hardlink() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("target.flac");
        std::fs::write(&source, b"FLAC data").unwrap();
        hardlink_or_copy(&source, &target).await.unwrap();

        assert!(same_file(&source, &target).await.unwrap());
    }

    #[tokio::test]
    async fn same_file_false_for_missing_target() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("target.flac");
        std::fs::write(&source, b"FLAC data").unwrap();

        assert!(!same_file(&source, &target).await.unwrap());
    }

    #[tokio::test]
    async fn same_file_false_for_distinct_content_and_size() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("target.flac");
        std::fs::write(&source, b"short").unwrap();
        std::fs::write(&target, b"a much longer distinct payload").unwrap();

        assert!(!same_file(&source, &target).await.unwrap());
    }

    #[tokio::test]
    async fn same_file_false_for_distinct_content_same_size() {
        // FIX 2: a genuinely different file that merely matches the target's
        // byte length must NOT be treated as identical — the removed
        // size-only heuristic silently dropped such files as "already
        // present" (data loss). Both are 16 bytes, distinct content.
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("target.flac");
        std::fs::write(&source, b"aaaaaaaaaaaaaaaa").unwrap();
        std::fs::write(&target, b"bbbbbbbbbbbbbbbb").unwrap();

        assert!(!same_file(&source, &target).await.unwrap());
    }

    #[tokio::test]
    async fn same_file_true_for_identical_cross_device_copy() {
        // FIX 2 (re-review): a cross-device import copies to a FRESH inode, so
        // a crash-replay re-import must still be recognized as already-present
        // by content — otherwise every retry suffixes a duplicate. Distinct
        // inodes, identical bytes ⇒ true.
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("target.flac");
        std::fs::write(&source, b"identical bytes across a copy").unwrap();
        copy_file(&source, &target).await.unwrap();

        // Distinct inodes (a copy, not a hardlink)...
        use std::os::unix::fs::MetadataExt;
        assert_ne!(
            std::fs::metadata(&source).unwrap().ino(),
            std::fs::metadata(&target).unwrap().ino()
        );
        // ...but identical content ⇒ same file for idempotency.
        assert!(same_file(&source, &target).await.unwrap());
    }
}
