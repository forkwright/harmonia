use std::path::Path;

use crate::error::ErgasiaError;

pub fn extract_7z(archive_path: &Path, output_dir: &Path) -> Result<(), ErgasiaError> {
    sevenz_rust2::decompress_file(archive_path, output_dir).map_err(|e| {
        crate::error::ExtractFileSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    Ok(())
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
