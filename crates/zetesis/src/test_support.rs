//! Shared test fixtures — one-shot HTTP servers for indexer client tests.
#![cfg(test)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Spawns a TCP server that answers exactly one HTTP request with the given
/// raw response bytes, then resolves to the raw request head it received.
pub(crate) async fn spawn_raw_http(raw_response: Vec<u8>) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];

        loop {
            let n = stream.read(&mut chunk).await.unwrap();
            assert!(n > 0, "client closed before sending full headers");
            buf.extend_from_slice(&chunk[..n]);
            if find_subslice(&buf, b"\r\n\r\n").is_some() {
                break;
            }
        }

        stream.write_all(&raw_response).await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.unwrap();

        String::from_utf8_lossy(&buf).into_owned()
    });

    (base_url, handle)
}

/// Spawns a TCP server that answers exactly one HTTP request with `status`,
/// the given extra headers, and `body` (with a correct `Content-Length`).
pub(crate) async fn spawn_one_shot_http(
    status: u16,
    reason: &str,
    extra_headers: &[(&str, &str)],
    body: &str,
) -> (String, JoinHandle<String>) {
    let mut response = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in extra_headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str(&format!(
        "content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    ));
    spawn_raw_http(response.into_bytes()).await
}

/// Spawns a TCP server that accepts one connection and never responds.
pub(crate) async fn spawn_hang_http() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut chunk = [0u8; 4096];
        // WHY: keep reading so the client's request write succeeds, but never
        // write a response — the connection hangs until the task is dropped.
        loop {
            let n = stream.read(&mut chunk).await.unwrap_or(0);
            if n == 0 {
                std::future::pending::<()>().await;
            }
        }
    });

    (base_url, handle)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
