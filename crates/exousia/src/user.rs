use serde::{Deserialize, Serialize};
use themelion::ids::{ApiKeyId, UserId};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "member")]
    Member,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Member => "member",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(UserRole::Admin),
            "member" => Some(UserRole::Member),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("display_name", &self.display_name)
            .field("password_hash", &"[redacted]")
            .field("role", &self.role)
            .field("is_active", &self.is_active)
            .field("created_at", &self.created_at)
            .field("last_login_at", &self.last_login_at)
            .finish()
    }
}

#[derive(Clone)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub user_id: UserId,
    pub short_token: String,
    pub long_token_hash: String,
    pub label: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked: bool,
}
impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKey")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("short_token", &self.short_token)
            .field("long_token_hash", &"[redacted]")
            .field("label", &self.label)
            .field("created_at", &self.created_at)
            .field("last_used_at", &self.last_used_at)
            .field("revoked", &self.revoked)
            .finish()
    }
}

pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: UserRole,
}
impl std::fmt::Debug for CreateUserRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateUserRequest")
            .field("username", &self.username)
            .field("display_name", &self.display_name)
            .field("password", &"[redacted]")
            .field("role", &self.role)
            .finish()
    }
}
