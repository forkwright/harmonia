use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use exousia::AuthenticatedUser;
use serde::Deserialize;

use crate::{error::ParocheError, routes::music::chrono_now_pub, state::AppState};

use super::{
    catalog::{OpdsOpenSearchResponse, OpdsV1Response, OpdsV2Response},
    types_v1::{AtomEntry, AtomFeed, AtomLink, open_search_description},
    types_v2::{FeedMetadata, MIME_OPDS_V2, OpdsFeed, OpdsLink},
};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn search_v2(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(sq): Query<SearchQuery>,
) -> Result<OpdsV2Response, ParocheError> {
    let query = sq.q.as_deref().unwrap_or("").trim().to_string();
    let page_size = i64::try_from(state.config.paroche.opds_page_size).unwrap_or_default();

    let books = apotheke::repo::book::search_books(&state.db.read, &query, page_size, 0).await?;
    let comics =
        apotheke::repo::comic::search_comics(&state.db.read, &query, page_size, 0).await?;

    let mut publications: Vec<_> = books
        .iter()
        .map(super::catalog::book_to_publication)
        .collect();
    publications.extend(comics.iter().map(super::catalog::comic_to_publication));

    let count = publications.len() as u64;

    Ok(OpdsV2Response(OpdsFeed {
        metadata: FeedMetadata {
            title: format!("Search results for \"{query}\""),
            number_of_items: Some(count),
            items_per_page: None,
            current_page: None,
        },
        links: vec![OpdsLink::new(
            "self",
            format!("/opds/v2/search?q={}", urlencoded(&query)),
            MIME_OPDS_V2,
        )],
        navigation: vec![],
        publications,
    }))
}

pub async fn search_v1(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(sq): Query<SearchQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if let Some(query) = sq.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let page_size = i64::try_from(state.config.paroche.opds_page_size).unwrap_or_default();
        let now = chrono_now_pub();

        let books =
            apotheke::repo::book::search_books(&state.db.read, query, page_size, 0).await?;
        let comics =
            apotheke::repo::comic::search_comics(&state.db.read, query, page_size, 0).await?;

        let mut entries: Vec<AtomEntry> = books
            .iter()
            .map(super::catalog::book_to_atom_entry)
            .collect();
        entries.extend(comics.iter().map(super::catalog::comic_to_atom_entry));

        let feed = AtomFeed {
            id: format!("urn:harmonia:search:{}", urlencoded(query)),
            title: format!("Search: {query}"),
            updated: now,
            links: vec![AtomLink {
                rel: "self".to_string(),
                href: format!("/opds/v1/search.xml?q={}", urlencoded(query)),
                link_type: "application/atom+xml;profile=opds-catalog".to_string(),
                title: None,
            }],
            entries,
        };
        Ok(OpdsV1Response(feed.to_xml()).into_response())
    } else {
        Ok(OpdsOpenSearchResponse(open_search_description()).into_response())
    }
}

fn urlencoded(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                vec![c]
            } else {
                format!("%{:02X}", u32::from(c))
                    .chars()
                    .collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::opds::opds_routes;
    use crate::test_helpers::test_state;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::{
        AuthService,
        user::{CreateUserRequest, UserRole},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

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

    #[tokio::test]
    async fn search_unauthenticated_returns_401() {
        let (state, _auth) = test_state().await;
        let app = opds_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v2/search?q=dune")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn search_finds_book_by_title() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let book = apotheke::repo::book::Book {
            id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            registry_id: None,
            title: "Dune".to_string(),
            subtitle: None,
            isbn: None,
            isbn13: None,
            openlibrary_id: None,
            goodreads_id: None,
            publisher: Some("Ace Books".to_string()),
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
                    .uri("/v2/search?q=Dune")
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
        assert!(!pubs.is_empty());
        assert_eq!(pubs[0]["metadata"]["title"], "Dune");
    }

    #[tokio::test]
    async fn search_finds_comic_by_writer() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let comic = apotheke::repo::comic::Comic {
            id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            registry_id: None,
            series_name: "Sandman".to_string(),
            volume: Some(1),
            issue_number: Some(1.0),
            title: Some("Preludes & Nocturnes".to_string()),
            publisher: Some("DC Comics".to_string()),
            release_date: None,
            page_count: None,
            summary: None,
            language: None,
            comicinfo_writer: Some("Neil Gaiman".to_string()),
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

        let app = opds_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v2/search?q=Neil%20Gaiman")
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
        assert!(!pubs.is_empty());
    }

    #[tokio::test]
    async fn search_v1_no_query_returns_opensearch_description() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let app = opds_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/search.xml")
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
        assert!(ct.contains("opensearchdescription+xml"));
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(xml.contains("OpenSearchDescription"));
    }
}
