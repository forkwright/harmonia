use std::sync::Arc;

use apotheke::DbPools;
use apotheke::repo::user as db;
use horismos::ExousiaConfig;
use rand::Rng;
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use themelion::ids::{ApiKeyId, UserId};

use crate::error::{
    ApiKeyRevokedSnafu, DatabaseSnafu, ExousiaError, InvalidCredentialsSnafu, InvalidPasswordSnafu,
    UserInactiveSnafu,
};
use crate::middleware::{AuthMethod, AuthenticatedUser};
use crate::user::{CreateUserRequest, User, UserRole};
use crate::{AuthService, TokenPair, api_key, jwt, password};

// WHY: argon2 cost scales with input length — an unbounded password is a
// cheap CPU-exhaustion vector, so both hashing and verification are capped.
const MAX_PASSWORD_BYTES: usize = 256;
const MIN_PASSWORD_CHARS: usize = 8;

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
        // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
        write!(s, "{b:02x}").ok();
        s
    })
}

fn generate_refresh_token() -> (String, String) {
    let mut rng = rand::rng();
    let mut bytes = [0u8; 64];
    rng.fill_bytes(&mut bytes);
    let token: String = bytes.iter().fold(String::with_capacity(128), |mut s, b| {
        use std::fmt::Write;
        // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
        write!(s, "{b:02x}").ok();
        s
    });
    let hash = sha256_hex(token.as_bytes());
    (token, hash)
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default() // WHY: SystemTime cannot be before UNIX_EPOCH on any supported platform
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
    (year.checked_rem(4) == Some(0) && year.checked_rem(100) != Some(0))
        || year.checked_rem(400) == Some(0)
}

// WHY: 9999-12-31T23:59:59Z — the last instant representable in the fixed-width
// four-digit-year ISO format; also keeps days_to_ymd's per-year loop bounded.
const MAX_EXPIRY_EPOCH_SECS: u64 = 253_402_300_799;

