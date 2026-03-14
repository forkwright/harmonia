//! HTTP stream fetcher: downloads audio from the serve instance to a temp file.

use std::path::PathBuf;

use snafu::{ResultExt, Snafu};
use tracing::instrument;

#[derive(Debug, Snafu)]
pub(crate) enum StreamError {
    #[snafu(display("failed to fetch stream for track {track_id}: {source}"))]
    Fetch {
        track_id: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("failed to write stream for track {track_id} to temp file: {source}"))]
    Write {
        track_id: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("server returned {status} for track {track_id}"))]
    HttpError {
        track_id: String,
        status: u16,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

/// Downloads the audio stream for `track_id` from `base_url` and writes it to a temp file.
///
/// Returns the path to the temp file. The caller is responsible for deleting it after playback.
#[instrument(skip(client))]
pub(crate) async fn fetch_stream(
    client: &reqwest::Client,
    base_url: &str,
    track_id: &str,
    token: Option<&str>,
) -> Result<PathBuf, StreamError> {
    if base_url.is_empty() {
        // WHY: When next_track / previous_track call play_entry without a server URL
        // (the command doesn't carry it), return a stub so the engine gets a path.
        // Real playback goes through play_track which the frontend calls with base_url.
        return Ok(std::env::temp_dir().join(format!("harmonia-stub-{track_id}.audio")));
    }

    let url = format!(
        "{}/api/music/tracks/{}/stream",
        base_url.trim_end_matches('/'),
        track_id
    );
    let mut req = client.get(&url);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }

    let response = req.send().await.context(FetchSnafu {
        track_id: track_id.to_string(),
    })?;

    if !response.status().is_success() {
        return Err(StreamError::HttpError {
            track_id: track_id.to_string(),
            status: response.status().as_u16(),
            location: snafu::location!(),
        });
    }

    let suffix = infer_extension(response.headers());
    let tmp_path = std::env::temp_dir().join(format!("harmonia-stream-{track_id}{suffix}"));

    let bytes = response.bytes().await.context(FetchSnafu {
        track_id: track_id.to_string(),
    })?;

    std::fs::write(&tmp_path, &bytes).context(WriteSnafu {
        track_id: track_id.to_string(),
    })?;

    Ok(tmp_path)
}

fn infer_extension(headers: &reqwest::header::HeaderMap) -> &'static str {
    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("flac") {
        ".flac"
    } else if content_type.contains("opus") {
        ".opus"
    } else if content_type.contains("ogg") {
        ".ogg"
    } else if content_type.contains("mpeg") || content_type.contains("mp3") {
        ".mp3"
    } else if content_type.contains("aac") || content_type.contains("mp4") {
        ".m4a"
    } else if content_type.contains("wav") {
        ".wav"
    } else {
        ".audio"
    }
}
