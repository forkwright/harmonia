/// QUIC streaming client: connects to server and receives audio frames.
pub mod buffer;
pub mod session;

use std::net::SocketAddr;
use std::sync::Arc;

pub use buffer::JitterBuffer;
pub use session::ClientSession;
use snafu::ResultExt;
use tracing::{info, instrument};

use crate::error::{self, SyndesisError};
use crate::tls::{self, ObservedFingerprint};

pub struct StreamClient {
    endpoint: quinn::Endpoint,
}

impl StreamClient {
    /// Create a new QUIC client pinned to a known server-certificate
    /// fingerprint (hex-encoded SHA-256, persisted during pairing).
    #[instrument(skip_all)]
    pub fn new(bind_addr: SocketAddr, pinned_fingerprint: &str) -> Result<Self, SyndesisError> {
        let client_config = tls::build_client_config(pinned_fingerprint)?;
        let mut endpoint = quinn::Endpoint::client(bind_addr).context(error::BindSnafu)?;
        endpoint.set_default_client_config(client_config);
        Ok(Self { endpoint })
    }

    /// Create a client for the explicit first-contact pairing step.
    ///
    /// Returns the client plus the cell that receives the server fingerprint
    /// observed during the handshake. The caller MUST persist that fingerprint
    /// and construct every subsequent client via [`StreamClient::new`].
    #[instrument(skip_all)]
    pub fn for_pairing(
        bind_addr: SocketAddr,
    ) -> Result<(Self, Arc<ObservedFingerprint>), SyndesisError> {
        let (client_config, observed) = tls::build_pairing_client_config()?;
        let mut endpoint = quinn::Endpoint::client(bind_addr).context(error::BindSnafu)?;
        endpoint.set_default_client_config(client_config);
        Ok((Self { endpoint }, observed))
    }

    /// Create with a pre-built client config.
    pub fn with_config(
        bind_addr: SocketAddr,
        client_config: quinn::ClientConfig,
    ) -> Result<Self, SyndesisError> {
        let mut endpoint = quinn::Endpoint::client(bind_addr).context(error::BindSnafu)?;
        endpoint.set_default_client_config(client_config);
        Ok(Self { endpoint })
    }

    /// Connect to a streaming server and return a session.
    #[instrument(skip_all, fields(%server_addr))]
    pub async fn connect(
        &self,
        server_addr: SocketAddr,
        server_name: &str,
    ) -> Result<ClientSession, SyndesisError> {
        let conn = self
            .endpoint
            .connect(server_addr, server_name)
            .context(error::ConnectSnafu)?
            .await
            .context(error::ConnectionSnafu)?;

        info!("connected to streaming server");
        Ok(ClientSession::new(conn))
    }
}

impl Drop for StreamClient {
    fn drop(&mut self) {
        self.endpoint.close(0u32.into(), b"client closed");
    }
}
