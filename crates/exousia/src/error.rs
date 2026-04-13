use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ExousiaError {
    #[snafu(display("invalid credentials"))]
    InvalidCredentials {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("token has expired"))]
    TokenExpired {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("token is invalid: {error}"))]
    TokenInvalid {
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("user not found: {username}"))]
    UserNotFound {
        username: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("user account is inactive"))]
    UserInactive {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("API key has been revoked"))]
    ApiKeyRevoked {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("insufficient permission for this operation"))]
    InsufficientPermission {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("password hashing error: {error}"))]
    PasswordHash {
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("JWT encoding error: {source}"))]
    JwtEncode {
        source: jsonwebtoken::errors::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("JWT decoding error: {source}"))]
    JwtDecode {
        source: jsonwebtoken::errors::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error: {source}"))]
    Database {
        source: apotheke::DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
