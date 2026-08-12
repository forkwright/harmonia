//! Plex viewing statistics — Wrapperr replacement.
//!
//! Queries Plex watch history to power Harmonia's listening/viewing stats
//! without a separate Wrapperr process.

use snafu::ResultExt;
use themelion::WatchRecord;

use super::BoxFuture;
use crate::error::{PlexApiCallSnafu, SyndesmodError};
use crate::plex::PlexClient;

/// Fetches viewing history from Plex.
pub(crate) trait StatsProvider: Send + Sync {
    /// Returns the watch history for one Plex account, or every account when
    /// `plex_user_id` is empty.
    fn fetch_watch_history<'a>(
        &'a self,
        plex_user_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<WatchRecord>, SyndesmodError>>;
}

impl StatsProvider for PlexClient {
    fn fetch_watch_history<'a>(
        &'a self,
        plex_user_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<WatchRecord>, SyndesmodError>> {
        Box::pin(async move {
            let url = format!(
                "{}/status/sessions/history/all",
                self.config.url.trim_end_matches('/')
            );
            let mut query: Vec<(&str, String)> = Vec::new();
            if !plex_user_id.is_empty() {
                query.push(("accountID", plex_user_id.to_string()));
            }
            let history: HistoryResponse = self
                .http
                .get(&url)
                .header("X-Plex-Token", &self.config.token)
                .header("Accept", "application/json")
                .query(&query)
                .send()
                .await
                .context(PlexApiCallSnafu)?
                .error_for_status()
                .context(PlexApiCallSnafu)?
                .json()
                .await
                .context(PlexApiCallSnafu)?;
            Ok(history.into_records())
        })
    }
}

// WHY: wire DTOs — Plex `GET /status/sessions/history/all` JSON shape. An
// empty history omits the `Metadata` key entirely.
#[derive(serde::Deserialize)]
struct HistoryResponse {
    #[serde(rename = "MediaContainer")]
    media_container: HistoryContainer,
}

#[derive(serde::Deserialize)]
struct HistoryContainer {
    #[serde(rename = "Metadata", default)]
    metadata: Vec<HistoryEntry>,
}

#[derive(serde::Deserialize)]
struct HistoryEntry {
    #[serde(rename = "ratingKey")]
    rating_key: Option<String>,
    title: Option<String>,
    #[serde(rename = "grandparentTitle")]
    grandparent_title: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(rename = "accountID")]
    account_id: Option<i64>,
    #[serde(rename = "viewedAt")]
    viewed_at: Option<i64>,
}

impl HistoryResponse {
    fn into_records(self) -> Vec<WatchRecord> {
        let entries = self.media_container.metadata;
        let mut skipped = 0usize;
        let mut records = Vec::with_capacity(entries.len());
        for entry in entries {
            match entry.rating_key.filter(|key| !key.is_empty()) {
                Some(source_ref) => records.push(WatchRecord {
                    source_ref,
                    title: entry.title.unwrap_or_default(),
                    grandparent_title: entry.grandparent_title,
                    media_kind: entry.kind.unwrap_or_default(),
                    account_id: entry.account_id,
                    viewed_at: entry.viewed_at,
                }),
                None => skipped += 1,
            }
        }
        if skipped > 0 {
            tracing::debug!(skipped, "plex history entries without ratingKey dropped");
        }
        records
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use horismos::PlexConfig;

    use super::*;

    #[derive(Default)]
    pub(crate) struct MockStatsProvider {
        pub(crate) calls: Mutex<Vec<String>>,
        pub(crate) records: Vec<WatchRecord>,
    }

    impl MockStatsProvider {
        pub(crate) fn new(records: Vec<WatchRecord>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                records,
            }
        }

        pub(crate) fn requested_users(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl StatsProvider for MockStatsProvider {
        fn fetch_watch_history<'a>(
            &'a self,
            plex_user_id: &'a str,
        ) -> BoxFuture<'a, Result<Vec<WatchRecord>, SyndesmodError>> {
            self.calls.lock().unwrap().push(plex_user_id.to_string());
            let records = self.records.clone();
            Box::pin(async move { Ok(records) })
        }
    }

