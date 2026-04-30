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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_response_serde_round_trip() {
        let original = ApiResponse {
            data: vec!["a".to_string(), "b".to_string()],
            total: Some(100),
            page: Some(0),
            per_page: Some(25),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ApiResponse<Vec<String>> = serde_json::from_str(&json).unwrap();
        assert_eq!(original.data, deserialized.data);
        assert_eq!(original.total, deserialized.total);
        assert_eq!(original.page, deserialized.page);
        assert_eq!(original.per_page, deserialized.per_page);
    }

    #[test]
    fn paginated_response_serde_round_trip() {
        let original = PaginatedResponse {
            items: vec![1, 2, 3],
            total: 10,
            page: 1,
            page_size: 3,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: PaginatedResponse<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(original.items, deserialized.items);
        assert_eq!(original.total, deserialized.total);
        assert_eq!(original.page, deserialized.page);
        assert_eq!(original.page_size, deserialized.page_size);
    }

    #[test]
    fn list_params_default() {
        let params = ListParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 50);
    }

    #[test]
    fn connection_status_default_and_equality() {
        assert_eq!(ConnectionStatus::default(), ConnectionStatus::Disconnected);
        assert_eq!(ConnectionStatus::Connecting, ConnectionStatus::Connecting);
        assert_eq!(ConnectionStatus::Connected, ConnectionStatus::Connected);
        assert_eq!(
            ConnectionStatus::Failed("err".to_string()),
            ConnectionStatus::Failed("err".to_string())
        );
        assert_ne!(
            ConnectionStatus::Failed("a".to_string()),
            ConnectionStatus::Failed("b".to_string())
        );
        assert_ne!(ConnectionStatus::Disconnected, ConnectionStatus::Connected);
    }

    #[test]
    fn connection_status_clone_and_debug() {
        let status = ConnectionStatus::Failed("network error".to_string());
        let cloned = status.clone();
        assert_eq!(status, cloned);

        let dbg = format!("{status:?}");
        assert!(
            dbg.contains("Failed"),
            "debug should contain variant: {dbg}"
        );
    }
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
#[non_exhaustive]
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
