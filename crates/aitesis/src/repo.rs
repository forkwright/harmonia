//! Database operations for the `requests` table.

use apotheke::error::QuerySnafu as DbQuerySnafu;
use snafu::ResultExt;
use sqlx::SqlitePool;
use themelion::{RequestId, UserId, WantId};

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
        // WHY: this is the decision site discarding a malformed row — the
        // filter_map/and_then callers swallow the None, so an unlogged drop
        // here makes a corrupt row silently vanish from every listing.
        let row_id = self.id.clone();
        match self.try_into_domain() {
            Ok(request) => Some(request),
            Err(field) => {
                tracing::warn!(
                    row_id = %format_args!("{row_id:02x?}"),
                    field,
                    "dropping requests row that failed to parse"
                );
                None
            }
        }
    }

    fn try_into_domain(self) -> Result<MediaRequest, &'static str> {
        use uuid::Uuid;

        let id = Uuid::from_slice(&self.id).map_err(|_| "id")?;
        let user_id_uuid = Uuid::from_slice(&self.user_id).map_err(|_| "user_id")?;
        let status = RequestStatus::parse(&self.status).ok_or("status")?;
        let media_type = media_type_from_str(&self.media_type).ok_or("media_type")?;

        let decided_by = self
            .decided_by
            .as_deref()
            .map(|b| Uuid::from_slice(b).map_err(|_| "decided_by"))
            .transpose()?
            .map(UserId::from_uuid);

        let decided_at = self
            .decided_at
            .as_deref()
            .map(|s| s.parse::<jiff::Timestamp>().map_err(|_| "decided_at"))
            .transpose()?;

        let want_id = self
            .want_id
            .as_deref()
            .map(|b| Uuid::from_slice(b).map_err(|_| "want_id"))
            .transpose()?
            .map(WantId::from_uuid);

        let created_at = self
            .created_at
            .parse::<jiff::Timestamp>()
            .map_err(|_| "created_at")?;

        Ok(MediaRequest {
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

fn media_type_from_str(s: &str) -> Option<themelion::MediaType> {
    use themelion::MediaType;
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

/// Inserts a request row.
///
/// Generic over the executor so the limit-check + insert sequence can run
/// inside one transaction (see `submit_request`).
pub async fn insert_request<'e, E>(
    executor: E,
    request: &MediaRequest,
) -> Result<(), crate::error::AitesisError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
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
    .execute(executor)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;
    Ok(())
}

/// Fetches a request by ID.
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

/// Parameters for [`update_status`].
///
/// Groups the mutation inputs applied to a request row: the new status plus
/// the admin decision metadata (decided-by / decided-at / deny-reason) and the
/// optional linked want for Monitoring transitions.
// WHY: wire DTO — parameter bundle for the update_status SQL call.
pub struct UpdateStatusParams<'a> {
    /// Request row to update.
    pub id: &'a RequestId,
    /// New request status.
    pub status: RequestStatus,
    /// Optional administrator or actor that made the decision.
    pub decided_by: Option<&'a UserId>,
    /// Optional decision timestamp.
    pub decided_at: Option<jiff::Timestamp>,
    /// Optional denial reason.
    pub deny_reason: Option<&'a str>,
    /// Optional wanted-media row linked during monitoring handoff.
    pub want_id: Option<&'a WantId>,
}

/// Updates status and decision metadata for a request row.
pub async fn update_status(
    pool: &SqlitePool,
    params: UpdateStatusParams<'_>,
) -> Result<(), crate::error::AitesisError> {
    let UpdateStatusParams {
        id,
        status,
        decided_by,
        decided_at,
        deny_reason,
        want_id,
    } = params;
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

/// Deletes a request row by ID.
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

/// Upper bound a single page may request, regardless of caller input.
const MAX_PAGE_LIMIT: u32 = 1000;

/// Pagination window for the list queries.
///
/// WHY: every listing carries an explicit LIMIT/OFFSET — an unbounded
/// `SELECT *` over a large request table is an allocation hazard.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    limit: u32,
    offset: u32,
}

impl Page {
    /// Builds a window, clamping `limit` into `1..=1000` so a hostile or
    /// buggy caller cannot request an unbounded page.
    #[must_use]
    pub fn new(limit: u32, offset: u32) -> Self {
        Self {
            limit: limit.clamp(1, MAX_PAGE_LIMIT),
            offset,
        }
    }

    fn limit_i64(self) -> i64 {
        i64::from(self.limit)
    }

    fn offset_i64(self) -> i64 {
        i64::from(self.offset)
    }
}

/// Lists requests submitted by a user, newest first.
pub async fn list_by_user(
    pool: &SqlitePool,
    user_id: &UserId,
    page: Page,
) -> Result<Vec<MediaRequest>, crate::error::AitesisError> {
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests WHERE user_id = ? ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(user_id.as_bytes().as_slice())
    .bind(page.limit_i64())
    .bind(page.offset_i64())
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(rows
        .into_iter()
        .filter_map(RequestRow::into_domain)
        .collect())
}

