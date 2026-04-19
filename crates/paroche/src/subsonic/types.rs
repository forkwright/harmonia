use axum::body::Body;
use axum::response::Response;
use serde_json::{Value, json};

pub(crate) const API_VERSION: &str = "1.16.1";
pub(crate) const SERVER_TYPE: &str = "harmonia";
pub(crate) const SERVER_VERSION: &str = "0.1.0";

// Subsonic error codes
pub(crate) const ERR_GENERIC: u32 = 0;
pub(crate) const ERR_MISSING_PARAM: u32 = 10;
pub(crate) const ERR_WRONG_CREDS: u32 = 40;
pub(crate) const ERR_NOT_FOUND: u32 = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    Xml,
    Json,
}

impl Format {
    pub(crate) fn parse(s: &str) -> Self {
        if s == "json" { Self::Json } else { Self::Xml }
    }
}

pub(crate) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn xml_wrapper(status: &str, inner: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><subsonic-response xmlns="http://subsonic.org/restapi" status="{status}" version="{API_VERSION}" type="{SERVER_TYPE}" serverVersion="{SERVER_VERSION}" openSubsonic="true">{inner}</subsonic-response>"#
    )
}

fn json_base(status: &str) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("status".into(), json!(status));
    m.insert("version".into(), json!(API_VERSION));
    m.insert("type".into(), json!(SERVER_TYPE));
    m.insert("serverVersion".into(), json!(SERVER_VERSION));
    m.insert("openSubsonic".into(), json!(true));
    m
}

#[expect(
    clippy::unwrap_used,
    reason = "Response::builder with static 200 status and known-good headers is infallible; serde_json::to_string of json!() macro VALUES is infallible"
)]
pub(crate) fn respond_ok(
    format: Format,
    xml_inner: &str,
    json_key: Option<(&str, Value)>,
) -> Response {
    match format {
        Format::Xml => {
            let body = xml_wrapper("ok", xml_inner);
            Response::builder()
                .status(200)
                .header("Content-Type", "text/xml; charset=UTF-8")
                .body(Body::from(body))
                .unwrap() // kanon:ignore RUST/unwrap -- Response::builder with static status + headers is infallible
        }
        Format::Json => {
            let mut obj = json_base("ok");
            if let Some((key, val)) = json_key {
                obj.insert(key.to_string(), val);
            }
            let body =
                serde_json::to_string(&json!({ "subsonic-response": Value::Object(obj) })).unwrap(); // kanon:ignore RUST/unwrap -- serde_json::to_string of json!() VALUES is infallible
            Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap() // kanon:ignore RUST/unwrap -- Response::builder with static status + headers is infallible
        }
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "Response::builder with static 200 status and known-good headers is infallible; serde_json::to_string of json!() macro VALUES is infallible"
)]
pub(crate) fn respond_error(format: Format, code: u32, message: &str) -> Response {
    match format {
        Format::Xml => {
            let inner = format!(
                r#"<error code="{code}" message="{}" />"#,
                xml_escape(message)
            );
            let body = xml_wrapper("failed", &inner);
            Response::builder()
                .status(200)
                .header("Content-Type", "text/xml; charset=UTF-8")
                .body(Body::from(body))
                .unwrap() // kanon:ignore RUST/unwrap -- Response::builder with static status + headers is infallible
        }
        Format::Json => {
            let mut obj = json_base("failed");
            obj.insert("error".into(), json!({ "code": code, "message": message }));
            let body =
                serde_json::to_string(&json!({ "subsonic-response": Value::Object(obj) })).unwrap(); // kanon:ignore RUST/unwrap -- serde_json::to_string of json!() VALUES is infallible
            Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap() // kanon:ignore RUST/unwrap -- Response::builder with static status + headers is infallible
        }
    }
}

pub(crate) fn uuid_str(bytes: &[u8]) -> String {
    uuid::Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

pub(crate) fn uuid_bytes(id: &str) -> Option<Vec<u8>> {
    uuid::Uuid::parse_str(id)
        .map(|u| u.as_bytes().to_vec())
        .ok()
}

pub(crate) fn codec_content_type(codec: Option<&str>) -> &'static str {
    match codec.map(|s| s.to_uppercase()).as_deref() {
        Some("FLAC") => "audio/flac",
        Some("MP3") => "audio/mpeg",
        Some("AAC") | Some("M4A") | Some("ALAC") => "audio/mp4",
        Some("OGG") => "audio/ogg",
        Some("OPUS") => "audio/ogg; codecs=opus",
        _ => "audio/mpeg",
    }
}

pub(crate) fn codec_suffix(codec: Option<&str>) -> &'static str {
    match codec.map(|s| s.to_uppercase()).as_deref() {
        Some("FLAC") => "flac",
        Some("MP3") => "mp3",
        Some("AAC") => "aac",
        Some("M4A") => "m4a",
        Some("ALAC") => "m4a",
        Some("OGG") => "ogg",
        Some("OPUS") => "opus",
        _ => "mp3",
    }
}

