//! HTTP client wrapping the Harmonia backend REST API.

use serde::de::DeserializeOwned;
use snafu::ResultExt as _;

use crate::types::{ApiResponse, ListParams, PaginatedResponse};

/// Errors FROM API requests.
#[derive(Debug, snafu::Snafu)]
#[non_exhaustive]
pub enum ApiError {
    /// HTTP request failed.
    #[snafu(display("request failed: {source}"))]
    Request { source: reqwest::Error },

    /// Response body could not be deserialized.
    #[snafu(display("response parse failed: {source}"))]
    Parse { source: reqwest::Error },

    /// Server returned a non-success status code.
    #[snafu(display("server error: HTTP {status}"))]
    Status { status: u16 },
}

/// HTTP client for the Harmonia backend.
///
/// Wraps reqwest and manages the base URL and auth token. All methods
/// correspond to endpoints served by `archon` (Axum).
#[derive(Debug, Clone)]
pub struct HarmoniaClient {
    inner: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl HarmoniaClient {
    /// Create a new client pointed at the given server.
    pub fn new(base_url: impl Into<String>) -> Self {
        let inner = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            inner,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: None,
        }
    }

    /// Set the authentication token for subsequent requests.
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    /// Clear the authentication token.
    pub fn clear_token(&mut self) {
        self.token = None;
    }

    /// Update the server base URL.
    pub fn set_base_url(&mut self, url: impl Into<String>) {
        self.base_url = url.into().trim_end_matches('/').to_owned();
    }

    /// Check server health.
    pub async fn health_check(&self) -> Result<bool, ApiError> {
        let resp = self
            .inner
            .get(format!("{}/health", self.base_url))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .context(RequestSnafu)?;

        Ok(resp.status().is_success())
    }

    // ── Music ────────────────────────────────────────────────────────

    /// List release groups (albums) with pagination.
    pub async fn list_release_groups<T: DeserializeOwned>(
        &self,
        params: ListParams,
    ) -> Result<ApiResponse<Vec<T>>, ApiError> {
        self.get_with_params(
            "/api/music/release-groups",
            &[
                ("page", params.page.to_string()),
                ("per_page", params.per_page.to_string()),
            ],
        )
        .await
    }

    /// Get a single release GROUP by ID.
    pub async fn get_release_group<T: DeserializeOwned>(
        &self,
        id: &str,
    ) -> Result<ApiResponse<T>, ApiError> {
        self.get(&format!("/api/music/release-groups/{id}")).await
    }

    /// List tracks with pagination.
    pub async fn list_tracks<T: DeserializeOwned>(
        &self,
        params: ListParams,
    ) -> Result<ApiResponse<Vec<T>>, ApiError> {
        self.get_with_params(
            "/api/music/tracks",
            &[
                ("page", params.page.to_string()),
                ("per_page", params.per_page.to_string()),
            ],
        )
        .await
    }

    /// List tracks for a specific album.
    pub async fn list_tracks_for_album<T: DeserializeOwned>(
        &self,
        album_id: &str,
    ) -> Result<ApiResponse<Vec<T>>, ApiError> {
        self.get(&format!("/api/music/release-groups/{album_id}/tracks"))
            .await
    }

    // ── Audiobooks ───────────────────────────────────────────────────

    /// List audiobooks with pagination.
    pub async fn list_audiobooks<T: DeserializeOwned>(
        &self,
        params: ListParams,
    ) -> Result<ApiResponse<Vec<T>>, ApiError> {
        self.get_with_params(
            "/api/audiobooks",
            &[
                ("page", params.page.to_string()),
                ("per_page", params.per_page.to_string()),
            ],
        )
        .await
    }

    /// Get audiobook details.
    pub async fn get_audiobook<T: DeserializeOwned>(
        &self,
        id: &str,
    ) -> Result<ApiResponse<T>, ApiError> {
        self.get(&format!("/api/audiobooks/{id}")).await
    }

    /// Get audiobook chapters.
    pub async fn get_audiobook_chapters<T: DeserializeOwned>(
        &self,
        id: &str,
    ) -> Result<ApiResponse<Vec<T>>, ApiError> {
        self.get(&format!("/api/audiobooks/{id}/chapters")).await
    }

