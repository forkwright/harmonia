// Server-side QUIC listener for renderer connections.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, info, warn};

use super::error::RenderError;
use super::protocol::{
    self, MSG_SESSION_ACCEPT, MSG_SESSION_INIT, MSG_STATUS_REPORT, SessionAccept, SessionInit,
    StatusReport,
};
use super::tls;

pub(crate) const DEFAULT_QUIC_PORT: u16 = 4433;

#[derive(Debug, Clone)]
pub struct ConnectedRenderer {
    pub name: String,
    pub session_id: String,
    pub connected_at: Instant,
    pub last_status: Option<StatusReport>,
}

pub struct RendererRegistry {
    entries: RwLock<Vec<ConnectedRenderer>>,
}

impl Default for RendererRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    pub async fn add(&self, renderer: ConnectedRenderer) {
        let mut entries = self.entries.write().await;
        entries.push(renderer);
    }

    pub async fn remove(&self, session_id: &str) {
        let mut entries = self.entries.write().await;
        entries.retain(|e| e.session_id != session_id);
    }

    pub async fn update_status(&self, session_id: &str, status: StatusReport) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.iter_mut().find(|e| e.session_id == session_id) {
            entry.last_status = Some(status);
        }
    }

    pub async fn list(&self) -> Vec<RendererInfo> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .map(|e| {
                let (buffer_depth_ms, latency_ms, state, underrun_count) = match &e.last_status {
                    Some(s) => (
                        s.buffer_depth_ms,
                        s.latency_ms,
                        s.device_state.to_string(),
                        s.underrun_count,
                    ),
                    None => (0.0, 0.0, "connecting".to_string(), 0),
                };
                RendererInfo {
                    name: e.name.clone(),
                    session_id: e.session_id.clone(),
                    connected_secs: e.connected_at.elapsed().as_secs(),
                    buffer_depth_ms,
                    latency_ms,
                    state,
                    underrun_count,
                }
            })
            .collect()
    }
}

/// Serializable renderer status for REST API responses.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RendererInfo {
    pub name: String,
    pub session_id: String,
    pub connected_secs: u64,
    pub buffer_depth_ms: f64,
    pub latency_ms: f64,
    pub state: String,
    pub underrun_count: u64,
}

/// Implements `DynRendererRegistry` so the registry can be injected into paroche's AppState.
impl paroche::state::DynRendererRegistry for RendererRegistry {
    fn list_renderers(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Vec<paroche::state::RendererInfo>> + Send + '_>,
    > {
        Box::pin(async move {
            self.list()
                .await
                .into_iter()
                .map(|r| paroche::state::RendererInfo {
                    name: r.name,
                    session_id: r.session_id,
                    connected_secs: r.connected_secs,
                    buffer_depth_ms: r.buffer_depth_ms,
                    latency_ms: r.latency_ms,
                    state: r.state,
                    underrun_count: r.underrun_count,
                })
                .collect()
        })
    }
}

pub async fn start_renderer_server(
    listen_addr: SocketAddr,
    cert_dir: &Path,
    registry: Arc<RendererRegistry>,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    let server_config = tls::load_or_generate_server_config(cert_dir)?;
    let endpoint = quinn::Endpoint::server(server_config, listen_addr).map_err(|e| {
        RenderError::Connection {
            message: e.to_string(),
            location: snafu::location!(),
        }
    })?;

    info!(addr = %listen_addr, "renderer QUIC server listening");

    loop {
        let incoming = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            incoming = endpoint.accept() => match incoming {
                Some(i) => i,
                None => break,
            },
        };

        let registry = Arc::clone(&registry);
        let ct = shutdown.child_token();

        tokio::spawn(
            async move {
                if let Err(e) = handle_renderer_connection(incoming, registry, ct).await {
                    warn!(error = %e, "renderer connection handler failed");
                }
            }
            .instrument(tracing::info_span!("renderer_conn")),
        );
    }

    endpoint.close(0u32.into(), b"server shutting down");
    info!("renderer QUIC server stopped");
    Ok(())
}

