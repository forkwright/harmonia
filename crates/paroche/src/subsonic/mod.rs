pub mod auth;
pub mod browsing;
pub mod lists;
pub mod media;
pub mod playlists;
pub mod retrieval;
pub mod search;
pub mod system;
pub mod types;

use axum::{Router, routing::get};

use crate::state::AppState;

pub fn subsonic_routes() -> Router<AppState> {
    Router::new()
        // System
        .route("/ping.view", get(system::ping).post(system::ping))
        .route(
            "/getLicense.view",
            get(system::get_license).post(system::get_license),
        )
        .route(
            "/getOpenSubsonicExtensions.view",
            get(system::get_open_subsonic_extensions).post(system::get_open_subsonic_extensions),
        )
        // Browsing
        .route(
            "/getMusicFolders.view",
            get(browsing::get_music_folders).post(browsing::get_music_folders),
        )
        .route(
            "/getIndexes.view",
            get(browsing::get_indexes).post(browsing::get_indexes),
        )
        .route(
            "/getArtists.view",
            get(browsing::get_artists).post(browsing::get_artists),
        )
        .route(
            "/getArtist.view",
            get(browsing::get_artist).post(browsing::get_artist),
        )
        .route(
            "/getAlbum.view",
            get(browsing::get_album).post(browsing::get_album),
        )
        .route(
            "/getSong.view",
            get(browsing::get_song).post(browsing::get_song),
        )
        .route(
            "/getMusicDirectory.view",
            get(browsing::get_music_directory).post(browsing::get_music_directory),
        )
        // Retrieval
        .route(
            "/stream.view",
            get(retrieval::stream).post(retrieval::stream),
        )
        .route(
            "/getCoverArt.view",
            get(retrieval::get_cover_art).post(retrieval::get_cover_art),
        )
        .route(
            "/getAvatar.view",
            get(retrieval::get_avatar).post(retrieval::get_avatar),
        )
        // Search
        .route("/search3.view", get(search::search3).post(search::search3))
        // Lists
        .route(
            "/getAlbumList2.view",
            get(lists::get_album_list2).post(lists::get_album_list2),
        )
        .route(
            "/getRandomSongs.view",
            get(lists::get_random_songs).post(lists::get_random_songs),
        )
        .route(
            "/getStarred2.view",
            get(lists::get_starred2).post(lists::get_starred2),
        )
        .route(
            "/getNowPlaying.view",
            get(lists::get_now_playing).post(lists::get_now_playing),
        )
        // Media annotation
        .route("/star.view", get(media::star).post(media::star))
        .route("/unstar.view", get(media::unstar).post(media::unstar))
        .route(
            "/setRating.view",
            get(media::set_rating).post(media::set_rating),
        )
        .route("/scrobble.view", get(media::scrobble).post(media::scrobble))
        // Playlists
        .route(
            "/getPlaylists.view",
            get(playlists::get_playlists).post(playlists::get_playlists),
        )
        .route(
            "/getPlaylist.view",
            get(playlists::get_playlist).post(playlists::get_playlist),
        )
        .route(
            "/createPlaylist.view",
            get(playlists::create_playlist).post(playlists::create_playlist),
        )
        .route(
            "/updatePlaylist.view",
            get(playlists::update_playlist).post(playlists::update_playlist),
        )
        .route(
            "/deletePlaylist.view",
            get(playlists::delete_playlist).post(playlists::delete_playlist),
        )
}

