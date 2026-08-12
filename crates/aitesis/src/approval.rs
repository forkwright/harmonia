//! Approval logic: Admin auto-approve on submission, Member requires explicit approval.

use aggelmata::{RequestId, UserId, WantId};
use sqlx::SqlitePool;
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
        media_type: aggelmata::MediaType,
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

    /// Removes the wanted-media entry previously created for `request`.
    ///
    /// Compensation hook: when an approval loses the request-status
    /// compare-and-swap after `create_want` already ran, the request settled
    /// in a state that does not own the want (denied, or cancelled and
    /// deleted). Leaving the entry behind would let the monitor acquire media
    /// the household refused, so the losing approval retracts it.
    ///
    /// INVARIANT: implementations MUST be idempotent — removing a want that
    /// no longer exists succeeds.
    async fn remove_want(
        &self,
        request: &MediaRequest,
        want_id: WantId,
    ) -> Result<(), AitesisError>;
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
///
/// Concurrency: the status update is the commit point. It compares-and-swaps
/// on the status read at entry, so a decision committed during the
/// identity/create_want awaits (an admin deny, a cancel) wins and this
/// approval returns [`AitesisError::StaleTransition`] (or
/// [`AitesisError::RequestNotFound`]) instead of overwriting it. A lost
/// approval retracts the want it created when the surviving state does not
/// own it — see [`retract_lost_want`].
///
/// [`AitesisError::StaleTransition`]: crate::error::AitesisError::StaleTransition
/// [`AitesisError::RequestNotFound`]: crate::error::AitesisError::RequestNotFound
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
    let committed = crate::repo::update_status(
        pool,
        crate::repo::UpdateStatusParams {
            id: &request_id,
            expected_status: request.status,
            status: monitoring.to(),
            decided_by: Some(&admin_id),
            decided_at: Some(now),
            deny_reason: None,
            want_id: Some(&want_id),
        },
    )
    .await;

    if let Err(error) = committed {
        if matches!(
            error,
            AitesisError::StaleTransition { .. } | AitesisError::RequestNotFound { .. }
        ) {
            retract_lost_want(pool, &request, want_id, monitor).await;
        }
        return Err(error);
    }

    crate::repo::get_request(pool, &request_id)
        .await?
        .ok_or_else(|| {
            RequestNotFoundSnafu {
                id: request_id.to_string(),
            }
            .build()
        })
}

/// Best-effort retraction of the want created by an approval that lost the
/// status compare-and-swap.
///
/// Retracts only when the surviving row provably does not own the want:
/// `Denied` is terminal (no transition leaves it) and a deleted row cannot
/// return, so in both cases the want could only acquire media the household
/// refused. Any other status means a concurrent approval won and owns the
/// want, so it stays. Failures log rather than propagate — the caller already
/// surfaces the conflict, and a want leaked here requires a crash inside this
/// window (the same residual as the documented create_want/update_status
/// retry window).
async fn retract_lost_want<M>(
    pool: &SqlitePool,
    request: &MediaRequest,
    want_id: WantId,
    monitor: &M,
) where
    M: MonitorService,
{
    let surviving = match crate::repo::get_request(pool, &request.id).await {
        Ok(row) => row,
        Err(error) => {
            tracing::warn!(
                request_id = %request.id,
                want_id = %want_id,
                %error,
                "lost-approval want retraction skipped: request re-read failed"
            );
            return;
        }
    };

    let owned = surviving.is_some_and(|row| row.status != RequestStatus::Denied);
    if owned {
        return;
    }

    if let Err(error) = monitor.remove_want(request, want_id).await {
        tracing::warn!(
            request_id = %request.id,
            want_id = %want_id,
            %error,
            "lost-approval want retraction failed: want may linger for a refused request"
        );
    }
}

