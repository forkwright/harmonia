//! Shared test fixtures — sequential one-shot HTTP servers for provider tests.
#![cfg(test)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Spawns a TCP server that answers `responses.len()` sequential HTTP
/// requests (one connection each) with the given status and body, then
/// resolves to the raw request bytes it received, in order.
///
/// WHY: also installs the process-wide rustls crypto provider on the way
/// in. reqwest builds with `rustls-no-provider` (fleet convention: install
/// explicitly, never let a library link one implicitly — see main.rs), so
/// `reqwest::Client::new()`/`::builder().build()` panics ("No rustls crypto
/// provider is configured") in any process that never called
/// `install_default()` — and a nextest test binary never runs `main()`.
/// Every provider test in this crate calls this helper before constructing
/// its client, so installing here (idempotent — install_default() on an
/// already-installed process just returns Err, discarded) covers the whole
/// crate's test suite instead of repeating the call at every test site.
pub(crate) async fn spawn_sequential_http(
    responses: Vec<(u16, String)>,
) -> (String, JoinHandle<Vec<String>>) {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];

            let header_end = loop {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0, "client closed before sending full headers");
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                    break pos + 4;
                }
            };

            let content_length = parse_content_length(&buf[..header_end]);
            while buf.len() < header_end + content_length {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0, "client closed before sending full body");
                buf.extend_from_slice(&chunk[..n]);
            }

            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();

            requests.push(String::from_utf8_lossy(&buf).into_owned());
        }
        requests
    });

    (base_url, handle)
}

/// Writes an executable stub script into `dir` and returns its path.
#[cfg(unix)]
pub(crate) fn write_stub_script(
    dir: &std::path::Path,
    name: &str,
    body: &str,
) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_content_length(headers: &[u8]) -> usize {
    String::from_utf8_lossy(headers)
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}
