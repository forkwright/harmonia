use axum::{
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    auth::authenticate,
    types::{
        ERR_NOT_FOUND, SubsonicCommon, album_json, album_xml_elem, artist_xml, codec_content_type,
        codec_suffix, index_letter, respond_error, respond_ok, song_json, song_xml_elem,
        uuid_bytes, uuid_str,
    },
};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// DB row types
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ArtistRow {
    id: Vec<u8>,
    name: String,
    album_count: i64,
}

#[derive(sqlx::FromRow)]
struct AlbumRow {
    id: Vec<u8>,
    name: String,
    year: Option<i64>,
    artist_id: Option<Vec<u8>>,
    artist_name: Option<String>,
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
// Query params
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
pub struct FolderQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    #[serde(rename = "musicFolderId")]
    pub music_folder_id: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct IdQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
}

// ---------------------------------------------------------------------------
// getMusicFolders
// ---------------------------------------------------------------------------

pub async fn get_music_folders(
    State(state): State<AppState>,
    Query(q): Query<FolderQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let xml = r#"<musicFolders><musicFolder id="1" name="Music" /></musicFolders>"#;
    let json_val = json!({
        "musicFolder": [{ "id": 1, "name": "Music" }]
    });
    respond_ok(user.format, xml, Some(("musicFolders", json_val)))
}

// ---------------------------------------------------------------------------
// getIndexes  -  same data as getArtists but different wrapper element
// ---------------------------------------------------------------------------

pub async fn get_indexes(State(state): State<AppState>, Query(q): Query<FolderQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    match fetch_artist_index(&state).await {
        Ok((xml_inner, json_val)) => respond_ok(
            user.format,
            &format!(
                r#"<indexes lastModified="0" ignoredArticles="The El La Los Las Le Les">{xml_inner}</indexes>"#
            ),
            Some((
                "indexes",
                json!({
                    "lastModified": 0,
                    "ignoredArticles": "The El La Los Las Le Les",
                    "index": json_val
                }),
            )),
        ),
        Err(_) => respond_error(user.format, 0, "database error"),
    }
}

// ---------------------------------------------------------------------------
// getArtists
// ---------------------------------------------------------------------------

pub async fn get_artists(State(state): State<AppState>, Query(q): Query<FolderQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    match fetch_artist_index(&state).await {
        Ok((xml_inner, json_val)) => respond_ok(
            user.format,
            &format!(
                r#"<artists lastModified="0" ignoredArticles="The El La Los Las Le Les">{xml_inner}</artists>"#
            ),
            Some((
                "artists",
                json!({
                    "lastModified": 0,
                    "ignoredArticles": "The El La Los Las Le Les",
                    "index": json_val
                }),
            )),
        ),
        Err(_) => respond_error(user.format, 0, "database error"),
    }
}

