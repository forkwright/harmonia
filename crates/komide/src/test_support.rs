//! Shared test fixtures — scripted in-process HTTP servers for feed fetch tests.
#![cfg(test)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Installs the process-wide rustls crypto provider for tests.
///
/// WHY: reqwest builds with `rustls-no-provider` (fleet convention: install
/// explicitly, never let a library link one implicitly — see main.rs), so
/// `reqwest::Client::new()`/`::builder().build()` panics ("No rustls crypto
/// provider is configured") in any process that never called
/// `install_default()` — and a nextest test binary never runs `main()`.
/// Safe to call repeatedly: install_default() on an already-installed
/// process just returns Err, discarded here.
pub(crate) fn install_test_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Builds a raw HTTP/1.1 response with a correct `Content-Length` header and
/// `connection: close`.
pub(crate) fn http_response(
    status: u16,
    reason: &str,
    extra_headers: &[(&str, &str)],
    body: &[u8],
) -> Vec<u8> {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in extra_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "content-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    ));
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// Builds a raw HTTP/1.1 response WITHOUT `Content-Length`; the body is
/// delimited by connection close, exercising the streamed read path.
pub(crate) fn http_response_close_delimited(status: u16, reason: &str, body: &[u8]) -> Vec<u8> {
    let mut bytes = format!("HTTP/1.1 {status} {reason}\r\nconnection: close\r\n\r\n").into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// Spawns a TCP server that answers one HTTP request per scripted response,
/// in sequence, then resolves to the raw request heads it received.
///
/// Every scripted response should carry `connection: close` so the client
/// opens a fresh connection for the next request. Write-side failures are
/// tolerated: a client that aborts mid-body (e.g. an over-cap reject) must
/// not panic the server task the test still joins.
pub(crate) async fn spawn_scripted_http(
    responses: Vec<Vec<u8>>,
) -> (String, JoinHandle<Vec<String>>) {
    install_test_crypto_provider();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
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

            if stream.write_all(&response).await.is_ok() {
                stream.flush().await.ok();
                stream.shutdown().await.ok();
            }
            requests.push(String::from_utf8_lossy(&buf).into_owned());
        }
        requests
    });

    (base_url, handle)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
