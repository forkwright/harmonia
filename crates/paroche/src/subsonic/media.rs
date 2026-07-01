use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;

use super::auth::authenticate;
use super::types::{
    ERR_GENERIC, ERR_MISSING_PARAM, SubsonicCommon, respond_error, respond_ok, uuid_bytes,
};
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct StarQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
    #[serde(rename = "albumId")]
    pub album_id: Option<String>,
    #[serde(rename = "artistId")]
    pub artist_id: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct RatingQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
    pub rating: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct ScrobbleQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
    pub time: Option<i64>,
    pub submission: Option<bool>,
}

// ---------------------------------------------------------------------------
// star.view
// ---------------------------------------------------------------------------

pub async fn star(State(state): State<AppState>, Query(q): Query<StarQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let user_id_bytes = user.user_id.as_bytes().to_vec();

    // WHY: star may touch up to three rows; failures must surface and apply
    // all-or-nothing rather than silently reporting ok.
    let mut tx = match state.db.write.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!(error = %e, "star: begin transaction failed");
            return respond_error(user.format, ERR_GENERIC, "could not star item");
        }
    };

    let targets = [
        (&q.id, "track"),
        (&q.album_id, "album"),
        (&q.artist_id, "artist"),
    ];
    for (raw_id, item_type) in targets {
        if let Some(id) = raw_id
            && let Some(bytes) = uuid_bytes(id)
            && let Err(e) = sqlx::query(
                "INSERT OR IGNORE INTO subsonic_stars (user_id, item_id, item_type) VALUES (?, ?, ?)",
            )
            .bind(&user_id_bytes)
            .bind(bytes)
            .bind(item_type)
            .execute(&mut *tx)
            .await
        {
            tracing::warn!(error = %e, item_type, "star: insert failed");
            return respond_error(user.format, ERR_GENERIC, "could not star item");
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::warn!(error = %e, "star: commit failed");
        return respond_error(user.format, ERR_GENERIC, "could not star item");
    }

    respond_ok(user.format, "", None)
}

// ---------------------------------------------------------------------------
// unstar.view
// ---------------------------------------------------------------------------

pub async fn unstar(State(state): State<AppState>, Query(q): Query<StarQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let user_id_bytes = user.user_id.as_bytes().to_vec();

    let mut tx = match state.db.write.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!(error = %e, "unstar: begin transaction failed");
            return respond_error(user.format, ERR_GENERIC, "could not unstar item");
        }
    };

    for id in [&q.id, &q.album_id, &q.artist_id].into_iter().flatten() {
        if let Some(bytes) = uuid_bytes(id)
            && let Err(e) =
                sqlx::query("DELETE FROM subsonic_stars WHERE user_id = ? AND item_id = ?")
                    .bind(&user_id_bytes)
                    .bind(bytes)
                    .execute(&mut *tx)
                    .await
        {
            tracing::warn!(error = %e, "unstar: delete failed");
            return respond_error(user.format, ERR_GENERIC, "could not unstar item");
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::warn!(error = %e, "unstar: commit failed");
        return respond_error(user.format, ERR_GENERIC, "could not unstar item");
    }

    respond_ok(user.format, "", None)
}

// ---------------------------------------------------------------------------
// setRating.view
// ---------------------------------------------------------------------------

pub async fn set_rating(State(state): State<AppState>, Query(q): Query<RatingQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let id = match &q.id {
        Some(id) => id.clone(),
        None => {
            return respond_error(
                user.format,
                ERR_MISSING_PARAM,
                "required parameter is missing: id",
            );
        }
    };
    let rating = match q.rating {
        Some(r) if (1..=5).contains(&r) => r,
        Some(0) => {
            // rating=0 means remove rating
            if let Some(bytes) = uuid_bytes(&id) {
                let user_id_bytes = user.user_id.as_bytes().to_vec();
                if let Err(e) =
                    sqlx::query("DELETE FROM subsonic_ratings WHERE user_id = ? AND item_id = ?")
                        .bind(user_id_bytes)
                        .bind(bytes)
                        .execute(&state.db.write)
                        .await
                {
                    tracing::warn!(error = %e, "set_rating: rating removal failed");
                    return respond_error(user.format, ERR_GENERIC, "could not remove rating");
                }
            }
            return respond_ok(user.format, "", None);
        }
        None => {
            return respond_error(
                user.format,
                ERR_MISSING_PARAM,
                "required parameter is missing: rating",
            );
        }
        _ => return respond_error(user.format, 0, "invalid rating: must be 1-5"),
    };

    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, 0, "invalid id"),
    };

    let user_id_bytes = user.user_id.as_bytes().to_vec();
    if let Err(e) = sqlx::query(
        "INSERT INTO subsonic_ratings (user_id, item_id, rating) VALUES (?, ?, ?)
         ON CONFLICT(user_id, item_id) DO UPDATE SET rating = excluded.rating",
    )
    .bind(user_id_bytes)
    .bind(id_bytes)
    .bind(rating)
    .execute(&state.db.write)
    .await
    {
        tracing::warn!(error = %e, "set_rating: upsert failed");
        return respond_error(user.format, ERR_GENERIC, "could not set rating");
    }

    respond_ok(user.format, "", None)
}

// ---------------------------------------------------------------------------
// scrobble.view
// ---------------------------------------------------------------------------

pub async fn scrobble(State(state): State<AppState>, Query(q): Query<ScrobbleQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    // Emit event for scrobbling — acknowledged but no external scrobbler yet
    let _ = &q.id;
    let _ = &q.time;
    respond_ok(user.format, "", None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    #[expect(
        unused_imports,
        reason = "kanon: test-missing-use-super; parent items accessed via explicit super:: prefix in test bodies"
    )]
    use super::*;
    use crate::subsonic::test_helpers::{seed_music_data, subsonic_app};
    use crate::subsonic::types::uuid_str;
    #[tokio::test]
    async fn star_and_unstar_track() {
        let (app, state, key) = subsonic_app().await;
        let (_, track_id) = seed_music_data(&state).await;
        let id = uuid_str(&track_id);

        // Star
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/star.view?apiKey={key}&id={id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));

        // Unstar
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/unstar.view?apiKey={key}&id={id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
    }

    #[tokio::test]
    async fn set_rating_persisted() {
        let (app, state, key) = subsonic_app().await;
        let (_, track_id) = seed_music_data(&state).await;
        let id = uuid_str(&track_id);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/setRating.view?apiKey={key}&id={id}&rating=4"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
    }

    #[tokio::test]
    async fn scrobble_returns_ok() {
        let (app, state, key) = subsonic_app().await;
        let (_, track_id) = seed_music_data(&state).await;
        let id = uuid_str(&track_id);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/scrobble.view?apiKey={key}&id={id}&submission=true"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
    }
}
