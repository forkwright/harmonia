# HTTP media streaming (Paroche)

> Paroche HTTP serving layer: browser playback, OPDS feeds, file downloads, and API data.
> Auth design: [auth.md](../architecture/auth.md)
> Event types: [communication.md](../architecture/communication.md)
> Subsystem boundary: [subsystems.md](../architecture/subsystems.md)

---

## Scope

This document covers HTTP media serving via Paroche. Use cases:
- Browser `<audio>` element playback (web UI)
- OPDS catalog feeds (ebook/comic readers)
- File downloads (podcast episodes, subtitles, cover art)
- API data endpoints

For native client audio streaming (desktop, Android, renderer endpoints),
see [QUIC Streaming Protocol](quic-streaming.md) (syndesis subsystem).

---

## Overview

Paroche serves media files to web UI browsers and file download clients
(`<audio>` / `<video>` elements, OPDS readers, direct downloads). Two serving modes:

**Static file serving**: `tower_http::services::ServeFile` for files served in their
native format directly from disk. tower-http handles RFC 7233 range requests automatically:
206 Partial Content, Content-Range, Accept-Ranges, If-Range, ETag, multipart ranges,
416 Range Not Satisfiable. No hand-rolling required.

**Dynamic stream**: `tokio_util::io::ReaderStream` wrapping `tokio::fs::File` for
seekable async responses where manual range parsing is needed (e.g., to send a partial
range of a transcoded output, or to apply per-user seek restrictions). All file I/O goes
through `tokio::fs`; never `std::fs` in handlers.

---

## Range request handling

### Static files via tower-http ServeFile

For media served directly in native format, tower-http's `ServeFile` handles the full
RFC 7233 surface:

```rust
use tower_http::services::ServeFile;

// In axum router — ServeFile responds with 206 Partial Content on Range requests
// and sets Content-Range, Content-Type, Accept-Ranges headers automatically
let app = Router::new()
    .route_service("/api/media/:id/stream", ServeFile::new(path));
```

ServeFile handles: multipart ranges, `If-Range` conditional requests, ETag generation and
validation, 416 Range Not Satisfiable with `Content-Range: bytes */file_size`.

### Dynamic streaming via ReaderStream

When Paroche needs to control the response (custom headers, range within a seekable async
file, or streaming from an arbitrary offset):

```rust
use tokio_util::io::ReaderStream;
use axum::body::Body;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;

async fn stream_track(
    Path(media_id): Path<MediaId>,
    TypedHeader(range): TypedHeader<headers::Range>,
    user: AuthenticatedUser,
    State(state): State<Arc<ParocheState>>,
    ct: CancellationToken,
) -> Result<impl IntoResponse, ParocheError> {
    state.exousia.authorize(&user, Operation::Stream, ct).await?;

    let path = state.db.get_media_path(media_id, ct).await
        .map_err(|_| ParocheError::MediaNotFound { media_id })?;

    let file = tokio::fs::File::open(&path).await
        .context(FileAccessFailedSnafu { path: path.clone() })?;
    let meta = file.metadata().await
        .context(FileAccessFailedSnafu { path: path.clone() })?;
    let file_size = meta.len();

    let (start, end) = parse_byte_range(&range, file_size)
        .ok_or(ParocheError::RangeNotSatisfiable { file_size })?;
    let length = end - start + 1;

    let mut file = file;
    file.seek(std::io::SeekFrom::Start(start)).await
        .context(FileAccessFailedSnafu { path })?;
    let limited = file.take(length);

    let stream = ReaderStream::new(limited);
    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("Content-Type", mime_for_media(media_id))
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", length.to_string())
        .header("Content-Range", format!("bytes {start}-{end}/{file_size}"))
        .body(body)
        .unwrap())
}
```

**Key constraint:** All file operations use `tokio::fs`. Never `std::fs::File` inside an
async handler. `spawn_blocking` is reserved for CPU-bound work (format detection, tag
reading), not I/O.

---

