//! ProsthekeError — typed errors for the subtitle management subsystem.

use apotheke::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ProsthekeError {
    #[snafu(display("subtitle acquisition failed: {detail}"))]
    AcquisitionFailed {
        detail: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subtitle provider unavailable: {source}"))]
    ProviderDown {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("no subtitle found for language: {language}"))]
    LanguageNotFound {
        language: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("timing synchronization failed"))]
    TimingSyncFailed {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subtitle download failed: {detail}"))]
    DownloadFailed {
        detail: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subtitle file write failed: {source}"))]
    FileWriteFailed {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("match score {score:.2} below minimum threshold {min:.2}"))]
    MinScoreNotMet {
        score: f64,
        min: f64,
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
