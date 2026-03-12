use std::future::Future;
use std::pin::Pin;

use tokio_util::sync::CancellationToken;

use crate::cf_bypass::{CloudflareProxy, ProxyResponse};
use crate::error::ZetesisError;

pub struct NoProxy;

impl CloudflareProxy for NoProxy {
    fn get(
        &self,
        url: &str,
        _ct: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProxyResponse, ZetesisError>> + Send + '_>> {
        let url = url.to_string();
        Box::pin(async move {
            Err(ZetesisError::NoCfBypass {
                url,
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn no_proxy_returns_error() {
        let proxy = NoProxy;
        let ct = CancellationToken::new();
        let result = proxy.get("https://example.com", ct).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ZetesisError::NoCfBypass { .. }),
            "expected NoCfBypass, got {err:?}"
        );
    }
}
