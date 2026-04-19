use std::sync::Arc;

use apotheke::{DbPools, repo::user as db};
use horismos::ExousiaConfig;
use rand::Rng;
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use themelion::ids::{ApiKeyId, UserId};

use crate::{
    AuthService, TokenPair, api_key,
    error::{
        ApiKeyRevokedSnafu, DatabaseSnafu, ExousiaError, InvalidCredentialsSnafu, UserInactiveSnafu,
    },
    jwt,
    middleware::{AuthMethod, AuthenticatedUser},
    password,
    user::{CreateUserRequest, User, UserRole},
};

pub struct ExousiaServiceImpl {
    pools: Arc<DbPools>,
    config: ExousiaConfig,
}

impl ExousiaServiceImpl {
    pub fn new(pools: Arc<DbPools>, config: ExousiaConfig) -> Self {
        Self { pools, config }
    }

    pub fn pools(&self) -> &DbPools {
        &self.pools
    }
}

fn sha256_hex(input: &[u8]) -> String {
    let result = Sha256::digest(input);
    result.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn generate_refresh_token() -> (String, String) {
    let mut rng = rand::rng();
    let mut bytes = [0u8; 64];
    rng.fill_bytes(&mut bytes);
    let token: String = bytes.iter().fold(String::with_capacity(128), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    });
    let hash = sha256_hex(token.as_bytes());
    (token, hash)
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = seconds_to_datetime(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn seconds_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = hours / 24;
    let (y, mo, d) = days_to_ymd(days);
    (y, mo, d, h, mi, s)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    let mut remaining = days;
    loop {
        let leap = is_leap(y);
        let days_in_year = if leap { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut mo = 1u64;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        mo += 1;
    }
    (y, mo, remaining + 1)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn add_days_to_iso_now(days: u64) -> String {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let future_secs = now_secs + days * 86400;
    let (y, mo, d, h, mi, s) = seconds_to_datetime(future_secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn user_id_to_bytes(id: UserId) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn bytes_to_user_id(bytes: &[u8]) -> Option<UserId> {
    uuid::Uuid::from_slice(bytes).ok().map(UserId::from_uuid)
}

fn db_user_to_domain(u: db::User) -> Option<User> {
    let id = bytes_to_user_id(&u.id)?;
    let role = UserRole::parse(&u.role)?;
    Some(User {
        id,
        username: u.username,
        display_name: u.display_name,
        password_hash: u.password_hash,
        role,
        is_active: u.is_active != 0,
        created_at: u.created_at,
        last_login_at: u.last_login_at,
    })
}

impl AuthService for ExousiaServiceImpl {
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, ExousiaError> {
        let row = db::get_user_by_username(&self.pools.read, username)
            .await
            .context(DatabaseSnafu)?;
        let row = row.ok_or_else(|| ExousiaError::InvalidCredentials {
            location: snafu::location!(),
        })?;
        let user = db_user_to_domain(row).ok_or_else(|| ExousiaError::InvalidCredentials {
            location: snafu::location!(),
        })?;
        if !user.is_active {
            return Err(UserInactiveSnafu.build());
        }
        if !password::verify_password(password, &user.password_hash)? {
            return Err(InvalidCredentialsSnafu.build());
        }
        let access_token = jwt::create_access_token(
            &user,
            self.config.jwt_secret.as_bytes(),
            self.config.access_token_ttl_secs,
        )?;
        let (refresh_token, token_hash) = generate_refresh_token();
        let token_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        let now = now_iso();
        let expires_at = add_days_to_iso_now(self.config.refresh_token_ttl_days);
        let refresh_row = db::RefreshToken {
            id: token_id,
            user_id: user_id_to_bytes(user.id),
            token_hash,
            created_at: now.clone(),
            expires_at,
            revoked: 0,
        };
        db::insert_refresh_token(&self.pools.write, &refresh_row)
            .await
            .context(DatabaseSnafu)?;
        db::record_login(&self.pools.write, &user_id_to_bytes(user.id), &now)
            .await
            .context(DatabaseSnafu)?;
        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }

    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, ExousiaError> {
        let token_hash = sha256_hex(refresh_token.as_bytes());
        let row = db::get_refresh_token_by_hash(&self.pools.read, &token_hash)
            .await
            .context(DatabaseSnafu)?;
        let row = row.ok_or_else(|| ExousiaError::TokenInvalid {
            error: "refresh token not found".to_string(),
            location: snafu::location!(),
        })?;
        if row.revoked != 0 {
            return Err(ExousiaError::TokenInvalid {
                error: "refresh token revoked".to_string(),
                location: snafu::location!(),
            });
        }
        let now = now_iso();
        if row.expires_at < now {
            return Err(ExousiaError::TokenExpired {
                location: snafu::location!(),
            });
        }
        let user_row = db::get_user(&self.pools.read, &row.user_id)
            .await
            .context(DatabaseSnafu)?
            .ok_or_else(|| ExousiaError::TokenInvalid {
                error: "user not found for refresh token".to_string(),
                location: snafu::location!(),
            })?;
        let user = db_user_to_domain(user_row).ok_or_else(|| ExousiaError::TokenInvalid {
            error: "invalid user data".to_string(),
            location: snafu::location!(),
        })?;
        if !user.is_active {
            return Err(UserInactiveSnafu.build());
        }
        db::revoke_refresh_token(&self.pools.write, &row.id)
            .await
            .context(DatabaseSnafu)?;
        let access_token = jwt::create_access_token(
            &user,
            self.config.jwt_secret.as_bytes(),
            self.config.access_token_ttl_secs,
        )?;
        let (new_refresh_token, new_hash) = generate_refresh_token();
        let token_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        let new_row = db::RefreshToken {
            id: token_id,
            user_id: row.user_id,
            token_hash: new_hash,
            created_at: now_iso(),
            expires_at: add_days_to_iso_now(self.config.refresh_token_ttl_days),
            revoked: 0,
        };
        db::insert_refresh_token(&self.pools.write, &new_row)
            .await
            .context(DatabaseSnafu)?;
        Ok(TokenPair {
            access_token,
            refresh_token: new_refresh_token,
        })
    }

    async fn logout(&self, refresh_token: &str) -> Result<(), ExousiaError> {
        let token_hash = sha256_hex(refresh_token.as_bytes());
        let row = db::get_refresh_token_by_hash(&self.pools.read, &token_hash)
            .await
            .context(DatabaseSnafu)?;
        if let Some(row) = row {
            db::revoke_refresh_token(&self.pools.write, &row.id)
                .await
                .context(DatabaseSnafu)?;
        }
        Ok(())
    }

    async fn validate_bearer(&self, token: &str) -> Result<AuthenticatedUser, ExousiaError> {
        let claims = jwt::validate_token(token, self.config.jwt_secret.as_bytes())?;
        let uuid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| ExousiaError::TokenInvalid {
            error: "invalid sub claim".to_string(),
            location: snafu::location!(),
        })?;
        let user_id = UserId::from_uuid(uuid);
        let role = UserRole::parse(&claims.role).ok_or_else(|| ExousiaError::TokenInvalid {
            error: "invalid role claim".to_string(),
            location: snafu::location!(),
        })?;
        Ok(AuthenticatedUser {
            user_id,
            role,
            auth_method: AuthMethod::Bearer,
        })
    }

    async fn validate_api_key(&self, key: &str) -> Result<AuthenticatedUser, ExousiaError> {
        let parts: Vec<&str> = key.split('_').collect();
        let short_token = match parts.as_slice() {
            ["hmn", short, _long] => *short,
            ["hmn", "rnd", short, _long] => *short,
            _ => {
                return Err(ExousiaError::TokenInvalid {
                    error: "invalid API key format".to_string(),
                    location: snafu::location!(),
                });
            }
        };
        let row = db::get_api_key_by_short_token(&self.pools.read, short_token)
            .await
            .context(DatabaseSnafu)?
            .ok_or_else(|| ExousiaError::TokenInvalid {
                error: "API key not found".to_string(),
                location: snafu::location!(),
            })?;
        if row.revoked != 0 {
            return Err(ApiKeyRevokedSnafu.build());
        }
        if !api_key::validate_api_key(key, &row.long_token_hash) {
            return Err(ExousiaError::TokenInvalid {
                error: "API key validation failed".to_string(),
                location: snafu::location!(),
            });
        }
        let user_row = db::get_user(&self.pools.read, &row.user_id)
            .await
            .context(DatabaseSnafu)?
            .ok_or_else(|| ExousiaError::TokenInvalid {
                error: "user not found for API key".to_string(),
                location: snafu::location!(),
            })?;
        let user = db_user_to_domain(user_row).ok_or_else(|| ExousiaError::TokenInvalid {
            error: "invalid user data".to_string(),
            location: snafu::location!(),
        })?;
        if !user.is_active {
            return Err(UserInactiveSnafu.build());
        }
        db::update_api_key_last_used(&self.pools.write, &row.id, &now_iso())
            .await
            .context(DatabaseSnafu)?;
        Ok(AuthenticatedUser {
            user_id: user.id,
            role: user.role,
            auth_method: AuthMethod::ApiKey,
        })
    }

    async fn create_user(&self, req: CreateUserRequest) -> Result<User, ExousiaError> {
        let id = UserId::new();
        let hash = password::hash_password(&req.password)?;
        let now = now_iso();
        let row = db::User {
            id: user_id_to_bytes(id),
            username: req.username.clone(),
            display_name: req.display_name.clone(),
            password_hash: hash.clone(),
            role: req.role.as_str().to_string(),
            is_active: 1,
            created_at: now.clone(),
            last_login_at: None,
        };
        db::insert_user(&self.pools.write, &row)
            .await
            .context(DatabaseSnafu)?;
        Ok(User {
            id,
            username: req.username,
            display_name: req.display_name,
            password_hash: hash,
            role: req.role,
            is_active: true,
            created_at: now,
            last_login_at: None,
        })
    }

    async fn create_api_key(&self, user_id: UserId, label: &str) -> Result<String, ExousiaError> {
        let (full_key, record) = api_key::generate_api_key();
        let now = now_iso();
        let row = db::ApiKey {
            id: record.id.as_bytes().to_vec(),
            user_id: user_id_to_bytes(user_id),
            short_token: record.short_token,
            long_token_hash: record.long_token_hash,
            label: label.to_string(),
            created_at: now,
            last_used_at: None,
            revoked: 0,
        };
        db::insert_api_key(&self.pools.write, &row)
            .await
            .context(DatabaseSnafu)?;
        Ok(full_key)
    }

    async fn revoke_api_key(&self, key_id: ApiKeyId) -> Result<(), ExousiaError> {
        db::revoke_api_key(&self.pools.write, key_id.as_bytes())
            .await
            .context(DatabaseSnafu)?;
        Ok(())
    }
}
