// Filesystem-walk inventory: derives ExtractedFile lists from what is actually on disk.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::extract::pipeline::ExtractedFile;

pub(crate) fn snapshot_paths(dir: &Path) -> HashSet<PathBuf> {
    let mut paths = HashSet::new();
    visit(dir, &mut |path, _| {
        paths.insert(path.to_path_buf());
    });
    paths
}

pub(crate) fn collect_files_excluding(
    dir: &Path,
    exclude: &HashSet<PathBuf>,
    files: &mut Vec<ExtractedFile>,
) {
    visit(dir, &mut |path, size| {
        if !exclude.contains(path) {
            files.push(ExtractedFile {
                path: path.to_path_buf(),
                size_bytes: size,
            });
        }
    });
}

fn visit(dir: &Path, on_file: &mut impl FnMut(&Path, u64)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // SAFETY: DirEntry::file_type does not follow symlinks, so a symlinked
        // directory is inventoried as a file instead of being traversed.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            visit(&path, on_file);
        } else if let Ok(meta) = path.symlink_metadata() {
            on_file(&path, meta.len());
        }
    }
}
