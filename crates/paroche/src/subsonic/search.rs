use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};

use super::auth::authenticate;
use super::types::{
    SubsonicCommon, codec_content_type, codec_suffix, respond_ok, song_json, song_xml_elem,
    uuid_str,
};
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct Search3Query {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub query: Option<String>,
    #[serde(rename = "artistCount")]
    pub artist_count: Option<i64>,
    #[serde(rename = "artistOffset")]
    pub artist_offset: Option<i64>,
    #[serde(rename = "albumCount")]
    pub album_count: Option<i64>,
    #[serde(rename = "albumOffset")]
    pub album_offset: Option<i64>,
    #[serde(rename = "songCount")]
    pub song_count: Option<i64>,
    #[serde(rename = "songOffset")]
    pub song_offset: Option<i64>,
    #[serde(rename = "musicFolderId")]
    pub music_folder_id: Option<String>,
}

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
    artist_name: Option<String>,
    artist_id: Option<Vec<u8>>,
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

pub async fn search3(State(state): State<AppState>, Query(q): Query<Search3Query>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let query = q.query.as_deref().unwrap_or("").trim().to_string();
    if query.is_empty() {
        // Empty query — return empty results
        return respond_ok(
            user.format,
            r#"<searchResult3 />"#,
            Some((
                "searchResult3",
                json!({ "artist": [], "album": [], "song": [] }),
            )),
        );
    }

    let pattern = format!("%{query}%");
    let artist_limit = q.artist_count.unwrap_or(20).min(500);
    let artist_offset = q.artist_offset.unwrap_or(0);
    let album_limit = q.album_count.unwrap_or(20).min(500);
    let album_offset = q.album_offset.unwrap_or(0);
    let song_limit = q.song_count.unwrap_or(20).min(500);
    let song_offset = q.song_offset.unwrap_or(0);

    let artists = sqlx::query_as::<_, ArtistRow>(
        "SELECT mr.id, mr.display_name as name,
                COUNT(DISTINCT mrga.release_group_id) as album_count
         FROM media_registry mr
         LEFT JOIN music_release_group_artists mrga ON mrga.artist_id = mr.id AND mrga.role = 'primary'
         WHERE mr.entity_type = 'person' AND mr.display_name LIKE ?
         GROUP BY mr.id
         ORDER BY UPPER(mr.display_name)
         LIMIT ? OFFSET ?",
    )
    .bind(&pattern)
    .bind(artist_limit)
    .bind(artist_offset)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_default();

    let albums = sqlx::query_as::<_, AlbumRow>(
        "SELECT mrg.id, mrg.title as name, mrg.year,
                mr.display_name as artist_name, mr.id as artist_id
         FROM music_release_groups mrg
         LEFT JOIN music_release_group_artists mrga ON mrga.release_group_id = mrg.id AND mrga.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mrga.artist_id
         WHERE mrg.title LIKE ?
         ORDER BY UPPER(mrg.title)
         LIMIT ? OFFSET ?",
    )
    .bind(&pattern)
    .bind(album_limit)
    .bind(album_offset)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_default();

    let songs = sqlx::query_as::<_, SongRow>(
        "SELECT t.id, t.title, t.position, t.duration_ms, t.codec,
                mrg.id as album_id, mrg.title as album_title, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM music_tracks t
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases r ON r.id = mm.release_id
         JOIN music_release_groups mrg ON mrg.id = r.release_group_id
         LEFT JOIN music_track_artists mta ON mta.track_id = t.id AND mta.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mta.artist_id
         WHERE t.title LIKE ?
         ORDER BY UPPER(t.title)
         LIMIT ? OFFSET ?",
    )
    .bind(&pattern)
    .bind(song_limit)
    .bind(song_offset)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_default();

    // Build XML
    let mut xml_artists = String::new();
    let mut json_artists: Vec<Value> = Vec::new();
    for a in &artists {
        let id = uuid_str(&a.id);
        xml_artists.push_str(&format!(
            r#"<artist id="{}" name="{}" albumCount="{}" />"#,
            super::types::xml_escape(&id),
            super::types::xml_escape(&a.name),
            a.album_count
        ));
        json_artists.push(json!({ "id": id, "name": a.name, "albumCount": a.album_count }));
    }

    let mut xml_albums = String::new();
    let mut json_albums: Vec<Value> = Vec::new();
    for a in &albums {
        let id = uuid_str(&a.id);
        let artist_id = a.artist_id.as_deref().map(uuid_str).unwrap_or_default();
        let artist_name = a.artist_name.as_deref().unwrap_or("");
        let year_attr = a
            .year
            .map(|y| format!(r#" year="{y}""#))
            .unwrap_or_default();
        xml_albums.push_str(&format!(
            r#"<album id="{}" name="{}" artist="{}" artistId="{}"{year_attr} />"#,
            super::types::xml_escape(&id),
            super::types::xml_escape(&a.name),
            super::types::xml_escape(artist_name),
            super::types::xml_escape(&artist_id),
        ));
        json_albums.push(json!({
            "id": id, "name": a.name,
            "artist": artist_name, "artistId": artist_id,
            "year": a.year
        }));
    }

    let mut xml_songs = String::new();
    let mut json_songs: Vec<Value> = Vec::new();
    for s in &songs {
        let id = uuid_str(&s.id);
        let album_id = uuid_str(&s.album_id);
        let artist_id = s.artist_id.as_deref().map(uuid_str).unwrap_or_default();
        let artist_name = s.artist_name.as_deref().unwrap_or("");
        let duration_secs = s.duration_ms.map(|d| d / 1000);
        let ct = codec_content_type(s.codec.as_deref());
        let sfx = codec_suffix(s.codec.as_deref());
        xml_songs.push_str(&song_xml_elem(
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
        json_songs.push(song_json(
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

    let xml = format!("<searchResult3>{xml_artists}{xml_albums}{xml_songs}</searchResult3>");
    let json_val = json!({
        "artist": json_artists,
        "album": json_albums,
        "song": json_songs
    });
    respond_ok(user.format, &xml, Some(("searchResult3", json_val)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    use crate::subsonic::test_helpers::{seed_music_data, subsonic_app};

    #[tokio::test]
    async fn search3_finds_artists_albums_songs() {
        let (app, state, key) = subsonic_app().await;
        seed_music_data(&state).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/search3.view?apiKey={key}&query=Test"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("searchResult3"));
        // Should find our seeded "Test Artist", "Test Album", "Test Track"
        assert!(body.contains("Test"));
    }

    #[tokio::test]
    async fn search3_empty_query_returns_empty_results() {
        let (app, _state, key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/search3.view?apiKey={key}&query="))
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
