//! Approval logic: Admin auto-approve on submission, Member requires explicit approval.

use sqlx::SqlitePool;
use themelion::{RequestId, UserId, WantId};
use tracing::instrument;

use crate::error::{AitesisError, InsufficientPermissionSnafu, RequestNotFoundSnafu};
use crate::types::{MediaRequest, RequestStatus, UserRole};
use crate::workflow::validate_transition;

/// Trait boundary to Epignosis — validates that requested media is identifiable.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since 1.75; dyn dispatch not required here"
)]
pub trait IdentityValidator: Send + Sync {
    async fn validate(
        &self,
        media_type: themelion::MediaType,
        title: &str,
        external_id: Option<&str>,
    ) -> Result<(), AitesisError>;
}

/// Trait boundary to Episkope — begins monitoring for a requested item.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since 1.75; dyn dispatch not required here"
)]
pub trait MonitorService: Send + Sync {
    async fn create_want(&self, request: &MediaRequest) -> Result<WantId, AitesisError>;
}

/// Trait boundary to Exousia — looks up a user's role without coupling to the auth crate.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since 1.75; dyn dispatch not required here"
)]
pub trait UserRoleProvider: Send + Sync {
    async fn role_of(&self, user_id: UserId) -> Result<UserRole, AitesisError>;
}

/// Approves a request: validates identity, creates a want, transitions to Monitoring.
///
/// Requires `admin_id` to have the Admin role.
#[instrument(skip(pool, identity, monitor), fields(request_id = %request_id, admin_id = %admin_id))]
pub(crate) async fn approve_request<I, M>(
    pool: &SqlitePool,
    request_id: RequestId,
    admin_id: UserId,
    admin_role: UserRole,
    identity: &I,
    monitor: &M,
) -> Result<MediaRequest, AitesisError>
where
    I: IdentityValidator,
    M: MonitorService,
{
    if admin_role != UserRole::Admin {
        return InsufficientPermissionSnafu.fail();
    }

    let request = crate::repo::get_request(pool, &request_id)
        .await?
        .ok_or_else(|| {
            RequestNotFoundSnafu {
                id: request_id.to_string(),
            }
            .build()
        })?;

    validate_transition(request.status, RequestStatus::Approved)?;

    identity
        .validate(
            request.media_type,
            &request.title,
            request.external_id.as_deref(),
        )
        .await?;

    let want_id = monitor.create_want(&request).await?;

    let now = jiff::Timestamp::now();
    crate::repo::update_status(
        pool,
        crate::repo::UpdateStatusParams {
            id: &request_id,
            status: RequestStatus::Monitoring,
            decided_by: Some(&admin_id),
            decided_at: Some(now),
            deny_reason: None,
            want_id: Some(&want_id),
        },
    )
    .await?;

    crate::repo::get_request(pool, &request_id)
        .await?
        .ok_or_else(|| {
            RequestNotFoundSnafu {
                id: request_id.to_string(),
            }
            .build()
        })
}

/// Denies a request: transitions to Denied with an optional reason.
///
/// Requires `admin_id` to have the Admin role.
#[instrument(skip(pool), fields(request_id = %request_id, admin_id = %admin_id))]
pub(crate) async fn deny_request(
    pool: &SqlitePool,
    request_id: RequestId,
    admin_id: UserId,
    admin_role: UserRole,
    reason: Option<String>,
) -> Result<MediaRequest, AitesisError> {
    if admin_role != UserRole::Admin {
        return InsufficientPermissionSnafu.fail();
    }

    let request = crate::repo::get_request(pool, &request_id)
        .await?
        .ok_or_else(|| {
            RequestNotFoundSnafu {
                id: request_id.to_string(),
            }
            .build()
        })?;

    validate_transition(request.status, RequestStatus::Denied)?;

    let now = jiff::Timestamp::now();
    crate::repo::update_status(
        pool,
        crate::repo::UpdateStatusParams {
            id: &request_id,
            status: RequestStatus::Denied,
            decided_by: Some(&admin_id),
            decided_at: Some(now),
            deny_reason: reason.as_deref(),
            want_id: None,
        },
    )
    .await?;

    crate::repo::get_request(pool, &request_id)
        .await?
        .ok_or_else(|| {
            RequestNotFoundSnafu {
                id: request_id.to_string(),
            }
            .build()
        })
}

#[cfg(test)]
pub(crate) mod tests {
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;
    use themelion::{MediaType, RequestId, UserId, WantId};

    use super::*;
    use crate::repo::insert_request;
    use crate::types::{MediaRequest, RequestStatus};

    pub(crate) struct AlwaysValidIdentity;
    impl IdentityValidator for AlwaysValidIdentity {
        async fn validate(
            &self,
            _media_type: themelion::MediaType,
            _title: &str,
            _external_id: Option<&str>,
        ) -> Result<(), AitesisError> {
            Ok(())
        }
    }

    pub(crate) struct AlwaysCreateMonitor;
    impl MonitorService for AlwaysCreateMonitor {
        async fn create_want(&self, _request: &MediaRequest) -> Result<WantId, AitesisError> {
            Ok(WantId::new())
        }
    }

    pub(crate) async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn submitted_request(user_id: UserId) -> MediaRequest {
        MediaRequest {
            id: RequestId::new(),
            user_id,
            media_type: MediaType::Music,
            title: "Led Zeppelin IV".to_string(),
            external_id: None,
            status: RequestStatus::Submitted,
            decided_by: None,
            decided_at: None,
            deny_reason: None,
            want_id: None,
            created_at: jiff::Timestamp::now(),
        }
    }

    #[tokio::test]
    async fn approve_transitions_to_monitoring() {
        let pool = setup().await;
        let user_id = UserId::new();
        let admin_id = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        let updated = approve_request(
            &pool,
            req_id,
            admin_id,
            UserRole::Admin,
            &AlwaysValidIdentity,
            &AlwaysCreateMonitor,
        )
        .await
        .unwrap();

        assert_eq!(updated.status, RequestStatus::Monitoring);
        assert_eq!(updated.decided_by, Some(admin_id));
        assert!(updated.want_id.is_some());
    }

    #[tokio::test]
    async fn approve_by_member_returns_insufficient_permission() {
        let pool = setup().await;
        let user_id = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        let err = approve_request(
            &pool,
            req_id,
            user_id,
            UserRole::Member,
            &AlwaysValidIdentity,
            &AlwaysCreateMonitor,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));
    }

    #[tokio::test]
    async fn deny_transitions_to_denied_with_reason() {
        let pool = setup().await;
        let user_id = UserId::new();
        let admin_id = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        let updated = deny_request(
            &pool,
            req_id,
            admin_id,
            UserRole::Admin,
            Some("Not available in this region".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(updated.status, RequestStatus::Denied);
        assert_eq!(
            updated.deny_reason.as_deref(),
            Some("Not available in this region")
        );
        assert_eq!(updated.decided_by, Some(admin_id));
    }

    #[tokio::test]
    async fn deny_already_denied_returns_invalid_transition() {
        let pool = setup().await;
        let user_id = UserId::new();
        let admin_id = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        deny_request(&pool, req_id, admin_id, UserRole::Admin, None)
            .await
            .unwrap();

        let err = deny_request(&pool, req_id, admin_id, UserRole::Admin, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::InvalidTransition { .. }));
    }
}
