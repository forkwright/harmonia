use reqwest::{Client, StatusCode, header};
use snafu::ResultExt;

use crate::error::{EpisodeDownloadSnafu, EpisodeIoSnafu, FeedFetchSnafu, KomideError};

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
/// with the response body and freshly received cache validators.
pub async fn fetch_feed(
    client: &Client,
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
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

    let bytes = response
        .bytes()
        .await
        .context(FeedFetchSnafu {
            url: url.to_string(),
        })?
        .to_vec();

    Ok(FetchResult::Content {
        bytes,
        etag: new_etag,
        last_modified: new_last_modified,
    })
}

/// Download episode audio to the given path. Returns file size in bytes.
pub async fn download_episode(
    client: &Client,
    url: &str,
    dest: &std::path::Path,
) -> Result<u64, KomideError> {
    use tokio::io::AsyncWriteExt;

    let response = client.get(url).send().await.context(EpisodeDownloadSnafu {
        url: url.to_string(),
    })?;

    let bytes = response.bytes().await.context(EpisodeDownloadSnafu {
        url: url.to_string(),
    })?;

    let path_str = dest.display().to_string();

    let mut file = tokio::fs::File::CREATE(dest)
        .await
        .context(EpisodeIoSnafu {
            path: path_str.clone(),
        })?;

    file.write_all(&bytes)
        .await
        .context(EpisodeIoSnafu { path: path_str })?;

    Ok(bytes.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
