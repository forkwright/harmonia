use harmonia_common::ids::ApiKeyId;
use rand::Rng;
use sha2::{Digest, Sha256};

pub struct ApiKeyRecord {
    pub id: ApiKeyId,
    pub short_token: String,
    pub long_token_hash: String,
}

fn random_alphanum(len: usize) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    let mut buf = [0u8; 32];
    rng.fill_bytes(&mut buf);
    buf.iter()
        .take(len)
        .map(|b| CHARS[(*b as usize) % CHARS.len()] as char)
        .collect()
}

fn sha256_hex(input: &[u8]) -> String {
    let result = Sha256::digest(input);
    result.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn build_key(prefix: &str) -> (String, ApiKeyRecord) {
    let short_token = random_alphanum(8);
    let long_token = random_alphanum(24);
    let long_token_hash = sha256_hex(long_token.as_bytes());
    let full_key = format!("{prefix}_{short_token}_{long_token}");
    let record = ApiKeyRecord {
        id: ApiKeyId::new(),
        short_token,
        long_token_hash,
    };
    (full_key, record)
}

pub fn generate_api_key() -> (String, ApiKeyRecord) {
    build_key("hmn")
}

pub fn generate_renderer_key() -> (String, ApiKeyRecord) {
    build_key("hmn_rnd")
}

/// Validates a full API key string against the stored SHA-256 hash of the long token.
pub fn validate_api_key(key: &str, stored_hash: &str) -> bool {
    let parts: Vec<&str> = key.split('_').collect();
    let long_token = match parts.as_slice() {
        ["hmn", _short, long] => *long,
        ["hmn", "rnd", _short, long] => *long,
        _ => return false,
    };
    sha256_hex(long_token.as_bytes()) == stored_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_format_correct() {
        let (key, record) = generate_api_key();
        assert!(key.starts_with("hmn_"), "key={key}");
        let parts: Vec<&str> = key.split('_').collect();
        assert_eq!(parts.len(), 3, "expected 3 parts, got: {parts:?}");
        assert_eq!(
            parts.get(1).copied().unwrap_or_default().len(),
            8,
            "short token len"
        );
        assert_eq!(
            parts.get(2).copied().unwrap_or_default().len(),
            24,
            "long token len"
        );
        assert_eq!(
            record.short_token,
            parts.get(1).copied().unwrap_or_default()
        );
    }

    #[test]
    fn renderer_key_format_correct() {
        let (key, record) = generate_renderer_key();
        assert!(key.starts_with("hmn_rnd_"), "key={key}");
        let parts: Vec<&str> = key.split('_').collect();
        assert_eq!(parts.len(), 4, "expected 4 parts");
        assert_eq!(
            parts.get(2).copied().unwrap_or_default().len(),
            8,
            "short token len"
        );
        assert_eq!(
            parts.get(3).copied().unwrap_or_default().len(),
            24,
            "long token len"
        );
        assert_eq!(
            record.short_token,
            parts.get(2).copied().unwrap_or_default()
        );
    }

    #[test]
    fn validate_api_key_succeeds_with_correct_hash() {
        let (key, record) = generate_api_key();
        assert!(validate_api_key(&key, &record.long_token_hash));
    }

    #[test]
    fn validate_api_key_fails_with_wrong_hash() {
        let (key, _) = generate_api_key();
        assert!(!validate_api_key(&key, "wronghash"));
    }

    #[test]
    fn validate_renderer_key_succeeds() {
        let (key, record) = generate_renderer_key();
        assert!(validate_api_key(&key, &record.long_token_hash));
    }

    #[test]
    fn keys_are_unique() {
        let (k1, _) = generate_api_key();
        let (k2, _) = generate_api_key();
        assert_ne!(k1, k2);
    }
}
