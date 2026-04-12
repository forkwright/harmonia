use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::cf_bypass::{CloudflareProxy, Cookie, ProxyResponse};
use crate::error::{self, ZetesisError};

pub struct ByparrProxy {
    client: reqwest::Client,
    endpoint: String,
    timeout: Duration,
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
    pub fn new(endpoint: String, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout + Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            endpoint,
            timeout,
        }
    }

    pub async fn health_check(&self) -> bool {
        let req = ByparrRequest {
            cmd: "request.get",
            url: "http://localhost".to_string(),
            max_timeout: 5000,
        };

        let url = format!("{}/v1", self.endpoint.trim_end_matches('/'));
        matches!(
            self.client.post(&url).json(&req).send().await,
            Ok(resp) if resp.status().is_success() || resp.status().is_client_error()
        )
    }
}

impl CloudflareProxy for ByparrProxy {
    fn get(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProxyResponse, ZetesisError>> + Send + '_>> {
        let url = url.to_string();
        Box::pin(async move { self.get_inner(&url, ct).await })
    }
}

impl ByparrProxy {
    #[instrument(skip(self, ct), fields(endpoint = %self.endpoint))]
    async fn get_inner(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<ProxyResponse, ZetesisError> {
        let req = ByparrRequest {
            cmd: "request.get",
            url: url.to_string(),
            max_timeout: self.timeout.as_millis() as u32,
        };

        let endpoint_url = format!("{}/v1", self.endpoint.trim_end_matches('/'));

        let response = tokio::select! {
            result = self.client.post(&endpoint_url).json(&req).send() => {
                result.context(error::HttpRequestSnafu { url })?
            }
            () = ct.cancelled() => {
                return Err(ZetesisError::CfProxyTimeout {
                    url: url.to_string(),
                    timeout: self.timeout.as_secs() as u32,
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        };

        let byparr_resp: ByparrResponse = response
            .json()
            .await
            .context(error::HttpRequestSnafu { url })?;

        if byparr_resp.status != "ok" {
            return Err(ZetesisError::CfProxyError {
                url: url.to_string(),
                status: byparr_resp.status,
                message: byparr_resp.message,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }

        let solution = byparr_resp
            .solution
            .ok_or_else(|| ZetesisError::CfProxyError {
                url: url.to_string(),
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
