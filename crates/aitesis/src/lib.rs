//! Aitesis — household media request management for Harmonia.
//!
//! Replaces Overseerr. Handles submission, approval workflow, per-user limits,
//! and handoff to an injected monitor service for wanted-media tracking.

#![deny(missing_docs)]

pub mod approval;
pub mod error;
pub mod limits;
pub mod repo;
pub mod types;
pub mod workflow;

use aggelmata::{RequestId, UserId};
use apotheke::error::TransactionSnafu;
pub use approval::{IdentityValidator, MonitorService, UserRoleProvider};
pub use error::AitesisError;
use horismos::{AitesisConfig, Section};
use snafu::ResultExt;
use sqlx::SqlitePool;
use tracing::instrument;
pub use types::{CreateRequestInput, MediaRequest, RequestStatus, UserRole};

use crate::error::{InsufficientPermissionSnafu, RequestNotFoundSnafu};

/// Service trait for the full request lifecycle.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since 1.75; dyn dispatch not required here"
)]
pub trait RequestService: Send + Sync {
    /// Submits a new request. Admin users auto-approve when `auto_approve_admins` is set.
    ///
    /// The request row is always persisted as `Submitted` first; the
    /// auto-approve handoff (identity validation, want creation, transition to
    /// `Monitoring`) runs afterwards, so a handoff failure leaves a
    /// recoverable `Submitted` row that an admin can re-approve.
    async fn submit_request(
        &self,
        user_id: UserId,
        input: CreateRequestInput,
    ) -> Result<MediaRequest, AitesisError>;

    /// Approves a Submitted request — requires Admin role.
    async fn approve(
        &self,
        request_id: RequestId,
        admin_id: UserId,
    ) -> Result<MediaRequest, AitesisError>;

    /// Denies a Submitted request — requires Admin role.
    async fn deny(
        &self,
        request_id: RequestId,
        admin_id: UserId,
        reason: Option<String>,
    ) -> Result<MediaRequest, AitesisError>;

    /// Returns a single request by ID.
    ///
    /// Authorization: admins may read any request; members may only read
    /// their own (a non-owner member is rejected with
    /// [`AitesisError::InsufficientPermission`]).
    async fn get_request(
        &self,
        request_id: RequestId,
        caller_id: UserId,
    ) -> Result<MediaRequest, AitesisError>;