    /// Get audiobook reading progress.
    pub async fn get_audiobook_progress<T: DeserializeOwned>(
        &self,
        id: &str,
    ) -> Result<ApiResponse<T>, ApiError> {
        self.get(&format!("/api/audiobooks/{id}/progress")).await
    }

    /// Update audiobook reading progress.
    pub async fn update_audiobook_progress<T: DeserializeOwned>(
        &self,
        id: &str,
        progress: &impl serde::Serialize,
    ) -> Result<ApiResponse<T>, ApiError> {
        self.put(&format!("/api/audiobooks/{id}/progress"), progress)
            .await
    }

    /// List bookmarks for an audiobook.
    pub async fn get_bookmarks<T: DeserializeOwned>(
        &self,
        audiobook_id: &str,
    ) -> Result<ApiResponse<Vec<T>>, ApiError> {
        self.get(&format!("/api/audiobooks/{audiobook_id}/bookmarks"))
            .await
    }

    /// Create a bookmark.
    pub async fn create_bookmark<T: DeserializeOwned>(
        &self,
        audiobook_id: &str,
        bookmark: &impl serde::Serialize,
    ) -> Result<ApiResponse<T>, ApiError> {
        self.post(
            &format!("/api/audiobooks/{audiobook_id}/bookmarks"),
            bookmark,
        )
        .await
    }

    /// Delete a bookmark.
    pub async fn delete_bookmark(&self, bookmark_id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/bookmarks/{bookmark_id}")).await
    }

    // ── Podcasts ─────────────────────────────────────────────────────

    /// Get podcast subscriptions.
    pub async fn get_subscriptions<T: DeserializeOwned>(&self) -> Result<Vec<T>, ApiError> {
        self.get("/api/podcasts/subscriptions").await
    }

    /// Subscribe to a podcast feed.
    pub async fn subscribe<T: DeserializeOwned>(&self, feed_url: &str) -> Result<T, ApiError> {
        self.post(
            "/api/podcasts/subscriptions",
            &serde_json::json!({ "feedUrl": feed_url }),
        )
        .await
    }

    /// Unsubscribe FROM a podcast.
    pub async fn unsubscribe(&self, podcast_id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/podcasts/subscriptions/{podcast_id}"))
            .await
    }

