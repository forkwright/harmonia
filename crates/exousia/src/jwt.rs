use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::{ExousiaError, JwtEncodeSnafu};
use crate::user::User;

fn new_jti() -> String {
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub jti: String,
    pub role: String,
    pub display_name: String,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn create_access_token(
    user: &User,
    secret: &[u8],
    ttl_secs: u64,
) -> Result<String, ExousiaError> {
    let now = unix_now();
    let claims = Claims {
        sub: user.id.into_uuid().to_string(),
        iss: "harmonia".to_string(),
        aud: "harmonia-clients".to_string(),
        exp: now + ttl_secs,
        iat: now,
        jti: new_jti(),
        role: user.role.as_str().to_string(),
        display_name: user.display_name.clone(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .context(JwtEncodeSnafu)
}

pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims, ExousiaError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&["harmonia"]);
    validation.set_audience(&["harmonia-clients"]);

    decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map(|data| data.claims)
        .map_err(|e| {
            if *e.kind() == jsonwebtoken::errors::ErrorKind::ExpiredSignature {
                ExousiaError::TokenExpired {
                    location: snafu::location!(),
                }
            } else {
                ExousiaError::TokenInvalid {
                    error: e.to_string(),
                    location: snafu::location!(),
                }
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::UserRole;
    use themelion::ids::UserId;

    fn test_user() -> User {
        User {
            id: UserId::new(),
            username: "alice".to_string(),
            display_name: "Alice".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Member,
            is_active: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_login_at: None,
        }
    }

    #[test]
    fn roundtrip_access_token() {
        let secret = b"test-secret-32-bytes-long-enough!";
        let user = test_user();
        let token = create_access_token(&user, secret, 900).unwrap();
        let claims = validate_token(&token, secret).unwrap();
        assert_eq!(claims.iss, "harmonia");
        assert_eq!(claims.aud, "harmonia-clients");
        assert_eq!(claims.role, "member");
        assert_eq!(claims.display_name, "Alice");
        assert_eq!(claims.sub, user.id.into_uuid().to_string());
    }

    #[test]
    fn expired_token_returns_token_expired() {
        let secret = b"test-secret-32-bytes-long-enough!";
        let user = test_user();
        let now = unix_now();
        let claims = Claims {
            sub: user.id.into_uuid().to_string(),
            iss: "harmonia".to_string(),
            aud: "harmonia-clients".to_string(),
            exp: now - 100,
            iat: now - 1000,
            jti: new_jti(),
            role: "member".to_string(),
            display_name: "Alice".to_string(),
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();
        let err = validate_token(&token, secret).unwrap_err();
        assert!(matches!(err, ExousiaError::TokenExpired { .. }));
    }

    #[test]
    fn wrong_secret_returns_token_invalid() {
        let secret = b"test-secret-32-bytes-long-enough!";
        let user = test_user();
        let token = create_access_token(&user, secret, 900).unwrap();
        let err = validate_token(&token, b"wrong-secret-for-validation!!!!!").unwrap_err();
        assert!(matches!(err, ExousiaError::TokenInvalid { .. }));
    }

    #[test]
    fn admin_role_encoded_correctly() {
        let secret = b"test-secret-32-bytes-long-enough!";
        let mut user = test_user();
        user.role = UserRole::Admin;
        let token = create_access_token(&user, secret, 900).unwrap();
        let claims = validate_token(&token, secret).unwrap();
        assert_eq!(claims.role, "admin");
    }
}
