use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing;
use uuid::Uuid;

use super::auth::authenticate;
use super::types::{
    ERR_GENERIC, ERR_MISSING_PARAM, ERR_NOT_FOUND, SubsonicCommon, codec_content_type,
    codec_suffix, respond_error, respond_ok, song_json, song_xml_elem, uuid_bytes, uuid_str,
};
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct PlaylistsQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub username: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct GetPlaylistQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct CreatePlaylistQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    #[serde(rename = "playlistId")]
    pub playlist_id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "songId")]
    pub song_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
pub struct UpdatePlaylistQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    #[serde(rename = "playlistId")]
    pub playlist_id: Option<String>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub public: Option<bool>,
    #[serde(rename = "songIndexToRemove")]
    pub song_indexes_to_remove: Option<Vec<i64>>,
    #[serde(rename = "songIdToAdd")]
    pub song_ids_to_add: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
pub struct DeletePlaylistQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct PlaylistRow {
    id: Vec<u8>,
    name: String,
    comment: Option<String>,
    public: i64,
    owner: String,
    song_count: i64,
    duration: i64,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct SongRow {
    id: Vec<u8>,
    title: String,
    position: i64,
    duration_ms: Option<i64>,
    codec: Option<String>,
    album_id: Vec<u8>,
    album_title: String,
    year: Option<i64>,
    artist_id: Option<Vec<u8>>,
    artist_name: Option<String>,
}

// ---------------------------------------------------------------------------
// getPlaylists
// ---------------------------------------------------------------------------

pub async fn get_playlists(
    State(state): State<AppState>,
    Query(q): Query<PlaylistsQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let user_id_bytes = user.user_id.as_bytes().to_vec();

    let playlists = sqlx::query_as::<_, PlaylistRow>(
        "SELECT sp.id, sp.name, sp.comment, sp.public, u.username as owner,
                COUNT(spt.track_id) as song_count,
                COALESCE(SUM(t.duration_ms), 0) / 1000 as duration,
                sp.created_at, sp.updated_at
         FROM subsonic_playlists sp
         JOIN users u ON u.id = sp.owner_id
         LEFT JOIN subsonic_playlist_tracks spt ON spt.playlist_id = sp.id
         LEFT JOIN music_tracks t ON t.id = spt.track_id
         WHERE sp.owner_id = ? OR sp.public = 1
         GROUP BY sp.id
         ORDER BY sp.name",
    )
    .bind(&user_id_bytes)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "db query failed");
        vec![]
    });

    let (xml_items, json_items) = build_playlist_list(&playlists);
    let xml = format!("<playlists>{xml_items}</playlists>");
    respond_ok(
        user.format,
        &xml,
        Some(("playlists", json!({ "playlist": json_items }))),
    )
}

// ---------------------------------------------------------------------------
// getPlaylist
// ---------------------------------------------------------------------------

pub async fn get_playlist(
    State(state): State<AppState>,
    Query(q): Query<GetPlaylistQuery>,
) -> Response {
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
    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };
    let user_id_bytes = user.user_id.as_bytes().to_vec();

    let playlist = match sqlx::query_as::<_, PlaylistRow>(
        "SELECT sp.id, sp.name, sp.comment, sp.public, u.username as owner,
                COUNT(spt.track_id) as song_count,
                COALESCE(SUM(t.duration_ms), 0) / 1000 as duration,
                sp.created_at, sp.updated_at
         FROM subsonic_playlists sp
         JOIN users u ON u.id = sp.owner_id
         LEFT JOIN subsonic_playlist_tracks spt ON spt.playlist_id = sp.id
         LEFT JOIN music_tracks t ON t.id = spt.track_id
         WHERE sp.id = ? AND (sp.owner_id = ? OR sp.public = 1)
         GROUP BY sp.id",
    )
    .bind(&id_bytes)
    .bind(&user_id_bytes)
    .fetch_optional(&state.db.read)
    .await
    {
        Ok(Some(p)) => p,
        _ => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    let songs = sqlx::query_as::<_, SongRow>(
        "SELECT t.id, t.title, t.position, t.duration_ms, t.codec,
                mrg.id as album_id, mrg.title as album_title, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM subsonic_playlist_tracks spt
         JOIN music_tracks t ON t.id = spt.track_id
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases r ON r.id = mm.release_id
         JOIN music_release_groups mrg ON mrg.id = r.release_group_id
         LEFT JOIN music_track_artists mta ON mta.track_id = t.id AND mta.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mta.artist_id
         WHERE spt.playlist_id = ?
         ORDER BY spt.position",
    )
    .bind(&id_bytes)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "db query failed");
        vec![]
    });

    let (xml_songs, json_songs) = build_songs(&songs);
    let playlist_id = uuid_str(&playlist.id);
    let xml = format!(
        r#"<playlist id="{}" name="{}" comment="{}" owner="{}" public="{}" songCount="{}" duration="{}" created="{}" changed="{}">{xml_songs}</playlist>"#,
        super::types::xml_escape(&playlist_id),
        super::types::xml_escape(&playlist.name),
        super::types::xml_escape(playlist.comment.as_deref().unwrap_or("")),
        super::types::xml_escape(&playlist.owner),
        if playlist.public != 0 {
            "true"
        } else {
            "false"
        },
        playlist.song_count,
        playlist.duration,
        playlist.created_at,
        playlist.updated_at,
    );
    let json_val = json!({
        "id": playlist_id,
        "name": playlist.name,
        "comment": playlist.comment,
        "owner": playlist.owner,
        "public": playlist.public != 0,
        "songCount": playlist.song_count,
        "duration": playlist.duration,
        "created": playlist.created_at,
        "changed": playlist.updated_at,
        "entry": json_songs
    });
    respond_ok(user.format, &xml, Some(("playlist", json_val)))
}

