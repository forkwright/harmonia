//! Aitesis — household media request management for Harmonia.
//!
//! Replaces Overseerr. Handles submission, approval workflow, per-user limits,
//! and handoff to Episkope for monitoring.

pub mod approval;
pub mod error;
pub mod limits;
pub mod repo;
pub mod types;
pub mod workflow;

pub use approval::{IdentityValidator, MonitorService, UserRoleProvider};
pub use error::AitesisError;
pub use types::{CreateRequestInput, MediaRequest, RequestStatus, UserRole};

use themelion::{RequestId, UserId};
use horismos::AitesisConfig;
use sqlx::SqlitePool;
use tracing::instrument;

use crate::error::{InsufficientPermissionSnafu, RequestNotFoundSnafu};

/// Service trait for the full request lifecycle.
#[expect(
    async_fn_in_trait,
    reason = "async fn in trait stable since 1.75; dyn dispatch not required here"
)]
pub trait RequestService: Send + Sync {
    /// Submits a new request. Admin users auto-approve when `auto_approve_admins` is set.
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
    async fn get_request(&self, request_id: RequestId) -> Result<MediaRequest, AitesisError>;

    /// Lists requests, optionally filtered by user or status.
    async fn list_requests(
        &self,
        user_id: Option<UserId>,
        status: Option<RequestStatus>,
    ) -> Result<Vec<MediaRequest>, AitesisError>;

    /// Cancels a request. Users may cancel their own; admins may cancel any.
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
    config: AitesisConfig,
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
    pub fn new(
        read: SqlitePool,
        write: SqlitePool,
        config: AitesisConfig,
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

        limits::check_limits(
            &self.read,
            &user_id,
            role,
            self.config.max_pending_per_user,
            self.config.max_requests_per_day,
        )
        .await?;

        let auto_approve = role == UserRole::Admin && self.config.auto_approve_admins;
        let status = if auto_approve {
            RequestStatus::Approved
        } else {
            RequestStatus::Submitted
        };

        let now = jiff::Timestamp::now();
        let request = MediaRequest {
            id: RequestId::new(),
            user_id,
            media_type: input.media_type,
            title: input.title,
            external_id: input.external_id,
            status,
            decided_by: None,
            decided_at: None,
            deny_reason: None,
            want_id: None,
            created_at: now,
        };

        repo::insert_request(&self.write, &request).await?;

        // Admin auto-approve: immediately validate identity, create the want, and
        // transition to Monitoring in a single submit call.
        if auto_approve {
            self.identity
                .validate(
                    request.media_type,
                    &request.title,
                    request.external_id.as_deref(),
                )
                .await?;
            let want_id = self.monitor.create_want(&request).await?;
            repo::update_status(
                &self.write,
                &request.id,
                RequestStatus::Monitoring,
                Some(&user_id),
                Some(jiff::Timestamp::now()),
                None,
                Some(&want_id),
            )
            .await?;
            return repo::get_request(&self.read, &request.id)
                .await?
                .ok_or_else(|| {
                    RequestNotFoundSnafu {
                        id: request.id.to_string(),
                    }
                    .build()
                });
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

    #[instrument(skip(self), fields(request_id = %request_id))]
    async fn get_request(&self, request_id: RequestId) -> Result<MediaRequest, AitesisError> {
        repo::get_request(&self.read, &request_id)
            .await?
            .ok_or_else(|| {
                RequestNotFoundSnafu {
                    id: request_id.to_string(),
                }
                .build()
            })
    }

    #[instrument(skip(self))]
    async fn list_requests(
        &self,
        user_id: Option<UserId>,
        status: Option<RequestStatus>,
    ) -> Result<Vec<MediaRequest>, AitesisError> {
        match (user_id, status) {
            (Some(uid), Some(st)) => {
                let all = repo::list_by_user(&self.read, &uid).await?;
                Ok(all.into_iter().filter(|r| r.status == st).collect())
            }
            (Some(uid), None) => repo::list_by_user(&self.read, &uid).await,
            (None, Some(st)) => repo::list_by_status(&self.read, st).await,
            (None, None) => repo::list_all(&self.read).await,
        }
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

        repo::delete_request(&self.write, &request_id).await
    }
}

#[cfg(test)]
mod tests {
    use themelion::{MediaType, UserId, WantId};
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
            _media_type: themelion::MediaType,
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
    }

    fn default_config() -> AitesisConfig {
        AitesisConfig::default()
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

        let result = svc.get_request(req.id).await;
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

    // ── Limit tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn member_blocked_when_pending_limit_reached() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let config = AitesisConfig {
            max_pending_per_user: 2,
            max_requests_per_day: 100,
            auto_approve_admins: true,
        };
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
        let config = AitesisConfig {
            max_pending_per_user: 100,
            max_requests_per_day: 2,
            auto_approve_admins: true,
        };
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
        let config = AitesisConfig {
            max_pending_per_user: 1,
            max_requests_per_day: 1,
            // WHY: auto_approve disabled so requests stay in Submitted/Monitoring counts
            // would normally trigger the limit — but admin is exempt regardless.
            auto_approve_admins: false,
        };
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

        let alice_requests = alice_svc.list_requests(Some(alice), None).await.unwrap();
        assert_eq!(alice_requests.len(), 2);
        assert!(alice_requests.iter().all(|r| r.user_id == alice));
    }

    #[tokio::test]
    async fn list_requests_filter_by_status() {
        let (svc, _pool) = make_service(UserRole::Member).await;
        let user_id = UserId::new();

        svc.submit_request(user_id, music_input()).await.unwrap();
        svc.submit_request(user_id, music_input()).await.unwrap();

        let submitted = svc
            .list_requests(None, Some(RequestStatus::Submitted))
            .await
            .unwrap();
        assert_eq!(submitted.len(), 2);

        let monitoring = svc
            .list_requests(None, Some(RequestStatus::Monitoring))
            .await
            .unwrap();
        assert!(monitoring.is_empty());
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

        // Simulate Episkope updating status to Fulfilled
        crate::repo::update_status(
            &pool,
            &req.id,
            RequestStatus::Fulfilled,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let fulfilled = admin_svc.get_request(req.id).await.unwrap();
        assert_eq!(fulfilled.status, RequestStatus::Fulfilled);
    }
}
