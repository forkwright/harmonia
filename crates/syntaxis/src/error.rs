//! Error types for the syntaxis crate.

use apotheke::DbError;
use ergasia::ErgasiaError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum SyntaxisError {
    #[snafu(display("failed to enqueue download: {reason}"))]
    EnqueueFailed {
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("download queue is full"))]
    QueueFull {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("queue item not found: {id}"))]
    ItemNotFound {
        id: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("post-processing pipeline aborted: {reason}"))]
    PipelineAborted {
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("archive extraction failed"))]
    ExtractionFailed {
        source: ErgasiaError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("import failed: {reason}"))]
    ImportFailed {
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("retry budget exhausted after {attempts} attempts"))]
    RetryExhausted {
        attempts: u32,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error"))]
    Database {
        source: DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to dispatch download to engine"))]
    DispatchFailed {
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
