//! Shared types for Harmonia frontend clients.

use serde::{Deserialize, Serialize};

/// Paginated API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Response payload.
    pub data: T,
    /// Total number of items across all pages.
    pub total: Option<u64>,
    /// Current page number (0-indexed).
    pub page: Option<u32>,
    /// Items per page.
    pub per_page: Option<u32>,
}

/// Paginated response with explicit pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Page items.
    pub items: Vec<T>,
    /// Total item count.
    pub total: u64,
    /// Current page.
    pub page: u32,
    /// Page size.
    pub page_size: u32,
}

/// Pagination parameters for list endpoints.
#[derive(Debug, Clone, Copy)]
pub struct ListParams {
    /// Page number (1-indexed).
    pub page: u32,
    /// Items per page.
    pub per_page: u32,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 50,
        }
    }
}

/// Server health status.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Not connected to any server.
    #[default]
    Disconnected,
    /// Attempting to connect.
    Connecting,
    /// Connected and healthy.
    Connected,
    /// Connection failed.
    Failed(String),
}

