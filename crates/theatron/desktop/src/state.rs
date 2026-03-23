//! Global application state managed via Dioxus signals.

use theatron_core::types::ConnectionStatus;

/// Root application state.
#[derive(Debug, Clone, PartialEq)]
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

impl Default for AppState {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".to_owned(),
            auth_token: None,
            connection_status: ConnectionStatus::Disconnected,
            sidebar_visible: true,
        }
    }
}
