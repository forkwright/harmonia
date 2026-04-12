use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum DbError {
    #[snafu(display("database pool initialization failed: {source}"))]
    PoolInit {
        source: sqlx::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("migration failed: {source}"))]
    Migration {
        source: sqlx::migrate::MigrateError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("query failed on {table}: {source}"))]
    Query {
        table: String,
        source: sqlx::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("record not found in {table}: {id}"))]
    NotFound {
        table: String,
        id: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
