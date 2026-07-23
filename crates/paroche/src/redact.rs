//! Credential redaction for indexer URLs leaving the API boundary.

/// Replacement value for redacted credential query parameters.
const REDACTED: &str = "REDACTED";

/// Query keys that are credentials when matched exactly (case-insensitive).
///
/// `r` is the conventional Torznab passkey parameter; `api` is used bare by
/// several Newznab indexers.
const EXACT_CREDENTIAL_KEYS: &[&str] = &["r", "api"];

/// Substrings that mark a query key as credential-bearing (case-insensitive).
///
/// Covers the Torznab/Newznab conventions: `apikey`, `api_key`, `passkey`,
/// `torrent_pass`, `authkey`, `auth_key`, `token`, `secret`, and variants.
/// Over-matching is deliberate — a redacted-but-harmless parameter costs
/// nothing, a leaked passkey compromises a private-tracker account.
const CREDENTIAL_KEY_SUBSTRINGS: &[&str] = &["key", "pass", "token", "secret", "auth"];

fn is_credential_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    EXACT_CREDENTIAL_KEYS.contains(&key.as_str())
        || CREDENTIAL_KEY_SUBSTRINGS
            .iter()
            .any(|marker| key.contains(marker))
}

/// Redacts credential-bearing query parameter values from a download URL.
///
/// Torznab/Newznab download URLs embed the indexer `apikey`/`passkey` in the
/// query string; returning them raw hands the operator's private-tracker
/// credentials to any authenticated member. Only parameter VALUES are
/// replaced — keys, ordering, separators, and the fragment survive, so the
/// URL stays recognizable in the UI.
///
/// The transformation is purely lexical (split on `#`, `?`, `&`, `=`), so a
/// URL the `url` crate would reject (for example a magnet URI) is still
/// redacted rather than passed through raw.
#[must_use]
pub fn redact_download_url(url: &str) -> String {
    let (head, fragment) = match url.split_once('#') {
        Some((head, fragment)) => (head, Some(fragment)),
        None => (url, None),
    };
    let Some((base, query)) = head.split_once('?') else {
        return url.to_string();
    };

    let redacted_query = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((key, _)) if is_credential_key(key) => format!("{key}={REDACTED}"),
            _ => pair.to_string(),
        })
        .collect::<Vec<_>>()
        .join("&");

    match fragment {
        Some(fragment) => format!("{base}?{redacted_query}#{fragment}"),
        None => format!("{base}?{redacted_query}"),
    }
}

/// Recursively redacts every `download_url` string field in a JSON value.
///
/// Search results flow through paroche as opaque `serde_json::Value` trees
/// (indexer -> eksetasis -> route), so the redaction walks the tree instead of
/// a typed response struct.
pub fn redact_download_urls_in_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, entry) in map.iter_mut() {
                if key == "download_url" {
                    if let serde_json::Value::String(url) = entry {
                        *url = redact_download_url(url);
                    }
                } else {
                    redact_download_urls_in_json(entry);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                redact_download_urls_in_json(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_apikey_value_keeping_structure() {
        assert_eq!(
            redact_download_url("https://indexer.example/api?t=get&id=42&apikey=SECRET"),
            "https://indexer.example/api?t=get&id=42&apikey=REDACTED"
        );
    }

    #[test]
    fn redacts_all_conventional_credential_keys() {
        for key in [
            "apikey",
            "api_key",
            "passkey",
            "authkey",
            "auth_key",
            "token",
            "secret",
            "r",
            "torrent_pass",
            "password",
        ] {
            let url = format!("https://indexer.example/dl?{key}=SECRET&file=x.torrent");
            let redacted = redact_download_url(&url);
            assert_eq!(
                redacted,
                format!("https://indexer.example/dl?{key}=REDACTED&file=x.torrent"),
                "{key} must be redacted"
            );
        }
    }

    #[test]
    fn matches_keys_case_insensitively() {
        assert_eq!(
            redact_download_url("https://indexer.example/dl?ApiKey=SECRET"),
            "https://indexer.example/dl?ApiKey=REDACTED"
        );
    }

    #[test]
    fn keeps_non_credential_params_and_fragment() {
        assert_eq!(
            redact_download_url("https://indexer.example/dl?t=get&id=42#frag"),
            "https://indexer.example/dl?t=get&id=42#frag"
        );
    }

    #[test]
    fn leaves_query_free_urls_untouched() {
        assert_eq!(
            redact_download_url("https://indexer.example/dl/42.torrent"),
            "https://indexer.example/dl/42.torrent"
        );
    }

    #[test]
    fn redacts_magnet_uri_credentials_without_touching_xt() {
        assert_eq!(
            redact_download_url("magnet:?xt=urn:btih:abc123&passkey=SECRET"),
            "magnet:?xt=urn:btih:abc123&passkey=REDACTED"
        );
    }

    #[test]
    fn json_walk_redacts_nested_download_urls_only() {
        let mut value = serde_json::json!({
            "results": [{
                "title": "Album",
                "download_url": "https://indexer.example/dl?apikey=SECRET",
                "info_url": "https://indexer.example/details/42"
            }]
        });
        redact_download_urls_in_json(&mut value);
        assert_eq!(
            value["results"][0]["download_url"],
            "https://indexer.example/dl?apikey=REDACTED"
        );
        assert_eq!(
            value["results"][0]["info_url"],
            "https://indexer.example/details/42"
        );
    }
}
