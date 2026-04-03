use harmonia_common::ids::{ApiKeyId, UserId};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub password_hash: SecretString,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub user_id: UserId,
    pub short_token: SecretString,
    pub long_token_hash: SecretString,
    pub label: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked: bool,
}

#[derive(Debug)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub password: SecretString,
    pub role: UserRole,
}
