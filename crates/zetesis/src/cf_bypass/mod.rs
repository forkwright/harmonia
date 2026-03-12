pub mod byparr;
pub mod cookies;
pub mod noop;

use std::future::Future;
use std::pin::Pin;

use tokio_util::sync::CancellationToken;

use crate::error::ZetesisError;

#[derive(Debug)]
pub struct ProxyResponse {
    pub status: u16,
    pub body: String,
    pub cookies: Vec<Cookie>,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
}

pub trait CloudflareProxy: Send + Sync {
    fn get(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProxyResponse, ZetesisError>> + Send + '_>>;
}
