use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

const RAR_MAGIC: [u8; 4] = [0x52, 0x61, 0x72, 0x21];
const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
const SEVENZ_MAGIC: [u8; 4] = [0x37, 0x7A, 0xBC, 0xAF];

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveFormat {
    Rar,
    Zip,
    SevenZip,
}

impl std::fmt::Display for ArchiveFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rar => f.write_str("RAR"),
            Self::Zip => f.write_str("ZIP"),
            Self::SevenZip => f.write_str("7z"),
        }
    }
}

pub fn has_archive_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            ext == "rar" || ext == "zip" || ext == "7z"
        })
        .unwrap_or(false)
}

pub fn detect_archive_format(path: &Path) -> Option<ArchiveFormat> {
    if !has_archive_extension(path) {
        return None;
    }

    detect_by_magic_bytes(path)
}

pub fn detect_by_magic_bytes(path: &Path) -> Option<ArchiveFormat> {
    let mut file = File::open(path).ok()?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).ok()?;

    match magic {
        RAR_MAGIC => Some(ArchiveFormat::Rar),
        ZIP_MAGIC => Some(ArchiveFormat::Zip),
        SEVENZ_MAGIC => Some(ArchiveFormat::SevenZip),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detect_rar_magic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rar");
        let mut f = File::create(&path).unwrap();
        f.write_all(&RAR_MAGIC).unwrap();
        f.write_all(&[0u8; 100]).unwrap();

        assert_eq!(detect_archive_format(&path), Some(ArchiveFormat::Rar));
    }

    #[test]
    fn detect_zip_magic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.zip");
        let mut f = File::create(&path).unwrap();
        f.write_all(&ZIP_MAGIC).unwrap();
        f.write_all(&[0u8; 100]).unwrap();

        assert_eq!(detect_archive_format(&path), Some(ArchiveFormat::Zip));
    }

    #[test]
    fn detect_7z_magic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.7z");
        let mut f = File::create(&path).unwrap();
        f.write_all(&SEVENZ_MAGIC).unwrap();
        f.write_all(&[0u8; 100]).unwrap();

        assert_eq!(detect_archive_format(&path), Some(ArchiveFormat::SevenZip));
    }

    #[test]
    fn reject_unknown_magic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.zip");
        let mut f = File::create(&path).unwrap();
        f.write_all(&[0xFF, 0xFE, 0xFD, 0xFC]).unwrap();
        f.write_all(&[0u8; 100]).unwrap();

        assert_eq!(detect_archive_format(&path), None);
    }

    #[test]
    fn reject_non_archive_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(&ZIP_MAGIC).unwrap();
        f.write_all(&[0u8; 100]).unwrap();

        assert_eq!(detect_archive_format(&path), None);
    }
}
