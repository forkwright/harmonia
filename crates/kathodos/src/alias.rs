use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use snafu::Snafu;

use crate::sanitize::sanitize_component as sanitize_path_segment;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AliasError {
    #[snafu(display("alias '{alias}' conflicts with real directory at {path:?}"))]
    ConflictsWithDirectory {
        alias: String,
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("canonical artist '{canonical}' does not exist at {path:?}"))]
    CanonicalNotFound {
        canonical: String,
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("alias '{alias}' does not exist at {path:?}"))]
    AliasNotFound {
        alias: String,
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("symlink operation failed at {path:?}: {source}"))]
    Io {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

/// Create a symlink from an alias name to the canonical artist directory.
/// The symlink lives alongside the canonical directory (same parent).
///
/// - Both names are sanitized through the standard path sanitization pipeline.
/// - If the alias path already exists as a symlink, its target is updated.
/// - If the alias path already exists as a real directory, an error is returned.
pub fn create_artist_alias(
    library_root: &Path,
    canonical_name: &str,
    alias_name: &str,
) -> Result<(), AliasError> {
    let canonical_safe = sanitize_path_segment(canonical_name);
    let alias_safe = sanitize_path_segment(alias_name);

    let canonical_path = library_root.join(&canonical_safe);
    let alias_path = library_root.join(&alias_safe);

    if !canonical_path.exists() {
        return Err(AliasError::CanonicalNotFound {
            canonical: canonical_safe,
            path: canonical_path,
            location: snafu::location!(),
        });
    }

    if alias_path.exists() && !alias_path.is_symlink() {
        return Err(AliasError::ConflictsWithDirectory {
            alias: alias_safe,
            path: alias_path,
            location: snafu::location!(),
        });
    }

    // Remove existing symlink before creating a new one.
    if alias_path.is_symlink() {
        std::fs::remove_file(&alias_path).map_err(|e| AliasError::Io {
            path: alias_path.clone(),
            source: e,
            location: snafu::location!(),
        })?;
    }

    symlink(&canonical_safe, &alias_path).map_err(|e| AliasError::Io {
        path: alias_path,
        source: e,
        location: snafu::location!(),
    })
}

/// Remove an artist alias symlink.
///
/// Returns an error if the path does not exist or is not a symlink.
pub fn remove_artist_alias(library_root: &Path, alias_name: &str) -> Result<(), AliasError> {
    let alias_safe = sanitize_path_segment(alias_name);
    let alias_path = library_root.join(&alias_safe);

    if !alias_path.is_symlink() {
        return Err(AliasError::AliasNotFound {
            alias: alias_safe,
            path: alias_path,
            location: snafu::location!(),
        });
    }

    std::fs::remove_file(&alias_path).map_err(|e| AliasError::Io {
        path: alias_path,
        source: e,
        location: snafu::location!(),
    })
}

/// List all alias symlinks in `library_root` that point to the canonical artist directory.
///
/// Only entries that are symlinks whose target resolves (relative to the library root) to
/// the canonical directory are included.
pub fn list_artist_aliases(
    library_root: &Path,
    canonical_name: &str,
) -> Result<Vec<String>, AliasError> {
    let canonical_safe = sanitize_path_segment(canonical_name);

    let mut aliases = Vec::new();

    let entries = std::fs::read_dir(library_root).map_err(|e| AliasError::Io {
        path: library_root.to_path_buf(),
        source: e,
        location: snafu::location!(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| AliasError::Io {
            path: library_root.to_path_buf(),
            source: e,
            location: snafu::location!(),
        })?;

        let entry_path = entry.path();

        if !entry_path.is_symlink() {
            continue;
        }

        let link_target = std::fs::read_link(&entry_path).map_err(|e| AliasError::Io {
            path: entry_path.clone(),
            source: e,
            location: snafu::location!(),
        })?;

        // The symlink target is stored as a relative path (the canonical safe name).
        // Resolve it relative to the library root for comparison.
        let resolved_target = if link_target.is_absolute() {
            link_target
        } else {
            library_root.join(&link_target)
        };

        let canonical_path = library_root.join(&canonical_safe);

        if resolved_target == canonical_path
            && let Some(name) = entry_path.file_name().and_then(|n| n.to_str())
        {
            aliases.push(name.to_owned());
        }
    }

    aliases.sort();
    Ok(aliases)
}

/// Resolve an artist name to the canonical directory path.
///
/// - If `name` is a symlink, follows it to find the canonical path.
/// - If `name` is a real directory (not a symlink), it is already canonical.
/// - The returned path is the canonical artist directory under `library_root`.
pub fn resolve_artist(library_root: &Path, name: &str) -> Result<PathBuf, AliasError> {
    let safe_name = sanitize_path_segment(name);
    let artist_path = library_root.join(&safe_name);

    if artist_path.is_symlink() {
        let link_target = std::fs::read_link(&artist_path).map_err(|e| AliasError::Io {
            path: artist_path.clone(),
            source: e,
            location: snafu::location!(),
        })?;

        let canonical = if link_target.is_absolute() {
            link_target
        } else {
            library_root.join(&link_target)
        };

        Ok(canonical)
    } else {
        Ok(artist_path)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn setup_canonical(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(sanitize_path_segment(name));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn create_alias_symlink_exists_and_points_to_canonical() {
        let dir = TempDir::new().unwrap();
        setup_canonical(&dir, "The Beatles");

        create_artist_alias(dir.path(), "The Beatles", "Beatles, The").unwrap();

        let alias_path = dir.path().join("Beatles, The");
        assert!(alias_path.is_symlink(), "alias should be a symlink");

        let target = std::fs::read_link(&alias_path).unwrap();
        assert_eq!(target, PathBuf::from("The Beatles"));
    }

    #[test]
    fn remove_alias_symlink_gone() {
        let dir = TempDir::new().unwrap();
        setup_canonical(&dir, "The Beatles");
        create_artist_alias(dir.path(), "The Beatles", "Beatles, The").unwrap();

        remove_artist_alias(dir.path(), "Beatles, The").unwrap();

        let alias_path = dir.path().join("Beatles, The");
        assert!(!alias_path.exists(), "alias should be gone after removal");
        assert!(!alias_path.is_symlink(), "alias symlink should be gone");
    }

    #[test]
    fn list_aliases_returns_all_pointing_to_canonical() {
        let dir = TempDir::new().unwrap();
        setup_canonical(&dir, "The Beatles");
        // Also create a second artist to ensure it isn't included.
        setup_canonical(&dir, "Led Zeppelin");

        create_artist_alias(dir.path(), "The Beatles", "Beatles, The").unwrap();
        create_artist_alias(dir.path(), "The Beatles", "ビートルズ").unwrap();
        // Unrelated alias should not appear.
        create_artist_alias(dir.path(), "Led Zeppelin", "LZ").unwrap();

        let aliases = list_artist_aliases(dir.path(), "The Beatles").unwrap();
        assert_eq!(aliases, vec!["Beatles, The", "ビートルズ"]);
    }

    #[test]
    fn resolve_canonical_name_returns_itself() {
        let dir = TempDir::new().unwrap();
        setup_canonical(&dir, "The Beatles");

        let resolved = resolve_artist(dir.path(), "The Beatles").unwrap();
        assert_eq!(resolved, dir.path().join("The Beatles"));
    }

    #[test]
    fn resolve_alias_returns_canonical_path() {
        let dir = TempDir::new().unwrap();
        setup_canonical(&dir, "The Beatles");
        create_artist_alias(dir.path(), "The Beatles", "Beatles, The").unwrap();

        let resolved = resolve_artist(dir.path(), "Beatles, The").unwrap();
        assert_eq!(resolved, dir.path().join("The Beatles"));
    }

    #[test]
    fn error_alias_name_conflicts_with_real_directory() {
        let dir = TempDir::new().unwrap();
        setup_canonical(&dir, "The Beatles");
        // Create a real directory where the alias would go.
        std::fs::create_dir_all(dir.path().join("Beatles, The")).unwrap();

        let err = create_artist_alias(dir.path(), "The Beatles", "Beatles, The").unwrap_err();
        assert!(
            matches!(err, AliasError::ConflictsWithDirectory { .. }),
            "expected ConflictsWithDirectory, got: {err}"
        );
    }

    #[test]
    fn create_alias_updates_existing_symlink() {
        let dir = TempDir::new().unwrap();
        setup_canonical(&dir, "The Beatles");
        setup_canonical(&dir, "Led Zeppelin");

        // First create alias pointing to The Beatles.
        create_artist_alias(dir.path(), "The Beatles", "Fab Four").unwrap();

        // Now update it to point to Led Zeppelin.
        create_artist_alias(dir.path(), "Led Zeppelin", "Fab Four").unwrap();

        let alias_path = dir.path().join("Fab Four");
        assert!(alias_path.is_symlink());
        let target = std::fs::read_link(&alias_path).unwrap();
        assert_eq!(target, PathBuf::from("Led Zeppelin"));
    }

    #[test]
    fn remove_alias_error_when_not_symlink() {
        let dir = TempDir::new().unwrap();
        // Attempt to remove a name that does not exist as a symlink.
        let err = remove_artist_alias(dir.path(), "Nonexistent Artist").unwrap_err();
        assert!(
            matches!(err, AliasError::AliasNotFound { .. }),
            "expected AliasNotFound, got: {err}"
        );
    }

    #[test]
    fn create_alias_error_when_canonical_missing() {
        let dir = TempDir::new().unwrap();
        let err = create_artist_alias(dir.path(), "Nonexistent Artist", "Some Alias").unwrap_err();
        assert!(
            matches!(err, AliasError::CanonicalNotFound { .. }),
            "expected CanonicalNotFound, got: {err}"
        );
    }
}
