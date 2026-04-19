use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use rand::Rng;
use serde::Serialize;

pub(crate) fn new_correlation_id() -> String {
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[derive(Serialize)]
pub struct Meta {
    pub page: u64,
    pub per_page: u64,
    pub total: u64,
}

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    pub correlation_id: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub(crate) fn ok(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::OK,
            Json(Self {
                data,
                meta: None,
                correlation_id: new_correlation_id(),
            }),
        )
    }

    pub(crate) fn created(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::CREATED,
            Json(Self {
                data,
                meta: None,
                correlation_id: new_correlation_id(),
            }),
        )
    }

    pub(crate) fn paginated(
        data: T,
        page: u64,
        per_page: u64,
        total: u64,
    ) -> (StatusCode, Json<Self>) {
        (
            StatusCode::OK,
            Json(Self {
                data,
                meta: Some(Meta {
                    page,
                    per_page,
                    total,
                }),
                correlation_id: new_correlation_id(),
            }),
        )
    }
}

pub(crate) fn deleted() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