/// Artist index grouping — returns uppercase first letter, stripping common articles.
pub(crate) fn index_letter(name: &str) -> String {
    let lower = name.to_lowercase();
    let stripped = [
        "the ", "a ", "an ", "el ", "la ", "los ", "las ", "le ", "les ",
    ]
    .iter()
    .find_map(|prefix| lower.strip_prefix(prefix))
    .unwrap_or(name);

    stripped
        .chars()
        .next()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                c.to_ascii_uppercase().to_string()
            } else {
                "#".to_string()
            }
        })
        .unwrap_or_else(|| "#".to_string())
}

/// Build Subsonic artist XML element.
pub(crate) fn artist_xml(id: &str, name: &str, album_count: i64) -> String {
    format!(
        r#"<artist id="{}" name="{}" albumCount="{album_count}" />"#,
        xml_escape(id),
        xml_escape(name)
    )
}

/// Build Subsonic AlbumID3 XML element (without children).
pub(crate) fn album_xml_elem(
    id: &str,
    name: &str,
    artist: &str,
    artist_id: &str,
    year: Option<i64>,
    song_count: i64,
    duration: i64,
) -> String {
    let year_attr = year.map(|y| format!(r#" year="{y}""#)).unwrap_or_default();
    format!(
        r#"<album id="{}" name="{}" artist="{}" artistId="{}" songCount="{song_count}" duration="{duration}"{year_attr} />"#,
        xml_escape(id),
        xml_escape(name),
        xml_escape(artist),
        xml_escape(artist_id),
    )
}

/// Build Subsonic song/Child XML element.
#[expect(
    clippy::too_many_arguments,
    reason = "SubsonicResponse constructor mirrors the full Subsonic API response structure"
)]
pub(crate) fn song_xml_elem(
    id: &str,
    title: &str,
    album: &str,
    album_id: &str,
    artist: &str,
    artist_id: &str,
    track: Option<i64>,
    year: Option<i64>,
    duration_secs: Option<i64>,
    bit_rate: Option<i64>,
    content_type: &str,
    suffix: &str,
    is_dir: bool,
) -> String {
    let track_attr = track
        .map(|t| format!(r#" track="{t}""#))
        .unwrap_or_default();
    let year_attr = year.map(|y| format!(r#" year="{y}""#)).unwrap_or_default();
    let dur_attr = duration_secs
        .map(|d| format!(r#" duration="{d}""#))
        .unwrap_or_default();
    let br_attr = bit_rate
        .map(|b| format!(r#" bitRate="{b}""#))
        .unwrap_or_default();
    format!(
        r#"<song id="{}" parent="{}" isDir="{is_dir}" title="{}" album="{}" albumId="{}" artist="{}" artistId="{}"{track_attr}{year_attr}{dur_attr}{br_attr} contentType="{content_type}" suffix="{suffix}" />"#,
        xml_escape(id),
        xml_escape(album_id),
        xml_escape(title),
        xml_escape(album),
        xml_escape(album_id),
        xml_escape(artist),
        xml_escape(artist_id),
    )
}

/// Build JSON object for an AlbumID3.
pub(crate) fn album_json(
    id: &str,
    name: &str,
    artist: &str,
    artist_id: &str,
    year: Option<i64>,
    song_count: i64,
    duration: i64,
) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("id".into(), json!(id));
    m.insert("name".into(), json!(name));
    m.insert("artist".into(), json!(artist));
    m.insert("artistId".into(), json!(artist_id));
    if let Some(y) = year {
        m.insert("year".into(), json!(y));
    }
    m.insert("songCount".into(), json!(song_count));
    m.insert("duration".into(), json!(duration));
    Value::Object(m)
}

/// Build JSON object for a song/Child.
#[expect(
    clippy::too_many_arguments,
    reason = "SubsonicResponse constructor mirrors the full Subsonic API response structure"
)]
pub(crate) fn song_json(
    id: &str,
    title: &str,
    album: &str,
    album_id: &str,
    artist: &str,
    artist_id: &str,
    track: Option<i64>,
    year: Option<i64>,
    duration_secs: Option<i64>,
    bit_rate: Option<i64>,
    content_type: &str,
    suffix: &str,
) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("id".into(), json!(id));
    m.insert("parent".into(), json!(album_id));
    m.insert("isDir".into(), json!(false));
    m.insert("title".into(), json!(title));
    m.insert("album".into(), json!(album));
    m.insert("albumId".into(), json!(album_id));
    m.insert("artist".into(), json!(artist));
    m.insert("artistId".into(), json!(artist_id));
    if let Some(t) = track {
        m.insert("track".into(), json!(t));
    }
    if let Some(y) = year {
        m.insert("year".into(), json!(y));
    }
    if let Some(d) = duration_secs {
        m.insert("duration".into(), json!(d));
    }
    if let Some(b) = bit_rate {
        m.insert("bitRate".into(), json!(b));
    }
    m.insert("contentType".into(), json!(content_type));
    m.insert("suffix".into(), json!(suffix));
    Value::Object(m)
}

/// Shared query params present on every Subsonic request.
#[derive(serde::Deserialize, Default, Clone)]
pub struct SubsonicCommon {
    pub u: Option<String>,
    pub t: Option<String>,
    pub s: Option<String>,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    pub f: Option<String>,
    pub v: Option<String>,
    pub c: Option<String>,
}

impl SubsonicCommon {
    pub(crate) fn format(&self) -> Format {
        self.f.as_deref().map(Format::parse).unwrap_or(Format::Xml)
    }
}
