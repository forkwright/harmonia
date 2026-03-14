//! Database operations for the `requests` table.

use harmonia_common::{RequestId, UserId, WantId};
use harmonia_db::error::QuerySnafu as DbQuerySnafu;
use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::DatabaseSnafu;
use crate::types::{MediaRequest, RequestStatus};

/// Row type for SQLx fetches from the `requests` table.
#[derive(sqlx::FromRow)]
struct RequestRow {
    id: Vec<u8>,
    user_id: Vec<u8>,
    media_type: String,
    title: String,
    external_id: Option<String>,
    status: String,
    decided_by: Option<Vec<u8>>,
    decided_at: Option<String>,
    deny_reason: Option<String>,
    want_id: Option<Vec<u8>>,
    created_at: String,
}

impl RequestRow {
    fn into_domain(self) -> Option<MediaRequest> {
        use uuid::Uuid;

        let id = Uuid::from_slice(&self.id).ok()?;
        let user_id_uuid = Uuid::from_slice(&self.user_id).ok()?;
        let status = RequestStatus::parse(&self.status)?;
        let media_type = media_type_from_str(&self.media_type)?;

        let decided_by = self
            .decided_by
            .as_deref()
            .and_then(|b| Uuid::from_slice(b).ok())
            .map(UserId::from_uuid);

        let decided_at = self
            .decided_at
            .as_deref()
            .and_then(|s| s.parse::<jiff::Timestamp>().ok());

        let want_id = self
            .want_id
            .as_deref()
            .and_then(|b| Uuid::from_slice(b).ok())
            .map(WantId::from_uuid);

        let created_at = self.created_at.parse::<jiff::Timestamp>().ok()?;

        Some(MediaRequest {
            id: RequestId::from_uuid(id),
            user_id: UserId::from_uuid(user_id_uuid),
            media_type,
            title: self.title,
            external_id: self.external_id,
            status,
            decided_by,
            decided_at,
            deny_reason: self.deny_reason,
            want_id,
            created_at,
        })
    }
}

fn media_type_from_str(s: &str) -> Option<harmonia_common::MediaType> {
    use harmonia_common::MediaType;
    match s {
        "music" => Some(MediaType::Music),
        "audiobook" => Some(MediaType::Audiobook),
        "book" => Some(MediaType::Book),
        "comic" => Some(MediaType::Comic),
        "podcast" => Some(MediaType::Podcast),
        "news" => Some(MediaType::News),
        "movie" => Some(MediaType::Movie),
        "tv" => Some(MediaType::Tv),
        _ => None,
    }
}