async fn fetch_artist_index(state: &AppState) -> Result<(String, Value), sqlx::Error> {
    let rows = sqlx::query_as::<_, ArtistRow>(
        "SELECT mr.id, mr.display_name as name,
                COUNT(DISTINCT mrga.release_group_id) as album_count
         FROM media_registry mr
         LEFT JOIN music_release_group_artists mrga ON mrga.artist_id = mr.id AND mrga.role = 'primary'
         WHERE mr.entity_type = 'person'
         GROUP BY mr.id
         ORDER BY UPPER(mr.display_name)",
    )
    .fetch_all(&state.db.read)
    .await?;

    // Group by first letter
    let mut groups: std::collections::BTreeMap<String, Vec<&ArtistRow>> =
        std::collections::BTreeMap::new();
    for row in &rows {
        let letter = index_letter(&row.name);
        groups.entry(letter).or_default().push(row);
    }

    let mut xml_parts = String::new();
    let mut json_indexes: Vec<Value> = Vec::new();

    for (letter, artists) in &groups {
        let mut xml_artists = String::new();
        let mut json_artists: Vec<Value> = Vec::new();

        for artist in artists {
            let id = uuid_str(&artist.id);
            xml_artists.push_str(&artist_xml(&id, &artist.name, artist.album_count));
            json_artists.push(json!({
                "id": id,
                "name": artist.name,
                "albumCount": artist.album_count
            }));
        }

        xml_parts.push_str(&format!(r#"<index name="{letter}">{xml_artists}</index>"#));
        json_indexes.push(json!({ "name": letter, "artist": json_artists }));
    }

    Ok((xml_parts, Value::Array(json_indexes)))
}

// ---------------------------------------------------------------------------
// getMusicDirectory
// ---------------------------------------------------------------------------

pub async fn get_music_directory(
    State(state): State<AppState>,
    Query(q): Query<IdQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let id = match &q.id {
        Some(id) => id.clone(),
        None => return respond_error(user.format, 10, "required parameter is missing: id"),
    };

    // Music folder virtual root
    if id == "1" {
        return match fetch_artist_index(&state).await {
            Ok((xml_artists_inner, _)) => {
                // Flatten all artists as directory children
                let children =
                    format!("<directory id=\"1\" name=\"Music\">{xml_artists_inner}</directory>");
                respond_ok(
                    user.format,
                    &children,
                    Some(("directory", json!({ "id": "1", "name": "Music" }))),
                )
            }
            Err(_) => respond_error(user.format, 0, "database error"),
        };
    }

    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    // Try as artist → list albums
    let albums = sqlx::query_as::<_, AlbumRow>(
        "SELECT mrg.id, mrg.title as name, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM music_release_groups mrg
         JOIN music_release_group_artists mrga ON mrga.release_group_id = mrg.id AND mrga.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mrga.artist_id
         WHERE mrga.artist_id = ?
         ORDER BY mrg.year, mrg.title",
    )
    .bind(&id_bytes)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_default();

    if !albums.is_empty() {
        let mut xml_children = String::new();
        for a in &albums {
            let album_id = uuid_str(&a.id);
            let artist_id = a.artist_id.as_deref().map(uuid_str).unwrap_or_default();
            let artist_name = a.artist_name.as_deref().unwrap_or("");
            xml_children.push_str(&format!(
                r#"<child id="{}" parent="{id}" isDir="true" title="{}" artist="{}" artistId="{}" />"#,
                super::types::xml_escape(&album_id),
                super::types::xml_escape(&a.name),
                super::types::xml_escape(artist_name),
                super::types::xml_escape(&artist_id),
            ));
        }
        let xml = format!(r#"<directory id="{id}" name="">{xml_children}</directory>"#);
        return respond_ok(user.format, &xml, Some(("directory", json!({ "id": id }))));
    }

    // Try as album → list tracks
    match fetch_songs_for_album(&state, &id_bytes).await {
        Ok(songs) if !songs.is_empty() => {
            let album_name = songs
                .first()
                .map(|s| s.album_title.clone())
                .unwrap_or_default();
            let mut xml_songs = String::new();
            let mut json_songs: Vec<Value> = Vec::new();
            for s in &songs {
                let (xml_s, json_s) = song_tuple(s, &uuid_str(&s.album_id));
                xml_songs.push_str(&xml_s);
                json_songs.push(json_s);
            }
            let xml = format!(
                r#"<directory id="{id}" name="{}">{xml_songs}</directory>"#,
                super::types::xml_escape(&album_name)
            );
            respond_ok(user.format, &xml, Some(("directory", json!({ "id": id }))))
        }
        _ => respond_error(user.format, ERR_NOT_FOUND, "not found"),
    }
}

// ---------------------------------------------------------------------------
// getArtist
// ---------------------------------------------------------------------------

pub async fn get_artist(State(state): State<AppState>, Query(q): Query<IdQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let id = match &q.id {
        Some(id) => id.clone(),
        None => return respond_error(user.format, 10, "required parameter is missing: id"),
    };
    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    #[derive(sqlx::FromRow)]
    struct ArtistInfo {
        name: String,
    }
    let artist = match sqlx::query_as::<_, ArtistInfo>(
        "SELECT display_name as name FROM media_registry WHERE id = ? AND entity_type = 'person'",
    )
    .bind(&id_bytes)
    .fetch_optional(&state.db.read)
    .await
    {
        Ok(Some(a)) => a,
        _ => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    let albums = sqlx::query_as::<_, AlbumRow>(
        "SELECT mrg.id, mrg.title as name, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM music_release_groups mrg
         JOIN music_release_group_artists mrga ON mrga.release_group_id = mrg.id AND mrga.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mrga.artist_id
         WHERE mrga.artist_id = ?
         ORDER BY mrg.year, mrg.title",
    )
    .bind(&id_bytes)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_default();

    let album_count = albums.len() as i64;
    let mut xml_albums = String::new();
    let mut json_albums: Vec<Value> = Vec::new();

    for a in &albums {
        let album_id = uuid_str(&a.id);
        let (song_count, duration) = fetch_album_stats(&state, &a.id).await;
        let a_artist_id = a.artist_id.as_deref().map(uuid_str).unwrap_or_default();
        let a_artist = a.artist_name.as_deref().unwrap_or("");
        xml_albums.push_str(&album_xml_elem(
            &album_id,
            &a.name,
            a_artist,
            &a_artist_id,
            a.year,
            song_count,
            duration,
        ));
        json_albums.push(album_json(
            &album_id,
            &a.name,
            a_artist,
            &a_artist_id,
            a.year,
            song_count,
            duration,
        ));
    }

    let xml = format!(
        r#"<artist id="{id}" name="{}" albumCount="{album_count}">{xml_albums}</artist>"#,
        super::types::xml_escape(&artist.name)
    );
    let json_val = json!({
        "id": id,
        "name": artist.name,
        "albumCount": album_count,
        "album": json_albums
    });
    respond_ok(user.format, &xml, Some(("artist", json_val)))
}

// ---------------------------------------------------------------------------
// getAlbum
// ---------------------------------------------------------------------------

pub async fn get_album(State(state): State<AppState>, Query(q): Query<IdQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let id = match &q.id {
        Some(id) => id.clone(),
        None => return respond_error(user.format, 10, "required parameter is missing: id"),
    };
    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    #[derive(sqlx::FromRow)]
    struct AlbumInfo {
        title: String,
        year: Option<i64>,
    }
    let album = match sqlx::query_as::<_, AlbumInfo>(
        "SELECT title, year FROM music_release_groups WHERE id = ?",
    )
    .bind(&id_bytes)
    .fetch_optional(&state.db.read)
    .await
    {
        Ok(Some(a)) => a,
        _ => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    let artist_row = sqlx::query_as::<_, ArtistRow>(
        "SELECT mr.id, mr.display_name as name, 0 as album_count
         FROM media_registry mr
         JOIN music_release_group_artists mrga ON mrga.artist_id = mr.id
         WHERE mrga.release_group_id = ? AND mrga.role = 'primary'
         LIMIT 1",
    )
    .bind(&id_bytes)
    .fetch_optional(&state.db.read)
    .await
    .unwrap_or(None);

    let artist_id = artist_row
        .as_ref()
        .map(|r| uuid_str(&r.id))
        .unwrap_or_default();
    let artist_name = artist_row.as_ref().map(|r| r.name.as_str()).unwrap_or("");

    let songs = fetch_songs_for_album(&state, &id_bytes)
        .await
        .unwrap_or_default();

    let song_count = songs.len() as i64;
    let duration: i64 = songs
        .iter()
        .map(|s| s.duration_ms.unwrap_or(0) / 1000)
        .sum();

    let mut xml_songs = String::new();
    let mut json_songs: Vec<Value> = Vec::new();
    for s in &songs {
        let (xml_s, json_s) = song_tuple(s, &id);
        xml_songs.push_str(&xml_s);
        json_songs.push(json_s);
    }

    let year_attr = album
        .year
        .map(|y| format!(r#" year="{y}""#))
        .unwrap_or_default();
    let xml = format!(
        r#"<album id="{id}" name="{}" artist="{}" artistId="{}" songCount="{song_count}" duration="{duration}"{year_attr}>{xml_songs}</album>"#,
        super::types::xml_escape(&album.title),
        super::types::xml_escape(artist_name),
        super::types::xml_escape(&artist_id),
    );
    let json_val = json!({
        "id": id,
        "name": album.title,
        "artist": artist_name,
        "artistId": artist_id,
        "year": album.year,
        "songCount": song_count,
        "duration": duration,
        "song": json_songs
    });
    respond_ok(user.format, &xml, Some(("album", json_val)))
}

// ---------------------------------------------------------------------------
// getSong
// ---------------------------------------------------------------------------

pub async fn get_song(State(state): State<AppState>, Query(q): Query<IdQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let id = match &q.id {
        Some(id) => id.clone(),
        None => return respond_error(user.format, 10, "required parameter is missing: id"),
    };
    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    let song = match sqlx::query_as::<_, SongRow>(
        "SELECT t.id, t.title, t.position, t.duration_ms, t.codec,
                mrg.id as album_id, mrg.title as album_title, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM music_tracks t
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases r ON r.id = mm.release_id
         JOIN music_release_groups mrg ON mrg.id = r.release_group_id
         LEFT JOIN music_track_artists mta ON mta.track_id = t.id AND mta.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mta.artist_id
         WHERE t.id = ?",
    )
    .bind(&id_bytes)
    .fetch_optional(&state.db.read)
    .await
    {
        Ok(Some(s)) => s,
        _ => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    let album_id = uuid_str(&song.album_id);
    let (xml_s, json_s) = song_tuple(&song, &album_id);
    respond_ok(user.format, &xml_s, Some(("song", json_s)))
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

async fn fetch_songs_for_album(
    state: &AppState,
    group_id: &[u8],
) -> Result<Vec<SongRow>, sqlx::Error> {
    sqlx::query_as::<_, SongRow>(
        "SELECT t.id, t.title, t.position, t.duration_ms, t.codec,
                mrg.id as album_id, mrg.title as album_title, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM music_tracks t
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases r ON r.id = mm.release_id
         JOIN music_release_groups mrg ON mrg.id = r.release_group_id
         LEFT JOIN music_track_artists mta ON mta.track_id = t.id AND mta.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mta.artist_id
         WHERE r.release_group_id = ?
         ORDER BY r.release_date, mm.position, t.position",
    )
    .bind(group_id)
    .fetch_all(&state.db.read)
    .await
}

async fn fetch_album_stats(state: &AppState, group_id: &[u8]) -> (i64, i64) {
    #[derive(sqlx::FromRow)]
    struct Stats {
        count: i64,
        total_ms: i64,
    }
    let stats = sqlx::query_as::<_, Stats>(
        "SELECT COUNT(*) as count, COALESCE(SUM(t.duration_ms), 0) as total_ms
         FROM music_tracks t
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases r ON r.id = mm.release_id
         WHERE r.release_group_id = ?",
    )
    .bind(group_id)
    .fetch_one(&state.db.read)
    .await
    .unwrap_or(Stats {
        count: 0,
        total_ms: 0,
    });
    (stats.count, stats.total_ms / 1000)
}

fn song_tuple(s: &SongRow, album_id: &str) -> (String, Value) {
    let id = uuid_str(&s.id);
    let artist_id = s.artist_id.as_deref().map(uuid_str).unwrap_or_default();
    let artist_name = s.artist_name.as_deref().unwrap_or("");
    let duration_secs = s.duration_ms.map(|d| d / 1000);
    let ct = codec_content_type(s.codec.as_deref());
    let suffix = codec_suffix(s.codec.as_deref());

    let xml = song_xml_elem(
        &id,
        &s.title,
        &s.album_title,
        album_id,
        artist_name,
        &artist_id,
        Some(s.position),
        s.year,
        duration_secs,
        None,
        ct,
        suffix,
        false,
    );
    let json = song_json(
        &id,
        &s.title,
        &s.album_title,
        album_id,
        artist_name,
        &artist_id,
        Some(s.position),
        s.year,
        duration_secs,
        None,
        ct,
        suffix,
    );
    (xml, json)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsonic::test_helpers::{seed_music_data, subsonic_app};
    use axum::{body::Body, body::to_bytes, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn get_music_folders_returns_one_folder() {
        let (app, _state, key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getMusicFolders.view?apiKey={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("musicFolder"));
        assert!(body.contains("id=\"1\""));
    }

    #[tokio::test]
    async fn get_artists_empty_library() {
        let (app, _state, key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getArtists.view?apiKey={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("artists"));
    }

    #[tokio::test]
    async fn get_artists_alphabetical_index() {
        let (app, state, key) = subsonic_app().await;
        seed_music_data(&state).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getArtists.view?apiKey={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        // Should have the seeded artist
        assert!(body.contains("Test Artist"));
    }

    #[tokio::test]
    async fn get_album_returns_correct_song_children() {
        let (app, state, key) = subsonic_app().await;
        let (group_id, _) = seed_music_data(&state).await;
        let id = uuid_str(&group_id);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getAlbum.view?apiKey={key}&id={id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("<song"));
        assert!(body.contains("Test Track"));
    }

    #[tokio::test]
    async fn get_song_returns_single_song() {
        let (app, state, key) = subsonic_app().await;
        let (_, track_id) = seed_music_data(&state).await;
        let id = uuid_str(&track_id);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getSong.view?apiKey={key}&id={id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("Test Track"));
    }

    #[tokio::test]
    async fn get_album_not_found_returns_error_70() {
        let (app, _state, key) = subsonic_app().await;
        let fake_id = uuid::Uuid::now_v7();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getAlbum.view?apiKey={key}&id={fake_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"failed\""));
        assert!(body.contains("code=\"70\""));
    }

    #[tokio::test]
    async fn folder_simulation_artists_appear_as_directories() {
        let (app, state, key) = subsonic_app().await;
        seed_music_data(&state).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getMusicDirectory.view?apiKey={key}&id=1"))
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
