use snafu::Snafu;

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum CommonError {
    #[snafu(display("invalid media type: {value}"))]
    InvalidMediaType {
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
