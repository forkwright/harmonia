use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum HostError {
    #[snafu(display("configuration error: {source}"))]
    Config {
        source: horismos::HorismosError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error: {source}"))]
    Database {
        source: harmonia_db::DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("scanner error: {source}"))]
    Scanner {
        #[snafu(source(from(taxis::error::TaxisError, Box::new)))]
        source: Box<taxis::error::TaxisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("feed scheduler error: {source}"))]
    FeedScheduler {
        #[snafu(source(from(komide::KomideError, Box::new)))]
        source: Box<komide::KomideError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("server error: {source}"))]
    Server {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("auth error: {source}"))]
    Auth {
        #[snafu(source(from(exousia::ExousiaError, Box::new)))]
        source: Box<exousia::ExousiaError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("tracing init error: {message}"))]
    Tracing {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
