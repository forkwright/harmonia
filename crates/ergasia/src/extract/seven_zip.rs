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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

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

    #[test]
    fn extract_7z_success() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = create_test_7z(
            dir.path(),
            &[("hello.txt", b"Hello, 7z!"), ("data.bin", &[0xAB; 64])],
        );
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        extract_7z(&archive_path, &output_dir).unwrap();

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

        let err = extract_7z(&archive_path, &output_dir).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::ExtractFile { .. }),
            "expected ExtractFile for a corrupt archive, got: {err}"
        );
    }

    #[test]
    fn extract_7z_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let err = extract_7z(&dir.path().join("nonexistent.7z"), &output_dir).unwrap_err();
        assert!(
            matches!(err, ErgasiaError::ExtractFile { .. }),
            "expected ExtractFile for a missing archive, got: {err}"
        );
    }

    #[test]
    fn declared_size_sums_entries() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path =
            create_test_7z(dir.path(), &[("a.bin", &[0u8; 100]), ("b.bin", &[0u8; 50])]);

        assert_eq!(declared_uncompressed_size(&archive_path).unwrap(), 150);
    }
}
