use apotheke::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum KritikeError {
    #[snafu(display("quality profile {id} not found"))]
    ProfileNotFound {
        id: i64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("invalid quality score: {score}"))]
    InvalidScore {
        score: i32,
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
