pub mod error;
pub mod middleware;
pub mod response;
pub mod routes;
pub mod state;
pub mod ws;

use std::time::Duration;

use axum::Router;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};

use crate::{middleware::RequestIdLayer, state::AppState, ws::ws_handler};

pub use error::ParocheError;

pub fn build_router(state: AppState) -> Router {
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
        .merge(routes::stream::stream_routes())
        .route("/api/ws", axum::routing::get(ws_handler))
        .layer(RequestIdLayer)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
pub mod test_helpers {
    use std::sync::Arc;

    use exousia::ExousiaServiceImpl;
    use harmonia_common::create_event_bus;
    use harmonia_db::{DbPools, migrate::MIGRATOR};
    use horismos::{Config, ExousiaConfig};
    use sqlx::SqlitePool;

    use crate::state::AppState;

    pub async fn test_state() -> (AppState, Arc<ExousiaServiceImpl>) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let pools = Arc::new(DbPools {
            read: pool.clone(),
            write: pool,
        });
        let config = Arc::new(Config::default());
        let (event_tx, _) = create_event_bus(64);

        let exousia_config = ExousiaConfig {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_string(),
        };
        let auth = Arc::new(ExousiaServiceImpl::new(pools.clone(), exousia_config));

        let import = crate::state::make_import_service(|| async { Ok(vec![]) });

        let state = AppState::with_stubs(pools, config, event_tx, auth.clone(), import);

        (state, auth)
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::test_state;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

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
}
