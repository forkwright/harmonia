use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use super::*;
use crate::opds::opds_routes;
use crate::test_helpers::{admin_token, test_state};

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

async fn insert_book_with_file(state: &AppState, dir: &std::path::Path) -> uuid::Uuid {
    let file_path = dir.join("foundation.epub");
    tokio::fs::write(&file_path, b"epub-file-contents")
        .await
        .unwrap();
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
        file_path: Some(file_path.to_string_lossy().into_owned()),
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
    id
}

async fn fetch_book_publication(state: AppState, token: &str, id: uuid::Uuid) -> serde_json::Value {
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
    body["publications"][0].clone()
}

#[tokio::test]
async fn single_book_has_cover_art_links_with_probed_type() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let dir = tempfile::TempDir::new().unwrap();
    let id = insert_book_with_file(&state, dir.path()).await;
    tokio::fs::write(
        dir.path().join("cover.png"),
        b"\x89PNG\r\n\x1a\n-fake-png-body",
    )
    .await
    .unwrap();

    let publication = fetch_book_publication(state, &token, id).await;
    let images = publication["images"].as_array().unwrap();
    let cover = images
        .iter()
        .find(|l| l["rel"].as_str() == Some("http://opds-spec.org/image"))
        .unwrap();
    let href = cover["href"].as_str().unwrap();
    assert!(href.contains("/api/books/"));
    assert!(href.contains("/cover"));
    // WHY: the advertised type must reflect the actual sidecar, not a
    // hardcoded image/jpeg.
    assert_eq!(cover["type"], "image/png");
}

#[tokio::test]
async fn book_without_cover_omits_image_links() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    let dir = tempfile::TempDir::new().unwrap();
    let id = insert_book_with_file(&state, dir.path()).await;

    let publication = fetch_book_publication(state, &token, id).await;
    assert!(
        publication.get("images").is_none(),
        "a coverless book must not advertise image links: {publication}"
    );
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

async fn insert_authors(state: &AppState, n: usize) {
    for i in 0..n {
        let person_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO media_registry (id, entity_type, display_name)
             VALUES (?, 'person', ?)",
        )
        .bind(&person_id)
        .bind(format!("Author {i:04}"))
        .execute(&state.db.write)
        .await
        .unwrap();

        let book_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        sqlx::query("INSERT INTO books (id, title) VALUES (?, ?)")
            .bind(&book_id)
            .bind(format!("Authored Book {i:04}"))
            .execute(&state.db.write)
            .await
            .unwrap();
        sqlx::query("INSERT INTO book_authors (book_id, person_id, role) VALUES (?, ?, 'author')")
            .bind(&book_id)
            .bind(&person_id)
            .execute(&state.db.write)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn shelf_authors_lists_author_navigation() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    insert_authors(&state, 3).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/shelf/authors")
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
    assert_eq!(nav.len(), 3);
    let titles: Vec<_> = nav.iter().map(|n| n["title"].as_str().unwrap()).collect();
    assert!(titles.contains(&"Author 0000"));
    let hrefs: Vec<_> = nav.iter().map(|n| n["href"].as_str().unwrap()).collect();
    assert!(
        hrefs
            .iter()
            .all(|h| h.starts_with("/opds/v2/search?q=Author")),
        "author entries must link to the scoped search feed: {hrefs:?}"
    );
}

#[tokio::test]
async fn shelf_authors_paginates_with_next_link() {
    let (state, auth) = test_state().await;
    let token = admin_token(&auth).await;
    // Default page size is 50; insert 51 to trigger next link
    insert_authors(&state, 51).await;
    let app = opds_routes().with_state(state);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v2/shelf/authors")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["navigation"].as_array().unwrap().len(), 50);
    let links = body["links"].as_array().unwrap();
    assert!(
        links.iter().any(|l| l["rel"].as_str() == Some("next")),
        "expected next link for 51 authors"
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v2/shelf/authors?page=2")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["navigation"].as_array().unwrap().len(), 1);
    let links = body["links"].as_array().unwrap();
    assert!(
        !links.iter().any(|l| l["rel"].as_str() == Some("next")),
        "no next link on the last page"
    );
}
