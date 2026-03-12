use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum CommonError {
    #[snafu(display("invalid media type: {value}"))]
    InvalidMediaType {
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