// ---------------------------------------------------------------------------
// createPlaylist
// ---------------------------------------------------------------------------

pub async fn create_playlist(
    State(state): State<AppState>,
    Query(q): Query<CreatePlaylistQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let name = match &q.name {
        Some(n) if !n.trim().is_empty() => n.clone(),
        _ => {
            return respond_error(
                user.format,
                ERR_MISSING_PARAM,
                "required parameter is missing: name",
            );
        }
    };

    let playlist_id = Uuid::now_v7().as_bytes().to_vec();
    let user_id_bytes = user.user_id.as_bytes().to_vec();

    // WHY: single transaction — a failed playlist INSERT must not admit track
    // rows or report success, and a failed track INSERT must roll back the
    // playlist row (drop of an uncommitted tx rolls back).
    let mut tx = match state.db.write.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!(error = %e, "create_playlist: begin transaction failed");
            return respond_error(user.format, ERR_GENERIC, "could not create playlist");
        }
    };

    if let Err(e) =
        sqlx::query("INSERT INTO subsonic_playlists (id, owner_id, name) VALUES (?, ?, ?)")
            .bind(&playlist_id)
            .bind(&user_id_bytes)
            .bind(&name)
            .execute(&mut *tx)
            .await
    {
        tracing::warn!(error = %e, "create_playlist: playlist insert failed");
        return respond_error(user.format, ERR_GENERIC, "could not create playlist");
    }

    if let Some(song_ids) = &q.song_ids {
        for (pos, sid) in song_ids.iter().enumerate() {
            if let Some(track_bytes) = uuid_bytes(sid)
                && let Err(e) = sqlx::query(
                    "INSERT OR IGNORE INTO subsonic_playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
                )
                .bind(&playlist_id)
                .bind(track_bytes)
                .bind(pos as i64) // INVARIANT: pos is a Vec enumerate index, bounded by collection size; i64 overflow impossible
                .execute(&mut *tx)
                .await
            {
                tracing::warn!(error = %e, "create_playlist: track insert failed");
                return respond_error(user.format, ERR_GENERIC, "could not create playlist");
            }
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::warn!(error = %e, "create_playlist: commit failed");
        return respond_error(user.format, ERR_GENERIC, "could not create playlist");
    }

    let pl_id_str = uuid_str(&playlist_id);
    let xml = format!(
        r#"<playlist id="{}" name="{}" owner="{}" public="false" songCount="0" duration="0" />"#,
        super::types::xml_escape(&pl_id_str),
        super::types::xml_escape(&name),
        super::types::xml_escape(&user.username),
    );
    let json_val = json!({
        "id": pl_id_str,
        "name": name,
        "owner": user.username,
        "public": false,
        "songCount": 0,
        "duration": 0
    });
    respond_ok(user.format, &xml, Some(("playlist", json_val)))
}

// ---------------------------------------------------------------------------
// updatePlaylist
// ---------------------------------------------------------------------------

