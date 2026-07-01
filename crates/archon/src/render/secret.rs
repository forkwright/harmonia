// Owner-only (0600) persistence for secret files: TLS keys and pairing credentials.

use std::io::Write as _;
use std::path::Path;

const SECRET_MODE: u32 = 0o600;

/// Write `contents` to `path` with mode 0600, never exposing a readable window.
///
/// The bytes land in a same-directory temp file created with 0600, then move
/// into place atomically, so no reader ever observes the secret with loose
/// permissions or half-written contents.
pub(crate) fn write_secret_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let tmp_path = path.with_extension("tmp");
    // WHY: a stale temp file FROM an interrupted earlier write would make create_new fail
    if tmp_path.exists() {
        std::fs::remove_file(&tmp_path)?;
    }
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(SECRET_MODE)
            .open(&tmp_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        // WHY: best-effort cleanup; the failed rename error is the one worth surfacing
        std::fs::remove_file(&tmp_path).ok();
        return Err(e);
    }
    Ok(())
}

/// Tighten `path` to mode 0600 when group/other bits are present.
///
/// Returns `true` when a repair happened so the caller can log it.
pub(crate) fn ensure_secret_file_mode(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    if metadata.permissions().mode() & 0o077 == 0 {
        return Ok(false);
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(SECRET_MODE))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use tempfile::TempDir;

    use super::*;

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    }

    #[test]
    fn write_secret_file_sets_mode_0600() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("secret.der");

        write_secret_file(&path, b"key material").expect("write");

        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(std::fs::read(&path).expect("read"), b"key material");
    }

    #[test]
    fn write_secret_file_overwrites_and_keeps_mode() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("secret.der");

        write_secret_file(&path, b"first").expect("first write");
        write_secret_file(&path, b"second").expect("second write");

        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(std::fs::read(&path).expect("read"), b"second");
    }

    #[test]
    fn ensure_secret_file_mode_repairs_loose_permissions() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("secret.der");
        std::fs::write(&path, b"leaky").expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod");

        let repaired = ensure_secret_file_mode(&path).expect("repair");

        assert!(repaired);
        assert_eq!(mode_of(&path), 0o600);
    }

    #[test]
    fn ensure_secret_file_mode_leaves_tight_permissions_alone() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("secret.der");
        write_secret_file(&path, b"tight").expect("write");

        let repaired = ensure_secret_file_mode(&path).expect("check");

        assert!(!repaired);
        assert_eq!(mode_of(&path), 0o600);
    }
}