## Route design

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/media/:id/stream` | Primary streaming endpoint; range-aware, 206 for partial, 200 for full |
| `GET` | `/api/media/:id/download` | Full file download; no range support, `Content-Disposition: attachment; filename="..."` |
| `GET` | `/api/media/:id/info` | Media metadata: `{ duration_secs, format, bitrate_kbps, size_bytes, mime_type }` |

**Content-Type mapping** (file extension → MIME type):

| Extension | MIME Type |
|-----------|-----------|
| `.flac` | `audio/flac` |
| `.mp3` | `audio/mpeg` |
| `.m4a` | `audio/mp4` |
| `.aac` | `audio/aac` |
| `.ogg` | `audio/ogg` |
| `.opus` | `audio/opus` |
| `.m4b` | `audio/mp4` |
| `.epub` | `application/epub+zip` |
| `.cbz` | `application/zip` |
| `.mp4` | `video/mp4` |

Unknown extensions fall back to `application/octet-stream`.

---

## Authentication for streaming

Paroche uses the three-path `AuthenticatedUser` extractor established in
[auth.md](../architecture/auth.md). All three paths produce the same `AuthenticatedUser`
struct; auth method does not affect stream access.

| Priority | Method | Credential | Client |
|----------|--------|------------|--------|
| 1 | Bearer JWT | `Authorization: Bearer <token>` | Akouo web UI, Android app |
| 2 | API Key | `X-Api-Key: hmn_{short}_{long}` | OPDS e-readers, long-lived automation |
| 3 | Query param | `?token=<jwt>` | Browser `<audio>` / `<video>` elements |

The query parameter path exists because browser media elements cannot set custom headers.
Query param tokens carry the same JWT payload and expiry as Bearer tokens; they are not weaker
credentials, just differently delivered.

**Authorization:** After authentication, Paroche calls `Exousia::authorize(&user,
Operation::Stream, ct)` before opening any file. 403 Forbidden returned if the user is
inactive or the operation is denied.

---

## Format negotiation

Default: serve native format. Transcoding is opt-in, never automatic.

**Rules:**

1. Parse `Accept` header from the client request.
2. If client's `Accept` matches or includes the file's native MIME type (or `*/*`):
   serve the native file directly.
3. If client sends `Accept` that does not match the native format, or includes a
   `?format=mp3` (or similar) query parameter: redirect to the transcoding endpoint
   (`POST /api/transcode`) with the requested format. Return 303 See Other with
   `Location: /api/transcode` pointing to the transcode session.
4. Never transcode transparently; the client is always informed.

**Client-specific behavior:**

- **Akouo web UI:** serves native format if the browser supports it (FLAC is
  supported in Chrome/Firefox). UI offers a "Convert for compatibility" button that
  triggers the transcode flow; transcoding is never automatic.
- **OPDS readers:** typically request EPUB/CBZ acquisition links. Audio via OPDS is an
  M4B acquisition link; no format negotiation needed (served as-is).

> Native clients (Android app, desktop) receive audio via the syndesis QUIC protocol,
> not HTTP. See [quic-streaming.md](quic-streaming.md).

---

## Playback session tracking

Paroche maintains a `PlaybackSession` per active stream to implement scrobble threshold
timing and now-playing notification.

```rust
pub struct PlaybackSession {
    pub session_id: SessionId,
    pub media_id: MediaId,
    pub user_id: UserId,
    pub started_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub threshold_met: bool,
}
```

**Session lifecycle:**

1. **Stream start**: register session in an in-memory map (keyed by `session_id`),
   emit `NowPlayingStarted` event via Aggelia, spawn the threshold timer task.
2. **Timer task**: waits until scrobble threshold is met, then emits `ScrobbleRequired`.
3. **Stream end / disconnect**: clean up session from the map, cancel the timer task
   via `CancellationToken`.

### Scrobble threshold

Official Last.fm rule: submit when track has played for **4 minutes OR 50% of track
duration, whichever is earlier**. Track must be longer than 30 seconds.

```rust
// Paroche spawns this task when playback begins
async fn playback_timer_task(
    track_id: MediaId,
    user_id: UserId,
    duration_secs: u64,
    event_tx: broadcast::Sender<HarmoniaEvent>,
    ct: CancellationToken,
) {
    if duration_secs <= 30 {
        // Track too short — no scrobble (Last.fm rule)
        return;
    }

    // Emit NowPlaying immediately on stream start
    event_tx.send(HarmoniaEvent::NowPlayingStarted {
        track_id,
        user_id,
    }).ok();

    // Threshold: 4 minutes OR 50% of duration, whichever is earlier
    let threshold_secs = (duration_secs / 2).min(4 * 60);

    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(threshold_secs)) => {
            event_tx.send(HarmoniaEvent::ScrobbleRequired {
                track_id,
                user_id,
            }).ok();
        }
        _ = ct.cancelled() => {
            // Stream ended before threshold — no scrobble
        }
    }
}
```

### NowPlayingStarted event

`NowPlayingStarted` is a new `HarmoniaEvent` variant (the enum is `#[non_exhaustive]`
and designed to grow, per Phase 3 communication.md design):

```rust
/// Paroche began streaming a track to a user.
/// Subscribers: Syndesmos (call Last.fm track.updateNowPlaying)
NowPlayingStarted {
    track_id: MediaId,
    user_id: UserId,
},
```

Syndesmos reacts by calling `track.updateNowPlaying` via `rustfm-scrobble`. If Last.fm is
not configured, no subscriber processes the event; this is acceptable.

---

## Concurrency

No blocking in streaming handlers.

| Operation | Correct approach |
|-----------|-----------------|
| Open file | `tokio::fs::File::open()` |
| Read file bytes | `tokio_util::io::ReaderStream` |
| Seek within file | `tokio::io::AsyncSeekExt::seek()` |
| Detect audio format | `tokio::task::spawn_blocking(|| ...)` |
| Read audio tags | `tokio::task::spawn_blocking(|| ...)` |
| Range header parse | Sync; pure computation, no I/O |

**Connection limit:** `max_concurrent_streams` in `[paroche]` config. Enforced via a
tokio semaphore at the handler entry point. Requests that exceed the limit receive
503 `{ "error": "stream_capacity_exceeded" }`.

---

## Error handling

`ParocheError` snafu enum with HTTP status mapping:

```rust
#[derive(Debug, Snafu)]
pub enum ParocheError {
    #[snafu(display("media {media_id} not found"))]
    MediaNotFound {
        media_id: MediaId,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unauthorized"))]
    Unauthorized {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("forbidden"))]
    Forbidden {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("byte range not satisfiable (file size: {file_size})"))]
    RangeNotSatisfiable {
        file_size: u64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("file access failed: {path:?}"))]
    FileAccessFailed {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("transcoding unavailable: FFmpeg not found in PATH"))]
    TranscodingUnavailable {
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
```

**HTTP status mapping:**

| Error | HTTP Status |
|-------|-------------|
| `MediaNotFound` | 404 Not Found |
| `Unauthorized` | 401 Unauthorized |
| `Forbidden` | 403 Forbidden |
| `RangeNotSatisfiable` | 416 Range Not Satisfiable |
| `FileAccessFailed` | 500 Internal Server Error |
| `TranscodingUnavailable` | 503 Service Unavailable |

Error responses follow the standard Harmonia format:
`{ "error": "snake_case_variant", "detail": "...", "correlation_id": "..." }`

---

## Horismos configuration

`[paroche]` section in `harmonia.toml`:

```toml
[paroche]
max_concurrent_streams = 100        # Semaphore limit on active streams
stream_buffer_size = 65536          # ReaderStream chunk size in bytes (default 64KB)
session_cleanup_interval_secs = 300 # Interval to remove stale PlaybackSessions
```

**Defaults are load-tested baselines.** `stream_buffer_size` trades memory per stream
against syscall frequency; 64KB is appropriate for local network FLAC serving.
`session_cleanup_interval_secs` removes sessions that did not receive a clean disconnect
(e.g., browser tab closed without teardown).
