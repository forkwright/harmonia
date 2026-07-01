/// QUIC streaming server: accepts renderer connections and streams audio.
pub mod auth;
pub mod session;
pub mod source;
pub mod zone;

use std::net::SocketAddr;

pub use auth::{
    SessionOutcome, build_pairing_challenge, build_pairing_complete, handle_session_init,
};
pub use session::StreamSession;
use snafu::ResultExt;
pub use source::AudioSource;
use tokio::task::JoinSet;
use tracing::{info, instrument, warn};
pub use zone::ZoneStream;

use crate::config::{ClockConfig, ServerConfig};
use crate::error::{self, SyndesisError};
use crate::tls;

pub struct StreamServer {
    endpoint: quinn::Endpoint,
    sessions: JoinSet<()>,
    server_config: ServerConfig,
    cert_fingerprint: Option<String>,
}

impl StreamServer {
    /// Bind a QUIC streaming server to the given address.
    #[instrument(skip_all, fields(%bind_addr))]
    pub fn bind(bind_addr: SocketAddr) -> Result<Self, SyndesisError> {
        Self::bind_with_server_config(bind_addr, ServerConfig::default())
    }

    /// Bind with explicit syndesis server tuning (session cap, watermarks).
    #[instrument(skip_all, fields(%bind_addr))]
    pub fn bind_with_server_config(
        bind_addr: SocketAddr,
        server_config: ServerConfig,
    ) -> Result<Self, SyndesisError> {
        let (certs, key) = tls::generate_self_signed(&["localhost".into()])?;
        let cert_fingerprint = certs.first().map(|c| tls::compute_fingerprint(c.as_ref()));
        let quinn_config = tls::build_server_config(certs, key)?;

        let endpoint =
            quinn::Endpoint::server(quinn_config, bind_addr).context(error::BindSnafu)?;

        info!("syndesis server listening");
        Ok(Self {
            endpoint,
            sessions: JoinSet::new(),
            server_config,
            cert_fingerprint,
        })
    }

    /// Bind with a pre-built quinn config (for testing or custom certs).
    pub fn bind_with_config(
        bind_addr: SocketAddr,
        server_config: quinn::ServerConfig,
    ) -> Result<Self, SyndesisError> {
        let endpoint =
            quinn::Endpoint::server(server_config, bind_addr).context(error::BindSnafu)?;
        Ok(Self {
            endpoint,
            sessions: JoinSet::new(),
            server_config: ServerConfig::default(),
            cert_fingerprint: None,
        })
    }

    /// Hex-encoded SHA-256 fingerprint of the server's TLS certificate.
    ///
    /// Clients pin this value (via pairing) to authenticate the server.
    /// `None` when the server was bound with a pre-built quinn config.
    #[must_use]
    pub fn cert_fingerprint(&self) -> Option<&str> {
        self.cert_fingerprint.as_deref()
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
                    // WHY: JoinSet::len() counts finished-but-unreaped tasks;
                    // reap first so the session cap reflects live sessions only.
                    while self.sessions.try_join_next().is_some() {}
                    if self.sessions.len() >= self.server_config.max_sessions {
                        // WHY: refuse pre-handshake (no TLS work spent) with a
                        // retryable CONNECTION_REFUSED instead of accepting and
                        // dropping mid-session.
                        warn!(
                            max_sessions = self.server_config.max_sessions,
                            "refusing connection: session limit reached"
                        );
                        incoming.refuse();
                        continue;
                    }
                    let source = source.clone();
                    let cancel = cancel.clone();
                    let server_config = self.server_config.clone();
                    self.sessions.spawn(async move {
                        match incoming.await {
                            Ok(conn) => {
                                let addr = conn.remote_address();
                                info!(%addr, "renderer connected");
                                let mut session = StreamSession::with_configs(
                                    conn,
                                    server_config,
                                    ClockConfig::default(),
                                );
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
