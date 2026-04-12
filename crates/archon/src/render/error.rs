// Error types for the renderer subsystem.

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum RenderError {
    #[snafu(display("renderer config error: {source}"))]
    Config {
        source: Box<figment::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("protocol error: {message}"))]
    Protocol {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("QUIC connection error: {message}"))]
    Connection {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("audio output error: {message}"))]
    AudioOutput {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("TLS error: {message}"))]
    Tls {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("I/O error: {source}"))]
    Io {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("address parse error: {source}"))]
    AddrParse {
        source: std::net::AddrParseError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

impl From<quinn::ConnectionError> for RenderError {
    fn from(e: quinn::ConnectionError) -> Self {
        RenderError::Connection {
            message: e.to_string(),
            location: snafu::location!(),
        }
    }
}

impl From<quinn::WriteError> for RenderError {
    fn from(e: quinn::WriteError) -> Self {
        RenderError::Connection {
            message: e.to_string(),
            location: snafu::location!(),
        }
    }
}

impl From<quinn::ReadExactError> for RenderError {
    fn from(e: quinn::ReadExactError) -> Self {
        RenderError::Connection {
            message: e.to_string(),
            location: snafu::location!(),
        }
    }
}
