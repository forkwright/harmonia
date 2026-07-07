pub mod acquisition;
pub mod auth;
pub mod catalog;
pub mod content;
pub(crate) mod cover;
pub mod search;
pub mod types_v1;
pub mod types_v2;

use axum::Router;

use crate::state::AppState;

// NOTE: `/content/{id}` is registered by the streaming route group in
// `crate::build_router` — byte-serving routes are exempt from the API
// response timeout.
pub fn opds_routes() -> Router<AppState> {
    use axum::routing::get;
    Router::new()
        .route("/auth", get(auth::auth_document))
        .route("/v2/catalog", get(catalog::catalog_v2))
        .route("/v2/books", get(catalog::books_v2))
        .route("/v2/books/{id}", get(catalog::book_v2))
        .route("/v2/comics", get(catalog::comics_v2))
        .route("/v2/comics/{id}", get(catalog::comic_v2))
        .route("/v2/search", get(search::search_v2))
        .route("/v2/shelf/{shelf}", get(catalog::shelf_v2))
        .route("/v1/catalog.xml", get(catalog::catalog_v1))
        .route("/v1/books.xml", get(catalog::books_v1))
        .route("/v1/comics.xml", get(catalog::comics_v1))
        .route("/v1/search.xml", get(search::search_v1))
        .route("/v1/entry/{id}", get(catalog::entry_v1))
}
