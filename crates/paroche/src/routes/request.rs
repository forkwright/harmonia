//! Media request workflow endpoints.

use aitesis::{CreateRequestInput, MediaRequest, RequestStatus};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
use themelion::{MediaType, RequestId, UserId};
use uuid::Uuid;

use crate::error::ParocheError;
use crate::response::{ApiResponse, deleted};
use crate::state::{AppState, RequestServiceError};

// ---------------------------------------------------------------------------
// Response conversion
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct RequestResponse {
    pub id: String,
    pub user_id: String,
    pub media_type: String,
    pub title: String,
    pub external_id: Option<String>,
    pub status: String,
    pub decided_by: Option<String>,
    pub decided_at: Option<String>,
    pub deny_reason: Option<String>,
    pub want_id: Option<String>,
    pub created_at: String,
}

impl From<MediaRequest> for RequestResponse {
    fn from(request: MediaRequest) -> Self {
        Self {
            id: request.id.as_uuid().to_string(),
            user_id: request.user_id.as_uuid().to_string(),
            media_type: request.media_type.to_string(),
            title: request.title,
            external_id: request.external_id,
            status: request.status.as_str().to_string(),
            decided_by: request.decided_by.map(|id| id.as_uuid().to_string()),
            decided_at: request.decided_at.map(|ts| ts.to_string()),
            deny_reason: request.deny_reason,
            want_id: request.want_id.map(|id| id.as_uuid().to_string()),
            created_at: request.created_at.to_string(),
        }
    }
}

fn parse_request_id(id: &str) -> Result<RequestId, ParocheError> {
    Uuid::parse_str(id)
        .map(RequestId::from_uuid)
        .map_err(|_| ParocheError::InvalidId)
}

fn parse_user_id(id: &str) -> Result<UserId, ParocheError> {
    Uuid::parse_str(id)
        .map(UserId::from_uuid)
        .map_err(|_| ParocheError::InvalidId)
}

fn parse_media_type(value: &str) -> Result<MediaType, ParocheError> {
    match value {
        "music_album" => return Ok(MediaType::Music),
        "tv_series" => return Ok(MediaType::Tv),
        _ => {}
    }
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|_| {
        ParocheError::Validation {
            message: format!("unsupported media_type: {value}"),
        }
    })
}

fn parse_request_status(value: &str) -> Result<RequestStatus, ParocheError> {
    RequestStatus::parse(value).ok_or_else(|| ParocheError::Validation {
        message: format!("unsupported request status: {value}"),
    })
}

fn map_request_service_error(error: RequestServiceError) -> ParocheError {
    match error {
        RequestServiceError::NotAvailable => ParocheError::Unavailable,
        RequestServiceError::Domain(error) => match error {
            aitesis::AitesisError::RequestLimitExceeded { .. } => ParocheError::RateLimited,
            aitesis::AitesisError::RequestNotFound { .. } => ParocheError::NotFound,
            aitesis::AitesisError::RequestAlreadyExists { .. }
            | aitesis::AitesisError::MediaIdentityInvalid { .. }
            | aitesis::AitesisError::InvalidTransition { .. } => ParocheError::Validation {
                message: error.to_string(),
            },
            aitesis::AitesisError::InsufficientPermission { .. } => ParocheError::Forbidden,
            aitesis::AitesisError::Database { source, .. } => ParocheError::Database { source },
            _ => ParocheError::Internal,
        },
    }
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct RequestFilterQuery {
    pub user_id: Option<String>,
    pub status: Option<String>,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}
fn default_page() -> u64 {
    1
}
fn default_per_page() -> u64 {
    20
}

#[derive(Deserialize)]
pub struct CreateRequestBody {
    pub media_type: String,
    pub title: String,
    pub external_id: Option<String>,
}

#[derive(Deserialize)]
pub struct DenyRequestBody {
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_requests(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(filter): Query<RequestFilterQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = filter.per_page.clamp(1, 100);
    let page = filter.page.max(1);
    let offset = (page - 1) * per_page;

    let user_id = filter.user_id.as_deref().map(parse_user_id).transpose()?;
    // WHY: a member with no explicit filter is scoped to their own requests;
    // the query parameter is never trusted for authorization — aitesis
    // re-checks against the authenticated caller and rejects a member naming
    // anyone else.
    let user_id = if auth.role == exousia::UserRole::Admin {
        user_id
    } else {
        user_id.or(Some(auth.user_id))
    };
    let status = filter
        .status
        .as_deref()
        .map(parse_request_status)
        .transpose()?;
    let requests = state
        .requests
        .list_requests(auth.user_id, user_id, status)
        .await
        .map_err(map_request_service_error)?;

    let total = requests.len() as u64;
    let data: Vec<RequestResponse> = requests
        .into_iter()
        .skip(offset as usize)
        .take(per_page as usize)
        .map(Into::into)
        .collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_request(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let request_id = parse_request_id(&id)?;
    let request = state
        .requests
        .get_request(request_id)
        .await
        .map_err(map_request_service_error)?;

    Ok(ApiResponse::ok(RequestResponse::from(request)))
}

pub async fn submit_request(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateRequestBody>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }
    if body.media_type.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "media_type is required".to_string(),
        });
    }

    let input = CreateRequestInput {
        media_type: parse_media_type(body.media_type.trim())?,
        title: body.title,
        external_id: body.external_id,
    };
    let request = state
        .requests
        .submit_request(auth.user_id, input)
        .await
        .map_err(map_request_service_error)?;

    Ok(ApiResponse::created(RequestResponse::from(request)))
}