    /// Lists requests, optionally filtered by user or status, windowed by
    /// `limit`/`offset` (newest first).
    ///
    /// Authorization: admins may list any user's requests or all requests;
    /// members may only list their own (`user_id` must equal
    /// `Some(caller_id)`).
    async fn list_requests(
        &self,
        caller_id: UserId,
        user_id: Option<UserId>,
        status: Option<RequestStatus>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MediaRequest>, AitesisError>;

    /// Counts requests matching the same filters as [`Self::list_requests`].
    ///
    /// Same authorization rules as `list_requests`.
    async fn count_requests(
        &self,
        caller_id: UserId,
        user_id: Option<UserId>,
        status: Option<RequestStatus>,
    ) -> Result<u64, AitesisError>;

    /// Cancels a request. Users may cancel their own; admins may cancel any.
    ///
    /// Errors with [`AitesisError::StaleTransition`] when the request's
    /// status changes between the authorization read and the delete — the
    /// concurrent decision wins and the caller must re-read before retrying.
    ///
    /// [`AitesisError::StaleTransition`]: crate::error::AitesisError::StaleTransition
    async fn cancel_request(
        &self,
        request_id: RequestId,
        user_id: UserId,
    ) -> Result<(), AitesisError>;
}

/// Live implementation backed by SQLite.
///
/// Type parameters allow injecting mock role providers, identity validators, and
/// monitor services for tests without requiring heap allocation via `dyn Trait`.
pub struct AitesisServiceImpl<R, I, M> {
    read: SqlitePool,
    write: SqlitePool,
    // WHY: a live `Section` (not a frozen `AitesisConfig`) — #529 step 8
    // makes `max_pending_per_user`/`max_requests_per_day`/`auto_approve_admins`
    // live; `submit_request` takes ONE snapshot per call (below), so a
    // mid-submission reload cannot mix an old limit with a new one.
    config: Section<AitesisConfig>,
    user_roles: R,
    identity: I,
    monitor: M,
}

impl<R, I, M> AitesisServiceImpl<R, I, M>
where
    R: UserRoleProvider,
    I: IdentityValidator,
    M: MonitorService,
{
    /// Creates a request service with explicit storage and boundary adapters.
    pub fn new(
        read: SqlitePool,
        write: SqlitePool,
        config: Section<AitesisConfig>,
        user_roles: R,
        identity: I,
        monitor: M,
    ) -> Self {
        Self {
            read,
            write,
            config,
            user_roles,
            identity,
            monitor,
        }
    }
}

impl<R, I, M> RequestService for AitesisServiceImpl<R, I, M>
where
    R: UserRoleProvider,
    I: IdentityValidator,
    M: MonitorService,
{
    #[instrument(skip(self), fields(user_id = %user_id))]
    async fn submit_request(
        &self,
        user_id: UserId,
        input: CreateRequestInput,
    ) -> Result<MediaRequest, AitesisError> {
        let role = self.user_roles.role_of(user_id).await?;

        // WHY: one snapshot for the whole operation — a mid-submit reload
        // cannot mix an old `auto_approve_admins` read with a new
        // `max_pending_per_user`/`max_requests_per_day` FROM after it.
        let config = self.config.get();
        let auto_approve = role == UserRole::Admin && config.auto_approve_admins;

        let now = jiff::Timestamp::now();
        let request = MediaRequest {
            id: RequestId::new(),
            user_id,
            media_type: input.media_type,
            title: input.title,
            external_id: input.external_id,
            status: RequestStatus::Submitted,
            decided_by: None,
            decided_at: None,
            deny_reason: None,
            want_id: None,
            created_at: now,
        };

        // WHY: the limit check and the insert run in ONE write transaction —
        // checked against the pool they race, letting concurrent submissions
        // exceed max_pending_per_user / max_requests_per_day.
        let mut tx = self
            .write
            .begin()
            .await
            .context(TransactionSnafu)
            .context(crate::error::DatabaseSnafu)?;
        limits::check_limits(
            &mut tx,
            &user_id,
            role,
            config.max_pending_per_user,
            config.max_requests_per_day,
        )
        .await?;
        repo::insert_request(&mut *tx, &request).await?;
        tx.commit()
            .await
            .context(TransactionSnafu)
            .context(crate::error::DatabaseSnafu)?;

        // WHY: the row is persisted as Submitted BEFORE the identity/monitor
        // handoff, so a handoff failure leaves a recoverable Submitted row
        // (re-approvable by an admin) instead of an orphaned terminal state.
        // The auto-approve path reuses approve_request so the
        // validate -> create-want -> update-status sequence exists once.
        if auto_approve {
            return approval::approve_request(
                &self.write,
                request.id,
                user_id,
                role,
                &self.identity,
                &self.monitor,
            )
            .await;
        }

        Ok(request)
    }

    #[instrument(skip(self), fields(request_id = %request_id, admin_id = %admin_id))]
    async fn approve(
        &self,
        request_id: RequestId,
        admin_id: UserId,
    ) -> Result<MediaRequest, AitesisError> {
        let role = self.user_roles.role_of(admin_id).await?;
        approval::approve_request(
            &self.write,
            request_id,
            admin_id,
            role,
            &self.identity,
            &self.monitor,
        )
        .await
    }

    #[instrument(skip(self), fields(request_id = %request_id, admin_id = %admin_id))]
    async fn deny(
        &self,
        request_id: RequestId,
        admin_id: UserId,
        reason: Option<String>,
    ) -> Result<MediaRequest, AitesisError> {
        let role = self.user_roles.role_of(admin_id).await?;
        approval::deny_request(&self.write, request_id, admin_id, role, reason).await
    }

    #[instrument(skip(self), fields(request_id = %request_id, caller_id = %caller_id))]
    async fn get_request(
        &self,
        request_id: RequestId,
        caller_id: UserId,
    ) -> Result<MediaRequest, AitesisError> {
        let request = repo::get_request(&self.read, &request_id)
            .await?
            .ok_or_else(|| {
                RequestNotFoundSnafu {
                    id: request_id.to_string(),
                }
                .build()
            })?;

        // WHY: same ownership boundary as cancel_request — a member reading
        // another user's request by UUID is an IDOR (title, decided_by,
        // deny_reason, want_id all leak).
        let role = self.user_roles.role_of(caller_id).await?;
        let is_owner = request.user_id == caller_id;
        let is_admin = role == UserRole::Admin;

        if !is_owner && !is_admin {
            return InsufficientPermissionSnafu.fail();
        }

        Ok(request)
    }

    #[instrument(skip(self), fields(caller_id = %caller_id))]
    async fn list_requests(
        &self,
        caller_id: UserId,
        user_id: Option<UserId>,
        status: Option<RequestStatus>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MediaRequest>, AitesisError> {
        let caller_role = self.user_roles.role_of(caller_id).await?;
        if caller_role != UserRole::Admin && user_id != Some(caller_id) {
            return InsufficientPermissionSnafu.fail();
        }

        let page = repo::Page::new(limit, offset);
        match (user_id, status) {
            (Some(uid), Some(st)) => {
                repo::list_by_user_and_status(&self.read, &uid, st, page).await
            }
            (Some(uid), None) => repo::list_by_user(&self.read, &uid, page).await,
            (None, Some(st)) => repo::list_by_status(&self.read, st, page).await,
            (None, None) => repo::list_all(&self.read, page).await,
        }
    }

    #[instrument(skip(self), fields(caller_id = %caller_id))]
    async fn count_requests(
        &self,
        caller_id: UserId,
        user_id: Option<UserId>,
        status: Option<RequestStatus>,
    ) -> Result<u64, AitesisError> {
        let caller_role = self.user_roles.role_of(caller_id).await?;
        if caller_role != UserRole::Admin && user_id != Some(caller_id) {
            return InsufficientPermissionSnafu.fail();
        }

        let count = repo::count_requests(&self.read, user_id.as_ref(), status).await?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    #[instrument(skip(self), fields(request_id = %request_id, user_id = %user_id))]
    async fn cancel_request(
        &self,
        request_id: RequestId,
        user_id: UserId,
    ) -> Result<(), AitesisError> {
        let request = repo::get_request(&self.read, &request_id)
            .await?
            .ok_or_else(|| {
                RequestNotFoundSnafu {
                    id: request_id.to_string(),
                }
                .build()
            })?;

        let role = self.user_roles.role_of(user_id).await?;
        let is_owner = request.user_id == user_id;
        let is_admin = role == UserRole::Admin;

        if !is_owner && !is_admin {
            return InsufficientPermissionSnafu.fail();
        }

        // Terminal statuses cannot be cancelled.
        if matches!(
            request.status,
            RequestStatus::Fulfilled | RequestStatus::Failed | RequestStatus::Denied
        ) {
            return crate::error::InvalidTransitionSnafu {
                from: request.status.as_str().to_string(),
                to: "cancelled".to_string(),
            }
            .fail();
        }

        // WHY: the delete compares-and-swaps on the status this cancel was
        // authorized against — a decision committed after that read (an
        // approve or deny) wins instead of being silently erased.
        repo::delete_request(&self.write, &request_id, request.status).await
    }
}

#[cfg(test)]
mod tests {
    use aggelmata::{MediaType, UserId, WantId};
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    use super::*;
    use crate::approval::{IdentityValidator, MonitorService, UserRoleProvider};
    use crate::types::{CreateRequestInput, MediaRequest, RequestStatus, UserRole};

    // ── Mock helpers ──────────────────────────────────────────────────────────

    struct MockRoles {
        role: UserRole,
    }

    impl UserRoleProvider for MockRoles {
        async fn role_of(&self, _user_id: UserId) -> Result<UserRole, AitesisError> {
            Ok(self.role)
        }
    }

    struct AlwaysValidIdentity;
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

    struct AlwaysCreateMonitor;
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

    struct RejectingIdentity;
    impl IdentityValidator for RejectingIdentity {
        async fn validate(
            &self,
            _media_type: aggelmata::MediaType,
            _title: &str,
            _external_id: Option<&str>,
        ) -> Result<(), AitesisError> {
            crate::error::MediaIdentityInvalidSnafu {
                detail: "injected validation failure".to_string(),
            }
            .fail()
        }
    }

    struct FailingMonitor;
    impl MonitorService for FailingMonitor {
        async fn create_want(&self, _request: &MediaRequest) -> Result<WantId, AitesisError> {
            crate::error::MediaIdentityInvalidSnafu {
                detail: "injected create_want failure".to_string(),
            }
            .fail()
        }

        async fn remove_want(
            &self,
            _request: &MediaRequest,
            _want_id: WantId,
        ) -> Result<(), AitesisError> {
            Ok(())
        }
    }

    fn default_config() -> Section<AitesisConfig> {
        Section::fixed(AitesisConfig::default())
    }

    type TestService = AitesisServiceImpl<MockRoles, AlwaysValidIdentity, AlwaysCreateMonitor>;

    async fn make_service(role: UserRole) -> (TestService, SqlitePool) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let config = default_config();
        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            config,
            MockRoles { role },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        (svc, pool)
    }

    fn music_input() -> CreateRequestInput {
        CreateRequestInput {
            media_type: MediaType::Music,
            title: "Kind of Blue".to_string(),
            external_id: None,
        }
    }

    // ── Submit tests ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn member_submit_status_is_submitted() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let user_id = UserId::new();
        let req = svc.submit_request(user_id, music_input()).await.unwrap();
        assert_eq!(req.status, RequestStatus::Submitted);
    }

    #[tokio::test]
    async fn admin_submit_with_auto_approve_status_is_monitoring() {
        let (svc, _pool) = make_service(UserRole::Admin).await;
        let user_id = UserId::new();
        let req = svc.submit_request(user_id, music_input()).await.unwrap();
        // auto_approve_admins is true by default — goes straight to Monitoring
        assert_eq!(req.status, RequestStatus::Monitoring);
        assert_eq!(req.decided_by, Some(user_id));
        assert!(req.want_id.is_some());
    }

    #[tokio::test]
    async fn auto_approve_validation_failure_leaves_row_submitted() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            RejectingIdentity,
            AlwaysCreateMonitor,
        );
        let user_id = UserId::new();