pub async fn update_playlist(
    State(state): State<AppState>,
    Query(q): Query<UpdatePlaylistQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let id = match &q.playlist_id {
        Some(id) => id.clone(),
        None => {
            return respond_error(
                user.format,
                ERR_MISSING_PARAM,
                "required parameter is missing: playlistId",
            );
        }
    };
    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };
    let user_id_bytes = user.user_id.as_bytes().to_vec();

    // WHY: single transaction — partial metadata/track updates must not
    // survive a mid-flight failure, and failures must surface to the client.
    let mut tx = match state.db.write.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!(error = %e, "update_playlist: begin transaction failed");
            return respond_error(user.format, ERR_GENERIC, "could not update playlist");
        }
    };

    // WHY: the ownership check runs inside the write transaction and every
    // mutation repeats the owner predicate — a concurrent delete/transfer
    // between check and write cannot slip a mutation onto a playlist the
    // caller no longer owns (the old pre-transaction SELECT was a TOCTOU).
    let owned: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM subsonic_playlists WHERE id = ? AND owner_id = ?")
            .bind(&id_bytes)
            .bind(&user_id_bytes)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None);

    if owned.is_none() {
        return respond_error(user.format, ERR_NOT_FOUND, "not found");
    }

    if let Some(name) = &q.name
        && let Err(e) = sqlx::query(
            "UPDATE subsonic_playlists SET name = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ? AND owner_id = ?",
        )
        .bind(name)
        .bind(&id_bytes)
        .bind(&user_id_bytes)
        .execute(&mut *tx)
        .await
    {
        tracing::warn!(error = %e, "update_playlist: name update failed");
        return respond_error(user.format, ERR_GENERIC, "could not update playlist");
    }

    if let Some(comment) = &q.comment
        && let Err(e) = sqlx::query(
            "UPDATE subsonic_playlists SET comment = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ? AND owner_id = ?",
        )
        .bind(comment)
        .bind(&id_bytes)
        .bind(&user_id_bytes)
        .execute(&mut *tx)
        .await
    {
        tracing::warn!(error = %e, "update_playlist: comment update failed");
        return respond_error(user.format, ERR_GENERIC, "could not update playlist");
    }

    if let Some(public) = q.public
        && let Err(e) = sqlx::query(
            "UPDATE subsonic_playlists SET public = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ? AND owner_id = ?",
        )
        .bind(if public { 1i64 } else { 0i64 })
        .bind(&id_bytes)
        .bind(&user_id_bytes)
        .execute(&mut *tx)
        .await
    {
        tracing::warn!(error = %e, "update_playlist: public update failed");
        return respond_error(user.format, ERR_GENERIC, "could not update playlist");
    }

    // Append songs
    if let Some(song_ids) = &q.song_ids_to_add {
        // Get current max position
        let max_pos: i64 = match sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), -1) FROM subsonic_playlist_tracks WHERE playlist_id = ?",
        )
        .bind(&id_bytes)
        .fetch_one(&mut *tx)
        .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "update_playlist: max position query failed");
                return respond_error(user.format, ERR_GENERIC, "could not update playlist");
            }
        };

        for (i, sid) in song_ids.iter().enumerate() {
            if let Some(track_bytes) = uuid_bytes(sid)
                && let Err(e) = sqlx::query(
                    "INSERT OR IGNORE INTO subsonic_playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
                )
                .bind(&id_bytes)
                .bind(track_bytes)
                .bind(max_pos + 1 + i as i64) // INVARIANT: i is a Vec enumerate index; i64 overflow impossible
                .execute(&mut *tx)
                .await
            {
                tracing::warn!(error = %e, "update_playlist: track append failed");
                return respond_error(user.format, ERR_GENERIC, "could not update playlist");
            }
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::warn!(error = %e, "update_playlist: commit failed");
        return respond_error(user.format, ERR_GENERIC, "could not update playlist");
    }

    respond_ok(user.format, "", None)
}

// ---------------------------------------------------------------------------
// deletePlaylist
// ---------------------------------------------------------------------------

