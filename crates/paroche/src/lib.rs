pub mod discovery;
pub mod error;
pub mod middleware;
pub mod net_validate;
pub mod opds;
pub mod redact;
pub mod response;
pub mod routes;
pub mod state;
pub mod subsonic;
pub mod ws;

use std::time::Duration;

use axum::Router;
pub use error::ParocheError;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::middleware::RequestIdLayer;
use crate::state::AppState;
use crate::ws::ws_handler;

// WHY: bounds time-to-response-headers for request/response API routes; the
// byte-serving routes in `streaming_routes` are deliberately outside it.
const API_TIMEOUT: Duration = Duration::from_secs(30);

fn api_routes() -> Router<AppState> {
    Router::new()
        .nest("/api/auth", routes::user::auth_routes())
        .nest("/api/users", routes::user::user_routes())
        .nest("/api/music", routes::music::music_routes())
        .nest("/api/audiobooks", routes::audiobook::audiobook_routes())
        .nest("/api/books", routes::book::book_routes())
        .nest("/api/comics", routes::comic::comic_routes())
        .nest("/api/podcasts", routes::podcast::podcast_routes())
        .nest("/api/news", routes::news::news_routes())
        .nest("/api/movies", routes::movie::movie_routes())
        .nest("/api/tv", routes::tv::tv_routes())
        .nest("/api/library", routes::library::library_routes())
        .nest("/api/system", routes::system::system_routes())
        .nest("/api/v1/indexers", routes::indexer::indexer_routes())
        .nest("/api/v1/search", routes::search::search_routes())
        .nest("/api/v1/metadata", routes::metadata::metadata_routes())
        .nest("/api/v1/curation", routes::curation::curation_routes())
        .nest("/api/v1/downloads", routes::download::download_routes())
        .nest("/api/v1/requests", routes::request::request_routes())
        .nest("/api/v1/wanted", routes::wanted::wanted_routes())
        .nest("/api/v1/plex", routes::plex::plex_routes())
        .nest("/api/v1", routes::subtitle::subtitle_routes())
        .nest("/api/renderers", routes::renderer::renderer_routes())
        .nest("/opds", opds::opds_routes())
        .nest("/kosync", routes::kosync::kosync_routes())
        .merge(routes::read::reader_routes())
        .nest("/rest", subsonic::subsonic_routes())
        .nest("/api/zones", routes::zone::zone_routes())
        .route("/api/ws", axum::routing::get(ws_handler))
        .nest_service(
            "/static/reader",
            ServeDir::new("crates/paroche/assets/reader"),
        )
}

// WHY: byte-serving routes (media downloads, covers, OPDS content, audio
// streaming) are exempt from `API_TIMEOUT` — a cold disk or a large archive
// probe can legitimately hold first-byte past 30s, and these routes must
// never race a request/response deadline (#581). The guard for normal API
// routes stays at 30s; do NOT fold these back under the timed group.
fn streaming_routes() -> Router<AppState> {
    use axum::routing::get;
    Router::new()
        .route("/api/books/{id}/download", get(routes::book::download_book))
        .route("/api/books/{id}/cover", get(routes::book::book_cover))
        .route(
            "/api/comics/{id}/download",
            get(routes::comic::download_comic),
        )
        .route("/api/comics/{id}/cover", get(routes::comic::comic_cover))
        .route("/opds/content/{id}", get(opds::content::content))
        .merge(routes::stream::stream_routes())
}

fn compose_router<S>(api: Router<S>, streaming: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    api.layer(TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        API_TIMEOUT,
    ))
    .merge(streaming)
    .layer(RequestIdLayer)
    .layer(TraceLayer::new_for_http())
    .layer(CompressionLayer::new())
    .layer(CorsLayer::permissive())
}

pub fn build_router(state: AppState) -> Router {
    compose_router(api_routes(), streaming_routes()).with_state(state)
}

#[cfg(test)]
pub mod test_helpers {
    use std::sync::Arc;

    use apotheke::DbPools;
    use apotheke::migrate::MIGRATOR;
    use exousia::user::{CreateUserRequest, UserRole};
    use exousia::{AuthService, ExousiaServiceImpl};
    use horismos::{Config, ConfigHandle, ExousiaConfig, Section};
    use sqlx::SqlitePool;
    use themelion::create_event_bus;

    use crate::state::AppState;

    #[expect(
        unused_imports,
        reason = "kanon: test-missing-use-super; parent items accessed via explicit super:: prefix in test bodies"
    )]
    use super::*;
    pub async fn test_state() -> (AppState, Arc<ExousiaServiceImpl>) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let pools = Arc::new(DbPools {
            read: pool.clone(),
            write: pool,
        });
        let config = ConfigHandle::fixed(Config::default());
        let (event_tx, _) = create_event_bus(64);

        let exousia_config = ExousiaConfig {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_string(),
        };
        let auth = Arc::new(ExousiaServiceImpl::new(
            pools.clone(),
            Section::fixed(exousia_config),
        ));

        let import = crate::state::make_import_service(|| async { Ok(vec![]) });

        let state = AppState::with_stubs(pools, config, event_tx, auth.clone(), import);

        (state, auth)
    }

    pub async fn admin_token(auth: &Arc<ExousiaServiceImpl>) -> String {
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
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::test_helpers::test_state;
    #[expect(
        unused_imports,
        reason = "kanon: test-missing-use-super; parent items accessed via explicit super:: prefix in test bodies"
    )]
    use super::*;
    #[tokio::test]
    async fn build_router_serves_health() {
        let (state, _) = test_state().await;
        let app = super::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/system/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    async fn slow_first_byte() -> &'static str {
        tokio::time::sleep(std::time::Duration::from_secs(45)).await;
        "done"
    }

    fn get_req(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    // WHY: the #581 scoping contract, provable with a paused clock — the same
    // slow handler 408s under the timed group but completes in the streaming
    // group, so a byte-serving route can never lose a race against
    // API_TIMEOUT.
    #[tokio::test(start_paused = true)]
    async fn streaming_group_is_exempt_from_api_timeout() {
        use axum::routing::get;
        let app = super::compose_router::<()>(
            axum::Router::new().route("/timed", get(slow_first_byte)),
            axum::Router::new().route("/streaming", get(slow_first_byte)),
        )
        .with_state(());

        let resp = app.clone().oneshot(get_req("/timed")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::REQUEST_TIMEOUT);

        let resp = app.oneshot(get_req("/streaming")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"done");
    }

    // WHY: pins the transfer contract on the streaming group — a body that
    // takes well past API_TIMEOUT to stream arrives complete, never severed.
    #[tokio::test(start_paused = true)]
    async fn streaming_body_longer_than_api_timeout_completes() {
        use axum::routing::get;

        async fn drip() -> Body {
            Body::from_stream(futures_util::stream::unfold(0u32, |chunk| async move {
                if chunk >= 5 {
                    return None;
                }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                Some((
                    Ok::<_, std::convert::Infallible>(axum::body::Bytes::from_static(b"chunk-")),
                    chunk + 1,
                ))
            }))
        }

        let app = super::compose_router::<()>(
            axum::Router::new().route("/unused", get(slow_first_byte)),
            axum::Router::new().route("/drip", get(drip)),
        )
        .with_state(());

        let resp = app.oneshot(get_req("/drip")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        // WHY: 5 chunks x 10 virtual seconds = a 50s transfer, well past the
        // 30s API_TIMEOUT — every chunk must still arrive.
        assert_eq!(&body[..], b"chunk-".repeat(5).as_slice());
    }
}
