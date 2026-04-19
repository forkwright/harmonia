use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand_core::OsRng;

use crate::error::ExousiaError;

pub(crate) fn hash_password(password: &str) -> Result<String, ExousiaError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ExousiaError::PasswordHash {
            error: e.to_string(),
            location: snafu::location!(),
        })
}

pub(crate) fn verify_password(password: &str, hash: &str) -> Result<bool, ExousiaError> {
    let parsed = PasswordHash::new(hash).map_err(|e| ExousiaError::PasswordHash {
        error: e.to_string(),
        location: snafu::location!(),
    })?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_correct_password() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(verify_password("correct-horse-battery-staple", &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails_verification() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn hashes_are_unique_for_same_password() {
        let h1 = hash_password("password").unwrap();
        let h2 = hash_password("password").unwrap();
        assert_ne!(h1, h2); // different salts
    }
}
