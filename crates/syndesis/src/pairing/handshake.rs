// Pairing handshake: generates an API key and persists the renderer record
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand_core::OsRng;
use snafu::ResultExt;
use sqlx::SqlitePool;

use harmonia_db::repo::renderer::{self, Renderer};

use crate::error::{DatabaseSnafu, SyndesisError};

/// Generate a cryptographically random 32-byte API key, base64url-encoded.
pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 32];
    use rand_core::RngCore;
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash an API key using argon2id in PHC format.
pub fn hash_api_key(api_key: &str) -> Result<String, SyndesisError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(api_key.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| SyndesisError::Argon2 {
            message: e.to_string(),
            location: snafu::location!(),
        })
}

/// Verify an API key against a stored argon2id hash.
pub fn verify_api_key(api_key: &str, stored_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(api_key.as_bytes(), &parsed)
        .is_ok()
}

/// Parameters for initiating a pairing with a new renderer.
pub struct PairingRequest<'a> {
    pub renderer_name: &'a str,
    pub renderer_id: &'a str,
    pub cert_fingerprint: &'a str,
}

/// Outcome of a successful pairing: the plaintext API key to send to the renderer.
pub struct PairingOutcome {
    pub api_key: String,
}

/// Complete the pairing flow server-side:
/// generate an API key, hash it, persist the renderer, return the plaintext key.
pub async fn complete_pairing(
    write_pool: &SqlitePool,
    req: PairingRequest<'_>,
) -> Result<PairingOutcome, SyndesisError> {
    let api_key = generate_api_key();
    let api_key_hash = hash_api_key(&api_key)?;

    renderer::create_renderer(
        write_pool,
        req.renderer_id,
        req.renderer_name,
        &api_key_hash,
        req.cert_fingerprint,
    )
    .await
    .context(DatabaseSnafu)?;

    Ok(PairingOutcome { api_key })
}

/// Authenticate a renderer by its API key.
/// Returns the renderer record on success.
pub async fn authenticate_renderer(
    read_pool: &SqlitePool,
    write_pool: &SqlitePool,
    api_key: &str,
    cert_fingerprint: &str,
) -> Result<Renderer, SyndesisError> {
    let renderers = renderer::list_renderers(read_pool)
        .await
        .context(DatabaseSnafu)?;

    // WHY: brute-force over list since renderer count is tiny (<100);
    // avoids timing-equivalent hash-then-lookup ordering issues.
    let matched = renderers
        .into_iter()
        .find(|r| verify_api_key(api_key, &r.api_key_hash));

    let r = matched.ok_or_else(|| SyndesisError::InvalidApiKey {
        location: snafu::location!(),
    })?;

    if r.enabled == 0 {
        return Err(SyndesisError::RendererDisabled {
            location: snafu::location!(),
        });
    }

    if r.cert_fingerprint != cert_fingerprint {
        tracing::warn!(
            renderer_id = %r.id,
            stored = %r.cert_fingerprint,
            presented = %cert_fingerprint,
            "TOFU violation: cert fingerprint mismatch"
        );
        return Err(SyndesisError::FingerprintMismatch {
            location: snafu::location!(),
        });
    }

    renderer::update_last_seen(write_pool, &r.id)
        .await
        .context(DatabaseSnafu)?;

    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_is_non_empty_base64url() {
        let key = generate_api_key();
        assert!(!key.is_empty());
        assert!(
            key.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        );
        // 32 bytes → 43 base64url chars (no padding)
        assert_eq!(key.len(), 43);
    }

    #[test]
    fn api_key_round_trip_hash_and_verify() {
        let key = generate_api_key();
        let hash = hash_api_key(&key).unwrap();
        assert!(verify_api_key(&key, &hash));
        assert!(!verify_api_key("wrong-key", &hash));
    }

    #[test]
    fn hashes_differ_for_same_key() {
        let key = generate_api_key();
        let h1 = hash_api_key(&key).unwrap();
        let h2 = hash_api_key(&key).unwrap();
        assert_ne!(h1, h2);
    }
}
