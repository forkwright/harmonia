use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum HorismosError {
    #[snafu(display("configuration parse error: {source}"))]
    ConfigParse {
        #[snafu(source(from(figment::Error, Box::new)))]
        source: Box<figment::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("configuration validation failed: {message}"))]
    Validation {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("configuration merge round-trip failed: {source}"))]
    MergeRoundTrip {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("configuration merge failed: path '{path}' missing from config tree"))]
    MergePath {
        path: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
