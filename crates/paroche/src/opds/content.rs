use std::path::{Path as FsPath, PathBuf};

use axum::extract::{Path, Request, State};
use axum::http::HeaderValue;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use exousia::AuthenticatedUser;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use uuid::Uuid;

use crate::error::ParocheError;
use crate::opds::acquisition::effective_mime;
use crate::state::AppState;

pub(crate) async fn serve_file_response(path: impl AsRef<FsPath>, request: Request) -> Response {
    ServeFile::new(path)
        .oneshot(request)
        .await
        .unwrap_or_else(|never| match never {})
        .into_response()
}

// WHY: ServeFile's extension-based Content-Type guess has no entry for
// `.cbz`/`.mobi`/etc — override with the format-aware acquisition mime so
// OPDS clients receive the type advertised in the catalog feed.
pub(crate) async fn serve_media_file(
    file_path: &str,
    mime: &'static str,
    request: Request,
) -> Response {
    let mut response = serve_file_response(file_path, request).await;
    if response.status().is_success() {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(mime));
    }
    response
}

pub(crate) fn attachment_disposition(file_path: &str) -> Option<HeaderValue> {
    let name = FsPath::new(file_path).file_name()?.to_str()?;
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        return None;
    }
    HeaderValue::from_str(&format!("attachment; filename=\"{sanitized}\"")).ok()
}

// NOTE: sidecar cover convention — a cover image lives beside the media file
// as `cover.{jpg,jpeg,png,webp}`; there is no dedicated cover column/table.
pub(crate) async fn find_sidecar_cover(media_file_path: &str) -> Option<PathBuf> {
    let parent = FsPath::new(media_file_path).parent()?;
    for ext in ["jpg", "jpeg", "png", "webp"] {
        let candidate = parent.join(format!("cover.{ext}"));
        if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
            return Some(candidate);
        }
    }
    None
}

