use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};

use crate::error::ExousiaError;

pub fn hash_password(password: &str) -> Result<String, ExousiaError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| ExousiaError::PasswordHash {
            error: e.to_string(),
            location: snafu::location!(),
        })
}

// WHY: test-only call spy so login()'s constant-time miss-paths can prove
// they actually exercised an Argon2id verify, without any counting overhead
// in the production binary. Thread-local: each #[tokio::test] runs its async
// body on its own dedicated OS thread under the default current-thread
// runtime, so counts never leak across concurrently running tests.
#[cfg(test)]
thread_local! {
    pub(crate) static VERIFY_CALL_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, ExousiaError> {
    #[cfg(test)]
    VERIFY_CALL_COUNT.with(|count| count.set(count.get() + 1));

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
