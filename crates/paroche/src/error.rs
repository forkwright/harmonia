use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use snafu::Snafu;

fn new_correlation_id() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write;
        write!(s, "{b:02x}").unwrap();
        s
    })
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ParocheError {
    #[snafu(display("resource not found"))]
    NotFound,
    #[snafu(display("authentication required"))]
    Unauthorized,
    #[snafu(display("access forbidden"))]
    Forbidden,
    #[snafu(display("validation error: {message}"))]
    Validation { message: String },
    #[snafu(display("rate limited"))]
    RateLimited,
    #[snafu(display("service temporarily unavailable"))]
    Unavailable,
    #[snafu(display("database error"))]
    Database { source: harmonia_db::DbError },
    #[snafu(display("internal error"))]
    Internal,
    #[snafu(display("invalid id format"))]
    InvalidId,
}

impl IntoResponse for ParocheError {
    fn into_response(self) -> Response {
        let cid = new_correlation_id();
        let (status, code, message) = match &self {
            ParocheError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            ParocheError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string())
            }
            ParocheError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string()),
            ParocheError::Validation { .. } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                self.to_string(),
            ),
            ParocheError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                self.to_string(),
            ),
            ParocheError::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "UNAVAILABLE",
                self.to_string(),
            ),
            ParocheError::Database { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                self.to_string(),
            ),
            ParocheError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                self.to_string(),
            ),
            ParocheError::InvalidId => (StatusCode::BAD_REQUEST, "INVALID_ID", self.to_string()),
        };
        (
            status,
            Json(json!({ "error": message, "code": code, "correlation_id": cid })),
        )
            .into_response()
    }
}

impl From<harmonia_db::DbError> for ParocheError {
    fn from(source: harmonia_db::DbError) -> Self {
        ParocheError::Database { source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn status_and_body(err: ParocheError) -> (StatusCode, serde_json::Value) {
        let resp = err.into_response();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, body)
    }

    #[tokio::test]
    async fn not_found_returns_404() {
        let (status, body) = status_and_body(ParocheError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "NOT_FOUND");
        assert!(body["correlation_id"].is_string());
    }

    #[tokio::test]
    async fn unauthorized_returns_401() {
        let (status, body) = status_and_body(ParocheError::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "UNAUTHORIZED");
    }

    #[tokio::test]
    async fn forbidden_returns_403() {
        let (status, body) = status_and_body(ParocheError::Forbidden).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "FORBIDDEN");
    }

    #[tokio::test]
    async fn validation_returns_422() {
        let (status, body) = status_and_body(ParocheError::Validation {
            message: "bad input".to_string(),
        })
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "VALIDATION_ERROR");
    }

    #[tokio::test]
    async fn internal_returns_500() {
        let (status, body) = status_and_body(ParocheError::Internal).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "INTERNAL_ERROR");
    }

    #[tokio::test]
    async fn invalid_id_returns_400() {
        let (status, body) = status_and_body(ParocheError::InvalidId).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "INVALID_ID");
    }

    #[tokio::test]
    async fn error_body_has_required_fields() {
        let (_, body) = status_and_body(ParocheError::NotFound).await;
        assert!(body.get("error").is_some());
        assert!(body.get("code").is_some());
        assert!(body.get("correlation_id").is_some());
    }
}
