/// QUIC streaming client: connects to server and receives audio frames.
pub mod buffer;
pub mod session;

use std::net::SocketAddr;

use snafu::ResultExt;
use tracing::{info, instrument};

use crate::error::{self, SyndesisError};
use crate::tls;

pub use buffer::JitterBuffer;
pub use session::ClientSession;

pub struct StreamClient {
    endpoint: quinn::Endpoint,
}

impl StreamClient {
    /// Create a new QUIC client bound to a local address.
    #[instrument(skip_all)]
    pub fn new(bind_addr: SocketAddr) -> Result<Self, SyndesisError> {
        let client_config = tls::build_client_config()?;
        let mut endpoint = quinn::Endpoint::client(bind_addr).context(error::BindSnafu)?;
        endpoint.set_default_client_config(client_config);
        Ok(Self { endpoint })
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
