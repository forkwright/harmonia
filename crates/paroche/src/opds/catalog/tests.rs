use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use exousia::AuthService;
use exousia::user::{CreateUserRequest, UserRole};
use tower::ServiceExt;

use super::*;
use crate::opds::opds_routes;
use crate::test_helpers::test_state;

async fn admin_token(auth: &Arc<exousia::ExousiaServiceImpl>) -> String {
    auth.create_user(CreateUserRequest {
        username: "admin".to_string(),
        display_name: "Admin".to_string(),
        password: "password123".to_string(),
        role: UserRole::Admin,
    })
    .await
    .unwrap();
    auth.login("admin", "password123")
        .await
        .unwrap()
        .access_token
}

async fn insert_books(state: &AppState, n: usize) {
    for i in 0..n {
        let book = apotheke::repo::book::Book {
            id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            registry_id: None,
            title: format!("Book {:04}", i),
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
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        apotheke::repo::book::insert_book(&state.db.write, &book)
            .await
            .unwrap();
    }
}

async fn insert_comics(state: &AppState, n: usize) {
    for i in 0..n {
        let comic = apotheke::repo::comic::Comic {
            id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            registry_id: None,
            series_name: format!("Series {:04}", i),
            volume: Some(1),
            issue_number: Some(1.0),
            title: Some(format!("Issue {:04}", i)),
            publisher: None,
            release_date: None,
            page_count: None,
            summary: None,
            language: None,
            comicinfo_writer: None,
            comicinfo_penciller: None,
            comicinfo_inker: None,
            comicinfo_colorist: None,
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        apotheke::repo::comic::insert_comic(&state.db.write, &comic)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn catalog_v2_unauthenticated_returns_401() {
    let (state, _auth) = test_state().await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/catalog")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn catalog_v2_returns_navigation_links() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/catalog")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let nav = body["navigation"].as_array().unwrap();
    let hrefs: Vec<_> = nav.iter().map(|n| n["href"].as_str().unwrap()).collect();
    assert!(hrefs.contains(&"/opds/v2/books"));
    assert!(hrefs.contains(&"/opds/v2/comics"));
}

#[tokio::test]
async fn catalog_v2_has_opds_content_type() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/catalog")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("application/opds+json"));
}

#[tokio::test]
async fn catalog_v2_has_search_link() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/catalog")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let links = body["links"].as_array().unwrap();
    let search_link = links.iter().find(|l| l["rel"].as_str() == Some("search"));
    assert!(search_link.is_some());
}

#[tokio::test]
async fn books_v2_next_link_when_more_items() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    // Default page size is 50; insert 51 to trigger next link
    insert_books(&state, 51).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/books")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let links = body["links"].as_array().unwrap();
    let next = links.iter().find(|l| l["rel"].as_str() == Some("next"));
    assert!(next.is_some(), "expected next link for 51 books");
}

#[tokio::test]
async fn books_v2_correct_page_size() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    insert_books(&state, 51).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/books")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let pubs = body["publications"].as_array().unwrap();
    assert_eq!(pubs.len(), 50);
}

#[tokio::test]
async fn books_v2_last_page_no_next_link() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    insert_books(&state, 5).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/books")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let links = body["links"].as_array().unwrap();
    let next = links.iter().find(|l| l["rel"].as_str() == Some("next"));
    assert!(next.is_none(), "no next link expected on last page");
}

#[tokio::test]
async fn comics_v2_returns_entries() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    insert_comics(&state, 3).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/comics")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let pubs = body["publications"].as_array().unwrap();
    assert_eq!(pubs.len(), 3);
}

#[tokio::test]
async fn single_book_has_acquisition_link_with_correct_mime() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let id = uuid::Uuid::now_v7();
    let book = apotheke::repo::book::Book {
        id: id.as_bytes().to_vec(),
        registry_id: None,
        title: "Dune".to_string(),
        subtitle: None,
        isbn: None,
        isbn13: None,
        openlibrary_id: None,
        goodreads_id: None,
        publisher: Some("Ace Books".to_string()),
        publish_date: None,
        language: Some("en".to_string()),
        page_count: None,
        description: None,
        file_path: None,
        file_format: Some("epub".to_string()),
        file_size_bytes: None,
        quality_score: None,
        quality_profile_id: None,
        source_type: "local".to_string(),
        added_at: "2026-01-01T00:00:00Z".to_string(),
    };
    apotheke::repo::book::insert_book(&state.db.write, &book)
        .await
        .unwrap();

    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/v2/books/{}", id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let pub_links = &body["publications"][0]["links"];
    let acq = pub_links
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["rel"].as_str() == Some("http://opds-spec.org/acquisition"));
    assert!(acq.is_some());
    assert_eq!(acq.unwrap()["type"], "application/epub+zip");
}

#[tokio::test]
async fn single_book_has_cover_art_links() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let id = uuid::Uuid::now_v7();
    let book = apotheke::repo::book::Book {
        id: id.as_bytes().to_vec(),
        registry_id: None,
        title: "Foundation".to_string(),
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
        file_path: None,
        file_format: None,
        file_size_bytes: None,
        quality_score: None,
        quality_profile_id: None,
        source_type: "local".to_string(),
        added_at: "2026-01-01T00:00:00Z".to_string(),
    };
    apotheke::repo::book::insert_book(&state.db.write, &book)
        .await
        .unwrap();

    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/v2/books/{}", id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let images = &body["publications"][0]["images"];
    let cover = images
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["rel"].as_str() == Some("http://opds-spec.org/image"));
    assert!(cover.is_some());
    let href = cover.unwrap()["href"].as_str().unwrap();
    assert!(href.contains("/api/books/"));
    assert!(href.contains("/cover"));
}

#[tokio::test]
async fn catalog_v1_returns_atom_feed() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/catalog.xml")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("application/atom+xml"));
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let xml = std::str::from_utf8(&bytes).unwrap();
    assert!(xml.contains("<feed"));
    assert!(xml.contains("</feed>"));
}

#[tokio::test]
async fn catalog_v1_has_book_and_comic_navigation() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/catalog.xml")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let xml = std::str::from_utf8(&bytes).unwrap();
    assert!(xml.contains("books.xml"));
    assert!(xml.contains("comics.xml"));
}

#[tokio::test]
async fn books_v1_unauthenticated_returns_401() {
    let (state, _auth) = test_state().await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/books.xml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
