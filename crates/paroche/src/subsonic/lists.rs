use axum::{
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    auth::authenticate,
    types::{
        SubsonicCommon, album_json, album_xml_elem, codec_content_type, codec_suffix, respond_ok,
        song_json, song_xml_elem, uuid_str,
    },
};
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct AlbumListQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    #[serde(rename = "type")]
    pub list_type: Option<String>,
    pub size: Option<i64>,
    pub offset: Option<i64>,
    #[serde(rename = "fromYear")]
    pub from_year: Option<i64>,
    #[serde(rename = "toYear")]
    pub to_year: Option<i64>,
    pub genre: Option<String>,
    #[serde(rename = "musicFolderId")]
    pub music_folder_id: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct RandomSongsQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub size: Option<i64>,
    #[serde(rename = "fromYear")]
    pub from_year: Option<i64>,
    #[serde(rename = "toYear")]
    pub to_year: Option<i64>,
    pub genre: Option<String>,
    #[serde(rename = "musicFolderId")]
    pub music_folder_id: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct CommonQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
}

#[derive(sqlx::FromRow)]
struct AlbumRow {
    id: Vec<u8>,
    name: String,
    year: Option<i64>,
    artist_name: Option<String>,
    artist_id: Option<Vec<u8>>,
    song_count: i64,
    duration: i64,
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
// getAlbumList2
// ---------------------------------------------------------------------------

pub async fn get_album_list2(
    State(state): State<AppState>,
    Query(q): Query<AlbumListQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let list_type = q.list_type.as_deref().unwrap_or("alphabeticalByName");
    let limit = q.size.unwrap_or(10).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);

    let order_clause = match list_type {
        "newest" | "recent" => "ORDER BY mrg.added_at DESC",
        "frequent" => "ORDER BY mrg.title",
        "alphabeticalByName" | "alphabeticalByArtist" => "ORDER BY UPPER(mrg.title)",
        "byYear" => "ORDER BY mrg.year",
        "random" => "ORDER BY RANDOM()",
        "starred" => "ORDER BY mrg.added_at DESC", // stub — real starred needs join
        _ => "ORDER BY UPPER(mrg.title)",
    };

    let mut year_filter = String::new();
    if list_type == "byYear" {
        if let Some(from) = q.from_year {
            year_filter.push_str(&format!(" AND mrg.year >= {from}"));
        }
        if let Some(to) = q.to_year {
            year_filter.push_str(&format!(" AND mrg.year <= {to}"));
        }
    }

    let sql = format!(
        "SELECT mrg.id, mrg.title as name, mrg.year,
                mr.display_name as artist_name, mr.id as artist_id,
                COUNT(DISTINCT t.id) as song_count,
                COALESCE(SUM(t.duration_ms), 0) / 1000 as duration
         FROM music_release_groups mrg
         LEFT JOIN music_release_group_artists mrga ON mrga.release_group_id = mrg.id AND mrga.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mrga.artist_id
         LEFT JOIN music_releases r ON r.release_group_id = mrg.id
         LEFT JOIN music_media mm ON mm.release_id = r.id
         LEFT JOIN music_tracks t ON t.medium_id = mm.id
         WHERE 1=1{year_filter}
         GROUP BY mrg.id
         {order_clause}
         LIMIT ? OFFSET ?"
    );

    let albums = sqlx::query_as::<_, AlbumRow>(&sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db.read)
        .await
        .unwrap_or_default();

    let mut xml_albums = String::new();
    let mut json_albums: Vec<Value> = Vec::new();
    for a in &albums {
        let id = uuid_str(&a.id);
        let artist_id = a.artist_id.as_deref().map(uuid_str).unwrap_or_default();
        let artist_name = a.artist_name.as_deref().unwrap_or("");
        xml_albums.push_str(&album_xml_elem(
            &id,
            &a.name,
            artist_name,
            &artist_id,
            a.year,
            a.song_count,
            a.duration,
        ));
        json_albums.push(album_json(
            &id,
            &a.name,
            artist_name,
            &artist_id,
            a.year,
            a.song_count,
            a.duration,
        ));
    }

    let xml = format!("<albumList2>{xml_albums}</albumList2>");
    respond_ok(
        user.format,
        &xml,
        Some(("albumList2", json!({ "album": json_albums }))),
    )
}

// ---------------------------------------------------------------------------
// getRandomSongs
// ---------------------------------------------------------------------------

pub async fn get_random_songs(
    State(state): State<AppState>,
    Query(q): Query<RandomSongsQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let limit = q.size.unwrap_or(10).clamp(1, 500);

    let mut year_filter = String::new();
    if let Some(from) = q.from_year {
        year_filter.push_str(&format!(" AND mrg.year >= {from}"));
    }
    if let Some(to) = q.to_year {
        year_filter.push_str(&format!(" AND mrg.year <= {to}"));
    }

    let sql = format!(
        "SELECT t.id, t.title, t.position, t.duration_ms, t.codec,
                mrg.id as album_id, mrg.title as album_title, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM music_tracks t
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases r ON r.id = mm.release_id
         JOIN music_release_groups mrg ON mrg.id = r.release_group_id
         LEFT JOIN music_track_artists mta ON mta.track_id = t.id AND mta.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mta.artist_id
         WHERE 1=1{year_filter}
         ORDER BY RANDOM()
         LIMIT ?"
    );

    let songs = sqlx::query_as::<_, SongRow>(&sql)
        .bind(limit)
        .fetch_all(&state.db.read)
        .await
        .unwrap_or_default();

    let (xml_songs, json_songs) = build_song_lists(&songs);
    let xml = format!("<randomSongs>{xml_songs}</randomSongs>");
    respond_ok(
        user.format,
        &xml,
        Some(("randomSongs", json!({ "song": json_songs }))),
    )
}

