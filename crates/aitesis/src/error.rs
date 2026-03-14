//! AitesisError — typed errors for the request management subsystem.

use snafu::Snafu;

use harmonia_db::DbError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AitesisError {
    #[snafu(display("request limit exceeded"))]
    RequestLimitExceeded {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("request not found: {id}"))]
    RequestNotFound {
        id: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("request already exists"))]
    RequestAlreadyExists {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("media identity invalid: {detail}"))]
    MediaIdentityInvalid {
        detail: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("invalid status transition: {from} -> {to}"))]
    InvalidTransition {
        from: String,
        to: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("insufficient permission for this action"))]
    InsufficientPermission {
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
