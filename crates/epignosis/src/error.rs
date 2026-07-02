use std::path::PathBuf;
use std::time::Duration;

use apotheke::DbError;
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

    #[snafu(display("failed to parse response FROM {provider}: {source}"))]
    ProviderParse {
        provider: String,
        source: serde_json::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("response FROM {provider} exceeds the {limit}-byte cap"))]
    ProviderResponseTooLarge {
        provider: String,
        limit: u64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("{provider} answered with HTTP {status}"))]
    ProviderHttpStatus {
        provider: String,
        status: reqwest::StatusCode,
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

    #[snafu(display("rate LIMIT exceeded for {provider}, retry after {retry_after:?}"))]
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

    #[snafu(display("failed to run fpcalc for {path:?}: {source}"))]
    FingerprintProcess {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("fpcalc output for {path:?} was not valid JSON: {source}"))]
    FingerprintOutputParse {
        path: PathBuf,
        source: serde_json::Error,
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
