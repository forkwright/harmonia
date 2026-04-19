use std::path::{Path, PathBuf};

use crate::error::TaxisError;

pub(crate) const DEFAULT_MAX_SUFFIX: usize = 99;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConflictOutcome {
    /// Target path does not exist — proceed as-is.
    Clear(PathBuf),
    /// Same media item, new quality is higher — replace.
    Upgrade(PathBuf),
    /// Path collision — suffix appended to avoid overwrite.
    Suffixed(PathBuf),
    /// Same quality or lower — skip import.
    Skip,
}

/// Resolve a target path conflict.
///
/// - `target`: the desired target path
/// - `existing_quality`: quality score of the file currently at that path (if any)
/// - `new_quality`: quality score of the file being imported
/// - `is_same_item`: true if the existing and incoming files represent the same media item
/// - `max_suffix`: maximum numeric suffix to try before erroring
pub(crate) fn resolve_conflict(
    target: &Path,
    existing_quality: Option<u32>,
    new_quality: u32,
    is_same_item: bool,
    max_suffix: usize,
) -> Result<ConflictOutcome, TaxisError> {
    if !target.exists() {
        return Ok(ConflictOutcome::Clear(target.to_path_buf()));
    }

    match existing_quality {
        Some(existing_q) if is_same_item => {
            if new_quality > existing_q {
                Ok(ConflictOutcome::Upgrade(target.to_path_buf()))
            } else {
                Ok(ConflictOutcome::Skip)
            }
        }
        _ => {
            let suffixed = find_suffixed_path(target, max_suffix)?;
            Ok(ConflictOutcome::Suffixed(suffixed))
        }
    }
}

fn find_suffixed_path(target: &Path, max_suffix: usize) -> Result<PathBuf, TaxisError> {
    let stem = target.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = target.extension().and_then(|e| e.to_str());
    let parent = target.parent().unwrap_or(Path::new(""));

    for n in 2..=max_suffix {
        let name = match ext {
            Some(e) => format!("{stem}_{n}.{e}"),
            None => format!("{stem}_{n}"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(TaxisError::ConflictResolution {
        target_path: target.to_path_buf(),
        max: max_suffix,
        location: snafu::Location::new(file!(), line!(), column!()),
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn no_conflict_when_target_missing() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("track.flac");
        let outcome = resolve_conflict(&target, None, 300, false, DEFAULT_MAX_SUFFIX).unwrap();
        assert_eq!(outcome, ConflictOutcome::Clear(target));
    }

    #[test]
    fn skip_on_same_quality() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("track.flac");
        std::fs::write(&target, b"existing").unwrap();
        let outcome = resolve_conflict(&target, Some(300), 300, true, DEFAULT_MAX_SUFFIX).unwrap();
        assert_eq!(outcome, ConflictOutcome::Skip);
    }

    #[test]
    fn skip_on_lower_quality() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("track.flac");
        std::fs::write(&target, b"existing").unwrap();
        let outcome = resolve_conflict(&target, Some(300), 100, true, DEFAULT_MAX_SUFFIX).unwrap();
        assert_eq!(outcome, ConflictOutcome::Skip);
    }

    #[test]
    fn upgrade_on_higher_quality_same_item() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("track.flac");
        std::fs::write(&target, b"existing").unwrap();
        let outcome = resolve_conflict(&target, Some(100), 300, true, DEFAULT_MAX_SUFFIX).unwrap();
        assert_eq!(outcome, ConflictOutcome::Upgrade(target));
    }

    #[test]
    fn suffix_on_different_item() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("track.flac");
        std::fs::write(&target, b"existing").unwrap();
        let outcome = resolve_conflict(&target, Some(300), 300, false, DEFAULT_MAX_SUFFIX).unwrap();
        match outcome {
            ConflictOutcome::Suffixed(p) => {
                assert!(p.to_str().unwrap().contains("_2"));
            }
            other => panic!("expected Suffixed, got {other:?}"),
        }
    }

    #[test]
    fn suffix_increments_past_existing() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("track.flac");
        std::fs::write(&target, b"1").unwrap();
        std::fs::write(dir.path().join("track_2.flac"), b"2").unwrap();
        let outcome = resolve_conflict(&target, None, 300, false, DEFAULT_MAX_SUFFIX).unwrap();
        match outcome {
            ConflictOutcome::Suffixed(p) => {
                assert!(
                    p.to_str().unwrap().contains("_3"),
                    "expected _3, got {}",
                    p.display()
                );
            }
            other => panic!("expected Suffixed, got {other:?}"),
        }
    }

    #[test]
    fn conflict_error_when_max_suffix_exhausted() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("track.flac");
        std::fs::write(&target, b"orig").unwrap();
        for n in 2..=3 {
            std::fs::write(dir.path().join(format!("track_{n}.flac")), b"x").unwrap();
        }
        // max_suffix = 3 means try _2 and _3, both exist
        let result = resolve_conflict(&target, None, 300, false, 3);
        assert!(result.is_err());
    }
}
