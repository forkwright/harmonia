use axum::response::Response;
use exousia::AuthService;
use exousia::user::UserRole;
use themelion::ids::UserId;

use super::types::{
    ERR_GENERIC, ERR_MISSING_PARAM, ERR_WRONG_CREDS, Format, SubsonicCommon, respond_error,
};
use crate::state::AppState;

// WHY: subtle's ConstantTimeEq avoids the timing side-channel of a
// short-circuiting `!=` on secret material; unequal lengths return
// not-equal without comparing content (length is public here — the
// expected side is always a 32-char MD5 hex string).
fn constant_time_str_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

// WHY: wire DTO — Subsonic API auth user fields deserialized from the request.
#[derive(Debug, Clone)]
pub struct SubsonicUser {
    pub user_id: UserId,
    pub username: String,
    pub role: UserRole,
    pub format: Format,
}

/// Authenticate a Subsonic request FROM the common query params.
/// Returns `Ok(SubsonicUser)` or an HTTP-200 Subsonic error Response.
pub async fn authenticate(
    common: &SubsonicCommon,
    state: &AppState,
) -> Result<SubsonicUser, Response> {
    let fmt = common.format();

    // OpenSubsonic API key mode
    if let Some(api_key) = &common.api_key {
        let authed = state
            .auth
            .validate_api_key(api_key)
            .await
            .map_err(|_| respond_error(fmt, ERR_WRONG_CREDS, "wrong username or password"))?;

        let username = lookup_username(state, authed.user_id)
            .await
            .map_err(|_| respond_error(fmt, ERR_GENERIC, "internal error"))?;

        return Ok(SubsonicUser {
            user_id: authed.user_id,
            username,
            role: authed.role,
            format: fmt,
        });
    }

    // Legacy token auth: u + t + s
    let username = common
        .u
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| respond_error(fmt, ERR_MISSING_PARAM, "required parameter is missing: u"))?;
    let token = common
        .t
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| respond_error(fmt, ERR_MISSING_PARAM, "required parameter is missing: t"))?;
    let salt = common
        .s
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| respond_error(fmt, ERR_MISSING_PARAM, "required parameter is missing: s"))?;

    let row = lookup_subsonic_user(state, username)
        .await
        .map_err(|_| respond_error(fmt, ERR_WRONG_CREDS, "wrong username or password"))?;

    let expected = format!("{:x}", md5::compute(format!("{}{}", row.password, salt)));
    if !constant_time_str_eq(&expected, token) {
        return Err(respond_error(
            fmt,
            ERR_WRONG_CREDS,
            "wrong username or password",
        ));
    }

    Ok(SubsonicUser {
        user_id: row.user_id,
        username: username.to_string(),
        role: row.role,
        format: fmt,
    })
}

struct SubsonicUserRow {
    user_id: UserId,
    role: UserRole,
    password: String,
}

async fn lookup_subsonic_user(
    state: &AppState,
    username: &str,
) -> Result<SubsonicUserRow, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: Vec<u8>,
        role: String,
        password: String,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT u.id, u.role, sp.password
         FROM users u
         JOIN subsonic_passwords sp ON sp.user_id = u.id
         WHERE u.username = ? AND u.is_active = 1",
    )
    .bind(username)
    .fetch_one(&state.db.read)
    .await?;

    let uuid = uuid::Uuid::from_slice(&row.id)
        .map_err(|_| sqlx::Error::Decode("invalid uuid bytes".into()))?;
    let user_id = UserId::from_uuid(uuid);
    let role = match row.role.as_str() {
        "admin" => UserRole::Admin,
        _ => UserRole::Member,
    };

    Ok(SubsonicUserRow {
        user_id,
        role,
        password: row.password,
    })
}

async fn lookup_username(state: &AppState, user_id: UserId) -> Result<String, sqlx::Error> {
    let id_bytes = user_id.as_bytes().to_vec();
    sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = ?")
        .bind(id_bytes)
        .fetch_one(&state.db.read)
        .await
}

/// Set a subsonic password for a user (used in tests and admin setup).
pub async fn set_subsonic_password(
    pool: &sqlx::SqlitePool,
    user_id: UserId,
    password: &str,
) -> Result<(), sqlx::Error> {
    let id_bytes = user_id.as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO subsonic_passwords (user_id, password) VALUES (?, ?)
         ON CONFLICT(user_id) DO UPDATE SET password = excluded.password",
    )
    .bind(id_bytes)
    .bind(password)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::constant_time_str_eq;

    fn flip_hex_char(c: char) -> char {
        if c == '0' { '1' } else { '0' }
    }

    fn with_flipped_at(s: &str, index: usize) -> String {
        s.chars()
            .enumerate()
            .map(|(i, c)| if i == index { flip_hex_char(c) } else { c })
            .collect()
    }

    #[test]
    fn constant_time_str_eq_matches_token_semantics() {
        let expected = format!("{:x}", md5::compute("secretsalt"));
        assert!(constant_time_str_eq(&expected, &expected.clone()));

        // Near-misses differing in the first and last byte both reject
        let early = with_flipped_at(&expected, 0);
        assert!(!constant_time_str_eq(&expected, &early));
        let late = with_flipped_at(&expected, expected.len() - 1);
        assert!(!constant_time_str_eq(&expected, &late));

        // Length mismatch
        assert!(!constant_time_str_eq(&expected, ""));
    }
}