pub async fn delete_playlist(
    State(state): State<AppState>,
    Query(q): Query<DeletePlaylistQuery>,
) -> Response {
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
    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };
    let user_id_bytes = user.user_id.as_bytes().to_vec();

    let result = match sqlx::query("DELETE FROM subsonic_playlists WHERE id = ? AND owner_id = ?")
        .bind(&id_bytes)
        .bind(user_id_bytes)
        .execute(&state.db.write)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!(error = %e, "delete_playlist: delete failed");
            return respond_error(user.format, ERR_GENERIC, "could not delete playlist");
        }
    };

    // WHY: zero rows means the playlist does not exist or belongs to another
    // user — a success response would mask the failed ownership check.
    if result.rows_affected() == 0 {
        return respond_error(user.format, ERR_NOT_FOUND, "not found");
    }

    respond_ok(user.format, "", None)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_playlist_list(playlists: &[PlaylistRow]) -> (String, Vec<Value>) {
    let mut xml = String::new();
    let mut json: Vec<Value> = Vec::new();
    for p in playlists {
        let id = uuid_str(&p.id);
        xml.push_str(&format!(
            r#"<playlist id="{}" name="{}" owner="{}" public="{}" songCount="{}" duration="{}" created="{}" changed="{}" />"#,
            super::types::xml_escape(&id),
            super::types::xml_escape(&p.name),
            super::types::xml_escape(&p.owner),
            if p.public != 0 { "true" } else { "false" },
            p.song_count,
            p.duration,
            p.created_at,
            p.updated_at,
        ));
        json.push(json!({
            "id": id,
            "name": p.name,
            "owner": p.owner,
            "public": p.public != 0,
            "songCount": p.song_count,
            "duration": p.duration,
            "created": p.created_at,
            "changed": p.updated_at
        }));
    }
    (xml, json)
}