    fn test_config(url: String) -> PlexConfig {
        PlexConfig {
            url,
            token: "plextok".to_string(),
            library_sections: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn fetch_watch_history_maps_entries_to_watch_records() {
        let body = r#"{"MediaContainer":{"size":2,"Metadata":[
            {"ratingKey":"101","title":"Gantz Graf","grandparentTitle":"Autechre","type":"track","accountID":1,"viewedAt":1700000000},
            {"ratingKey":"202","title":"Alien","type":"movie","librarySectionID":2,"accountID":2,"viewedAt":1700000100}
        ]}}"#;
        let (base_url, server) = crate::test_support::spawn_one_shot_http(200, "OK", body).await;
        let client = PlexClient::new(test_config(base_url));

        let records = client.fetch_watch_history("").await.unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].source_ref, "101");
        assert_eq!(records[0].title, "Gantz Graf");
        assert_eq!(records[0].grandparent_title.as_deref(), Some("Autechre"));
        assert_eq!(records[0].media_kind, "track");
        assert_eq!(records[0].account_id, Some(1));
        assert_eq!(records[0].viewed_at, Some(1_700_000_000));
        assert_eq!(records[1].source_ref, "202");
        assert_eq!(records[1].grandparent_title, None);

        let request = server.await.unwrap();
        let request_line = request.lines().next().unwrap();
        assert!(
            request_line.starts_with("GET /status/sessions/history/all"),
            "unexpected history request: {request_line}"
        );
        assert!(
            !request_line.contains("accountID"),
            "empty user id must not send an accountID filter: {request_line}"
        );
    }

    #[tokio::test]
    async fn fetch_watch_history_filters_by_account_id() {
        let body = r#"{"MediaContainer":{"size":1,"Metadata":[
            {"ratingKey":"101","title":"Gantz Graf","type":"track","accountID":42,"viewedAt":1700000000}
        ]}}"#;
        let (base_url, server) = crate::test_support::spawn_one_shot_http(200, "OK", body).await;
        let client = PlexClient::new(test_config(base_url));

        let records = client.fetch_watch_history("42").await.unwrap();

        assert_eq!(records.len(), 1);
        let request = server.await.unwrap();
        let request_line = request.lines().next().unwrap();
        assert!(
            request_line.contains("accountID=42"),
            "account filter must reach the wire: {request_line}"
        );
    }

    #[tokio::test]
    async fn fetch_watch_history_returns_empty_when_history_empty() {
        let body = r#"{"MediaContainer":{"size":0}}"#;
        let (base_url, server) = crate::test_support::spawn_one_shot_http(200, "OK", body).await;
        let client = PlexClient::new(test_config(base_url));

        let records = client.fetch_watch_history("").await.unwrap();

        assert!(records.is_empty());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn fetch_watch_history_drops_entries_without_rating_key() {
        let body = r#"{"MediaContainer":{"size":2,"Metadata":[
            {"title":"Untracked","type":"movie","viewedAt":1700000000},
            {"ratingKey":"101","title":"Gantz Graf","type":"track","viewedAt":1700000100}
        ]}}"#;
        let (base_url, server) = crate::test_support::spawn_one_shot_http(200, "OK", body).await;
        let client = PlexClient::new(test_config(base_url));

        let records = client.fetch_watch_history("").await.unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source_ref, "101");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn fetch_watch_history_errors_on_http_error_status() {
        let (base_url, server) =
            crate::test_support::spawn_one_shot_http(500, "Internal Server Error", "").await;
        let client = PlexClient::new(test_config(base_url));

        let result = client.fetch_watch_history("").await;

        assert!(matches!(result, Err(SyndesmodError::PlexApiCall { .. })));
        server.await.unwrap();
    }
}