/// Serves raw book/comic bytes for the foliate-js reader, resolving the id
/// against the book table first and falling back to comics (both are UUIDv7
/// in separate tables — mirrors the `entry_v1` catalog lookup).
pub async fn content(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: AuthenticatedUser,
    request: Request,
) -> Result<Response, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    if let Some(book) = apotheke::repo::book::get_book(&state.db.read, &id_bytes).await? {
        let file_path = book.file_path.ok_or(ParocheError::NotFound)?;
        let mime = effective_mime(book.file_format.as_deref(), Some(&file_path));
        return Ok(serve_media_file(&file_path, mime, request).await);
    }

    if let Some(comic) = apotheke::repo::comic::get_comic(&state.db.read, &id_bytes).await? {
        let file_path = comic.file_path.ok_or(ParocheError::NotFound)?;
        let mime = effective_mime(comic.file_format.as_deref(), Some(&file_path));
        return Ok(serve_media_file(&file_path, mime, request).await);
    }

    Err(ParocheError::NotFound)
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::StatusCode;
    use tempfile::TempDir;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;
    use crate::test_helpers::{admin_token, test_state};

    fn book_template(
        id: Uuid,
        file_path: Option<String>,
        file_format: Option<String>,
    ) -> apotheke::repo::book::Book {
        apotheke::repo::book::Book {
            id: id.as_bytes().to_vec(),
            registry_id: None,
            title: "Dune".to_string(),
            subtitle: None,
            isbn: None,
            isbn13: None,
            openlibrary_id: None,
            goodreads_id: None,
            publisher: None,
            publish_date: None,
            language: None,
            page_count: None,
            description: None,
            file_path,
            file_format,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn comic_template(
        id: Uuid,
        file_path: Option<String>,
        file_format: Option<String>,
    ) -> apotheke::repo::comic::Comic {
        apotheke::repo::comic::Comic {
            id: id.as_bytes().to_vec(),
            registry_id: None,
            series_name: "Saga".to_string(),
            volume: Some(1),
            issue_number: Some(1.0),
            title: Some("Chapter One".to_string()),
            publisher: None,
            release_date: None,
            page_count: None,
            summary: None,
            language: None,
            comicinfo_writer: None,
            comicinfo_penciller: None,
            comicinfo_inker: None,
            comicinfo_colorist: None,
            file_path,
            file_format,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    async fn insert_book_with_file(state: &AppState, dir: &TempDir, bytes: &[u8]) -> Uuid {
        let path = dir.path().join("book.epub");
        tokio::fs::write(&path, bytes).await.unwrap();
        let id = Uuid::now_v7();
        let book = book_template(
            id,
            Some(path.to_string_lossy().into_owned()),
            Some("epub".to_string()),
        );
        apotheke::repo::book::insert_book(&state.db.write, &book)
            .await
            .unwrap();
        id
    }

    async fn insert_comic_with_file(state: &AppState, dir: &TempDir, bytes: &[u8]) -> Uuid {
        let path = dir.path().join("comic.cbz");
        tokio::fs::write(&path, bytes).await.unwrap();
        let id = Uuid::now_v7();
        let comic = comic_template(
            id,
            Some(path.to_string_lossy().into_owned()),
            Some("cbz".to_string()),
        );
        apotheke::repo::comic::insert_comic(&state.db.write, &comic)
            .await
            .unwrap();
        id
    }

    async fn insert_book_without_file(state: &AppState) -> Uuid {
        let id = Uuid::now_v7();
        let book = book_template(id, None, None);
        apotheke::repo::book::insert_book(&state.db.write, &book)
            .await
            .unwrap();
        id
    }

    async fn write_cover(dir: &TempDir) {
        tokio::fs::write(dir.path().join("cover.jpg"), b"fake-jpeg-bytes")
            .await
            .unwrap();
    }

    fn auth_req(uri: String, token: &str) -> Request {
        Request::builder()
            .uri(uri)
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn book_download_serves_advertised_mime_and_bytes() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let bytes = b"epub-file-contents";
        let id = insert_book_with_file(&state, &dir, bytes).await;
        let app = crate::build_router(state);

        let resp = app
            .oneshot(auth_req(format!("/api/books/{id}/download"), &token))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(ct, "application/epub+zip");
        let disposition = resp
            .headers()
            .get("content-disposition")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(disposition.contains("book.epub"));
        let accept_ranges = resp
            .headers()
            .get("accept-ranges")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(accept_ranges, "bytes");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], bytes);
    }

    #[tokio::test]
    async fn book_download_supports_range() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let bytes = b"epub-file-contents";
        let id = insert_book_with_file(&state, &dir, bytes).await;
        let app = crate::build_router(state);

        let req = Request::builder()
            .uri(format!("/api/books/{id}/download"))
            .header("Authorization", format!("Bearer {token}"))
            .header("Range", "bytes=0-3")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        let content_range = resp
            .headers()
            .get("content-range")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(content_range, format!("bytes 0-3/{}", bytes.len()));
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(ct, "application/epub+zip");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], &bytes[0..4]);
    }

    #[tokio::test]
    async fn comic_download_serves_advertised_mime_and_bytes() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let bytes = b"cbz-file-contents";
        let id = insert_comic_with_file(&state, &dir, bytes).await;
        let app = crate::build_router(state);

        let resp = app
            .oneshot(auth_req(format!("/api/comics/{id}/download"), &token))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(ct, "application/x-cbz");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], bytes);
    }

    #[tokio::test]
    async fn opds_content_serves_book_with_range() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let bytes = b"epub-file-contents";
        let id = insert_book_with_file(&state, &dir, bytes).await;
        let app = crate::build_router(state);

        let resp = app
            .clone()
            .oneshot(auth_req(format!("/opds/content/{id}"), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(ct, "application/epub+zip");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], bytes);

        let req = Request::builder()
            .uri(format!("/opds/content/{id}"))
            .header("Authorization", format!("Bearer {token}"))
            .header("Range", "bytes=0-3")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        let content_range = resp
            .headers()
            .get("content-range")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(content_range, format!("bytes 0-3/{}", bytes.len()));
    }

    #[tokio::test]
    async fn opds_content_serves_comic_fallback() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let bytes = b"cbz-file-contents";
        let id = insert_comic_with_file(&state, &dir, bytes).await;
        let app = crate::build_router(state);

        let resp = app
            .oneshot(auth_req(format!("/opds/content/{id}"), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(ct, "application/x-cbz");
    }

    #[tokio::test]
    async fn book_cover_serves_sidecar_image() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let id = insert_book_with_file(&state, &dir, b"epub-file-contents").await;
        write_cover(&dir).await;
        let app = crate::build_router(state);

        let resp = app
            .clone()
            .oneshot(auth_req(format!("/api/books/{id}/cover"), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(ct, "image/jpeg");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"fake-jpeg-bytes");

        let resp = app
            .oneshot(auth_req(
                format!("/api/books/{id}/cover?size=thumbnail"),
                &token,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn comic_cover_serves_sidecar_image() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let id = insert_comic_with_file(&state, &dir, b"cbz-file-contents").await;
        write_cover(&dir).await;
        let app = crate::build_router(state);

        let resp = app
            .clone()
            .oneshot(auth_req(format!("/api/comics/{id}/cover"), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(ct, "image/jpeg");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"fake-jpeg-bytes");

        let resp = app
            .oneshot(auth_req(
                format!("/api/comics/{id}/cover?size=thumbnail"),
                &token,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cover_missing_returns_404() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        let id = insert_book_with_file(&state, &dir, b"epub-file-contents").await;
        let app = crate::build_router(state);

        let resp = app
            .oneshot(auth_req(format!("/api/books/{id}/cover"), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn missing_file_path_returns_404() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let id = insert_book_without_file(&state).await;
        let app = crate::build_router(state);

        let resp = app
            .clone()
            .oneshot(auth_req(format!("/api/books/{id}/download"), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let resp = app
            .oneshot(auth_req(format!("/opds/content/{id}"), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_id_returns_404() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let app = crate::build_router(state);
        let id = Uuid::now_v7();

        for uri in [
            format!("/api/books/{id}/download"),
            format!("/api/books/{id}/cover"),
            format!("/api/comics/{id}/download"),
            format!("/api/comics/{id}/cover"),
            format!("/opds/content/{id}"),
        ] {
            let resp = app
                .clone()
                .oneshot(auth_req(uri.clone(), &token))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND, "uri: {uri}");
        }
    }

    #[tokio::test]
    async fn malformed_id_returns_400() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let app = crate::build_router(state);

        for uri in [
            "/api/books/not-a-uuid/download".to_string(),
            "/api/books/not-a-uuid/cover".to_string(),
            "/api/comics/not-a-uuid/download".to_string(),
            "/api/comics/not-a-uuid/cover".to_string(),
            "/opds/content/not-a-uuid".to_string(),
        ] {
            let resp = app
                .clone()
                .oneshot(auth_req(uri.clone(), &token))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "uri: {uri}");
        }
    }

    #[tokio::test]
    async fn unauthenticated_returns_401() {
        let (state, _auth) = test_state().await;
        let app = crate::build_router(state);
        let id = Uuid::now_v7();

        for uri in [
            format!("/api/books/{id}/download"),
            format!("/api/books/{id}/cover"),
            format!("/api/comics/{id}/download"),
            format!("/api/comics/{id}/cover"),
            format!("/opds/content/{id}"),
        ] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(uri.clone())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "uri: {uri}");
        }
    }

    #[tokio::test]
    async fn advertised_v2_links_dispatch() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let dir = TempDir::new().unwrap();
        insert_book_with_file(&state, &dir, b"epub-file-contents").await;
        write_cover(&dir).await;
        let app = crate::build_router(state);

        let resp = app
            .clone()
            .oneshot(auth_req("/opds/v2/books".to_string(), &token))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        let download_href = body["publications"][0]["links"][0]["href"]
            .as_str()
            .unwrap()
            .to_string();
        let image_hrefs: Vec<String> = body["publications"][0]["images"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["href"].as_str().unwrap().to_string())
            .collect();

        let mut hrefs = vec![download_href];
        hrefs.extend(image_hrefs);
        assert_eq!(
            hrefs.len(),
            3,
            "expected download + cover + thumbnail hrefs"
        );

        for href in hrefs {
            let resp = app
                .clone()
                .oneshot(auth_req(href.clone(), &token))
                .await
                .unwrap();
            assert_ne!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "advertised href must dispatch: {href}"
            );
        }
    }
}