fn build_songs(songs: &[SongRow]) -> (String, Vec<Value>) {
    let mut xml = String::new();
    let mut json: Vec<Value> = Vec::new();
    for s in songs {
        let id = uuid_str(&s.id);
        let album_id = uuid_str(&s.album_id);
        let artist_id = s.artist_id.as_deref().map(uuid_str).unwrap_or_default(); // WHY: Option<Vec<u8>> chain — as_deref produces Option, not Err
        let artist_name = s.artist_name.as_deref().unwrap_or("");
        let duration_secs = s.duration_ms.map(|d| d / 1000);
        let ct = codec_content_type(s.codec.as_deref());
        let sfx = codec_suffix(s.codec.as_deref());
        xml.push_str(&song_xml_elem(
            &id,
            &s.title,
            &s.album_title,
            &album_id,
            artist_name,
            &artist_id,
            Some(s.position),
            s.year,
            duration_secs,
            None,
            ct,
            sfx,
            false,
        ));
        json.push(song_json(
            &id,
            &s.title,
            &s.album_title,
            &album_id,
            artist_name,
            &artist_id,
            Some(s.position),
            s.year,
            duration_secs,
            None,
            ct,
            sfx,
        ));
    }
    (xml, json)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;
    use crate::subsonic::test_helpers::subsonic_app;
    #[tokio::test]
    async fn playlist_crud() {
        let (app, _state, key) = subsonic_app().await;

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/createPlaylist.view?apiKey={key}&name=MyList"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("MyList"));

        // Get playlists
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getPlaylists.view?apiKey={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("MyList"));

        // Extract playlist id
        let id_start = body.find("id=\"").unwrap() + 4;
        let id_end = body[id_start..].find('"').unwrap() + id_start;
        let pl_id = &body[id_start..id_end];

        // Get single playlist
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getPlaylist.view?apiKey={key}&id={pl_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));

        // Delete
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/deletePlaylist.view?apiKey={key}&id={pl_id}"))
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
    async fn delete_playlist_not_owned_returns_not_found() {
        let (app, state, key) = subsonic_app().await;

        // Seed a second user owning a playlist the caller must not delete
        let other_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO users (id, username, display_name, password_hash, role, is_active, created_at)
             VALUES (?, 'other', 'Other', 'x', 'member', 1, '2026-01-01T00:00:00Z')",
        )
        .bind(&other_id)
        .execute(&state.db.write)
        .await
        .unwrap();
        let other_pl = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO subsonic_playlists (id, owner_id, name) VALUES (?, ?, 'Theirs')")
            .bind(other_pl.as_bytes().to_vec())
            .bind(&other_id)
            .execute(&state.db.write)
            .await
            .unwrap();

        for id in [other_pl.to_string(), uuid::Uuid::now_v7().to_string()] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/rest/deletePlaylist.view?apiKey={key}&id={id}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
            let body = std::str::from_utf8(&bytes).unwrap();
            assert!(
                body.contains("status=\"failed\""),
                "expected failed status for {id}, got: {body}"
            );
            assert!(
                body.contains(r#"code="70""#),
                "expected not-found code for {id}, got: {body}"
            );
        }

        // The other user's playlist survives
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subsonic_playlists WHERE id = ?")
            .bind(other_pl.as_bytes().to_vec())
            .fetch_one(&state.db.read)
            .await
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[tokio::test]
    async fn update_playlist_not_owned_returns_not_found() {
        let (app, state, key) = subsonic_app().await;

        let other_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO users (id, username, display_name, password_hash, role, is_active, created_at)
             VALUES (?, 'other2', 'Other2', 'x', 'member', 1, '2026-01-01T00:00:00Z')",
        )
        .bind(&other_id)
        .execute(&state.db.write)
        .await
        .unwrap();
        let other_pl = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO subsonic_playlists (id, owner_id, name) VALUES (?, ?, 'Theirs')")
            .bind(other_pl.as_bytes().to_vec())
            .bind(&other_id)
            .execute(&state.db.write)
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/updatePlaylist.view?apiKey={key}&playlistId={other_pl}&name=Stolen"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(
            body.contains(r#"code="70""#),
            "expected not-found code, got: {body}"
        );

        let name: String = sqlx::query_scalar("SELECT name FROM subsonic_playlists WHERE id = ?")
            .bind(other_pl.as_bytes().to_vec())
            .fetch_one(&state.db.read)
            .await
            .unwrap();
        assert_eq!(name, "Theirs", "non-owner update must not apply");
    }

    #[tokio::test]
    async fn create_playlist_insert_failure_returns_error() {
        let (app, state, key) = subsonic_app().await;

        // Force the playlist INSERT to fail deterministically
        sqlx::query(
            "CREATE TRIGGER force_pl_insert_fail BEFORE INSERT ON subsonic_playlists \
             BEGIN SELECT RAISE(ABORT, 'forced test failure'); END",
        )
        .execute(&state.db.write)
        .await
        .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/createPlaylist.view?apiKey={key}&name=Doomed"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(
            body.contains("status=\"failed\""),
            "expected failed status, got: {body}"
        );
        assert!(!body.contains("status=\"ok\""));
        assert!(body.contains(r#"code="0""#), "expected ERR_GENERIC code");
    }

    #[tokio::test]
    async fn create_playlist_failure_leaves_no_orphan_tracks() {
        // WHY: the issue's core bug is "failed playlist insert still inserts
        // tracks and returns ok". This asserts no track rows leak after a
        // failed create — the swallowed-write path is closed.
        let (app, state, key) = subsonic_app().await;

        sqlx::query(
            "CREATE TRIGGER force_pl_insert_fail BEFORE INSERT ON subsonic_playlists \
             BEGIN SELECT RAISE(ABORT, 'forced test failure'); END",
        )
        .execute(&state.db.write)
        .await
        .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/createPlaylist.view?apiKey={key}&name=Orphaned"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(
            body.contains("status=\"failed\""),
            "expected failed status, got: {body}"
        );

        let track_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subsonic_playlist_tracks")
            .fetch_one(&state.db.read)
            .await
            .unwrap();
        assert_eq!(
            track_rows, 0,
            "no track rows may be written on a failed create"
        );
    }

    #[tokio::test]
    async fn update_playlist_rolls_back_earlier_writes_on_failure() {
        // WHY: proves the transaction property directly — an earlier UPDATE
        // that succeeds inside the tx must be undone when a later statement in
        // the same tx fails. A trigger aborts the `public=1` UPDATE, so the
        // preceding `name` UPDATE must roll back and the response must be
        // failed (not a partial-write ok).
        let (app, state, key) = subsonic_app().await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/createPlaylist.view?apiKey={key}&name=Original"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let id_start = body.find("id=\"").unwrap() + 4;
        let id_end = body[id_start..].find('"').unwrap() + id_start;
        let pl_id = body[id_start..id_end].to_string();

        // Abort any UPDATE that sets public = 1 — a deterministic, reachable
        // mid-transaction failure after the name UPDATE has already applied.
        sqlx::query(
            "CREATE TRIGGER fail_on_public BEFORE UPDATE ON subsonic_playlists
             WHEN NEW.public = 1
             BEGIN SELECT RAISE(ABORT, 'boom'); END",
        )
        .execute(&state.db.write)
        .await
        .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/updatePlaylist.view?apiKey={key}&playlistId={pl_id}&name=Renamed&public=true"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(
            body.contains("status=\"failed\""),
            "expected failed status, got: {body}"
        );

        // The name UPDATE that ran earlier in the tx must have been rolled back.
        let id_bytes = uuid_bytes(&pl_id).unwrap();
        let name: String = sqlx::query_scalar("SELECT name FROM subsonic_playlists WHERE id = ?")
            .bind(&id_bytes)
            .fetch_one(&state.db.read)
            .await
            .unwrap();
        assert_eq!(
            name, "Original",
            "earlier name UPDATE must roll back when a later statement fails"
        );
    }
}