        let err = svc
            .submit_request(user_id, music_input())
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::MediaIdentityInvalid { .. }));

        let rows = crate::repo::list_all(&pool, crate::repo::Page::new(100, 0))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, RequestStatus::Submitted);
        assert!(rows[0].want_id.is_none());
    }

    #[tokio::test]
    async fn auto_approve_create_want_failure_leaves_row_submitted() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            FailingMonitor,
        );
        let user_id = UserId::new();

        let err = svc
            .submit_request(user_id, music_input())
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::MediaIdentityInvalid { .. }));

        let rows = crate::repo::list_all(&pool, crate::repo::Page::new(100, 0))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, RequestStatus::Submitted);
        assert!(rows[0].want_id.is_none());
    }

    #[tokio::test]
    async fn admin_can_retry_after_auto_approve_failure() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let failing_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            RejectingIdentity,
            AlwaysCreateMonitor,
        );
        let admin_id = UserId::new();

        failing_svc
            .submit_request(admin_id, music_input())
            .await
            .unwrap_err();
        let rows = crate::repo::list_all(&pool, crate::repo::Page::new(100, 0))
            .await
            .unwrap();
        assert_eq!(rows[0].status, RequestStatus::Submitted);

        let working_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let approved = working_svc.approve(rows[0].id, admin_id).await.unwrap();
        assert_eq!(approved.status, RequestStatus::Monitoring);
        assert!(approved.want_id.is_some());
    }

    // ── Approve tests ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn admin_approves_member_request_transitions_to_monitoring() {
        let (member_svc, pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();
        let req = member_svc
            .submit_request(member_id, music_input())
            .await
            .unwrap();
        assert_eq!(req.status, RequestStatus::Submitted);

        let admin_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let admin_id = UserId::new();
        let approved = admin_svc.approve(req.id, admin_id).await.unwrap();
        assert_eq!(approved.status, RequestStatus::Monitoring);
        assert_eq!(approved.decided_by, Some(admin_id));
        assert!(approved.want_id.is_some());
    }

    // ── Deny tests ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn admin_denies_request_records_reason() {
        let (member_svc, pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();
        let req = member_svc
            .submit_request(member_id, music_input())
            .await
            .unwrap();

        let admin_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let admin_id = UserId::new();
        let denied = admin_svc
            .deny(req.id, admin_id, Some("Out of scope".to_string()))
            .await
            .unwrap();

        assert_eq!(denied.status, RequestStatus::Denied);
        assert_eq!(denied.deny_reason.as_deref(), Some("Out of scope"));
    }

    // ── Cancel tests ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn user_cancels_own_request() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let user_id = UserId::new();
        let req = svc.submit_request(user_id, music_input()).await.unwrap();

        svc.cancel_request(req.id, user_id).await.unwrap();

        let result = svc.get_request(req.id, user_id).await;
        assert!(matches!(result, Err(AitesisError::RequestNotFound { .. })));
    }

    #[tokio::test]
    async fn member_cannot_cancel_other_user_request() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let alice = UserId::new();
        let bob = UserId::new();
        let req = svc.submit_request(alice, music_input()).await.unwrap();

        let err = svc.cancel_request(req.id, bob).await.unwrap_err();
        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));
    }

    /// Interposes a concurrent deny inside cancel's role-lookup await, making
    /// the cancel-vs-deny race deterministic: the deny commits after cancel
    /// authorized against the row it read but before the delete runs.
    struct DenyDuringRoleLookup {
        pool: SqlitePool,
        request_id: aggelmata::RequestId,
        denier: UserId,
    }

    impl UserRoleProvider for DenyDuringRoleLookup {
        async fn role_of(&self, _user_id: UserId) -> Result<UserRole, AitesisError> {
            crate::repo::update_status(
                &self.pool,
                crate::repo::UpdateStatusParams {
                    id: &self.request_id,
                    expected_status: RequestStatus::Submitted,
                    status: RequestStatus::Denied,
                    decided_by: Some(&self.denier),
                    decided_at: Some(jiff::Timestamp::now()),
                    deny_reason: Some("denied mid-cancel"),
                    want_id: None,
                },
            )
            .await?;
            Ok(UserRole::Member)
        }
    }

    #[tokio::test]
    async fn cancel_losing_to_concurrent_deny_keeps_denial() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let owner = UserId::new();
        let denier = UserId::new();
        let req = raw_request(owner);
        let req_id = req.id;
        crate::repo::insert_request(&pool, &req).await.unwrap();

        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            DenyDuringRoleLookup {
                pool: pool.clone(),
                request_id: req_id,
                denier,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );

        let err = svc.cancel_request(req_id, owner).await.unwrap_err();
        assert!(
            matches!(err, AitesisError::StaleTransition { .. }),
            "stale cancel must surface the conflict, got: {err:?}"
        );

        // The denial that won the race survives — not erased by the cancel.
        let row = crate::repo::get_request(&pool, &req_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, RequestStatus::Denied);
        assert_eq!(row.decided_by, Some(denier));
        assert_eq!(row.deny_reason.as_deref(), Some("denied mid-cancel"));
    }

    fn raw_request(user_id: UserId) -> MediaRequest {
        MediaRequest {
            id: aggelmata::RequestId::new(),
            user_id,
            media_type: MediaType::Music,
            title: "Test Album".to_string(),
            external_id: None,
            status: RequestStatus::Submitted,
            decided_by: None,
            decided_at: None,
            deny_reason: None,
            want_id: None,
            created_at: jiff::Timestamp::now(),
        }
    }

    // ── Pagination tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_requests_windows_by_limit_and_offset() {
        let (svc, pool) = make_service(UserRole::Admin).await;
        let admin_id = UserId::new();
        let user_id = UserId::new();
        for i in 0..5 {
            let mut req = raw_request(user_id);
            req.title = format!("Album {i}");
            crate::repo::insert_request(&pool, &req).await.unwrap();
        }

        let first = svc.list_requests(admin_id, None, None, 2, 0).await.unwrap();
        assert_eq!(first.len(), 2);

        let tail = svc.list_requests(admin_id, None, None, 2, 4).await.unwrap();
        assert_eq!(tail.len(), 1);

        let total = svc.count_requests(admin_id, None, None).await.unwrap();
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn count_requests_member_scoped_to_self() {
        let (svc, pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();
        let other_id = UserId::new();
        crate::repo::insert_request(&pool, &raw_request(member_id))
            .await
            .unwrap();
        crate::repo::insert_request(&pool, &raw_request(other_id))
            .await
            .unwrap();

        let own = svc
            .count_requests(member_id, Some(member_id), None)
            .await
            .unwrap();
        assert_eq!(own, 1);

        let err = svc
            .count_requests(member_id, Some(other_id), None)
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));
    }

    // ── Concurrency tests ────────────────────────────────────────────────────

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_submissions_cannot_exceed_pending_limit() {
        use std::sync::Arc;

        // WHY: file-backed DB — an in-memory pool gives each connection its
        // own database, which would hide the cross-connection race under test.
        let dir = tempfile::tempdir().unwrap();
        let url = format!(
            "sqlite://{}?mode=rwc",
            dir.path().join("aitesis.db").display()
        );
        let pool = SqlitePool::connect(&url).await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();

        let config = Section::fixed(AitesisConfig {
            max_pending_per_user: 1,
            max_requests_per_day: 100,
            auto_approve_admins: false,
        });
        let svc = Arc::new(AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            config,
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        ));
        let user_id = UserId::new();

        let mut handles = Vec::new();
        for _ in 0..8 {
            let svc = Arc::clone(&svc);
            handles.push(tokio::spawn(async move {
                svc.submit_request(user_id, music_input()).await
            }));
        }
        let mut ok = 0usize;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                ok += 1;
            }
        }

        assert_eq!(
            ok, 1,
            "exactly one concurrent submission may pass the limit"
        );
        let pending = crate::repo::count_pending_by_user(&pool, &user_id)
            .await
            .unwrap();
        assert_eq!(pending, 1, "the pending limit must hold in the database");
    }

    // ── Limit tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn member_blocked_when_pending_limit_reached() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let config = Section::fixed(AitesisConfig {
            max_pending_per_user: 2,
            max_requests_per_day: 100,
            auto_approve_admins: true,
        });
        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            config,
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let user_id = UserId::new();

        svc.submit_request(user_id, music_input()).await.unwrap();
        svc.submit_request(user_id, music_input()).await.unwrap();

        let err = svc
            .submit_request(user_id, music_input())
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::RequestLimitExceeded { .. }));
    }

    #[tokio::test]
    async fn member_blocked_when_daily_limit_reached() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let config = Section::fixed(AitesisConfig {
            max_pending_per_user: 100,
            max_requests_per_day: 2,
            auto_approve_admins: true,
        });
        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            config,
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let user_id = UserId::new();

        svc.submit_request(user_id, music_input()).await.unwrap();
        svc.submit_request(user_id, music_input()).await.unwrap();

        let err = svc
            .submit_request(user_id, music_input())
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::RequestLimitExceeded { .. }));
    }

    #[tokio::test]
    async fn admin_exempt_from_limits() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let config = Section::fixed(AitesisConfig {
            max_pending_per_user: 1,
            max_requests_per_day: 1,
            // WHY: auto_approve disabled so requests stay in Submitted/Monitoring counts
            // would normally trigger the limit — but admin is exempt regardless.
            auto_approve_admins: false,
        });
        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            config,
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let user_id = UserId::new();

        svc.submit_request(user_id, music_input()).await.unwrap();
        svc.submit_request(user_id, music_input()).await.unwrap();
        svc.submit_request(user_id, music_input()).await.unwrap();
    }

    // ── Invalid transition ────────────────────────────────────────────────────

    #[tokio::test]
    async fn denied_request_cannot_be_approved() {
        let (member_svc, pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();
        let req = member_svc
            .submit_request(member_id, music_input())
            .await
            .unwrap();

        let admin_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let admin_id = UserId::new();

        admin_svc.deny(req.id, admin_id, None).await.unwrap();

        let err = admin_svc.approve(req.id, admin_id).await.unwrap_err();
        assert!(matches!(err, AitesisError::InvalidTransition { .. }));
    }

    // ── List tests ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_requests_filter_by_user() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let config = default_config();

        let alice = UserId::new();
        let bob = UserId::new();

        let alice_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            config.clone(),
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let bob_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            config,
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );

        alice_svc
            .submit_request(alice, music_input())
            .await
            .unwrap();
        alice_svc
            .submit_request(alice, music_input())
            .await
            .unwrap();
        bob_svc.submit_request(bob, music_input()).await.unwrap();

        let alice_requests = alice_svc
            .list_requests(alice, Some(alice), None, 100, 0)
            .await
            .unwrap();
        assert_eq!(alice_requests.len(), 2);
        assert!(alice_requests.iter().all(|r| r.user_id == alice));
    }

    #[tokio::test]
    async fn list_requests_filter_by_status() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let member_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let admin_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let user_id = UserId::new();
        let admin_id = UserId::new();

        member_svc
            .submit_request(user_id, music_input())
            .await
            .unwrap();
        member_svc
            .submit_request(user_id, music_input())
            .await
            .unwrap();

        let submitted = admin_svc
            .list_requests(admin_id, None, Some(RequestStatus::Submitted), 100, 0)
            .await
            .unwrap();
        assert_eq!(submitted.len(), 2);

        let monitoring = admin_svc
            .list_requests(admin_id, None, Some(RequestStatus::Monitoring), 100, 0)
            .await
            .unwrap();
        assert!(monitoring.is_empty());
    }

    #[tokio::test]
    async fn member_list_requests_with_no_filter_is_denied() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();

        let err = svc
            .list_requests(member_id, None, None, 100, 0)
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));
    }

    #[tokio::test]
    async fn member_list_requests_for_other_user_is_denied() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();
        let other_id = UserId::new();
        svc.submit_request(other_id, music_input()).await.unwrap();

        let err = svc
            .list_requests(member_id, Some(other_id), None, 100, 0)
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));

        let err = svc
            .list_requests(
                member_id,
                Some(other_id),
                Some(RequestStatus::Submitted),
                100,
                0,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));
    }

    #[tokio::test]
    async fn member_list_requests_status_only_is_denied() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();

        let err = svc
            .list_requests(member_id, None, Some(RequestStatus::Submitted), 100, 0)
            .await
            .unwrap_err();
        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));
    }

    #[tokio::test]
    async fn member_list_requests_for_self_succeeds() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();
        let other_id = UserId::new();

        svc.submit_request(member_id, music_input()).await.unwrap();
        svc.submit_request(other_id, music_input()).await.unwrap();

        let own = svc
            .list_requests(member_id, Some(member_id), None, 100, 0)
            .await
            .unwrap();
        assert_eq!(own.len(), 1);
        assert!(own.iter().all(|r| r.user_id == member_id));
    }

    #[tokio::test]
    async fn admin_list_requests_with_no_filter_returns_all() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let member_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let admin_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let alice = UserId::new();
        let bob = UserId::new();
        let admin_id = UserId::new();

        member_svc
            .submit_request(alice, music_input())
            .await
            .unwrap();
        member_svc.submit_request(bob, music_input()).await.unwrap();

        let all = admin_svc
            .list_requests(admin_id, None, None, 100, 0)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);

        let bobs = admin_svc
            .list_requests(admin_id, Some(bob), None, 100, 0)
            .await
            .unwrap();
        assert_eq!(bobs.len(), 1);
        assert_eq!(bobs[0].user_id, bob);
    }

    // ── Full lifecycle ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn full_lifecycle_submitted_to_fulfilled() {
        let (member_svc, pool) = make_service(UserRole::Member).await;
        let member_id = UserId::new();
        let req = member_svc
            .submit_request(member_id, music_input())
            .await
            .unwrap();
        assert_eq!(req.status, RequestStatus::Submitted);

        let admin_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let admin_id = UserId::new();

        let monitoring = admin_svc.approve(req.id, admin_id).await.unwrap();
        assert_eq!(monitoring.status, RequestStatus::Monitoring);

        // Simulate the monitoring layer updating status to Fulfilled.
        crate::repo::update_status(
            &pool,
            crate::repo::UpdateStatusParams {
                id: &req.id,
                expected_status: RequestStatus::Monitoring,
                status: RequestStatus::Fulfilled,
                decided_by: None,
                decided_at: None,
                deny_reason: None,
                want_id: None,
            },
        )
        .await
        .unwrap();

        let fulfilled = admin_svc.get_request(req.id, admin_id).await.unwrap();
        assert_eq!(fulfilled.status, RequestStatus::Fulfilled);
    }

    // ── Get tests ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn owner_gets_own_request() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let user_id = UserId::new();
        let req = svc.submit_request(user_id, music_input()).await.unwrap();

        let fetched = svc.get_request(req.id, user_id).await.unwrap();
        assert_eq!(fetched.id, req.id);
        assert_eq!(fetched.user_id, user_id);
    }

    #[tokio::test]
    async fn member_cannot_get_other_user_request() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let alice = UserId::new();
        let bob = UserId::new();
        let req = svc.submit_request(alice, music_input()).await.unwrap();

        let err = svc.get_request(req.id, bob).await.unwrap_err();
        assert!(matches!(err, AitesisError::InsufficientPermission { .. }));
    }

    #[tokio::test]
    async fn admin_gets_any_user_request() {
        let (member_svc, pool) = make_service(UserRole::Member).await;
        let alice = UserId::new();
        let req = member_svc
            .submit_request(alice, music_input())
            .await
            .unwrap();

        let admin_svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            default_config(),
            MockRoles {
                role: UserRole::Admin,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let admin_id = UserId::new();
        let fetched = admin_svc.get_request(req.id, admin_id).await.unwrap();
        assert_eq!(fetched.id, req.id);
        assert_eq!(fetched.user_id, alice);
    }

    // ── Live config ───────────────────────────────────────────────────────────

    // #529 step 8: a `max_pending_per_user`/`max_requests_per_day` change made
    // through a REAL `ConfigManager::replace` must be visible on the NEXT
    // request op — no service rebuild.
    #[tokio::test]
    async fn limit_change_is_visible_on_next_submit_request() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();

        let mut boot = horismos::Config::default();
        boot.exousia.jwt_secret = "test-secret-that-is-long-enough-for-hs256".to_string();
        boot.aitesis = AitesisConfig {
            max_pending_per_user: 1,
            max_requests_per_day: 100,
            auto_approve_admins: false,
        };
        let (manager, handle) = horismos::ConfigManager::new(
            boot.clone(),
            std::path::PathBuf::from("unused.toml"),
            horismos::ConfigOverrides::default(),
        );

        let svc = AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            handle.section(|c| &c.aitesis),
            MockRoles {
                role: UserRole::Member,
            },
            AlwaysValidIdentity,
            AlwaysCreateMonitor,
        );
        let user_id = UserId::new();

        svc.submit_request(user_id, music_input()).await.unwrap();
        let err = svc
            .submit_request(user_id, music_input())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AitesisError::RequestLimitExceeded { .. }),
            "boot-time limit of 1 must reject a second pending request"
        );

        let mut raised = boot.clone();
        raised.aitesis.max_pending_per_user = 2;
        manager
            .replace(raised)
            .expect("replace applies the raised pending limit");

        svc.submit_request(user_id, music_input())
            .await
            .expect("the raised live limit must admit a third request on the NEXT op");
    }
}
