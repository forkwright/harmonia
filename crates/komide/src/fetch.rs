use reqwest::{Client, Response, StatusCode, header};
use snafu::{ResultExt, ensure};

use crate::error::{
    EpisodeDownloadSnafu, EpisodeIoSnafu, FeedFetchSnafu, KomideError, ResponseTooLargeSnafu,
};

#[non_exhaustive]
pub enum FetchResult {
    Content {
        bytes: Vec<u8>,
        etag: Option<String>,
        last_modified: Option<String>,
    },
    NotModified,
}

/// Fetch a feed URL using conditional GET if ETag or Last-Modified is provided.
///
/// Returns `FetchResult::NotModified` on HTTP 304, or `FetchResult::Content`
/// with the response body and freshly received cache validators. Non-success
/// statuses are errors; the body is streamed and rejected once it exceeds
/// `max_bytes`.
pub async fn fetch_feed(
    client: &Client,
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
    max_bytes: u64,
) -> Result<FetchResult, KomideError> {
    let mut req = client.get(url);

    if let Some(etag) = etag {
        req = req.header(header::IF_NONE_MATCH, etag);
    }
    if let Some(lm) = last_modified {
        req = req.header(header::IF_MODIFIED_SINCE, lm);
    }

    let response = req.send().await.context(FeedFetchSnafu {
        url: url.to_string(),
    })?;

    if response.status() == StatusCode::NOT_MODIFIED {
        return Ok(FetchResult::NotModified);
    }

    // WHY: a 4xx/5xx body is an error page, not feed content; reject before reading.
    let response = response.error_for_status().context(FeedFetchSnafu {
        url: url.to_string(),
    })?;

    let new_etag = response
        .headers()
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let new_last_modified = response
        .headers()
        .get(header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let bytes = read_body_capped(response, url, max_bytes).await?;

    Ok(FetchResult::Content {
        bytes,
        etag: new_etag,
        last_modified: new_last_modified,
    })
}

/// Stream a feed response body, failing with `ResponseTooLarge` once the
/// declared or accumulated size exceeds `max_bytes`.
pub(crate) async fn read_body_capped(
    mut response: Response,
    url: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, KomideError> {
    if let Some(declared) = response.content_length() {
        ensure!(
            declared <= max_bytes,
            ResponseTooLargeSnafu {
                url: url.to_string(),
                limit: max_bytes,
            }
        );
    }

    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.context(FeedFetchSnafu {
        url: url.to_string(),
    })? {
        let total = (bytes.len() as u64).saturating_add(chunk.len() as u64);
        ensure!(
            total <= max_bytes,
            ResponseTooLargeSnafu {
                url: url.to_string(),
                limit: max_bytes,
            }
        );
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

/// Download episode audio to the given path, streaming chunks straight to
/// disk. Returns the file size in bytes. Downloads whose declared or streamed
/// size exceeds `max_bytes` abort with `ResponseTooLarge`, and any partially
/// written file is removed.
pub async fn download_episode(
    client: &Client,
    url: &str,
    dest: &std::path::Path,
    max_bytes: u64,
) -> Result<u64, KomideError> {
    let response = client.get(url).send().await.context(EpisodeDownloadSnafu {
        url: url.to_string(),
    })?;

    // WHY: a 4xx/5xx body is an error page, not audio; reject before touching disk.
    let response = response.error_for_status().context(EpisodeDownloadSnafu {
        url: url.to_string(),
    })?;

    if let Some(declared) = response.content_length() {
        ensure!(
            declared <= max_bytes,
            ResponseTooLargeSnafu {
                url: url.to_string(),
                limit: max_bytes,
            }
        );
    }

    let path_str = dest.display().to_string();

    let file = tokio::fs::File::create(dest)
        .await
        .context(EpisodeIoSnafu {
            path: path_str.clone(),
        })?;

    match stream_body_to_file(response, file, url, &path_str, max_bytes).await {
        Ok(written) => Ok(written),
        Err(err) => {
            // WHY: never leave a truncated or over-cap partial file on disk.
            tokio::fs::remove_file(dest).await.ok();
            Err(err)
        }
    }
}

async fn stream_body_to_file(
    mut response: Response,
    mut file: tokio::fs::File,
    url: &str,
    path: &str,
    max_bytes: u64,
) -> Result<u64, KomideError> {
    use tokio::io::AsyncWriteExt;

    let mut written: u64 = 0;
    while let Some(chunk) = response.chunk().await.context(EpisodeDownloadSnafu {
        url: url.to_string(),
    })? {
        written = written.saturating_add(chunk.len() as u64);
        ensure!(
            written <= max_bytes,
            ResponseTooLargeSnafu {
                url: url.to_string(),
                limit: max_bytes,
            }
        );
        file.write_all(&chunk).await.context(EpisodeIoSnafu {
            path: path.to_string(),
        })?;
    }

    file.flush().await.context(EpisodeIoSnafu {
        path: path.to_string(),
    })?;

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{http_response, http_response_close_delimited, spawn_scripted_http};

    const CAP: u64 = 1024;

    #[test]
    fn fetch_result_not_modified_variant() {
        assert!(matches!(FetchResult::NotModified, FetchResult::NotModified));
    }

    #[test]
    fn fetch_result_content_holds_bytes() {
        let result = FetchResult::Content {
            bytes: vec![1, 2, 3],
            etag: Some("\"abc\"".to_string()),
            last_modified: None,
        };
        match result {
            FetchResult::Content { bytes, etag, .. } => {
                assert_eq!(bytes, vec![1, 2, 3]);
                assert_eq!(etag.as_deref(), Some("\"abc\""));
            }
            _ => panic!("expected Content variant"),
        }
    }

    #[test]
    fn conditional_request_stores_etag() {
        // Verifies the FetchResult::Content variant preserves ETag for subsequent requests.
        let result = FetchResult::Content {
            bytes: b"feed content".to_vec(),
            etag: Some("W/\"xyz-123\"".to_string()),
            last_modified: Some("Wed, 01 Jan 2026 00:00:00 GMT".to_string()),
        };
        match result {
            FetchResult::Content {
                etag,
                last_modified,
                ..
            } => {
                assert_eq!(etag.as_deref(), Some("W/\"xyz-123\""));
                assert!(last_modified.is_some());
            }
            FetchResult::NotModified => panic!("expected Content"),
        }
    }

    #[tokio::test]
    async fn fetch_feed_returns_content_and_captures_validators() {
        let (url, handle) = spawn_scripted_http(vec![http_response(
            200,
            "OK",
            &[
                ("etag", "\"v1\""),
                ("last-modified", "Wed, 01 Jan 2026 00:00:00 GMT"),
            ],
            b"<rss/>",
        )])
        .await;

        let client = Client::new();
        let result = fetch_feed(&client, &url, None, None, CAP).await.unwrap();
        match result {
            FetchResult::Content {
                bytes,
                etag,
                last_modified,
            } => {
                assert_eq!(bytes, b"<rss/>");
                assert_eq!(etag.as_deref(), Some("\"v1\""));
                assert_eq!(
                    last_modified.as_deref(),
                    Some("Wed, 01 Jan 2026 00:00:00 GMT")
                );
            }
            FetchResult::NotModified => panic!("expected Content"),
        }

        let requests = handle.await.unwrap();
        let head = requests[0].to_lowercase();
        assert!(
            !head.contains("if-none-match"),
            "unconditional fetch must not send validators"
        );
    }

    #[tokio::test]
    async fn fetch_feed_304_returns_not_modified_and_sends_validators() {
        // Regression guard for the status-check ORDER: 304 must short-circuit
        // to NotModified and never be treated as content or as an error.
        let (url, handle) =
            spawn_scripted_http(vec![http_response(304, "Not Modified", &[], b"")]).await;

        let client = Client::new();
        let result = fetch_feed(
            &client,
            &url,
            Some("\"v1\""),
            Some("Wed, 01 Jan 2026 00:00:00 GMT"),
            CAP,
        )
        .await
        .unwrap();
        assert!(matches!(result, FetchResult::NotModified));

        let requests = handle.await.unwrap();
        let head = requests[0].to_lowercase();
        assert!(head.contains("if-none-match: \"v1\""));
        assert!(head.contains("if-modified-since: wed, 01 jan 2026 00:00:00 gmt"));
    }

    #[tokio::test]
    async fn fetch_feed_500_returns_error_not_content() {
        // The 500 body parses as a feed; it must still be rejected on status.
        let (url, _handle) = spawn_scripted_http(vec![http_response(
            500,
            "Internal Server Error",
            &[],
            b"<rss version=\"2.0\"><channel><title>t</title></channel></rss>",
        )])
        .await;

        let client = Client::new();
        let result = fetch_feed(&client, &url, None, None, CAP).await;
        assert!(matches!(result, Err(KomideError::FeedFetch { .. })));
    }

    #[tokio::test]
    async fn fetch_feed_404_returns_error() {
        let (url, _handle) =
            spawn_scripted_http(vec![http_response(404, "Not Found", &[], b"missing")]).await;

        let client = Client::new();
        let result = fetch_feed(&client, &url, None, None, CAP).await;
        assert!(matches!(result, Err(KomideError::FeedFetch { .. })));
    }

    #[tokio::test]
    async fn fetch_feed_declared_over_cap_rejected_up_front() {
        let body = vec![b'x'; (CAP as usize) * 2];
        let (url, _handle) = spawn_scripted_http(vec![http_response(200, "OK", &[], &body)]).await;

        let client = Client::new();
        let result = fetch_feed(&client, &url, None, None, CAP).await;
        assert!(matches!(
            result,
            Err(KomideError::ResponseTooLarge { limit, .. }) if limit == CAP
        ));
    }

    #[tokio::test]
    async fn fetch_feed_streamed_over_cap_rejected_without_content_length() {
        // No Content-Length header: the cap must trip in the streaming loop.
        let body = vec![b'x'; (CAP as usize) * 4];
        let (url, _handle) =
            spawn_scripted_http(vec![http_response_close_delimited(200, "OK", &body)]).await;

        let client = Client::new();
        let result = fetch_feed(&client, &url, None, None, CAP).await;
        assert!(matches!(result, Err(KomideError::ResponseTooLarge { .. })));
    }

    #[tokio::test]
    async fn fetch_feed_body_exactly_at_cap_succeeds() {
        let body = vec![b'x'; CAP as usize];
        let (url, _handle) =
            spawn_scripted_http(vec![http_response_close_delimited(200, "OK", &body)]).await;

        let client = Client::new();
        let result = fetch_feed(&client, &url, None, None, CAP).await.unwrap();
        match result {
            FetchResult::Content { bytes, .. } => assert_eq!(bytes.len(), CAP as usize),
            FetchResult::NotModified => panic!("expected Content"),
        }
    }

    #[tokio::test]
    async fn fetch_feed_body_one_byte_over_cap_fails() {
        let body = vec![b'x'; (CAP as usize) + 1];
        let (url, _handle) =
            spawn_scripted_http(vec![http_response_close_delimited(200, "OK", &body)]).await;

        let client = Client::new();
        let result = fetch_feed(&client, &url, None, None, CAP).await;
        assert!(matches!(result, Err(KomideError::ResponseTooLarge { .. })));
    }

    #[tokio::test]
    async fn download_episode_writes_expected_content_and_size() {
        let body = b"pretend-this-is-audio-bytes";
        let (url, _handle) = spawn_scripted_http(vec![http_response(200, "OK", &[], body)]).await;

        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("episode.mp3");
        let client = Client::new();

        let written = download_episode(&client, &url, &dest, CAP).await.unwrap();
        assert_eq!(written, body.len() as u64);

        let on_disk = std::fs::read(&dest).unwrap();
        assert_eq!(on_disk, body);
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), written);
    }

    #[tokio::test]
    async fn download_episode_500_returns_error_and_writes_no_file() {
        let (url, _handle) = spawn_scripted_http(vec![http_response(
            500,
            "Internal Server Error",
            &[],
            b"<html>oops</html>",
        )])
        .await;

        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("episode.mp3");
        let client = Client::new();

        let result = download_episode(&client, &url, &dest, CAP).await;
        assert!(matches!(result, Err(KomideError::EpisodeDownload { .. })));
        assert!(!dest.exists(), "no file may be created on HTTP error");
    }

    #[tokio::test]
    async fn download_episode_declared_over_cap_fails_before_creating_file() {
        let body = vec![b'x'; (CAP as usize) * 2];
        let (url, _handle) = spawn_scripted_http(vec![http_response(200, "OK", &[], &body)]).await;

        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("episode.mp3");
        let client = Client::new();

        let result = download_episode(&client, &url, &dest, CAP).await;
        assert!(matches!(result, Err(KomideError::ResponseTooLarge { .. })));
        assert!(!dest.exists(), "over-cap download must not create the file");
    }

    // ── #549: a stalled feed host must time out, not hang forever ──────────

    #[tokio::test]
    async fn fetch_feed_stalled_body_times_out_via_client_configured_timeout() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // WHY: the server task is intentionally never joined — it outlives
        // the assertion (stalled well past the client's configured
        // timeout) and is cleaned up when this test's tokio runtime shuts
        // down at function return.
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _bytes_read = stream.read(&mut buf).await.unwrap();
            // WHY: declares a body well within the byte cap, then never
            // writes it — reproduces a feed host that stalls after headers.
            // Without a client-level timeout, response.chunk().await below
            // would block forever on exactly this shape of response.
            let head = format!("HTTP/1.1 200 OK\r\ncontent-length: {CAP}\r\n\r\n");
            stream.write_all(head.as_bytes()).await.ok();
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        });

        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap();

        let start = std::time::Instant::now();
        let result = fetch_feed(&client, &format!("http://{addr}"), None, None, CAP).await;
        let elapsed = start.elapsed();

        // WHY: FetchResult (the Ok payload) does not derive Debug, so
        // matching directly avoids requiring it just for this assertion.
        let Err(err) = result else {
            panic!("a stalled body must time out, not succeed");
        };
        assert!(
            matches!(err, KomideError::FeedFetch { .. }),
            "a stalled body must time out as a FeedFetch error, not hang forever: {err:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "the client-configured timeout must bound the stall well under the server's 5s hold, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn download_episode_streamed_over_cap_removes_partial_file() {
        // No Content-Length header: the cap trips mid-stream, after bytes may
        // already be on disk; the partial file must be removed.
        let body = vec![b'x'; (CAP as usize) * 4];
        let (url, _handle) =
            spawn_scripted_http(vec![http_response_close_delimited(200, "OK", &body)]).await;

        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("episode.mp3");
        let client = Client::new();

        let result = download_episode(&client, &url, &dest, CAP).await;
        assert!(matches!(result, Err(KomideError::ResponseTooLarge { .. })));
        assert!(!dest.exists(), "partial over-cap file must be removed");
    }
}
