use apotheke::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ZetesisError {
    #[snafu(display("HTTP request to indexer {url} failed"))]
    HttpRequest {
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to parse Torznab/Newznab XML response FROM {url}"))]
    ParseResponse {
        url: String,
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} returned auth failure (bad API key)"))]
    AuthFailed {
        indexer_id: i64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} rate limited — retry after {retry_after_seconds:?}s"))]
    RateLimited {
        indexer_id: i64,
        retry_after_seconds: Option<u64>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("CF bypass proxy not available for {url}"))]
    NoCfBypass {
        url: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("caps negotiation failed for indexer {indexer_id}"))]
    CapsUnavailable {
        indexer_id: i64,
        source: Box<ZetesisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Byparr did not respond within {timeout}s for {url}"))]
    CfProxyTimeout {
        url: String,
        timeout: u32,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Byparr returned error for {url}: [{status}] {message}"))]
    CfProxyError {
        url: String,
        status: String,
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("CF cookies expired for indexer {indexer_name}"))]
    CfCookieExpired {
        indexer_name: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error: {source}"))]
    Database {
        source: DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
