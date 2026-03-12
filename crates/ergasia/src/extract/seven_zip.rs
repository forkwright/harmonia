use std::path::Path;

use crate::error::ErgasiaError;
use crate::extract::pipeline::ExtractedFile;

pub fn extract_7z(
    archive_path: &Path,
    output_dir: &Path,
) -> Result<Vec<ExtractedFile>, ErgasiaError> {
    sevenz_rust2::decompress_file(archive_path, output_dir).map_err(|e| {
        crate::error::ExtractFileSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    let mut files = Vec::new();
    collect_files(output_dir, &mut files);
    Ok(files)
}

fn collect_files(dir: &Path, files: &mut Vec<ExtractedFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if let Ok(meta) = path.metadata() {
            files.push(ExtractedFile {
                path,
                size_bytes: meta.len(),
            });
        }
    }
}
