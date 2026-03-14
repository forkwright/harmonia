//! Error types for the syndesmos external API integration crate.

use harmonia_db::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum SyndesmodError {
    #[snafu(display("Plex API call failed: {source}"))]
    PlexApiCall {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Last.fm API call failed: {source}"))]
    LastfmApiCall {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Tidal API call failed: {source}"))]
    TidalApiCall {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("rate limit exceeded for {service}"))]
    RateLimitExceeded {
        service: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("external service {service} is down (circuit breaker open)"))]
    ExternalServiceDown {
        service: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("authentication failed for {service}"))]
    AuthenticationFailed {
        service: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("integration {service} is not configured"))]
    ConfigMissing {
        service: String,
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