pub async fn approve_request(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let request_id = parse_request_id(&id)?;
    let updated = state
        .requests
        .approve(request_id, admin.0.user_id)
        .await
        .map_err(map_request_service_error)?;

    Ok(ApiResponse::ok(RequestResponse::from(updated)))
}

pub async fn deny_request(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<DenyRequestBody>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let request_id = parse_request_id(&id)?;
    let updated = state
        .requests
        .deny(request_id, admin.0.user_id, body.reason)
        .await
        .map_err(map_request_service_error)?;

    Ok(ApiResponse::ok(RequestResponse::from(updated)))
}

pub async fn cancel_request(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let request_id = parse_request_id(&id)?;
    state
        .requests
        .cancel_request(request_id, auth.user_id)
        .await
        .map_err(map_request_service_error)?;

    Ok(deleted())
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn request_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list_requests).post(submit_request))
        .route("/{id}", get(get_request).delete(cancel_request))
        .route("/{id}/approve", post(approve_request))
        .route("/{id}/deny", post(deny_request))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aitesis::{IdentityValidator, MonitorService, RequestService, UserRoleProvider};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use exousia::{AuthService, CreateUserRequest, UserRole};
    use serde_json::json;
    use snafu::{ResultExt, Snafu};
    use themelion::{UserId, WantId};
    use tower::ServiceExt;

    use super::*;
    use crate::test_helpers::test_state;

    type TestResult<T> = Result<T, TestError>;

    #[derive(Debug, Snafu)]
    enum TestError {
        #[snafu(display("failed to serialize request body"))]
        SerializeRequestBody { source: serde_json::Error },
        #[snafu(display("failed to build HTTP request"))]
        BuildRequest { source: axum::http::Error },
        #[snafu(display("failed to create test user"))]
        CreateUser { source: exousia::ExousiaError },
        #[snafu(display("failed to log in test user"))]
        Login { source: exousia::ExousiaError },
    }

    type TestRequestService = aitesis::AitesisServiceImpl<TestRoles, TestIdentity, TestMonitor>;

    struct TestRequestAdapter(Arc<TestRequestService>);

    impl crate::state::DynRequestService for TestRequestAdapter {
        fn submit_request(
            &self,
            user_id: UserId,
            input: aitesis::CreateRequestInput,
        ) -> crate::state::RequestServiceFut<'_, aitesis::MediaRequest> {
            let service = Arc::clone(&self.0);
            Box::pin(async move {
                service
                    .submit_request(user_id, input)
                    .await
                    .map_err(Into::into)
            })
        }

        fn approve(
            &self,
            request_id: themelion::RequestId,
            admin_id: UserId,
        ) -> crate::state::RequestServiceFut<'_, aitesis::MediaRequest> {
            let service = Arc::clone(&self.0);
            Box::pin(async move {
                service
                    .approve(request_id, admin_id)
                    .await
                    .map_err(Into::into)
            })
        }

        fn deny(
            &self,
            request_id: themelion::RequestId,
            admin_id: UserId,
            reason: Option<String>,
        ) -> crate::state::RequestServiceFut<'_, aitesis::MediaRequest> {
            let service = Arc::clone(&self.0);
            Box::pin(async move {
                service
                    .deny(request_id, admin_id, reason)
                    .await
                    .map_err(Into::into)
            })
        }

        fn get_request(
            &self,
            request_id: themelion::RequestId,
        ) -> crate::state::RequestServiceFut<'_, aitesis::MediaRequest> {
            let service = Arc::clone(&self.0);
            Box::pin(async move { service.get_request(request_id).await.map_err(Into::into) })
        }

        fn list_requests(
            &self,
            caller_id: UserId,
            user_id: Option<UserId>,
            status: Option<aitesis::RequestStatus>,
        ) -> crate::state::RequestServiceFut<'_, Vec<aitesis::MediaRequest>> {
            let service = Arc::clone(&self.0);
            Box::pin(async move {
                service
                    .list_requests(caller_id, user_id, status)
                    .await
                    .map_err(Into::into)
            })
        }

        fn cancel_request(
            &self,
            request_id: themelion::RequestId,
            user_id: UserId,
        ) -> crate::state::RequestServiceFut<'_, ()> {
            let service = Arc::clone(&self.0);
            Box::pin(async move {
                service
                    .cancel_request(request_id, user_id)
                    .await
                    .map_err(Into::into)
            })
        }
    }

    struct TestRoles;

    impl UserRoleProvider for TestRoles {
        async fn role_of(
            &self,
            _user_id: UserId,
        ) -> Result<aitesis::UserRole, aitesis::AitesisError> {
            Ok(aitesis::UserRole::Member)
        }
    }

    struct TestIdentity;

    impl IdentityValidator for TestIdentity {
        async fn validate(
            &self,
            _media_type: MediaType,
            _title: &str,
            _external_id: Option<&str>,
        ) -> Result<(), aitesis::AitesisError> {
            Ok(())
        }
    }

    struct TestMonitor;

    impl MonitorService for TestMonitor {
        async fn create_want(
            &self,
            _request: &aitesis::MediaRequest,
        ) -> Result<WantId, aitesis::AitesisError> {
            Ok(WantId::new())
        }
    }

    async fn post_request(app: &axum::Router, token: &str) -> TestResult<StatusCode> {
        let body = json!({
            "media_type": "music",
            "title": "Kind of Blue",
            "external_id": null
        });
        let resp = match app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/requests")
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::from(
                        serde_json::to_vec(&body).context(SerializeRequestBodySnafu)?,
                    ))
                    .context(BuildRequestSnafu)?,
            )
            .await
        {
            Ok(resp) => resp,
            Err(error) => match error {},
        };
        Ok(resp.status())
    }

    #[tokio::test]
    async fn submit_request_enforces_aitesis_pending_limit() -> TestResult<()> {
        let (mut state, auth) = test_state().await;
        let config = horismos::AitesisConfig {
            max_pending_per_user: 1,
            max_requests_per_day: 100,
            auto_approve_admins: false,
        };
        let service = Arc::new(aitesis::AitesisServiceImpl::new(
            state.db.read.clone(),
            state.db.write.clone(),
            config,
            TestRoles,
            TestIdentity,
            TestMonitor,
        ));
        state.requests = Arc::new(TestRequestAdapter(service));

        auth.create_user(CreateUserRequest {
            username: "requester".to_string(),
            display_name: "Requester".to_string(),
            password: "password123".to_string(),
            role: UserRole::Member,
        })
        .await
        .context(CreateUserSnafu)?;
        let token = auth
            .login("requester", "password123")
            .await
            .context(LoginSnafu)?
            .access_token;
        let app = crate::build_router(state);

        assert_eq!(post_request(&app, &token).await?, StatusCode::CREATED);
        assert_eq!(
            post_request(&app, &token).await?,
            StatusCode::TOO_MANY_REQUESTS
        );
        Ok(())
    }
}
