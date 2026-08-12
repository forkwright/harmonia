use aggelmata::ids::ApiKeyId;
use rand::Rng;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

// WHY: wire DTO — API key fields returned from the database.
pub struct ApiKeyRecord {
    pub id: ApiKeyId,
    pub short_token: String,
    pub long_token_hash: String,
}

fn random_alphanum(len: usize) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    // WHY: the entropy buffer is sized to the request — a fixed buffer would
    // silently truncate output (and entropy) for any longer token length.
    let mut buf = vec![0u8; len];
    rng.fill_bytes(&mut buf);
    buf.iter()
        .map(|b| CHARS[(*b as usize) % CHARS.len()] as char)
        .collect()
}

fn sha256_hex(input: &[u8]) -> String {
    let result = Sha256::digest(input);
    result.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
        write!(s, "{b:02x}").ok();
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
///
/// WARNING: the hash comparison MUST stay constant-time (`ConstantTimeEq::ct_eq`).
/// A plain `==` on the hex digests short-circuits on the first differing byte and
/// leaks a timing side-channel on this authentication path.
pub fn validate_api_key(key: &str, stored_hash: &str) -> bool {
    let parts: Vec<&str> = key.split('_').collect();
    let long_token = match parts.as_slice() {
        ["hmn", _short, long] => *long,
        ["hmn", "rnd", _short, long] => *long,
        _ => return false,
    };
    // NOTE: ct_eq on differing-length slices returns false without inspecting
    // contents; both sides are 64 hex chars by construction, so no length leak.
    sha256_hex(long_token.as_bytes())
        .as_bytes()
        .ct_eq(stored_hash.as_bytes())
        .into()
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
    fn validate_api_key_fails_with_near_miss_hash() {
        let (key, record) = generate_api_key();
        let mut near_miss = record.long_token_hash.clone();
        let last = near_miss.pop().map(|c| if c == '0' { '1' } else { '0' });
        near_miss.push(last.unwrap_or('0'));
        assert_ne!(near_miss, record.long_token_hash);
        assert!(!validate_api_key(&key, &near_miss));
    }

    #[test]
    fn validate_api_key_fails_with_empty_stored_hash() {
        let (key, _) = generate_api_key();
        assert!(!validate_api_key(&key, ""));
    }

    #[test]
    fn validate_renderer_key_succeeds() {
        let (key, record) = generate_renderer_key();
        assert!(validate_api_key(&key, &record.long_token_hash));
    }

    #[test]
    fn random_alphanum_honors_lengths_beyond_32() {
        for len in [0, 1, 8, 24, 32, 40, 100] {
            assert_eq!(random_alphanum(len).len(), len, "len={len}");
        }
    }

    #[test]
    fn keys_are_unique() {
        let (k1, _) = generate_api_key();
        let (k2, _) = generate_api_key();
        assert_ne!(k1, k2);
    }
}