// ---------------------------------------------------------------------------
// getStarred2
// ---------------------------------------------------------------------------

pub async fn get_starred2(State(state): State<AppState>, Query(q): Query<CommonQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let user_id_bytes = user.user_id.as_bytes().to_vec();

    // Starred tracks
    let songs = sqlx::query_as::<_, SongRow>(
        "SELECT t.id, t.title, t.position, t.duration_ms, t.codec,
                mrg.id as album_id, mrg.title as album_title, mrg.year,
                mr.id as artist_id, mr.display_name as artist_name
         FROM subsonic_stars ss
         JOIN music_tracks t ON t.id = ss.item_id
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases r ON r.id = mm.release_id
         JOIN music_release_groups mrg ON mrg.id = r.release_group_id
         LEFT JOIN music_track_artists mta ON mta.track_id = t.id AND mta.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mta.artist_id
         WHERE ss.user_id = ? AND ss.item_type = 'track'
         ORDER BY ss.starred_at DESC",
    )
    .bind(&user_id_bytes)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_default();

    // Starred albums
    let albums = sqlx::query_as::<_, AlbumRow>(
        "SELECT mrg.id, mrg.title as name, mrg.year,
                mr.display_name as artist_name, mr.id as artist_id,
                COUNT(DISTINCT t.id) as song_count,
                COALESCE(SUM(t.duration_ms), 0) / 1000 as duration
         FROM subsonic_stars ss
         JOIN music_release_groups mrg ON mrg.id = ss.item_id
         LEFT JOIN music_release_group_artists mrga ON mrga.release_group_id = mrg.id AND mrga.role = 'primary'
         LEFT JOIN media_registry mr ON mr.id = mrga.artist_id
         LEFT JOIN music_releases r ON r.release_group_id = mrg.id
         LEFT JOIN music_media mm ON mm.release_id = r.id
         LEFT JOIN music_tracks t ON t.medium_id = mm.id
         WHERE ss.user_id = ? AND ss.item_type = 'album'
         GROUP BY mrg.id
         ORDER BY ss.starred_at DESC",
    )
    .bind(&user_id_bytes)
    .fetch_all(&state.db.read)
    .await
    .unwrap_or_default();

    let (xml_songs, json_songs) = build_song_lists(&songs);
    let mut xml_albums = String::new();
    let mut json_albums: Vec<Value> = Vec::new();
    for a in &albums {
        let id = uuid_str(&a.id);
        let artist_id = a.artist_id.as_deref().map(uuid_str).unwrap_or_default();
        let artist_name = a.artist_name.as_deref().unwrap_or("");
        xml_albums.push_str(&album_xml_elem(
            &id,
            &a.name,
            artist_name,
            &artist_id,
            a.year,
            a.song_count,
            a.duration,
        ));
        json_albums.push(album_json(
            &id,
            &a.name,
            artist_name,
            &artist_id,
            a.year,
            a.song_count,
            a.duration,
        ));
    }

    let xml = format!("<starred2>{xml_albums}{xml_songs}</starred2>");
    respond_ok(
        user.format,
        &xml,
        Some((
            "starred2",
            json!({ "album": json_albums, "song": json_songs }),
        )),
    )
}

// ---------------------------------------------------------------------------
// getNowPlaying
// ---------------------------------------------------------------------------

pub async fn get_now_playing(
    State(state): State<AppState>,
    Query(q): Query<CommonQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    // No real-time play state tracking yet — return empty
    respond_ok(
        user.format,
        "<nowPlaying />",
        Some(("nowPlaying", json!({}))),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_song_lists(songs: &[SongRow]) -> (String, Vec<Value>) {
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

    use crate::subsonic::test_helpers::{seed_music_data, subsonic_app};
    use axum::{body::Body, body::to_bytes, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn get_album_list2_alphabetical() {
        let (app, state, key) = subsonic_app().await;
        seed_music_data(&state).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/getAlbumList2.view?apiKey={key}&type=alphabeticalByName"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("albumList2"));
    }

    #[tokio::test]
    async fn get_album_list2_by_year() {
        let (app, state, key) = subsonic_app().await;
        seed_music_data(&state).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/getAlbumList2.view?apiKey={key}&type=byYear&fromYear=2020&toYear=2026"
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
    async fn get_random_songs_returns_songs() {
        let (app, state, key) = subsonic_app().await;
        seed_music_data(&state).await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getRandomSongs.view?apiKey={key}&size=5"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("randomSongs"));
    }

    #[tokio::test]
    async fn get_starred2_empty() {
        let (app, _state, key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/getStarred2.view?apiKey={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("starred2"));
    }
}