/// Lists requests matching a status, newest first.
pub async fn list_by_status(
    pool: &SqlitePool,
    status: RequestStatus,
    page: Page,
) -> Result<Vec<MediaRequest>, crate::error::AitesisError> {
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests WHERE status = ? ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(status.as_str())
    .bind(page.limit_i64())
    .bind(page.offset_i64())
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(rows
        .into_iter()
        .filter_map(RequestRow::into_domain)
        .collect())
}

/// Lists a user's requests matching a status, newest first.
///
/// WHY: filtering in SQL (not post-fetch) keeps LIMIT/OFFSET windows correct —
/// an in-memory status filter over a paginated fetch skips matching rows.
pub async fn list_by_user_and_status(
    pool: &SqlitePool,
    user_id: &UserId,
    status: RequestStatus,
    page: Page,
) -> Result<Vec<MediaRequest>, crate::error::AitesisError> {
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests WHERE user_id = ? AND status = ? ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(user_id.as_bytes().as_slice())
    .bind(status.as_str())
    .bind(page.limit_i64())
    .bind(page.offset_i64())
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(rows
        .into_iter()
        .filter_map(RequestRow::into_domain)
        .collect())
}

/// Lists all requests, newest first.
pub async fn list_all(
    pool: &SqlitePool,
    page: Page,
) -> Result<Vec<MediaRequest>, crate::error::AitesisError> {
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id, user_id, media_type, title, external_id, status,
                decided_by, decided_at, deny_reason, want_id, created_at
         FROM requests ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(page.limit_i64())
    .bind(page.offset_i64())
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;

    Ok(rows
        .into_iter()
        .filter_map(RequestRow::into_domain)
        .collect())
}

/// Counts requests matching the optional user/status filters.
pub async fn count_requests(
    pool: &SqlitePool,
    user_id: Option<&UserId>,
    status: Option<RequestStatus>,
) -> Result<i64, crate::error::AitesisError> {
    let query = match (user_id, status) {
        (Some(uid), Some(st)) => sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM requests WHERE user_id = ? AND status = ?",
        )
        .bind(uid.as_bytes().to_vec())
        .bind(st.as_str().to_string()),
        (Some(uid), None) => {
            sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM requests WHERE user_id = ?")
                .bind(uid.as_bytes().to_vec())
        }
        (None, Some(st)) => {
            sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM requests WHERE status = ?")
                .bind(st.as_str().to_string())
        }
        (None, None) => sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM requests"),
    };

    let row = query
        .fetch_one(pool)
        .await
        .context(DbQuerySnafu { table: "requests" })
        .context(DatabaseSnafu)?;
    Ok(row.0)
}

/// Count of requests in Submitted, Approved, or Monitoring states for a user.
pub async fn count_pending_by_user<'e, E>(
    executor: E,
    user_id: &UserId,
) -> Result<i64, crate::error::AitesisError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM requests
         WHERE user_id = ? AND status IN ('submitted', 'approved', 'monitoring')",
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_one(executor)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;
    Ok(row.0)
}

/// Count of requests created today (UTC) for a user.
pub async fn count_today_by_user<'e, E>(
    executor: E,
    user_id: &UserId,
) -> Result<i64, crate::error::AitesisError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM requests
         WHERE user_id = ?
           AND created_at >= strftime('%Y-%m-%dT00:00:00Z', 'now')",
    )
    .bind(user_id.as_bytes().as_slice())
    .fetch_one(executor)
    .await
    .context(DbQuerySnafu { table: "requests" })
    .context(DatabaseSnafu)?;
    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;
    use themelion::{MediaType, RequestId, UserId};

    use super::*;
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
            UpdateStatusParams {
                id: &req_id,
                status: RequestStatus::Approved,
                decided_by: Some(&admin_id),
                decided_at: Some(now),
                deny_reason: None,
                want_id: None,
            },
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

        let alice_requests = list_by_user(&pool, &alice, Page::new(100, 0))
            .await
            .unwrap();
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

        let submitted = list_by_status(&pool, RequestStatus::Submitted, Page::new(100, 0))
            .await
            .unwrap();
        assert_eq!(submitted.len(), 2);

        let approved = list_by_status(&pool, RequestStatus::Approved, Page::new(100, 0))
            .await
            .unwrap();
        assert_eq!(approved.len(), 1);
    }

    #[tokio::test]
    async fn list_paginates_and_counts() {
        let pool = setup().await;
        let user = UserId::new();

        for _ in 0..5 {
            insert_request(&pool, &make_request(user, RequestStatus::Submitted))
                .await
                .unwrap();
        }

        let first_page = list_by_user(&pool, &user, Page::new(2, 0)).await.unwrap();
        assert_eq!(first_page.len(), 2);
        let last_page = list_by_user(&pool, &user, Page::new(2, 4)).await.unwrap();
        assert_eq!(last_page.len(), 1);

        let total = count_requests(&pool, Some(&user), None).await.unwrap();
        assert_eq!(total, 5);
        let submitted = count_requests(&pool, None, Some(RequestStatus::Submitted))
            .await
            .unwrap();
        assert_eq!(submitted, 5);
        let denied = count_requests(&pool, None, Some(RequestStatus::Denied))
            .await
            .unwrap();
        assert_eq!(denied, 0);
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
