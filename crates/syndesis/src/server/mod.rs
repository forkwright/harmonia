/// QUIC streaming server: accepts renderer connections and streams audio.
pub mod session;
pub mod source;

use snafu::ResultExt;
use std::net::SocketAddr;
use tokio::task::JoinSet;
use tracing::{info, instrument, warn};

use crate::error::{self, SyndesisError};
use crate::tls;

pub use session::StreamSession;
pub use source::AudioSource;

pub struct StreamServer {
    endpoint: quinn::Endpoint,
    sessions: JoinSet<()>,
}

impl StreamServer {
    /// Bind a QUIC streaming server to the given address.
    #[instrument(skip_all, fields(%bind_addr))]
    pub fn bind(bind_addr: SocketAddr) -> Result<Self, SyndesisError> {
        let (certs, key) = tls::generate_self_signed(&["localhost".into()])?;
        let server_config = tls::build_server_config(certs, key)?;

        let endpoint =
            quinn::Endpoint::server(server_config, bind_addr).context(error::BindSnafu)?;

        info!("syndesis server listening");
        Ok(Self {
            endpoint,
            sessions: JoinSet::new(),
        })
    }

    /// Bind with a pre-built server config (for testing or custom certs).
    pub fn bind_with_config(
        bind_addr: SocketAddr,
        server_config: quinn::ServerConfig,
    ) -> Result<Self, SyndesisError> {
        let endpoint =
            quinn::Endpoint::server(server_config, bind_addr).context(error::BindSnafu)?;
        Ok(Self {
            endpoint,
            sessions: JoinSet::new(),
        })
    }

    /// Accept incoming connections and spawn session handlers.
    /// Runs until the endpoint is closed or the provided cancellation token fires.
    #[instrument(skip_all)]
    pub async fn run<S: AudioSource + Clone + Send + 'static>(
        &mut self,
        source: S,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) {
        loop {
            tokio::select! {
                biased;
                _ = wait_for_cancel(&cancel) => {
                    info!("server shutting down");
                    break;
                }
                incoming = self.endpoint.accept() => {
                    let Some(incoming) = incoming else {
                        info!("endpoint closed");
                        break;
                    };
                    let source = source.clone();
                    let cancel = cancel.clone();
                    self.sessions.spawn(async move {
                        match incoming.await {
                            Ok(conn) => {
                                let addr = conn.remote_address();
                                info!(%addr, "renderer connected");
                                let mut session = StreamSession::new(conn);
                                if let Err(e) = session.run(source, cancel).await {
                                    warn!(%addr, error = %e, "session ended with error");
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to accept connection");
                            }
                        }
                    });
                }
            }
        }
        self.sessions.shutdown().await;
        self.endpoint.close(0u32.into(), b"shutdown");
    }

    /// The local address the server is bound to.
    #[must_use]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.endpoint.local_addr().ok()
    }
}

async fn wait_for_cancel(cancel: &tokio::sync::watch::Receiver<bool>) {
    let mut cancel = cancel.clone();
    loop {
        if *cancel.borrow() {
            return;
        }
        if cancel.changed().await.is_err() {
            return;
        }
    }
}
