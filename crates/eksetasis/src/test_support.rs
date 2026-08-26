//! Shared test fixtures — one-shot HTTP servers for indexer client tests.
#![cfg(test)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Installs the process-wide rustls crypto provider before any test in this
/// binary runs.
///
/// WHY #[ctor]: reqwest builds with `rustls-no-provider` (fleet convention:
/// install explicitly, never let a library link one implicitly — see
/// main.rs), so `reqwest::Client::new()`/`::builder().build()` panics ("No
/// rustls crypto provider is configured") in any process that never called
/// `install_default()` — and a nextest test binary never runs `main()`. A
/// per-site or per-helper install call only covers the construction paths
/// someone remembered to wire it into — this crate found a new one on every
/// sweep (newznab/torznab's shared `client()` helper, cardigann's
/// three-tier wrapper, two direct construction sites bypassing it, and a
/// registry/factory entry point bypassing all of the above). `#[ctor]` runs
/// once at process load, before any `#[test]`/`#[tokio::test]` function in
/// this binary executes, so every construction path is covered
/// unconditionally rather than by convention.
#[ctor::ctor(unsafe)]
fn install_test_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

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

/// Spawns a TCP server that answers `responses.len()` sequential connections,
/// one HTTP response each in order, and resolves to the request head from
/// every connection it served.
///
/// Each response carries its extra headers, a correct `Content-Length`, and
/// `Connection: close` so the client opens a fresh connection per request —
/// which keeps the request/response pairing deterministic for multi-hop
/// login flows.
pub(crate) async fn spawn_sequence_http(
    responses: Vec<(u16, Vec<(String, String)>, String)>,
) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let mut heads = Vec::with_capacity(responses.len());
        for (status, extra_headers, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            let header_end;
            loop {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0, "client closed before sending full headers");
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                    header_end = pos + 4;
                    break;
                }
            }
            // WHY: read the declared request body too, so multi-hop login
            // tests can assert on POST form bodies without depending on TCP
            // segmentation.
            if let Some(len) = content_length(&buf[..header_end]) {
                while buf.len() < header_end + len {
                    let n = stream.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
            }
            heads.push(String::from_utf8_lossy(&buf).into_owned());

            let mut response = format!("HTTP/1.1 {status} OK\r\n");
            for (name, value) in &extra_headers {
                response.push_str(&format!("{name}: {value}\r\n"));
            }
            response.push_str(&format!(
                "content-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            ));
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
            stream.shutdown().await.unwrap();
        }
        heads
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

/// Parses the `Content-Length` header value out of a request head.
fn content_length(head: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(head);
    text.lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        })
        .and_then(|(_, v)| v.trim().parse().ok())
}