// ---------------------------------------------------------------------------
// Test helpers shared across subsonic test modules
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod test_helpers {

    use axum::Router;
    use exousia::AuthService;
    use themelion::ids::UserId;
    use uuid::Uuid;

    use crate::{state::AppState, subsonic::subsonic_routes, test_helpers::test_state};

    /// Build the subsonic sub-router wired with a fresh in-memory state.
    /// Returns (router, state, api_key_string)
    pub async fn subsonic_app() -> (Router, AppState, String) {
        let (state, auth) = test_state().await;

        // Create a test user and API key
        auth.create_user(exousia::user::CreateUserRequest {
            username: "subsonic_test".to_string(),
            display_name: "Subsonic Test".to_string(),
            password: "testpass".to_string(),
            role: exousia::user::UserRole::Admin,
        })
        .await
        .unwrap();

        let api_key = auth
            .create_api_key(get_user_id(&state, "subsonic_test").await, "test key")
            .await
            .unwrap();

        let router = Router::new()
            .nest("/rest", subsonic_routes())
            .with_state(state.clone());
        (router, state, api_key)
    }

    async fn get_user_id(state: &AppState, username: &str) -> UserId {
        let id_bytes: Vec<u8> = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(&state.db.read)
            .await
            .unwrap();
        let uuid = Uuid::from_slice(&id_bytes).unwrap();
        UserId::from_uuid(uuid)
    }

    /// Seed a minimal music hierarchy: one artist, one album, one track.
    /// Returns (release_group_id_bytes, track_id_bytes).
    pub async fn seed_music_data(state: &AppState) -> (Vec<u8>, Vec<u8>) {
        let now = "2026-01-01T00:00:00Z";

        // Artist in media_registry
        let artist_id = Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO media_registry (id, entity_type, display_name, created_at, updated_at)
             VALUES (?, 'person', 'Test Artist', ?, ?)",
        )
        .bind(&artist_id)
        .bind(now)
        .bind(now)
        .execute(&state.db.write)
        .await
        .unwrap();

        // Release group (album)
        let group_id = Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO music_release_groups (id, title, rg_type, year, added_at)
             VALUES (?, 'Test Album', 'album', 2024, ?)",
        )
        .bind(&group_id)
        .bind(now)
        .execute(&state.db.write)
        .await
        .unwrap();

        // Link artist → album
        sqlx::query(
            "INSERT INTO music_release_group_artists (release_group_id, artist_id, role)
             VALUES (?, ?, 'primary')",
        )
        .bind(&group_id)
        .bind(&artist_id)
        .execute(&state.db.write)
        .await
        .unwrap();

        // Release
        let release_id = Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO music_releases (id, release_group_id, title, release_date, added_at)
             VALUES (?, ?, 'Test Album', '2024-01-01', ?)",
        )
        .bind(&release_id)
        .bind(&group_id)
        .bind(now)
        .execute(&state.db.write)
        .await
        .unwrap();

        // Medium
        let medium_id = Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO music_media (id, release_id, position, format) VALUES (?, ?, 1, 'Digital')",
        )
        .bind(&medium_id)
        .bind(&release_id)
        .execute(&state.db.write)
        .await
        .unwrap();

        // Track
        let track_id = Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO music_tracks
             (id, medium_id, position, title, duration_ms, codec, source_type, added_at)
             VALUES (?, ?, 1, 'Test Track', 240000, 'FLAC', 'local', ?)",
        )
        .bind(&track_id)
        .bind(&medium_id)
        .bind(now)
        .execute(&state.db.write)
        .await
        .unwrap();

        // Link track → artist
        sqlx::query(
            "INSERT INTO music_track_artists (track_id, artist_id, role) VALUES (?, ?, 'primary')",
        )
        .bind(&track_id)
        .bind(&artist_id)
        .execute(&state.db.write)
        .await
        .unwrap();

        (group_id, track_id)
    }

    /// Helpers for legacy MD5 token auth tests.
    pub mod make_api_key {
        use super::*;
        use crate::subsonic::auth::set_subsonic_password;

        /// Create a user with a subsonic password and return (username, token, salt).
        pub async fn legacy_params(
            state: &AppState,
            username: &str,
            password: &str,
        ) -> (String, String, String) {
            // Create user if doesn't exist
            let _ = state
                .auth
                .create_user(exousia::user::CreateUserRequest {
                    username: username.to_string(),
                    display_name: username.to_string(),
                    password: "any_argon2_password".to_string(),
                    role: exousia::user::UserRole::Member,
                })
                .await;

            let user_id = get_user_id(state, username).await;
            set_subsonic_password(&state.db.write, user_id, password)
                .await
                .unwrap();

            let salt = "randomsalt";
            let token = format!("{:x}", md5::compute(format!("{password}{salt}")));
            (username.to_string(), token, salt.to_string())
        }
    }
}
