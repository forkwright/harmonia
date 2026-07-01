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
    /// Validates that a request can be resolved to a media identity.
    async fn validate(
        &self,
        media_type: themelion::MediaType,
        title: &str,
        external_id: Option<&str>,
    ) -> Result<(), AitesisError>;
}

/// Trait boundary to the monitoring layer — begins tracking a requested item.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since 1.75; dyn dispatch not required here"
)]
pub trait MonitorService: Send + Sync {
    /// Creates a wanted-media entry for an approved request.
    ///
    /// INVARIANT: implementations MUST be idempotent per request — a repeat
    /// call for the same `request.id` returns the already-created want's id
    /// instead of inserting a duplicate (e.g. an upsert keyed on
    /// `(source = 'request', source_ref = request id)`). The approval flow
    /// creates the want before persisting the request-status update and
    /// relies on this contract to make a retried approval safe after a
    /// partial failure.
    async fn create_want(&self, request: &MediaRequest) -> Result<WantId, AitesisError>;
}

/// Trait boundary to Exousia — looks up a user's role without coupling to the auth crate.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since 1.75; dyn dispatch not required here"
)]
pub trait UserRoleProvider: Send + Sync {
    /// Returns the household role for the given user.
    async fn role_of(&self, user_id: UserId) -> Result<UserRole, AitesisError>;
}

/// Approves a request: validates identity, creates a want, transitions to Monitoring.
///
/// Requires `admin_id` to have the Admin role.
///
/// Ordering: the want is created before the request row is updated. The
/// request table and the want store sit behind different write paths, so no
/// single transaction can span both; instead the operation is re-runnable — a
/// failure after `create_want` leaves the row `Submitted`, and a retried
/// approval resolves to the same want via the [`MonitorService::create_want`]
/// idempotency contract, then completes the status update.
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

    let approved = validate_transition(request.status, RequestStatus::Approved)?;

    identity
        .validate(
            request.media_type,
            &request.title,
            request.external_id.as_deref(),
        )
        .await?;

    let want_id = monitor.create_want(&request).await?;
    let monitoring = validate_transition(approved.to(), RequestStatus::Monitoring)?;

    let now = jiff::Timestamp::now();
    crate::repo::update_status(
        pool,
        crate::repo::UpdateStatusParams {
            id: &request_id,
            status: monitoring.to(),
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

    let denied = validate_transition(request.status, RequestStatus::Denied)?;

    let now = jiff::Timestamp::now();
    crate::repo::update_status(
        pool,
        crate::repo::UpdateStatusParams {
            id: &request_id,
            status: denied.to(),
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

    /// Mirrors the production upsert semantics: one want per request id.
    #[derive(Default)]
    struct IdempotentRecordingMonitor {
        wants: std::sync::Mutex<std::collections::HashMap<RequestId, WantId>>,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl MonitorService for IdempotentRecordingMonitor {
        async fn create_want(&self, request: &MediaRequest) -> Result<WantId, AitesisError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut wants = self.wants.lock().unwrap();
            Ok(*wants.entry(request.id).or_default())
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
    async fn retried_approve_after_update_status_failure_reuses_existing_want() {
        let pool = setup().await;
        let user_id = UserId::new();
        let admin_id = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        let monitor = IdempotentRecordingMonitor::default();

        // Inject a persistence failure between create_want and the status update.
        sqlx::query(
            "CREATE TRIGGER block_request_update BEFORE UPDATE ON requests
             BEGIN SELECT RAISE(ABORT, 'injected update failure'); END",
        )
        .execute(&pool)
        .await
        .unwrap();

        let err = approve_request(
            &pool,
            req_id,
            admin_id,
            UserRole::Admin,
            &AlwaysValidIdentity,
            &monitor,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AitesisError::Database { .. }));

        // The row is untouched — still Submitted, so the approval is retryable.
        let row = crate::repo::get_request(&pool, &req_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, RequestStatus::Submitted);
        assert!(row.want_id.is_none());

        sqlx::query("DROP TRIGGER block_request_update")
            .execute(&pool)
            .await
            .unwrap();

        let updated = approve_request(
            &pool,
            req_id,
            admin_id,
            UserRole::Admin,
            &AlwaysValidIdentity,
            &monitor,
        )
        .await
        .unwrap();

        assert_eq!(updated.status, RequestStatus::Monitoring);
        assert_eq!(
            monitor.calls.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "create_want runs on both attempts"
        );
        let wants = monitor.wants.lock().unwrap();
        assert_eq!(wants.len(), 1, "the retry must not create a second want");
        assert_eq!(updated.want_id, wants.get(&req_id).copied());
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