fn add_days_to_iso_now(days: u64) -> String {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default() // WHY: SystemTime cannot be before UNIX_EPOCH on any supported platform
        .as_secs();
    // WHY: an operator-configured TTL can overflow the multiply; a silent wrap
    // would mint tokens that expire near the epoch, so clamp to the format max.
    let future_secs = days
        .checked_mul(86400)
        .and_then(|d| now_secs.checked_add(d))
        .unwrap_or(u64::MAX)
        .min(MAX_EXPIRY_EPOCH_SECS);
    let (y, mo, d, h, mi, s) = seconds_to_datetime(future_secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Parses a `YYYY-MM-DDTHH:MM:SSZ` timestamp (the format `now_iso` /
/// `add_days_to_iso_now` produce) into epoch seconds.
///
/// WHY: expiry checks compare numerically instead of relying on the implicit
/// fixed-width lexicographic-ordering invariant of the stored strings.
fn iso_to_epoch_secs(iso: &str) -> Option<u64> {
    let bytes = iso.as_bytes();
    if bytes.len() != 20 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return None;
    }
    if bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || bytes.get(19) != Some(&b'Z')
    {
        return None;
    }
    let num = |range: std::ops::Range<usize>| iso.get(range)?.parse::<u64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if y < 1970 || !(1..=12).contains(&mo) || d == 0 || h > 23 || mi > 59 || s > 59 {
        return None;
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
    let days_in_month = *month_days.get(usize::try_from(mo).ok()?.checked_sub(1)?)?;
    if d > days_in_month {
        return None;
    }
    let mut days: u64 = 0;
    for year in 1970..y {
        days += if is_leap(year) { 366 } else { 365 };
    }
    days += month_days
        .iter()
        .take(usize::try_from(mo).ok()? - 1)
        .sum::<u64>();
    days += d - 1;
    Some(days * 86400 + h * 3600 + mi * 60 + s)
}

fn user_id_to_bytes(id: UserId) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn bytes_to_user_id(bytes: &[u8]) -> Option<UserId> {
    uuid::Uuid::from_slice(bytes).ok().map(UserId::from_uuid)
}

// NOTE: callers collapse a `None` into an opaque auth failure — this is the
// handled site, so row corruption is logged here (one log per error chain).
fn db_user_to_domain(u: db::User) -> Option<User> {
    let Some(id) = bytes_to_user_id(&u.id) else {
        tracing::error!(user_id = ?u.id, username = %u.username, "corrupt user row: invalid id bytes");
        return None;
    };
    let Some(role) = UserRole::parse(&u.role) else {
        tracing::error!(user_id = ?u.id, username = %u.username, role = %u.role, "corrupt user row: unknown role");
        return None;
    };
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

// WHY: a fixed, precomputed Argon2id PHC hash with no corresponding real
// account. login()'s username-miss path verifies the caller's password
// against this sentinel instead of skipping straight to an error, so a
// non-existent username costs the same Argon2id work (and wall-clock time)
// as a real wrong-password verify — otherwise response latency alone
// distinguishes existing from non-existing usernames (CWE-203 enumeration).
const SENTINEL_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$rCzKZ0rnCdm3pdE/YBMRJQ$3ddRg+rFxifluJSSSHl4wlaCVLzmVbRe1uZmGtKrQEI";

impl AuthService for ExousiaServiceImpl {
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, ExousiaError> {
        // WHY: cap argon2 input before any hashing work — an oversized password
        // is rejected in constant, bounded time (CPU-DoS guard).
        if password.len() > MAX_PASSWORD_BYTES {
            return Err(InvalidCredentialsSnafu.build());
        }
        let row = db::get_user_by_username(&self.pools.read, username)
            .await
            .context(DatabaseSnafu)?;
        let candidate = row.and_then(db_user_to_domain);
        // WHY: verify against the real hash when the user exists, otherwise
        // against the fixed sentinel hash — both branches run exactly one
        // Argon2id verify, so a non-existent (or too-corrupt-to-convert)
        // username costs the same wall-clock time as a real wrong-password
        // attempt (CWE-203 username-enumeration guard). Verifying BEFORE the
        // is_active check keeps inactive accounts indistinguishable too.
        let stored_hash = candidate
            .as_ref()
            .map_or(SENTINEL_PASSWORD_HASH, |user| user.password_hash.as_str());
        let password_ok = password::verify_password(password, stored_hash)?;
        let user = match candidate {
            Some(user) if password_ok => user,
            _ => return Err(InvalidCredentialsSnafu.build()),
        };
        if !user.is_active {
            tracing::warn!(user_id = %user.id.into_uuid(), "login attempt on inactive account");
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

    // INVARIANT: check-revoke-insert runs inside one BEGIN IMMEDIATE transaction
    // on the write pool. A concurrent refresh of the same token blocks at begin,
    // then observes revoked = 1 and fails — a refresh token can be used once.
    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, ExousiaError> {
        let token_hash = sha256_hex(refresh_token.as_bytes());
        let mut tx = apotheke::begin_immediate(&self.pools.write)
            .await
            .context(DatabaseSnafu)?;
        let row = db::get_refresh_token_by_hash(&mut *tx, &token_hash)
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
        // WHY: compare numerically — lexicographic string ordering only works
        // while both sides stay fixed-width, an invariant nothing enforces.
        // A malformed stored expiry fails closed as expired.
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default() // WHY: SystemTime cannot be before UNIX_EPOCH on any supported platform
            .as_secs();
        let expires_secs = iso_to_epoch_secs(&row.expires_at);
        if expires_secs.is_none() {
            tracing::error!("corrupt refresh_tokens row: unparseable expires_at");
        }
        if expires_secs.is_none_or(|e| e < now_secs) {
            return Err(ExousiaError::TokenExpired {
                location: snafu::location!(),
            });
        }
        let user_row = db::get_user(&mut *tx, &row.user_id)
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
        db::revoke_refresh_token(&mut *tx, &row.id)
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
        db::insert_refresh_token(&mut *tx, &new_row)
            .await
            .context(DatabaseSnafu)?;
        apotheke::commit_tx(tx).await.context(DatabaseSnafu)?;
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

    // INVARIANT: the JWT is authoritative only for identity (signature + sub);
    // role and is_active come from the live user row so revocation and role
    // demotion take effect within the token's lifetime, matching validate_api_key.
    async fn validate_bearer(&self, token: &str) -> Result<AuthenticatedUser, ExousiaError> {
        let claims = jwt::validate_token(token, self.config.jwt_secret.as_bytes())?;
        let uuid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| ExousiaError::TokenInvalid {
            error: "invalid sub claim".to_string(),
            location: snafu::location!(),
        })?;
        let user_id = UserId::from_uuid(uuid);
        let user_row = db::get_user(&self.pools.read, &user_id_to_bytes(user_id))
            .await
            .context(DatabaseSnafu)?
            .ok_or_else(|| ExousiaError::TokenInvalid {
                error: "user not found for bearer token".to_string(),
                location: snafu::location!(),
            })?;
        let user = db_user_to_domain(user_row).ok_or_else(|| ExousiaError::TokenInvalid {
            error: "invalid user data".to_string(),
            location: snafu::location!(),
        })?;
        if !user.is_active {
            return Err(UserInactiveSnafu.build());
        }
        Ok(AuthenticatedUser {
            user_id: user.id,
            role: user.role,
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
        if req.password.chars().count() < MIN_PASSWORD_CHARS {
            return Err(InvalidPasswordSnafu {
                reason: format!("must be at least {MIN_PASSWORD_CHARS} characters"),
            }
            .build());
        }
        if req.password.len() > MAX_PASSWORD_BYTES {
            return Err(InvalidPasswordSnafu {
                reason: format!("must be at most {MAX_PASSWORD_BYTES} bytes"),
            }
            .build());
        }
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

    async fn revoke_api_key(&self, user_id: UserId, key_id: ApiKeyId) -> Result<(), ExousiaError> {
        db::revoke_api_key_for_user(
            &self.pools.write,
            &user_id_to_bytes(user_id),
            key_id.as_bytes(),
        )
        .await
        .context(DatabaseSnafu)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    use super::*;
    use crate::user::CreateUserRequest;

    fn corrupt_row(id: Vec<u8>, role: &str) -> db::User {
        db::User {
            id,
            username: "testuser".to_string(),
            display_name: "Test".to_string(),
            password_hash: "$argon2id$placeholder".to_string(),
            role: role.to_string(),
            is_active: 1,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_login_at: None,
        }
    }

    #[test]
    fn db_user_to_domain_returns_none_for_malformed_uuid() {
        let row = corrupt_row(vec![1, 2, 3], "member");
        assert!(db_user_to_domain(row).is_none());
    }

    #[test]
    fn db_user_to_domain_returns_none_for_unknown_role() {
        let row = corrupt_row(uuid::Uuid::now_v7().as_bytes().to_vec(), "superuser");
        assert!(db_user_to_domain(row).is_none());
    }

    #[test]
    fn db_user_to_domain_accepts_valid_row() {
        let row = corrupt_row(uuid::Uuid::now_v7().as_bytes().to_vec(), "member");
        let user = db_user_to_domain(row).expect("valid row must convert");
        assert_eq!(user.role, UserRole::Member);
        assert!(user.is_active);
    }

    #[test]
    fn add_days_to_iso_now_clamps_on_overflow() {
        let clamped = add_days_to_iso_now(u64::MAX / 86400 + 1);
        assert_eq!(clamped, "9999-12-31T23:59:59Z");
    }

    #[test]
    fn iso_to_epoch_secs_roundtrips_now() {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let iso = now_iso();
        let parsed = iso_to_epoch_secs(&iso).expect("now_iso output must parse");
        assert!(
            parsed.abs_diff(now_secs) <= 1,
            "parsed={parsed} now={now_secs}"
        );
    }

    #[test]
    fn iso_to_epoch_secs_orders_across_day_boundary() {
        let before = iso_to_epoch_secs("2028-02-28T23:59:59Z").unwrap();
        let after = iso_to_epoch_secs("2028-02-29T00:00:00Z").unwrap();
        assert_eq!(after - before, 1, "leap-day rollover must be contiguous");
    }

    #[test]
    fn iso_to_epoch_secs_rejects_malformed() {
        for bad in [
            "",
            "not-a-date",
            "2026-13-01T00:00:00Z",
            "2026-02-30T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01 00:00:00Z",
            "2026-01-01T00:00:00",
        ] {
            assert!(iso_to_epoch_secs(bad).is_none(), "should reject {bad:?}");
        }
    }

    async fn setup() -> ExousiaServiceImpl {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let pools = Arc::new(DbPools {
            read: pool.clone(),
            write: pool,
        });
        let config = ExousiaConfig {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_string(),
        };
        ExousiaServiceImpl::new(pools, config)
    }

    async fn create_member(service: &ExousiaServiceImpl, username: &str, password: &str) -> User {
        service
            .create_user(CreateUserRequest {
                username: username.to_string(),
                display_name: username.to_string(),
                password: password.to_string(),
                role: UserRole::Member,
            })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn create_user_rejects_short_or_empty_password() {
        let service = setup().await;
        for pw in ["", "short"] {
            let result = service
                .create_user(CreateUserRequest {
                    username: "alice".to_string(),
                    display_name: "Alice".to_string(),
                    password: pw.to_string(),
                    role: UserRole::Member,
                })
                .await;
            assert!(
                matches!(result, Err(ExousiaError::InvalidPassword { .. })),
                "password {pw:?} must be rejected"
            );
        }
    }

    #[tokio::test]
    async fn create_user_rejects_oversized_password() {
        let service = setup().await;
        let result = service
            .create_user(CreateUserRequest {
                username: "alice".to_string(),
                display_name: "Alice".to_string(),
                password: "x".repeat(MAX_PASSWORD_BYTES + 1),
                role: UserRole::Member,
            })
            .await;
        assert!(matches!(result, Err(ExousiaError::InvalidPassword { .. })));
    }

    #[tokio::test]
    async fn create_user_accepts_minimum_length_password() {
        let service = setup().await;
        let user = create_member(&service, "alice", "password").await;
        assert_eq!(user.username, "alice");
    }

    #[tokio::test]
    async fn login_oversized_password_returns_invalid_credentials() {
        let service = setup().await;
        create_member(&service, "alice", "password123").await;
        let result = service.login("alice", &"x".repeat(10 * 1024 * 1024)).await;
        assert!(matches!(
            result,
            Err(ExousiaError::InvalidCredentials { .. })
        ));
    }

    #[tokio::test]
    async fn login_inactive_user_with_correct_password_returns_invalid_credentials() {
        let service = setup().await;
        let user = create_member(&service, "alice", "password123").await;
        db::deactivate_user(&service.pools.write, &user_id_to_bytes(user.id))
            .await
            .unwrap();
        // WHY: InvalidCredentials (not UserInactive) — a distinct error would
        // leak that the username exists but is deactivated.
        let result = service.login("alice", "password123").await;
        assert!(matches!(
            result,
            Err(ExousiaError::InvalidCredentials { .. })
        ));
    }

    #[tokio::test]
    async fn login_wrong_password_and_unknown_user_return_invalid_credentials() {
        let service = setup().await;
        create_member(&service, "alice", "password123").await;
        let wrong = service.login("alice", "wrong-password").await;
        assert!(matches!(
            wrong,
            Err(ExousiaError::InvalidCredentials { .. })
        ));
        let unknown = service.login("nobody", "password123").await;
        assert!(matches!(
            unknown,
            Err(ExousiaError::InvalidCredentials { .. })
        ));
    }

    #[test]
    fn sentinel_password_hash_constant_is_valid_and_never_matches() {
        // WHY: guards the constant itself — a malformed PHC string would make
        // verify_password return Err before doing any Argon2 work, silently
        // breaking the constant-time guarantee the sentinel verify exists for.
        let result = password::verify_password("anything", SENTINEL_PASSWORD_HASH);
        assert!(
            result.is_ok(),
            "SENTINEL_PASSWORD_HASH must parse as a valid PHC string"
        );
        assert!(
            !result.unwrap(),
            "SENTINEL_PASSWORD_HASH must never match a real login attempt"
        );
    }

    // WHY: proves the username-enumeration fix — both miss-paths must run an
    // Argon2id verify (equal work), not just return early on one of them.
    #[tokio::test]
    async fn login_unknown_username_still_performs_argon2_verify() {
        let service = setup().await;
        create_member(&service, "alice", "password123").await;

        let before = password::VERIFY_CALL_COUNT.with(std::cell::Cell::get);
        let result = service.login("no-such-user", "whatever-password").await;
        let after = password::VERIFY_CALL_COUNT.with(std::cell::Cell::get);

        assert!(matches!(
            result,
            Err(ExousiaError::InvalidCredentials { .. })
        ));
        assert_eq!(
            after - before,
            1,
            "unknown-username login must exercise exactly one sentinel-hash Argon2 verify"
        );
    }

    #[tokio::test]
    async fn login_wrong_password_for_existing_user_performs_argon2_verify() {
        let service = setup().await;
        create_member(&service, "alice", "password123").await;

        let before = password::VERIFY_CALL_COUNT.with(std::cell::Cell::get);
        let result = service.login("alice", "wrong-password").await;
        let after = password::VERIFY_CALL_COUNT.with(std::cell::Cell::get);

        assert!(matches!(
            result,
            Err(ExousiaError::InvalidCredentials { .. })
        ));
        assert_eq!(
            after - before,
            1,
            "wrong-password login must exercise exactly one real-hash Argon2 verify"
        );
    }

    #[tokio::test]
    async fn concurrent_refresh_calls_only_one_succeeds() {
        let service = setup().await;
        create_member(&service, "alice", "password123").await;
        let pair = service.login("alice", "password123").await.unwrap();

        let (a, b) = tokio::join!(
            service.refresh(&pair.refresh_token),
            service.refresh(&pair.refresh_token)
        );

        let ok_count = usize::from(a.is_ok()) + usize::from(b.is_ok());
        assert_eq!(ok_count, 1, "exactly one concurrent refresh may succeed");
        let err = if a.is_err() {
            a.unwrap_err()
        } else {
            b.unwrap_err()
        };
        assert!(
            matches!(err, ExousiaError::TokenInvalid { .. }),
            "loser must observe the token as already rotated: {err:?}"
        );
    }
}
