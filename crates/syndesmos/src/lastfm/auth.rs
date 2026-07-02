//! Last.fm Web Authentication — api_key + shared secret → session key.
//!
//! The user completes the auth flow once; the resulting session key is stored
//! in config. Subsequent calls use the stored key directly.

use snafu::ResultExt;
use tracing::instrument;

use crate::error::{LastfmApiCallSnafu, SyndesmodError};

const LASTFM_AUTH_URL: &str = "https://www.last.fm/api/auth";
const LASTFM_API_URL: &str = "https://ws.audioscrobbler.com/2.0";

/// Generates the Last.fm authorization URL for the user to visit.
///
/// After granting access, the user receives a token that must be exchanged
/// via `exchange_token` for a session key.
pub fn authorization_url(api_key: &str) -> String {
    format!("{}/?api_key={}", LASTFM_AUTH_URL, api_key)
}

/// MD5 signature as required by Last.fm API for authenticated calls.
///
/// The signature is `MD5(sorted_params + shared_secret)` where params are
/// concatenated as `key1value1key2value2...` in alphabetical order.
pub fn sign_params(params: &[(&str, &str)], shared_secret: &str) -> String {
    let mut sorted: Vec<(&str, &str)> = params
        .iter()
        .filter(|(k, _)| *k != "format")
        .copied()
        .collect();
    sorted.sort_by_key(|(k, _)| *k);

    let mut input = String::new();
    for (k, v) in &sorted {
        input.push_str(k);
        input.push_str(v);
    }
    input.push_str(shared_secret);

    md5_hex(input.as_bytes())
}

fn md5_hex(data: &[u8]) -> String {
    // WHY: MD5 is mandated by the Last.fm API signature protocol; this is a
    // protocol requirement, not a security choice.
    format!("{:x}", md5::compute(data))
}

/// Exchanges a temporary token for a Last.fm session key.
///
/// The session key must be stored in config after this call.
#[instrument(skip(http, api_key, shared_secret))]
pub async fn exchange_token(
    http: &reqwest::Client,
    api_key: &str,
    shared_secret: &str,
    token: &str,
) -> Result<String, SyndesmodError> {
    exchange_token_at(http, LASTFM_API_URL, api_key, shared_secret, token).await
}

// WHY: the api_url parameter exists so tests can point the exchange at a
// local mock server; production always passes LASTFM_API_URL.
async fn exchange_token_at(
    http: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    shared_secret: &str,
    token: &str,
) -> Result<String, SyndesmodError> {
    let sig_params = [
        ("api_key", api_key),
        ("method", "auth.getSession"),
        ("token", token),
    ];
    let api_sig = sign_params(&sig_params, shared_secret);

    let response = http
        .post(api_url)
        .form(&[
            ("method", "auth.getSession"),
            ("api_key", api_key),
            ("token", token),
            ("api_sig", &api_sig),
            ("format", "json"),
        ])
        .send()
        .await
        .context(LastfmApiCallSnafu)?;

    let body: serde_json::Value = response.json().await.context(LastfmApiCallSnafu)?;
    parse_session_key(&body)
}

fn parse_session_key(body: &serde_json::Value) -> Result<String, SyndesmodError> {
    body.get("session")
        .and_then(|s| s.get("key"))
        .and_then(|k| k.as_str())
        .map(|k| k.to_string())
        .ok_or_else(|| SyndesmodError::AuthenticationFailed {
            service: "lastfm".to_string(),
            location: snafu::location!(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_url_contains_api_key() {
        let url = authorization_url("mykey123");
        assert!(url.contains("api_key=mykey123"));
        assert!(url.contains("last.fm/api/auth"));
    }

    #[test]
    fn sign_params_sorts_keys_alphabetically() {
        // WHY: Last.fm signature spec requires params sorted by key before hashing.
        let params = [("track", "Roygbiv"), ("artist", "Boards of Canada")];
        let sig1 = sign_params(&params, "secret");
        let sig2 = sign_params(
            &[("artist", "Boards of Canada"), ("track", "Roygbiv")],
            "secret",
        );
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn sign_params_excludes_format_key() {
        let params_with_format = [("method", "track.scrobble"), ("format", "json")];
        let params_without_format = [("method", "track.scrobble")];
        let sig_with = sign_params(&params_with_format, "secret");
        let sig_without = sign_params(&params_without_format, "secret");
        assert_eq!(sig_with, sig_without);
    }

    #[test]
    fn md5_hex_matches_known_answer_vector() {
        // WHY: RFC 1321 test vector — fails if md5_hex regresses to any
        // non-MD5 encoding of the input bytes.
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn md5_hex_output_is_fixed_width_regardless_of_input_length() {
        // WHY: the old defect hex-encoded raw bytes, so output length scaled
        // with input length; a real digest is always 32 hex chars.
        assert_eq!(md5_hex(b"").len(), 32);
        assert_eq!(md5_hex(&[0u8; 1024]).len(), 32);
    }

    #[test]
    fn parse_session_key_extracts_key_from_valid_response() {
        let body = serde_json::json!({"session": {"key": "abc123", "name": "user"}});
        assert_eq!(parse_session_key(&body).unwrap(), "abc123");
    }

    #[test]
    fn parse_session_key_errors_on_missing_session() {
        let body = serde_json::json!({"error": 4, "message": "Invalid token"});
        let err = parse_session_key(&body).unwrap_err();
        assert!(
            matches!(err, SyndesmodError::AuthenticationFailed { ref service, .. } if service == "lastfm"),
            "expected AuthenticationFailed for lastfm, got {err:?}"
        );
    }

    #[tokio::test]
    async fn exchange_token_posts_signed_params_and_returns_key() {
        let (url, server) = crate::test_support::spawn_one_shot_http(
            200,
            "OK",
            r#"{"session":{"key":"session-key-789"}}"#,
        )
        .await;

        let key = exchange_token_at(&reqwest::Client::new(), &url, "key123", "sekrit", "tok456")
            .await
            .unwrap();
        assert_eq!(key, "session-key-789");

        let request = server.await.unwrap();
        let expected_sig = sign_params(
            &[
                ("api_key", "key123"),
                ("method", "auth.getSession"),
                ("token", "tok456"),
            ],
            "sekrit",
        );
        assert!(
            request.contains(&format!("api_sig={expected_sig}")),
            "request must carry the signed api_sig, got: {request}"
        );
        assert!(request.contains("method=auth.getSession"));
        assert!(request.contains("format=json"));
    }

    #[test]
    fn sign_params_matches_known_answer_signature() {
        // WHY: independently computed offline via `md5sum` over the exact
        // signing string "api_keykey123methodauth.getSessiontokentok456sekrit".
        let params = [
            ("method", "auth.getSession"),
            ("api_key", "key123"),
            ("token", "tok456"),
        ];
        let sig = sign_params(&params, "sekrit");
        assert_eq!(sig, "e0dcf82c53e5959c164a060bd886d6ff");
    }
}