/// Denies a request: transitions to Denied with an optional reason.
///
/// Requires `admin_id` to have the Admin role.
///
/// Concurrency: the status update compares-and-swaps on the status read at
/// entry, so a decision that commits between the read and the write wins and
/// the stale deny surfaces [`AitesisError::StaleTransition`] instead of
/// overwriting it.
///
/// [`AitesisError::StaleTransition`]: crate::error::AitesisError::StaleTransition
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
            expected_status: request.status,
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
    use aggelmata::{MediaType, RequestId, UserId, WantId};
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    use super::*;
    use crate::repo::insert_request;
    use crate::types::{MediaRequest, RequestStatus};

    pub(crate) struct AlwaysValidIdentity;
    impl IdentityValidator for AlwaysValidIdentity {
        async fn validate(
            &self,
            _media_type: aggelmata::MediaType,
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

        async fn remove_want(
            &self,
            _request: &MediaRequest,
            _want_id: WantId,
        ) -> Result<(), AitesisError> {
            Ok(())
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

        async fn remove_want(
            &self,
            request: &MediaRequest,
            _want_id: WantId,
        ) -> Result<(), AitesisError> {
            self.wants.lock().unwrap().remove(&request.id);
            Ok(())
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

    /// Interposes a concurrent deny inside the approve flow's external await,
    /// making the lost-update race deterministic: the deny commits after
    /// approve read the row but before approve's status write runs.
    struct DenyDuringCreateWant {
        pool: SqlitePool,
        denier: UserId,
        created: std::sync::Mutex<Option<WantId>>,
        removed: std::sync::Mutex<Vec<WantId>>,
    }

    impl MonitorService for DenyDuringCreateWant {
        async fn create_want(&self, request: &MediaRequest) -> Result<WantId, AitesisError> {
            deny_request(
                &self.pool,
                request.id,
                self.denier,
                UserRole::Admin,
                Some("denied mid-approve".to_string()),
            )
            .await?;
            let want_id = WantId::new();
            *self.created.lock().unwrap() = Some(want_id);
            Ok(want_id)
        }

        async fn remove_want(
            &self,
            _request: &MediaRequest,
            want_id: WantId,
        ) -> Result<(), AitesisError> {
            self.removed.lock().unwrap().push(want_id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn approve_losing_to_concurrent_deny_keeps_denial_and_retracts_want() {
        let pool = setup().await;
        let user_id = UserId::new();
        let approver = UserId::new();
        let denier = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        let monitor = DenyDuringCreateWant {
            pool: pool.clone(),
            denier,
            created: std::sync::Mutex::new(None),
            removed: std::sync::Mutex::new(Vec::new()),
        };

        let err = approve_request(
            &pool,
            req_id,
            approver,
            UserRole::Admin,
            &AlwaysValidIdentity,
            &monitor,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, AitesisError::StaleTransition { .. }),
            "losing approve must surface the conflict, got: {err:?}"
        );

        // The deny that won the race is untouched — not clobbered to Monitoring.
        let row = crate::repo::get_request(&pool, &req_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, RequestStatus::Denied);
        assert_eq!(row.decided_by, Some(denier));
        assert_eq!(row.deny_reason.as_deref(), Some("denied mid-approve"));
        assert!(row.want_id.is_none());

        // The losing approve retracted the want it created.
        let created = monitor.created.lock().unwrap().expect("want was created");
        assert_eq!(*monitor.removed.lock().unwrap(), vec![created]);
    }

    /// Interposes a full competing approval inside the outer approve's
    /// external await: the inner approval wins the status compare-and-swap
    /// and owns the want.
    struct ApproveDuringCreateWant {
        pool: SqlitePool,
        winner: UserId,
        removed: std::sync::atomic::AtomicBool,
    }

    impl MonitorService for ApproveDuringCreateWant {
        async fn create_want(&self, request: &MediaRequest) -> Result<WantId, AitesisError> {
            approve_request(
                &self.pool,
                request.id,
                self.winner,
                UserRole::Admin,
                &AlwaysValidIdentity,
                &AlwaysCreateMonitor,
            )
            .await?;
            Ok(WantId::new())
        }

        async fn remove_want(
            &self,
            _request: &MediaRequest,
            _want_id: WantId,
        ) -> Result<(), AitesisError> {
            self.removed
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn approve_losing_to_concurrent_approve_keeps_winner_want() {
        let pool = setup().await;
        let user_id = UserId::new();
        let loser = UserId::new();
        let winner = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        let monitor = ApproveDuringCreateWant {
            pool: pool.clone(),
            winner,
            removed: std::sync::atomic::AtomicBool::new(false),
        };

        let err = approve_request(
            &pool,
            req_id,
            loser,
            UserRole::Admin,
            &AlwaysValidIdentity,
            &monitor,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AitesisError::StaleTransition { .. }));

        // The winning approval's state and want survive intact.
        let row = crate::repo::get_request(&pool, &req_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, RequestStatus::Monitoring);
        assert_eq!(row.decided_by, Some(winner));
        assert!(row.want_id.is_some());
        assert!(
            !monitor.removed.load(std::sync::atomic::Ordering::SeqCst),
            "a want owned by the winning approval must not be retracted"
        );
    }

    #[tokio::test]
    async fn deny_losing_to_concurrent_transition_surfaces_conflict() {
        let pool = setup().await;
        let user_id = UserId::new();
        let admin_id = UserId::new();
        let req = submitted_request(user_id);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        // Simulate a transition committing between deny's read and its write:
        // fetch the row first, move it, then run the stale write directly.
        let stale_read = crate::repo::get_request(&pool, &req_id)
            .await
            .unwrap()
            .unwrap();
        approve_request(
            &pool,
            req_id,
            admin_id,
            UserRole::Admin,
            &AlwaysValidIdentity,
            &AlwaysCreateMonitor,
        )
        .await
        .unwrap();

        let err = crate::repo::update_status(
            &pool,
            crate::repo::UpdateStatusParams {
                id: &req_id,
                expected_status: stale_read.status,
                status: RequestStatus::Denied,
                decided_by: Some(&admin_id),
                decided_at: Some(jiff::Timestamp::now()),
                deny_reason: Some("stale deny"),
                want_id: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AitesisError::StaleTransition { .. }));

        let row = crate::repo::get_request(&pool, &req_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, RequestStatus::Monitoring);
        assert!(row.want_id.is_some());
    }
}
