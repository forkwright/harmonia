use std::fs::File;
use std::path::Path;

use crate::error::ErgasiaError;
use crate::extract::pipeline::ExtractedFile;

pub(crate) fn extract_zip(
    archive_path: &Path,
    output_dir: &Path,
) -> Result<Vec<ExtractedFile>, ErgasiaError> {
    let file = File::open(archive_path).map_err(|e| {
        crate::error::OpenArchiveSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        crate::error::OpenArchiveSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    archive.extract(output_dir).map_err(|e| {
        crate::error::ExtractFileSnafu {
            path: archive_path.to_path_buf(),
            error: e.to_string(),
        }
        .build()
    })?;

    let mut files = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| {
            crate::error::ExtractFileSnafu {
                path: archive_path.to_path_buf(),
                error: e.to_string(),
            }
            .build()
        })?;

        if !entry.is_dir() {
            let name = entry.name().to_string();
            let size = entry.size();
            files.push(ExtractedFile {
                path: output_dir.join(name),
                size_bytes: size,
            });
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn extract_zip_archive() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("hello.txt", options).unwrap();
            writer.write_all(b"Hello, World!").unwrap();
            writer.start_file("subdir/nested.txt", options).unwrap();
            writer.write_all(b"Nested content").unwrap();
            writer.finish().unwrap();
        }

        let files = extract_zip(&zip_path, &output_dir).unwrap();
        assert_eq!(files.len(), 2);

        let extracted_hello = output_dir.join("hello.txt");
        assert!(extracted_hello.exists());
        assert_eq!(
            std::fs::read_to_string(&extracted_hello).unwrap(),
            "Hello, World!"
        );
    }
}
