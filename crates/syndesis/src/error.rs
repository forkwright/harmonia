/// Error types for the syndesis streaming protocol.
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum SyndesisError {
    #[snafu(display("protocol error: {reason}"))]
    Protocol {
        reason: &'static str,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("QUIC connection error: {source}"))]
    Connection {
        source: quinn::ConnectionError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("QUIC write error: {source}"))]
    WriteStream {
        source: quinn::WriteError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("QUIC read error: {source}"))]
    ReadStream {
        source: quinn::ReadError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("QUIC read-to-end error: {source}"))]
    ReadToEnd {
        source: quinn::ReadToEndError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("QUIC datagram send error: {reason}"))]
    Datagram {
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("TLS error: {reason}"))]
    Tls {
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("session negotiation failed: {reason}"))]
    Negotiation {
        reason: &'static str,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("I/O error: {source}"))]
    Io {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("endpoint bind failed: {source}"))]
    Bind {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("QUIC connect error: {source}"))]
    Connect {
        source: quinn::ConnectError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
