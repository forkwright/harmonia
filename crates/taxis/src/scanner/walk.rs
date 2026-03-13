use std::path::{Path, PathBuf};

use harmonia_common::MediaType;
use tokio::sync::Semaphore;
use tracing::instrument;
use walkdir::WalkDir;

use crate::error::TaxisError;
use crate::scanner::filter::{HarmoniaIgnore, is_supported_extension};

#[derive(Debug)]
pub struct WalkResult {
    pub path: PathBuf,
    pub media_type: MediaType,
}

#[derive(Debug, Default)]
pub struct WalkStats {
    pub scanned: usize,
    pub skipped_ignored: usize,
    pub skipped_unsupported: usize,
}

/// Walk a library directory, yielding supported media files.
/// Respects .harmoniaignore files and the expected media type.
#[instrument(skip(semaphore))]
pub async fn walk_library(
    root: &Path,
    media_type: MediaType,
    semaphore: &Semaphore,
) -> Result<(Vec<WalkResult>, WalkStats), TaxisError> {
    let root = root.to_path_buf();
    let ignore = HarmoniaIgnore::load(&root);
    let _permit = semaphore
        .acquire()
        .await
        .map_err(|_| TaxisError::ScannerInit {
            source: std::io::Error::other("semaphore closed"),
            location: snafu::location!(),
        })?;

    let results =
        tokio::task::spawn_blocking(move || walk_library_blocking(&root, media_type, &ignore))
            .await
            .map_err(|e| TaxisError::BlockingTaskFailed {
                message: e.to_string(),
                location: snafu::location!(),
            })
            .and_then(|r| r)?;

    Ok(results)
}

fn walk_library_blocking(
    root: &Path,
    media_type: MediaType,
    ignore: &HarmoniaIgnore,
) -> Result<(Vec<WalkResult>, WalkStats), TaxisError> {
    let mut results = Vec::new();
    let mut stats = WalkStats::default();

    for entry in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = entry.map_err(|e| {
            let path = e.path().unwrap_or(root).to_path_buf();
            TaxisError::ScanWalk {
                path,
                source: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            }
        })?;

        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.into_path();
        stats.scanned += 1;

        if ignore.is_ignored(&path) {
            stats.skipped_ignored += 1;
            continue;
        }

        if !is_supported_extension(&path, media_type) {
            stats.skipped_unsupported += 1;
            continue;
        }

        results.push(WalkResult { path, media_type });
    }

    Ok((results, stats))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;
    use tokio::sync::Semaphore;

    use super::*;

    fn create_file(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, b"placeholder").unwrap();
        p
    }

    #[tokio::test]
    async fn walk_finds_supported_files() {
        let dir = TempDir::new().unwrap();
        create_file(dir.path(), "track1.flac");
        create_file(dir.path(), "track2.mp3");
        create_file(dir.path(), "cover.jpg"); // unsupported

        let sem = Semaphore::new(4);
        let (results, stats) = walk_library(dir.path(), MediaType::Music, &sem)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(stats.skipped_unsupported, 1);
    }

    #[tokio::test]
    async fn walk_respects_harmoniaignore() {
        let dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(dir.path().join(".harmoniaignore")).unwrap();
        writeln!(f, "*.part").unwrap();

        create_file(dir.path(), "track.flac");
        create_file(dir.path(), "download.part");

        let sem = Semaphore::new(4);
        let (results, stats) = walk_library(dir.path(), MediaType::Music, &sem)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path.file_name().unwrap(), "track.flac");
        assert_eq!(stats.skipped_ignored, 1);
    }

    #[tokio::test]
    async fn walk_recurses_subdirectories() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("Album");
        std::fs::create_dir(&sub).unwrap();
        create_file(&sub, "track.flac");
        create_file(dir.path(), "other.mp3");

        let sem = Semaphore::new(4);
        let (results, _) = walk_library(dir.path(), MediaType::Music, &sem)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn walk_skips_unsupported_extensions() {
        let dir = TempDir::new().unwrap();
        create_file(dir.path(), "image.jpg");
        create_file(dir.path(), "doc.txt");
        create_file(dir.path(), "track.flac");

        let sem = Semaphore::new(4);
        let (results, stats) = walk_library(dir.path(), MediaType::Music, &sem)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(stats.skipped_unsupported, 2);
    }
}
