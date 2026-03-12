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
        .expect("task panicked")?;
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
    .expect("task panicked")
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
    .expect("task panicked")
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
            std::fs::copy(&source, &tmp)
                .and_then(|_| std::fs::rename(&tmp, &target))
                .map(|_| {
                    let _ = std::fs::remove_file(&source);
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
    .expect("task panicked")
}

fn is_cross_device(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::CrossesDevices || e.raw_os_error() == Some(18) // EXDEV on Linux
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
}
