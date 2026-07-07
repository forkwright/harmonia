use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::cf_bypass::{CloudflareProxy, Cookie, ProxyResponse};
use crate::error::{self, SearchIndexerError};

pub struct ByparrProxy {
    client: reqwest::Client,
    endpoint: String,
    timeout: Duration,
    max_body_bytes: u64,
}

#[derive(Debug, Serialize)]
struct ByparrRequest {
    cmd: &'static str,
    url: String,
    #[serde(rename = "maxTimeout")]
    max_timeout: u32,
}

#[derive(Debug, Deserialize)]
struct ByparrResponse {
    status: String,
    message: String,
    solution: Option<ByparrSolution>,
}

#[derive(Debug, Deserialize)]
struct ByparrSolution {
    #[expect(dead_code)]
    url: String,
    status: u16,
    response: String,
    cookies: Vec<ByparrCookie>,
    #[serde(rename = "userAgent")]
    user_agent: String,
}

#[derive(Debug, Deserialize)]
struct ByparrCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    expires: f64,
    #[serde(rename = "httpOnly")]
    http_only: bool,
    secure: bool,
}

impl ByparrProxy {
    pub fn new(endpoint: String, timeout: Duration, max_body_bytes: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout + Duration::from_secs(5))
            .build()
            .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config

        Self {
            client,
            endpoint,
            timeout,
            max_body_bytes,
        }
    }
}

impl CloudflareProxy for ByparrProxy {
    fn get(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProxyResponse, SearchIndexerError>> + Send + '_>> {
        let url = url.to_string();
        Box::pin(async move { self.get_inner(&url, ct).await })
    }
}

impl ByparrProxy {
    // WHY: skip `url` — the indexer URL embeds `apikey=` and #[instrument]
    // would capture the raw value as a span field visible to any event
    // emitted in the span.
    #[instrument(skip(self, url, ct), fields(endpoint = %self.endpoint))]
    async fn get_inner(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<ProxyResponse, SearchIndexerError> {
        let req = ByparrRequest {
            cmd: "request.get",
            url: url.to_string(),
            max_timeout: self.timeout.as_millis() as u32,
        };

        let endpoint_url = format!("{}/v1", self.endpoint.trim_end_matches('/'));

        // WHY: errors embed the redacted form so the API key never reaches a
        // log line; the request itself still carries the real URL.
        let redacted = crate::client::redact_api_key(url);
        let response = tokio::select! {
            result = self.client.post(&endpoint_url).json(&req).send() => {
                result.context(error::HttpRequestSnafu { url: redacted.clone() })?
            }
            () = ct.cancelled() => {
                return Err(SearchIndexerError::Cancelled {
                    url: redacted,
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        };

        // WHY: enforce the body-size cap HERE — the cf_bypass fetch paths
        // return this ProxyResponse.body directly, skipping read_body_bounded,
        // so without a cap on the byparr envelope `max_response_body_bytes` is
        // a no-op for cf_bypass indexers. The cap bounds the byparr JSON
        // (Content-Length precheck + streamed running counter) before it is
        // buffered; the wrapped indexer body is a subset, so the effective
        // ceiling is conservative by the envelope overhead.
        let raw = crate::client::read_body_bytes_bounded(response, &redacted, self.max_body_bytes)
            .await?;
        let byparr_resp: ByparrResponse =
            serde_json::from_slice(&raw).map_err(|e| SearchIndexerError::ParseResponse {
                url: redacted.clone(),
                error: e.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        if byparr_resp.status != "ok" {
            return Err(SearchIndexerError::CfProxyError {
                url: redacted.clone(),
                status: byparr_resp.status,
                message: byparr_resp.message,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }

        let solution = byparr_resp
            .solution
            .ok_or_else(|| SearchIndexerError::CfProxyError {
                url: redacted,
                status: "ok".to_string(),
                message: "no solution in response".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        let cookies = solution
            .cookies
            .into_iter()
            .map(|c| Cookie {
                name: c.name,
                value: c.value,
                domain: c.domain,
                path: c.path,
                expires: c.expires,
                http_only: c.http_only,
                secure: c.secure,
            })
            .collect();

        Ok(ProxyResponse {
            status: solution.status,
            body: solution.response,
            cookies,
            user_agent: solution.user_agent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::spawn_one_shot_http;

    #[tokio::test]
    async fn get_enforces_body_size_cap() {
        // WHY: the cf_bypass fetch paths return ByparrProxy's body directly,
        // so the cap must live here or `max_response_body_bytes` is a no-op
        // for cf_bypass indexers. A byparr envelope declaring a size above the
        // cap must be rejected before it is buffered.
        let big = "x".repeat(10_000);
        let json = format!(
            r#"{{"status":"ok","message":"","solution":{{"url":"https://e.example","status":200,"response":"{big}","cookies":[],"userAgent":"UA"}}}}"#
        );
        let (endpoint, _server) = spawn_one_shot_http(200, "OK", &[], &json).await;
        let proxy = ByparrProxy::new(endpoint, Duration::from_secs(5), 64);
        let err = proxy
            .get(
                "https://indexer.example/api?apikey=secret",
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, SearchIndexerError::ResponseTooLarge { limit: 64, .. }),
            "expected ResponseTooLarge, got {err:?}"
        );
    }

    #[test]
    fn byparr_request_serialization() {
        let req = ByparrRequest {
            cmd: "request.get",
            url: "https://example.com".to_string(),
            max_timeout: 60000,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["cmd"], "request.get");
        assert_eq!(json["maxTimeout"], 60000);
    }

    #[test]
    fn byparr_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "message": "Challenge solved!",
            "solution": {
                "url": "https://example.com",
                "status": 200,
                "response": "<html>test</html>",
                "cookies": [
                    {
                        "name": "cf_clearance",
                        "value": "abc123",
                        "domain": ".example.com",
                        "path": "/",
                        "expires": 1709500000.0,
                        "httpOnly": false,
                        "secure": true
                    }
                ],
                "userAgent": "Mozilla/5.0 Test"
            }
        }"#;

        let resp: ByparrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "ok");
        let solution = resp.solution.unwrap();
        assert_eq!(solution.status, 200);
        assert_eq!(solution.cookies.len(), 1);
        assert_eq!(solution.cookies[0].name, "cf_clearance");
        assert_eq!(solution.user_agent, "Mozilla/5.0 Test");
    }

    #[test]
    fn byparr_error_response_deserialization() {
        let json = r#"{
            "status": "error",
            "message": "Error: Unable to solve the challenge. Timeout.",
            "solution": null
        }"#;

        let resp: ByparrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "error");
        assert!(resp.solution.is_none());
    }
}
