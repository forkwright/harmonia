//! AitesisError — typed errors for the request management subsystem.

use apotheke::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
/// Errors returned by the request management subsystem.
pub enum AitesisError {
    /// The user exceeded a configured pending or daily request limit.
    #[snafu(display("request LIMIT exceeded"))]
    RequestLimitExceeded {
        /// Source location captured when the error is constructed.
        #[snafu(implicit)]
        location: snafu::Location,
    },

    /// The requested media request row does not exist.
    #[snafu(display("request not found: {id}"))]
    RequestNotFound {
        /// Missing request identifier.
        id: String,
        /// Source location captured when the error is constructed.
        #[snafu(implicit)]
        location: snafu::Location,
    },

    /// A duplicate request already exists.
    #[snafu(display("request already exists"))]
    RequestAlreadyExists {
        /// Source location captured when the error is constructed.
        #[snafu(implicit)]
        location: snafu::Location,
    },

    /// The requested media could not be validated by identity lookup.
    #[snafu(display("media identity invalid: {detail}"))]
    MediaIdentityInvalid {
        /// Validation failure detail.
        detail: String,
        /// Source location captured when the error is constructed.
        #[snafu(implicit)]
        location: snafu::Location,
    },

    /// A request status transition is not allowed by the workflow.
    #[snafu(display("invalid status transition: {from} -> {to}"))]
    InvalidTransition {
        /// Current status.
        from: String,
        /// Requested next status.
        to: String,
        /// Source location captured when the error is constructed.
        #[snafu(implicit)]
        location: snafu::Location,
    },

    /// The user does not have permission for the requested action.
    #[snafu(display("insufficient permission for this action"))]
    InsufficientPermission {
        /// Source location captured when the error is constructed.
        #[snafu(implicit)]
        location: snafu::Location,
    },

    /// Storage layer failure.
    #[snafu(display("database error: {source}"))]
    Database {
        /// Database source error.
        source: DbError,
        /// Source location captured when the error is constructed.
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
