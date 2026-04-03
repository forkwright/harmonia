use axum::{
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{
    auth::authenticate,
    types::{
        ERR_MISSING_PARAM, ERR_NOT_FOUND, SubsonicCommon, codec_content_type, codec_suffix,
        respond_error, respond_ok, song_json, song_xml_elem, uuid_bytes, uuid_str,
    },
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
    .unwrap_or_default();

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
    .unwrap_or_default();

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

    let _ = sqlx::query("INSERT INTO subsonic_playlists (id, owner_id, name) VALUES (?, ?, ?)")
        .bind(&playlist_id)
        .bind(&user_id_bytes)
        .bind(&name)
        .execute(&state.db.write)
        .await;

    if let Some(song_ids) = &q.song_ids {
        for (pos, sid) in song_ids.iter().enumerate() {
            if let Some(track_bytes) = uuid_bytes(sid) {
                let _ = sqlx::query(
                    "INSERT OR IGNORE INTO subsonic_playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
                )
                .bind(&playlist_id)
                .bind(track_bytes)
                .bind(i64::try_from(pos).unwrap_or_default())
                .execute(&state.db.write)
                .await;
            }
        }
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

    // Verify ownership
    let owned: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM subsonic_playlists WHERE id = ? AND owner_id = ?")
            .bind(&id_bytes)
            .bind(&user_id_bytes)
            .fetch_optional(&state.db.read)
            .await
            .unwrap_or(None);

    if owned.is_none() {
        return respond_error(user.format, ERR_NOT_FOUND, "not found");
    }

    if let Some(name) = &q.name {
        let _ = sqlx::query(
            "UPDATE subsonic_playlists SET name = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
        )
        .bind(name)
        .bind(&id_bytes)
        .execute(&state.db.write)
        .await;
    }

    if let Some(comment) = &q.comment {
        let _ = sqlx::query(
            "UPDATE subsonic_playlists SET comment = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
        )
        .bind(comment)
        .bind(&id_bytes)
        .execute(&state.db.write)
        .await;
    }

    if let Some(public) = q.public {
        let _ = sqlx::query(
            "UPDATE subsonic_playlists SET public = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
        )
        .bind(if public { 1i64 } else { 0i64 })
        .bind(&id_bytes)
        .execute(&state.db.write)
        .await;
    }

    // Append songs
    if let Some(song_ids) = &q.song_ids_to_add {
        // Get current max position
        let max_pos: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), -1) FROM subsonic_playlist_tracks WHERE playlist_id = ?",
        )
        .bind(&id_bytes)
        .fetch_one(&state.db.read)
        .await
        .unwrap_or(-1);

        for (i, sid) in song_ids.iter().enumerate() {
            if let Some(track_bytes) = uuid_bytes(sid) {
                let _ = sqlx::query(
                    "INSERT OR IGNORE INTO subsonic_playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
                )
                .bind(&id_bytes)
                .bind(track_bytes)
                .bind(max_pos + 1 + i64::try_from(i).unwrap_or_default())
                .execute(&state.db.write)
                .await;
            }
        }
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

    let _ = sqlx::query("DELETE FROM subsonic_playlists WHERE id = ? AND owner_id = ?")
        .bind(&id_bytes)
        .bind(user_id_bytes)
        .execute(&state.db.write)
        .await;

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
        let artist_id = s.artist_id.as_deref().map(uuid_str).unwrap_or_default();
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

    use crate::subsonic::test_helpers::subsonic_app;
    use axum::{body::Body, body::to_bytes, http::Request};
    use tower::ServiceExt;

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
}
