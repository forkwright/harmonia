//! Global application state managed via Dioxus signals.

use skene::types::ConnectionStatus;

// WHY: default dev URL; overridden at runtime via HARMONIA_SERVER_URL env var
// kanon:ignore SECURITY/hardcoded-loopback-url
const DEFAULT_SERVER_URL: &str = "http://localhost:3000";

/// Root application state.
#[derive(Clone, PartialEq)]
pub struct AppState {
    /// Server URL for the Harmonia backend.
    pub server_url: String,
    /// Authentication token.
    pub auth_token: Option<String>,
    /// Current connection status.
    pub connection_status: ConnectionStatus,
    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,
}
impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("server_url", &self.server_url)
            .field("auth_token", &"[redacted]")
            .field("connection_status", &self.connection_status)
            .field("sidebar_visible", &self.sidebar_visible)
            .finish()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            server_url: std::env::var("HARMONIA_SERVER_URL")
                .unwrap_or_else(|_| DEFAULT_SERVER_URL.to_owned()),
            auth_token: None,
            connection_status: ConnectionStatus::Disconnected,
            sidebar_visible: true,
        }
    }
}