async fn handle_renderer_connection(
    incoming: quinn::Incoming,
    registry: Arc<RendererRegistry>,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    let connection = incoming.await?;
    let remote = connection.remote_address();
    info!(remote = %remote, "renderer connected");

    // Accept bidirectional control stream.
    let (mut ctrl_send, mut ctrl_recv) = connection.accept_bi().await?;

    // Read SessionInit.
    let (msg_type, payload) = protocol::recv_message(&mut ctrl_recv).await?;
    if msg_type != MSG_SESSION_INIT {
        return Err(RenderError::Protocol {
            message: format!("expected SessionInit (0x01), got 0x{msg_type:02x}"),
            location: snafu::location!(),
        });
    }
    let init: SessionInit =
        serde_json::from_slice(&payload).map_err(|e| RenderError::Protocol {
            message: e.to_string(),
            location: snafu::location!(),
        })?;
    info!(name = %init.name, version = init.protocol_version, "session init received");

    let session_id = generate_session_id();

    // Send SessionAccept.
    let accept = SessionAccept {
        session_id: session_id.clone(),
        sample_rate: 44100,
        channels: 2,
    };
    let accept_payload = serde_json::to_vec(&accept).map_err(|e| RenderError::Protocol {
        message: e.to_string(),
        location: snafu::location!(),
    })?;
    protocol::send_message(&mut ctrl_send, MSG_SESSION_ACCEPT, &accept_payload).await?;

    info!(
        session_id = %session_id,
        name = %init.name,
        "session established"
    );

    registry
        .add(ConnectedRenderer {
            name: init.name.clone(),
            session_id: session_id.clone(),
            connected_at: Instant::now(),
            last_status: None,
        })
        .await;

    // Open unidirectional stream for audio frames.
    let _audio_send = connection.open_uni().await?;

    // Read status reports from the control stream until disconnection.
    let result = read_status_loop(&mut ctrl_recv, &registry, &session_id, shutdown).await;

    registry.remove(&session_id).await;
    info!(
        session_id = %session_id,
        name = %init.name,
        "renderer disconnected"
    );

    result
}

async fn read_status_loop(
    ctrl_recv: &mut quinn::RecvStream,
    registry: &RendererRegistry,
    session_id: &str,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    loop {
        let result = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            r = protocol::recv_message(ctrl_recv) => r,
        };

        match result {
            Ok((msg_type, payload)) => {
                if msg_type == MSG_STATUS_REPORT
                    && let Ok(status) = serde_json::from_slice::<StatusReport>(&payload)
                {
                    registry.update_status(session_id, status).await;
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::protocol::DeviceState;

    #[tokio::test]
    async fn registry_add_and_list() {
        let registry = RendererRegistry::new();
        registry
            .add(ConnectedRenderer {
                name: "test-renderer".into(),
                session_id: "abc123".into(),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;

        let list = registry.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "test-renderer");
        assert_eq!(list[0].session_id, "abc123");
    }

    #[tokio::test]
    async fn registry_remove() {
        let registry = RendererRegistry::new();
        registry
            .add(ConnectedRenderer {
                name: "a".into(),
                session_id: "s1".into(),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;
        registry
            .add(ConnectedRenderer {
                name: "b".into(),
                session_id: "s2".into(),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;

        registry.remove("s1").await;
        let list = registry.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "b");
    }

    #[tokio::test]
    async fn registry_update_status() {
        let registry = RendererRegistry::new();
        registry
            .add(ConnectedRenderer {
                name: "test".into(),
                session_id: "s1".into(),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;

        let status = StatusReport {
            buffer_depth_ms: 95.0,
            latency_ms: 42.0,
            device_state: DeviceState::Playing,
            underrun_count: 1,
        };
        registry.update_status("s1", status).await;

        let list = registry.list().await;
        assert!((list[0].buffer_depth_ms - 95.0).abs() < f64::EPSILON);
        assert_eq!(list[0].underrun_count, 1);
    }

    #[test]
    fn session_id_is_32_hex_chars() {
        let id = generate_session_id();
        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