pub async fn insert_request(
    pool: &SqlitePool,
    request: &MediaRequest,
) -> Result<(), crate::error::AitesisError> {
    sqlx::query(
        "INSERT INTO requests
         (id, user_id, media_type, title, external_id, status,
          decided_by, decided_at, deny_reason, want_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(request.id.as_bytes().as_slice())
    .bind(request.user_id.as_bytes().as_slice())
    .bind(request.media_type.to_string())
    .bind(&request.title)
    .bind(&request.external_id)
    .bind(request.status.as_str())
    .bind(request.decided_by.as_ref().map(|id| id.as_bytes().to_vec()))
    .bind(request.decided_at.map(|t| t.to_string()))
    .bind(&request.deny_reason)
    .bind(request.want_id.as_ref().map(|id| id.as_bytes().to_vec()))
    .bind(request.created_at.to_string())
    .execute(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;
    Ok(())
}

pub async fn get_request(
    pool: &SqlitePool,
    id: &RequestId,
) -> Result<Option<MediaRequest>, crate::error::AitesisError> {
    let row = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests WHERE id = ?",
    )
    .bind(id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(row.and_then(RequestRow::into_domain))
}

pub async fn update_status(
    pool: &SqlitePool,
    id: &RequestId,
    status: RequestStatus,
    decided_by: Option<&UserId>,
    decided_at: Option<jiff::Timestamp>,
    deny_reason: Option<&str>,
    want_id: Option<&WantId>,
) -> Result<(), crate::error::AitesisError> {
    sqlx::query(
        "UPDATE requests
         SET status = ?, decided_by = ?, decided_at = ?, deny_reason = ?, want_id = ?
         WHERE id = ?",
    )
    .bind(status.as_str())
    .bind(decided_by.map(|uid| uid.as_bytes().to_vec()))
    .bind(decided_at.map(|t| t.to_string()))
    .bind(deny_reason)
    .bind(want_id.map(|wid| wid.as_bytes().to_vec()))
    .bind(id.as_bytes().as_slice())
    .execute(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;
    Ok(())
}

pub async fn delete_request(
    pool: &SqlitePool,
    id: &RequestId,
) -> Result<(), crate::error::AitesisError> {
    sqlx::query("DELETE FROM requests WHERE id = ?")
        .bind(id.as_bytes().as_slice())
        .execute(pool)
        .await
        .context(DbQuerySnafu { table: "requests" })
        .context(DatabaseSnafu)?;
    Ok(())
}

pub async fn list_by_user(
    pool: &SqlitePool,
    user_id: &UserId,
) -> Result<Vec<MediaRequest>, crate::error::AitesisError> {
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(rows
        .into_iter()
        .filter_map(RequestRow::into_domain)
        .collect())
}

pub async fn list_by_status(
    pool: &SqlitePool,
    status: RequestStatus,
) -> Result<Vec<MediaRequest>, crate::error::AitesisError> {
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests WHERE status = ? ORDER BY created_at DESC",
    )
    .bind(status.as_str())
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(rows
        .into_iter()
        .filter_map(RequestRow::into_domain)
        .collect())
}

pub async fn list_all(pool: &SqlitePool) -> Result<Vec<MediaRequest>, crate::error::AitesisError> {
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(rows
        .into_iter()
        .filter_map(RequestRow::into_domain)
        .collect())
}

/// Count of requests in Submitted, Approved, or Monitoring states for a user.
pub async fn count_pending_by_user(
    pool: &SqlitePool,
    user_id: &UserId,
) -> Result<i64, crate::error::AitesisError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM requests
         WHERE user_id = ? AND status IN ('submitted', 'approved', 'monitoring')",
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_one(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;
    Ok(row.0)
}

/// Count of requests created today (UTC) for a user.
pub async fn count_today_by_user(
    pool: &SqlitePool,
    user_id: &UserId,
) -> Result<i64, crate::error::AitesisError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM requests
         WHERE user_id = ?
           AND created_at >= strftime('%Y-%m-%dT00:00:00Z', 'now')",
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_one(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;
    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use harmonia_common::{MediaType, RequestId, UserId};
    use harmonia_db::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    use crate::types::{MediaRequest, RequestStatus};

    use super::*;

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
            title: "Test Album".to_string(),
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
    async fn insert_and_get_request() {
        let pool = setup().await;
        let user_id = UserId::new();
        let req = make_request(user_id, RequestStatus::Submitted);
        let req_id = req.id;

        insert_request(&pool, &req).await.unwrap();

        let fetched = get_request(&pool, &req_id).await.unwrap().unwrap();
        assert_eq!(fetched.id, req_id);
        assert_eq!(fetched.status, RequestStatus::Submitted);
        assert_eq!(fetched.title, "Test Album");
    }

    #[tokio::test]
    async fn get_request_returns_none_when_missing() {
        let pool = setup().await;
        let id = RequestId::new();
        let result = get_request(&pool, &id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn update_status_changes_request_status() {
        let pool = setup().await;
        let user_id = UserId::new();
        let admin_id = UserId::new();
        let req = make_request(user_id, RequestStatus::Submitted);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        let now = jiff::Timestamp::now();
        update_status(
            &pool,
            &req_id,
            RequestStatus::Approved,
            Some(&admin_id),
            Some(now),
            None,
            None,
        )
        .await
        .unwrap();

        let fetched = get_request(&pool, &req_id).await.unwrap().unwrap();
        assert_eq!(fetched.status, RequestStatus::Approved);
        assert_eq!(fetched.decided_by, Some(admin_id));
    }

    #[tokio::test]
    async fn delete_request_removes_row() {
        let pool = setup().await;
        let user_id = UserId::new();
        let req = make_request(user_id, RequestStatus::Submitted);
        let req_id = req.id;
        insert_request(&pool, &req).await.unwrap();

        delete_request(&pool, &req_id).await.unwrap();

        let result = get_request(&pool, &req_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_by_user_returns_only_that_user() {
        let pool = setup().await;
        let alice = UserId::new();
        let bob = UserId::new();

        insert_request(&pool, &make_request(alice, RequestStatus::Submitted))
            .await
            .unwrap();
        insert_request(&pool, &make_request(alice, RequestStatus::Monitoring))
            .await
            .unwrap();
        insert_request(&pool, &make_request(bob, RequestStatus::Submitted))
            .await
            .unwrap();

        let alice_requests = list_by_user(&pool, &alice).await.unwrap();
        assert_eq!(alice_requests.len(), 2);
        assert!(alice_requests.iter().all(|r| r.user_id == alice));
    }

    #[tokio::test]
    async fn list_by_status_filters_correctly() {
        let pool = setup().await;
        let user = UserId::new();

        insert_request(&pool, &make_request(user, RequestStatus::Submitted))
            .await
            .unwrap();
        insert_request(&pool, &make_request(user, RequestStatus::Submitted))
            .await
            .unwrap();
        insert_request(&pool, &make_request(user, RequestStatus::Approved))
            .await
            .unwrap();

        let submitted = list_by_status(&pool, RequestStatus::Submitted)
            .await
            .unwrap();
        assert_eq!(submitted.len(), 2);

        let approved = list_by_status(&pool, RequestStatus::Approved)
            .await
            .unwrap();
        assert_eq!(approved.len(), 1);
    }

    #[tokio::test]
    async fn count_pending_by_user_counts_active_statuses() {
        let pool = setup().await;
        let user = UserId::new();

        insert_request(&pool, &make_request(user, RequestStatus::Submitted))
            .await
            .unwrap();
        insert_request(&pool, &make_request(user, RequestStatus::Monitoring))
            .await
            .unwrap();
        insert_request(&pool, &make_request(user, RequestStatus::Fulfilled))
            .await
            .unwrap();
        insert_request(&pool, &make_request(user, RequestStatus::Denied))
            .await
            .unwrap();

        let count = count_pending_by_user(&pool, &user).await.unwrap();
        // Only Submitted + Monitoring count; Fulfilled and Denied do not
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn count_today_by_user_counts_all_todays_requests() {
        let pool = setup().await;
        let user = UserId::new();
        let other = UserId::new();

        insert_request(&pool, &make_request(user, RequestStatus::Submitted))
            .await
            .unwrap();
        insert_request(&pool, &make_request(user, RequestStatus::Denied))
            .await
            .unwrap();
        insert_request(&pool, &make_request(other, RequestStatus::Submitted))
            .await
            .unwrap();

        let count = count_today_by_user(&pool, &user).await.unwrap();
        // Both of user's requests were inserted today
        assert_eq!(count, 2);
    }
}