    /// Refresh a podcast feed.
    pub async fn refresh_feed(&self, podcast_id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!("/api/podcasts/{podcast_id}/refresh"))
            .await
    }

    /// Refresh all podcast feeds.
    pub async fn refresh_all_feeds(&self) -> Result<(), ApiError> {
        self.post_empty("/api/podcasts/refresh-all").await
    }

    /// Get episodes for a podcast.
    pub async fn get_episodes<T: DeserializeOwned>(
        &self,
        podcast_id: &str,
        page: u32,
        page_size: u32,
    ) -> Result<PaginatedResponse<T>, ApiError> {
        self.get_with_params(
            &format!("/api/podcasts/{podcast_id}/episodes"),
            &[
                ("page", page.to_string()),
                ("page_size", page_size.to_string()),
            ],
        )
        .await
    }

    /// Get episode details.
    pub async fn get_episode<T: DeserializeOwned>(&self, episode_id: &str) -> Result<T, ApiError> {
        self.get(&format!("/api/podcasts/episodes/{episode_id}"))
            .await
    }

    /// Get latest episodes across all subscriptions.
    pub async fn get_latest_episodes<T: DeserializeOwned>(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<PaginatedResponse<T>, ApiError> {
        self.get_with_params(
            "/api/podcasts/episodes/latest",
            &[
                ("page", page.to_string()),
                ("page_size", page_size.to_string()),
            ],
        )
        .await
    }

    /// Get episode playback progress.
    pub async fn get_episode_progress<T: DeserializeOwned>(
        &self,
        episode_id: &str,
    ) -> Result<T, ApiError> {
        self.get(&format!("/api/podcasts/episodes/{episode_id}/progress"))
            .await
    }

    /// Update episode playback progress.
    pub async fn update_episode_progress<T: DeserializeOwned>(
        &self,
        episode_id: &str,
        progress: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.put(
            &format!("/api/podcasts/episodes/{episode_id}/progress"),
            progress,
        )
        .await
    }

    /// Mark episode as completed.
    pub async fn mark_episode_completed(&self, episode_id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!("/api/podcasts/episodes/{episode_id}/complete"))
            .await
    }

    /// Mark episode as unplayed.
    pub async fn mark_episode_unplayed(&self, episode_id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!("/api/podcasts/episodes/{episode_id}/unplay"))
            .await
    }

    /// Download a podcast episode.
    pub async fn download_episode(&self, episode_id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!("/api/podcasts/episodes/{episode_id}/download"))
            .await
    }

    /// Cancel an episode download.
    pub async fn cancel_episode_download(&self, episode_id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!(
            "/api/podcasts/episodes/{episode_id}/download/cancel"
        ))
        .await
    }

    /// Delete a downloaded episode.
    pub async fn delete_episode_download(&self, episode_id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/podcasts/episodes/{episode_id}/download"))
            .await
    }

    /// Get podcast download queue.
    pub async fn get_podcast_download_queue<T: DeserializeOwned>(
        &self,
    ) -> Result<Vec<T>, ApiError> {
        self.get("/api/podcasts/downloads").await
    }

    // ── Library Management ───────────────────────────────────────────

    /// Trigger a library scan.
    pub async fn trigger_scan<T: DeserializeOwned>(
        &self,
        path: Option<&str>,
    ) -> Result<T, ApiError> {
        match path {
            Some(p) => {
                self.post("/api/manage/scan", &serde_json::json!({ "path": p }))
                    .await
            }
            None => self.post("/api/manage/scan", &serde_json::json!({})).await,
        }
    }

    /// Get current scan status.
    pub async fn get_scan_status<T: DeserializeOwned>(&self) -> Result<T, ApiError> {
        self.get("/api/manage/scan/status").await
    }

    /// List media items with filtering.
    pub async fn get_media_items<T: DeserializeOwned>(
        &self,
        params: &[(&str, String)],
    ) -> Result<PaginatedResponse<T>, ApiError> {
        self.get_with_params("/api/manage/media", params).await
    }

    /// Get media item details.
    pub async fn get_media_item<T: DeserializeOwned>(&self, id: &str) -> Result<T, ApiError> {
        self.get(&format!("/api/manage/media/{id}")).await
    }

    /// Delete a media item.
    pub async fn delete_media_item(&self, id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/manage/media/{id}")).await
    }

    /// Refresh metadata for a media item.
    pub async fn refresh_metadata(&self, id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!("/api/manage/media/{id}/refresh"))
            .await
    }

    /// Update metadata for a media item.
    pub async fn update_metadata<T: DeserializeOwned>(
        &self,
        id: &str,
        metadata: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.put(&format!("/api/manage/media/{id}/metadata"), metadata)
            .await
    }

    // ── Quality Profiles ─────────────────────────────────────────────

    /// List quality profiles.
    pub async fn get_quality_profiles<T: DeserializeOwned>(&self) -> Result<Vec<T>, ApiError> {
        self.get("/api/manage/quality-profiles").await
    }

    /// Get a quality profile by ID.
    pub async fn get_quality_profile<T: DeserializeOwned>(&self, id: &str) -> Result<T, ApiError> {
        self.get(&format!("/api/manage/quality-profiles/{id}"))
            .await
    }

    /// Update a quality profile.
    pub async fn update_quality_profile<T: DeserializeOwned>(
        &self,
        id: &str,
        profile: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.put(&format!("/api/manage/quality-profiles/{id}"), profile)
            .await
    }

    // ── Library Health ───────────────────────────────────────────────

    /// Get library health report.
    pub async fn get_library_health<T: DeserializeOwned>(&self) -> Result<T, ApiError> {
        self.get("/api/manage/health").await
    }

    // ── Search & Downloads ───────────────────────────────────────────

    /// Search indexers.
    pub async fn search_indexers<T: DeserializeOwned>(
        &self,
        query: &impl serde::Serialize,
    ) -> Result<Vec<T>, ApiError> {
        self.post("/api/manage/search", query).await
    }

    /// Trigger a download.
    pub async fn trigger_download<T: DeserializeOwned>(
        &self,
        request: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.post("/api/manage/downloads", request).await
    }

    /// Get download queue snapshot.
    pub async fn get_download_queue<T: DeserializeOwned>(&self) -> Result<T, ApiError> {
        self.get("/api/manage/downloads/queue").await
    }

    /// Cancel a download.
    pub async fn cancel_download(&self, download_id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/manage/downloads/{download_id}"))
            .await
    }

    /// Retry a failed download.
    pub async fn retry_download(&self, download_id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!("/api/manage/downloads/{download_id}/retry"))
            .await
    }

    // ── Requests ─────────────────────────────────────────────────────

    /// List media requests.
    pub async fn get_requests<T: DeserializeOwned>(
        &self,
        params: &[(&str, String)],
    ) -> Result<PaginatedResponse<T>, ApiError> {
        self.get_with_params("/api/manage/requests", params).await
    }

    /// Create a media request.
    pub async fn create_request<T: DeserializeOwned>(
        &self,
        request: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.post("/api/manage/requests", request).await
    }

    /// Approve a media request.
    pub async fn approve_request<T: DeserializeOwned>(
        &self,
        request_id: &str,
    ) -> Result<T, ApiError> {
        self.post(
            &format!("/api/manage/requests/{request_id}/approve"),
            &serde_json::json!({}),
        )
        .await
    }

    /// Deny a media request.
    pub async fn deny_request<T: DeserializeOwned>(
        &self,
        request_id: &str,
        reason: &str,
    ) -> Result<T, ApiError> {
        self.post(
            &format!("/api/manage/requests/{request_id}/deny"),
            &serde_json::json!({ "reason": reason }),
        )
        .await
    }

    /// Cancel a media request.
    pub async fn cancel_request(&self, request_id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/manage/requests/{request_id}"))
            .await
    }

    // ── Wanted ───────────────────────────────────────────────────────

    /// List wanted media items.
    pub async fn get_wanted<T: DeserializeOwned>(
        &self,
        params: &[(&str, String)],
    ) -> Result<PaginatedResponse<T>, ApiError> {
        self.get_with_params("/api/manage/wanted", params).await
    }

    /// Add a wanted media item.
    pub async fn add_wanted<T: DeserializeOwned>(
        &self,
        item: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.post("/api/manage/wanted", item).await
    }

    /// Remove a wanted media item.
    pub async fn remove_wanted(&self, id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/manage/wanted/{id}")).await
    }

    /// Trigger search for a wanted item.
    pub async fn trigger_wanted_search(&self, id: &str) -> Result<(), ApiError> {
        self.post_empty(&format!("/api/manage/wanted/{id}/search"))
            .await
    }

    // ── Indexers ─────────────────────────────────────────────────────

    /// List configured indexers.
    pub async fn get_indexers<T: DeserializeOwned>(&self) -> Result<Vec<T>, ApiError> {
        self.get("/api/manage/indexers").await
    }

    /// Add an indexer.
    pub async fn add_indexer<T: DeserializeOwned>(
        &self,
        config: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.post("/api/manage/indexers", config).await
    }

    /// Update an indexer.
    pub async fn update_indexer<T: DeserializeOwned>(
        &self,
        id: &str,
        config: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        self.put(&format!("/api/manage/indexers/{id}"), config)
            .await
    }

    /// Delete an indexer.
    pub async fn delete_indexer(&self, id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/manage/indexers/{id}")).await
    }

    /// Test an indexer connection.
    pub async fn test_indexer<T: DeserializeOwned>(&self, id: &str) -> Result<T, ApiError> {
        self.post(
            &format!("/api/manage/indexers/{id}/test"),
            &serde_json::json!({}),
        )
        .await
    }

    // ── Subtitles ────────────────────────────────────────────────────

    /// List subtitles for a media item.
    pub async fn get_subtitles<T: DeserializeOwned>(
        &self,
        media_id: &str,
    ) -> Result<Vec<T>, ApiError> {
        self.get(&format!("/api/manage/media/{media_id}/subtitles"))
            .await
    }

    /// Search for subtitles.
    pub async fn search_subtitles<T: DeserializeOwned>(
        &self,
        media_id: &str,
    ) -> Result<Vec<T>, ApiError> {
        self.get(&format!("/api/manage/media/{media_id}/subtitles/search"))
            .await
    }

    /// Download a subtitle.
    pub async fn download_subtitle(
        &self,
        media_id: &str,
        subtitle_id: &str,
    ) -> Result<(), ApiError> {
        self.post_empty(&format!(
            "/api/manage/media/{media_id}/subtitles/{subtitle_id}/download"
        ))
        .await
    }

    /// Delete a subtitle.
    pub async fn delete_subtitle(&self, subtitle_id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/api/manage/subtitles/{subtitle_id}"))
            .await
    }

    // ── Bulk Operations ──────────────────────────────────────────────

    /// Bulk refresh metadata.
    pub async fn bulk_refresh_metadata(&self, ids: &[String]) -> Result<(), ApiError> {
        self.post_empty_with_body(
            "/api/manage/media/bulk/refresh",
            &serde_json::json!({ "ids": ids }),
        )
        .await
    }

    /// Bulk delete media items.
    pub async fn bulk_delete(&self, ids: &[String]) -> Result<(), ApiError> {
        self.post_empty_with_body(
            "/api/manage/media/bulk/delete",
            &serde_json::json!({ "ids": ids }),
        )
        .await
    }

    /// Bulk SET quality profile.
    pub async fn bulk_set_quality_profile(
        &self,
        ids: &[String],
        quality_profile_id: &str,
    ) -> Result<(), ApiError> {
        self.post_empty_with_body(
            "/api/manage/media/bulk/quality-profile",
            &serde_json::json!({ "ids": ids, "qualityProfileId": quality_profile_id }),
        )
        .await
    }

    // ── Internal HTTP helpers ────────────────────────────────────────

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let mut req = self.inner.get(format!("{}{path}", self.base_url));
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context(RequestSnafu)?;
        if !resp.status().is_success() {
            return StatusSnafu {
                status: resp.status().as_u16(),
            }
            .fail();
        }
        resp.json().await.context(ParseSnafu)
    }

    async fn get_with_params<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, ApiError> {
        let mut req = self
            .inner
            .get(format!("{}{path}", self.base_url))
            .query(params);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context(RequestSnafu)?;
        if !resp.status().is_success() {
            return StatusSnafu {
                status: resp.status().as_u16(),
            }
            .fail();
        }
        resp.json().await.context(ParseSnafu)
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        let mut req = self
            .inner
            .post(format!("{}{path}", self.base_url))
            .json(body);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context(RequestSnafu)?;
        if !resp.status().is_success() {
            return StatusSnafu {
                status: resp.status().as_u16(),
            }
            .fail();
        }
        resp.json().await.context(ParseSnafu)
    }

    async fn post_empty(&self, path: &str) -> Result<(), ApiError> {
        let mut req = self
            .inner
            .post(format!("{}{path}", self.base_url))
            .json(&serde_json::json!({}));
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context(RequestSnafu)?;
        if !resp.status().is_success() {
            return StatusSnafu {
                status: resp.status().as_u16(),
            }
            .fail();
        }
        Ok(())
    }

    async fn post_empty_with_body(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<(), ApiError> {
        let mut req = self
            .inner
            .post(format!("{}{path}", self.base_url))
            .json(body);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context(RequestSnafu)?;
        if !resp.status().is_success() {
            return StatusSnafu {
                status: resp.status().as_u16(),
            }
            .fail();
        }
        Ok(())
    }

    async fn put<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, ApiError> {
        let mut req = self
            .inner
            .put(format!("{}{path}", self.base_url))
            .json(body);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context(RequestSnafu)?;
        if !resp.status().is_success() {
            return StatusSnafu {
                status: resp.status().as_u16(),
            }
            .fail();
        }
        resp.json().await.context(ParseSnafu)
    }

    async fn delete(&self, path: &str) -> Result<(), ApiError> {
        let mut req = self.inner.delete(format!("{}{path}", self.base_url));
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.context(RequestSnafu)?;
        if !resp.status().is_success() {
            return StatusSnafu {
                status: resp.status().as_u16(),
            }
            .fail();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_construction() {
        let client = HarmoniaClient::new("http://localhost:8080");
        assert!(client.token.is_none());
    }

    #[test]
    fn base_url_strips_trailing_slash() {
        let client = HarmoniaClient::new("http://localhost:8080/");
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn set_and_clear_token() {
        let mut client = HarmoniaClient::new("http://localhost:8080");
        client.set_token("test-token");
        assert_eq!(client.token.as_deref(), Some("test-token"));
        client.clear_token();
        assert!(client.token.is_none());
    }
}
