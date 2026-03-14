//! Per-user request limits: max pending and daily rate limit.
//!
//! Admin users are exempt from all limits.

use harmonia_common::UserId;
use sqlx::SqlitePool;

use crate::error::{AitesisError, RequestLimitExceededSnafu};
use crate::types::UserRole;

/// Checks per-user request limits and returns an error if any limit is exceeded.
///
/// Admin users are exempt.
pub(crate) async fn check_limits(
    pool: &SqlitePool,
    user_id: &UserId,
    role: UserRole,
    max_pending: u32,
    max_per_day: u32,
) -> Result<(), AitesisError> {
    if role == UserRole::Admin {
        return Ok(());
    }

    let pending = crate::repo::count_pending_by_user(pool, user_id).await?;
    if pending >= i64::from(max_pending) {
        return RequestLimitExceededSnafu.fail();
    }

    let today = crate::repo::count_today_by_user(pool, user_id).await?;
    if today >= i64::from(max_per_day) {
        return RequestLimitExceededSnafu.fail();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use harmonia_common::{MediaType, RequestId, UserId};
    use harmonia_db::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    use super::*;
    use crate::repo::insert_request;
    use crate::types::{MediaRequest, RequestStatus};

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn make_request(user_id: UserId, status: RequestStatus) -> MediaRequest {
        MediaRequest {
            id: RequestId::new(),
            user_id,
            media_type: MediaType::Music,
            title: "Album".to_string(),
            external_id: None,
            status,
            decided_by: None,
            decided_at: None,
            deny_reason: None,
            want_id: None,
            created_at: jiff::Timestamp::now(),
        }
    }

    #[tokio::test]
    async fn member_within_limits_passes() {
        let pool = setup().await;
        let user = UserId::new();
        let result = check_limits(&pool, &user, UserRole::Member, 25, 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn admin_is_exempt_from_limits() {
        let pool = setup().await;
        let user = UserId::new();
        // Insert requests beyond Member limits
        for _ in 0..30 {
            insert_request(&pool, &make_request(user, RequestStatus::Submitted))
                .await
                .unwrap();
        }
        let result = check_limits(&pool, &user, UserRole::Admin, 25, 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn member_exceeding_pending_limit_returns_error() {
        let pool = setup().await;
        let user = UserId::new();
        for _ in 0..3 {
            insert_request(&pool, &make_request(user, RequestStatus::Submitted))
                .await
                .unwrap();
        }
        let result = check_limits(&pool, &user, UserRole::Member, 3, 100).await;
        assert!(matches!(
            result,
            Err(AitesisError::RequestLimitExceeded { .. })
        ));
    }

    #[tokio::test]
    async fn member_exceeding_daily_limit_returns_error() {
        let pool = setup().await;
        let user = UserId::new();
        // Insert requests that are terminal (won't count toward pending)
        for _ in 0..2 {
            insert_request(&pool, &make_request(user, RequestStatus::Fulfilled))
                .await
                .unwrap();
        }
        let result = check_limits(&pool, &user, UserRole::Member, 25, 2).await;
        assert!(matches!(
            result,
            Err(AitesisError::RequestLimitExceeded { .. })
        ));
    }
}
