// Renderer-side credential storage for API key and server cert fingerprint
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Stored renderer credentials for authenticating with a harmonia server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererCredentials {
    pub api_key: String,
    pub server_fingerprint: String,
    pub server_name: String,
    pub paired_at: String,
}

/// Load renderer credentials FROM `<cert_dir>/credentials.toml`.
/// Returns `None` if the file does not exist.
pub fn load_credentials(cert_dir: &Path) -> Result<Option<RendererCredentials>, String> {
    let path = credentials_path(cert_dir);
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let creds = toml::from_str::<RendererCredentials>(&contents)
        .map_err(|e| format!("failed to parse credentials.toml: {e}"))?;
    Ok(Some(creds))
}

/// Persist renderer credentials to `<cert_dir>/credentials.toml`.
pub fn save_credentials(cert_dir: &Path, creds: &RendererCredentials) -> Result<(), String> {
    let path = credentials_path(cert_dir);
    std::fs::create_dir_all(cert_dir)
        .map_err(|e| format!("failed to CREATE cert_dir {}: {e}", cert_dir.display()))?;
    let contents =
        toml::to_string(creds).map_err(|e| format!("failed to serialize credentials: {e}"))?;
    std::fs::write(&path, contents)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    Ok(())
}

fn credentials_path(cert_dir: &Path) -> PathBuf {
    cert_dir.join("credentials.toml")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    fn test_creds() -> RendererCredentials {
        RendererCredentials {
            api_key: "test-api-key-abc123".to_string(),
            server_fingerprint: "a".repeat(64),
            server_name: "Harmonia".to_string(),
            paired_at: "2026-03-23T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let creds = test_creds();

        save_credentials(dir.path(), &creds).unwrap();
        let loaded = load_credentials(dir.path()).unwrap().unwrap();

        assert_eq!(loaded.api_key, creds.api_key);
        assert_eq!(loaded.server_fingerprint, creds.server_fingerprint);
        assert_eq!(loaded.server_name, creds.server_name);
        assert_eq!(loaded.paired_at, creds.paired_at);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = load_credentials(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn nonexistent_dir_returns_none() {
        let path = PathBuf::from("/tmp/harmonia-test-nonexistent-8675309");
        let result = load_credentials(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn creates_dir_on_save() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested").join("certs");
        let creds = test_creds();

        save_credentials(&nested, &creds).unwrap();
        assert!(nested.join("credentials.toml").exists());
    }
}
