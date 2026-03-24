/// QUIC streaming protocol for multi-room audio transport.
pub mod client;
pub mod clock;
pub mod error;
pub mod protocol;
pub mod server;
pub mod tls;

pub use error::SyndesisError;
