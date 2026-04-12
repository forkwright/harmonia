// syndesis error types
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

    #[snafu(display("TLS certificate generation failed: {source}"))]
    CertGen {
        source: rcgen::Error,
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

    #[snafu(display("argon2 error: {message}"))]
    Argon2 {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error: {source}"))]
    Database {
        source: apotheke::DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("renderer not found"))]
    RendererNotFound {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("renderer is disabled"))]
    RendererDisabled {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("API key is invalid"))]
    InvalidApiKey {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("cert fingerprint mismatch (TOFU violation)"))]
    FingerprintMismatch {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("frame serialization error: {source}"))]
    Frame {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
