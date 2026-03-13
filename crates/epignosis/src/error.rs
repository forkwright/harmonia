use std::{path::PathBuf, time::Duration};

use harmonia_db::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum EpignosisError {
    #[snafu(display("request to {provider} failed: {source}"))]
    ProviderRequest {
        provider: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to parse response from {provider}: {source}"))]
    ProviderParse {
        provider: String,
        source: serde_json::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("request to {provider} timed out: {url}"))]
    ProviderTimeout {
        provider: String,
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("rate limit exceeded for {provider}, retry after {retry_after:?}"))]
    ProviderRateLimited {
        provider: String,
        retry_after: Option<Duration>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("no match found in {provider} for query: {query}"))]
    IdentityNotResolved {
        provider: String,
        query: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("audio fingerprint computation failed for {path:?}: {message}"))]
    FingerprintFailed {
        path: PathBuf,
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("cache error: {message}"))]
    Cache {
        message: String,
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
