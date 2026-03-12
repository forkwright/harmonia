use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
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
}
