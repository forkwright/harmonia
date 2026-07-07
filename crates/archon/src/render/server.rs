// Server-side QUIC listener for renderer connections.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use horismos::{ParocheConfig, Section};
use themelion::{GateGuard, LiveGate};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, info, warn};

use super::error::{RenderError, UnauthorizedSnafu};
use super::protocol::{
    self, MSG_SESSION_ACCEPT, MSG_SESSION_INIT, MSG_STATUS_REPORT, RendererSessionId,
    SessionAccept, SessionInit, StatusReport,
};
use super::tls;

pub const DEFAULT_QUIC_PORT: u16 = 4433;

/// QUIC application close code sent when a peer fails authentication.
const CLOSE_CODE_AUTH_FAILURE: u32 = 0x1;

#[derive(Debug, Clone)]
pub struct ConnectedRenderer {
    pub name: String,
    pub session_id: RendererSessionId,
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

    pub async fn remove(&self, session_id: &RendererSessionId) {
        let mut entries = self.entries.write().await;
        entries.retain(|e| e.session_id != *session_id);
    }

    pub async fn update_status(&self, session_id: &RendererSessionId, status: StatusReport) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.iter_mut().find(|e| e.session_id == *session_id) {
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
    pub session_id: RendererSessionId,
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
                    session_id: r.session_id.0,
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
    section: Section<ParocheConfig>,
) -> Result<(), RenderError> {
    let server_config = tls::load_or_generate_server_config(cert_dir)?;
    let endpoint = quinn::Endpoint::server(server_config, listen_addr).map_err(|e| {
        RenderError::Connection {
            message: e.to_string(),
            location: snafu::location!(),
        }
    })?;

    let boot = section.get();
    if boot.renderer_api_key.as_deref().is_none_or(str::is_empty) {
        warn!(
            "paroche.renderer_api_key not configured; rejecting every renderer registration \
             until a key is SET"
        );
    }

    info!(
        addr = %listen_addr,
        max_connections = boot.renderer_max_connections,
        "renderer QUIC server listening"
    );

    // INVARIANT: bounds concurrent renderer connections/tasks server-wide —
    // mirrors syndesis::ServerConfig::max_sessions, the sibling QUIC admission
    // surface in this workspace. A guard is held for the connection's full
    // handling lifetime (see below), not just the pre-auth handshake, so an
    // unauthenticated peer that completes TLS then stalls (bounded separately
    // by session_init_timeout) still counts against the cap. Unlike the
    // fixed `Semaphore` this replaces, the cap is read LIVE from `section` on
    // every admission attempt: a reload changes it without rebuilding the
    // gate. A LOWERED cap applies to new admissions only — live connections
    // are never force-closed on a cap decrease.
    let admission = Arc::new(LiveGate::new());

    loop {
        let incoming = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            incoming = endpoint.accept() => match incoming {
                Some(i) => i,
                None => break,
            },
        };

        let max_connections = section.get().renderer_max_connections;
        let (incoming, guard) = match try_admit(incoming, &admission, max_connections) {
            Some(pair) => pair,
            None => continue,
        };

        let registry = Arc::clone(&registry);
        let conn_section = section.clone();
        let ct = shutdown.child_token();

        tokio::spawn(
            async move {
                // WHY: held until this task ends (success, error, or panic
                // unwind) so the admission cap always reflects live
                // connections, not just ones still in the pre-auth handshake.
                let _guard = guard;
                if let Err(e) =
                    handle_renderer_connection(incoming, registry, conn_section, ct).await
                {
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

/// Attempts to admit a freshly-accepted (pre-handshake) connection under the
/// concurrency cap. Refuses and returns `None` when the cap is reached,
/// spending no TLS work — mirrors `syndesis::StreamServer`'s session cap.
fn try_admit(
    incoming: quinn::Incoming,
    admission: &Arc<LiveGate>,
    max_connections: usize,
) -> Option<(quinn::Incoming, GateGuard)> {
    match admission.try_enter(max_connections) {
        Some(guard) => Some((incoming, guard)),
        None => {
            warn!(
                max_connections,
                "refusing renderer connection: admission cap reached"
            );
            incoming.refuse();
            None
        }
    }
}

async fn handle_renderer_connection(
    incoming: quinn::Incoming,
    registry: Arc<RendererRegistry>,
    section: Section<ParocheConfig>,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    let connection = incoming.await?;
    let remote = connection.remote_address();
    info!(remote = %remote, "renderer connected");

    // WHY: ONE snapshot for the whole handshake — the timeout deadline below
    // and the api key checked against it further down both come from this
    // same read, so a reload racing this connection cannot mix an old
    // timeout with a new key (the #529 single-op-snapshot invariant:
    // handle.rs's documented `Section::get()` idiom, one read per operation).
    let cfg = section.get();
    let session_init_timeout = Duration::from_secs(cfg.renderer_session_init_timeout_secs);

    // INVARIANT: bounds the whole pre-auth handshake (control-stream accept +
    // SessionInit read) so a peer that completes TLS then never sends
    // SessionInit cannot hold the connection — and its admission-cap slot —
    // open indefinitely.
    let (mut ctrl_send, mut ctrl_recv, init) =
        tokio::time::timeout(session_init_timeout, accept_session_init(&connection))
            .await
            .map_err(|_| RenderError::Connection {
                message: format!(
                    "renderer {remote} did not complete SessionInit within \
                     {session_init_timeout:?}"
                ),
                location: snafu::location!(),
            })??;
    validate_session_init(&init)?;
    info!(name = %init.name, version = init.protocol_version, "session init received");

    // INVARIANT: no session state (session_id, registry entry, streams) exists
    // before this guard passes; unauthenticated peers cannot register. A
    // rotated key affects NEW registrations only — this check uses the
    // snapshot taken above at connection-accept time, so an in-progress
    // handshake is judged consistently even if a reload lands mid-handshake;
    // established sessions (past this point) are never re-challenged or
    // evicted by a later rotation.
    if !api_key_matches(cfg.renderer_api_key.as_deref(), &init.api_key) {
        warn!(remote = %remote, name = %init.name, "renderer authentication failed; rejecting");
        connection.close(CLOSE_CODE_AUTH_FAILURE.into(), b"authentication failed");
        return UnauthorizedSnafu { name: init.name }.fail();
    }

    let session_id = RendererSessionId(generate_session_id());

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

    // WHY: the guard's Drop removes the entry even when run_renderer_session
    // panics and unwinds — the spawn wrapper only logs the JoinError, so
    // without the guard a panicking session leaked a stale registry entry.
    let _cleanup = RegistryCleanupGuard {
        registry: Arc::clone(&registry),
        session_id: session_id.clone(),
    };

    // INVARIANT: every fallible operation between add and remove lives inside
    // run_renderer_session, so no `?` can return past the registry cleanup.
    let result = run_renderer_session(
        &connection,
        &mut ctrl_recv,
        &registry,
        &session_id,
        shutdown,
    )
    .await;

    registry.remove(&session_id).await;
    info!(
        session_id = %session_id,
        name = %init.name,
        "renderer disconnected"
    );

    result
}

/// Accepts the control bidirectional stream and reads/parses the SessionInit
/// frame. Callers wrap this in a deadline — see `handle_renderer_connection`
/// — since it runs entirely on the unauthenticated pre-auth path.
async fn accept_session_init(
    connection: &quinn::Connection,
) -> Result<(quinn::SendStream, quinn::RecvStream, SessionInit), RenderError> {
    let (ctrl_send, mut ctrl_recv) = connection.accept_bi().await?;

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
    Ok((ctrl_send, ctrl_recv, init))
}

/// Removes the session's registry entry on drop, covering panic unwinds.
/// Removal is idempotent, so the normal-path explicit remove and this guard
/// coexist safely.
struct RegistryCleanupGuard {
    registry: Arc<RendererRegistry>,
    session_id: RendererSessionId,
}

impl Drop for RegistryCleanupGuard {
    fn drop(&mut self) {
        let registry = Arc::clone(&self.registry);
        let session_id = self.session_id.clone();
        // WHY: RendererRegistry::remove is async (RwLock) and Drop is sync —
        // spawn the removal; outside a runtime (process teardown) the entry
        // dies with the registry anyway.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                registry.remove(&session_id).await;
            });
        }
    }
}

/// Session body between registry add and remove: opens the audio stream and
/// consumes status reports until disconnection.
async fn run_renderer_session(
    connection: &quinn::Connection,
    ctrl_recv: &mut quinn::RecvStream,
    registry: &RendererRegistry,
    session_id: &RendererSessionId,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    // Open unidirectional stream for audio frames.
    let _audio_send = connection.open_uni().await?;

    // Read status reports from the control stream until disconnection.
    read_status_loop(ctrl_recv, registry, session_id, shutdown).await
}

async fn read_status_loop(
    ctrl_recv: &mut quinn::RecvStream,
    registry: &RendererRegistry,
    session_id: &RendererSessionId,
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
            // WHY: a protocol violation (oversized/malformed frame) is a real
            // failure the caller's warn! must surface; only transport-level
            // termination (peer closed, stream ended) is a clean disconnect.
            Err(e @ RenderError::Protocol { .. }) => return Err(e),
            Err(_) => break,
        }
    }
    Ok(())
}

/// Longest renderer name stored/logged verbatim. Anything longer is a
/// protocol violation, not a display concern.
const MAX_RENDERER_NAME_BYTES: usize = 128;

// WHY: SessionInit arrives from an unauthenticated peer — every field it
// carries into the registry/logs needs a bound before it is stored.
fn validate_session_init(init: &SessionInit) -> Result<(), RenderError> {
    if init.name.len() > MAX_RENDERER_NAME_BYTES {
        return Err(RenderError::Protocol {
            message: format!(
                "renderer name of {} bytes exceeds the {MAX_RENDERER_NAME_BYTES}-byte bound",
                init.name.len()
            ),
            location: snafu::location!(),
        });
    }
    Ok(())
}

// INVARIANT: registration requires a configured, non-empty server-side key;
// an absent key rejects every peer (fail closed), never accept-all.
fn api_key_matches(expected: Option<&str>, presented: &str) -> bool {
    use subtle::ConstantTimeEq;
    match expected {
        Some(expected) if !expected.is_empty() => {
            // NOTE: ct_eq on differing-length slices returns false without a
            // secret-dependent early exit; key length is not secret.
            bool::from(expected.as_bytes().ct_eq(presented.as_bytes()))
        }
        _ => false,
    }
}

fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write;
        // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
        write!(s, "{b:02x}").ok();
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
                session_id: RendererSessionId("abc123".into()),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;

        let list = registry.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "test-renderer");
        assert_eq!(list[0].session_id, RendererSessionId("abc123".into()));
    }

    #[tokio::test]
    async fn registry_remove() {
        let registry = RendererRegistry::new();
        registry
            .add(ConnectedRenderer {
                name: "a".into(),
                session_id: RendererSessionId("s1".into()),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;
        registry
            .add(ConnectedRenderer {
                name: "b".into(),
                session_id: RendererSessionId("s2".into()),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;

        registry.remove(&RendererSessionId("s1".into())).await;
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
                session_id: RendererSessionId("s1".into()),
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
        registry
            .update_status(&RendererSessionId("s1".into()), status)
            .await;

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

    #[test]
    fn session_init_name_over_bound_is_rejected() {
        let long = SessionInit {
            name: "x".repeat(MAX_RENDERER_NAME_BYTES + 1),
            protocol_version: 1,
            api_key: String::new(),
        };
        assert!(matches!(
            validate_session_init(&long),
            Err(RenderError::Protocol { .. })
        ));

        let ok = SessionInit {
            name: "x".repeat(MAX_RENDERER_NAME_BYTES),
            protocol_version: 1,
            api_key: String::new(),
        };
        assert!(validate_session_init(&ok).is_ok());
    }

    #[tokio::test]
    async fn cleanup_guard_removes_entry_on_panic_unwind() {
        let registry = Arc::new(RendererRegistry::new());
        let session_id = RendererSessionId("panicky".into());
        registry
            .add(ConnectedRenderer {
                name: "p".into(),
                session_id: session_id.clone(),
                connected_at: Instant::now(),
                last_status: None,
            })
            .await;

        let registry_task = Arc::clone(&registry);
        let sid = session_id.clone();
        let task = tokio::spawn(async move {
            let _guard = RegistryCleanupGuard {
                registry: registry_task,
                session_id: sid,
            };
            panic!("simulated session panic");
        });
        assert!(task.await.is_err(), "task must have panicked");

        // The guard spawns the removal; give it a bounded window to land.
        let mut cleaned = false;
        for _ in 0..100 {
            if registry.list().await.is_empty() {
                cleaned = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(cleaned, "panic unwind must not leak the registry entry");
    }

    #[test]
    fn api_key_matches_only_on_exact_configured_key() {
        assert!(api_key_matches(Some("key-123"), "key-123"));
        assert!(!api_key_matches(Some("key-123"), "key-124"));
        assert!(!api_key_matches(Some("key-123"), "key-1234"));
        assert!(!api_key_matches(Some("key-123"), ""));
    }

    #[test]
    fn api_key_matches_fails_closed_when_unconfigured() {
        assert!(!api_key_matches(None, "anything"));
        assert!(!api_key_matches(None, ""));
        assert!(!api_key_matches(Some(""), ""));
        assert!(!api_key_matches(Some(""), "anything"));
    }
}

#[cfg(test)]
mod handshake_tests {
    use std::time::Duration;

    use horismos::{Config, ConfigManager, ConfigOverrides};

    use super::*;
    use crate::render::protocol::PROTOCOL_VERSION;

    /// Generous default so tests that don't care about the admission cap
    /// never trip it.
    const DEFAULT_TEST_MAX_CONNECTIONS: usize = 100;

    struct TestServer {
        addr: SocketAddr,
        fingerprint: String,
        registry: Arc<RendererRegistry>,
        shutdown: CancellationToken,
        /// Owner side of the live config driving this server — tests call
        /// `.replace(...)` to exercise a reload (rotation, cap change) the
        /// exact way `archon::serve`'s SIGHUP path does.
        manager: ConfigManager,
        _cert_dir: tempfile::TempDir,
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.shutdown.cancel();
        }
    }

    fn hex_fingerprint(cert_der: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(cert_der)
            .iter()
            .fold(String::with_capacity(64), |mut s, b| {
                use std::fmt::Write as _;
                // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
                write!(s, "{b:02x}").ok();
                s
            })
    }

    fn renderer_test_jwt_secret() -> &'static str {
        "renderer-test-secret-that-is-at-least-32-bytes-long"
    }

    /// A full, otherwise-valid `Config` with only the renderer-relevant
    /// `paroche` fields and `exousia.jwt_secret` set — `ConfigManager::replace`
    /// validates the WHOLE config on every call, so every config a test
    /// publishes must clear validation, not just the fields under test.
    fn renderer_test_config(
        api_key: Option<&str>,
        max_connections: usize,
        session_init_timeout: Duration,
    ) -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret = renderer_test_jwt_secret().to_string();
        config.paroche.renderer_api_key = api_key.map(str::to_string);
        config.paroche.renderer_max_connections = max_connections;
        config.paroche.renderer_session_init_timeout_secs = session_init_timeout.as_secs();
        config
    }

    /// Reserves an ephemeral UDP port on loopback and releases it immediately
    /// — `start_renderer_server` binds its own `quinn::Endpoint` internally
    /// and never reports the address it chose, so tests must pick the port
    /// themselves to know where to dial the client.
    fn free_udp_addr() -> SocketAddr {
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind ephemeral udp port");
        socket.local_addr().expect("local addr")
    }

    async fn spawn_test_server(expected_key: Option<&str>) -> TestServer {
        spawn_test_server_with_timeout(expected_key, Duration::from_secs(5)).await
    }

    async fn spawn_test_server_with_timeout(
        expected_key: Option<&str>,
        session_init_timeout: Duration,
    ) -> TestServer {
        spawn_test_server_with_config(
            expected_key,
            DEFAULT_TEST_MAX_CONNECTIONS,
            session_init_timeout,
        )
        .await
    }

    /// Full harness: spawns the REAL `start_renderer_server` (not a
    /// hand-rolled stand-in), driven by a real `ConfigManager`/`Section`
    /// pair — every test in this module, including the pre-#529 ones,
    /// exercises the exact production admission + per-connection-read path.
    async fn spawn_test_server_with_config(
        expected_key: Option<&str>,
        max_connections: usize,
        session_init_timeout: Duration,
    ) -> TestServer {
        let cert_dir = tempfile::TempDir::new().expect("tempdir");
        // WHY: materialize the persisted cert up front so the fingerprint is
        // known before `start_renderer_server` (re)loads the SAME cert file
        // (`load_or_generate_server_config` loads existing material rather
        // than regenerating it).
        let server_config =
            tls::load_or_generate_server_config(cert_dir.path()).expect("server config");
        drop(server_config);
        let cert_der = std::fs::read(cert_dir.path().join("server.der")).expect("read cert");
        let fingerprint = hex_fingerprint(&cert_der);

        let config = renderer_test_config(expected_key, max_connections, session_init_timeout);
        let (manager, handle) = ConfigManager::new(
            config,
            std::path::PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let section = handle.section(|c| &c.paroche);

        let registry = Arc::new(RendererRegistry::new());
        let shutdown = CancellationToken::new();
        let addr = free_udp_addr();

        let server_registry = Arc::clone(&registry);
        let server_shutdown = shutdown.clone();
        let cert_dir_path = cert_dir.path().to_path_buf();
        tokio::spawn(async move {
            // WHY: server-side failure surfaces as Err by design (e.g. on
            // cancellation); tests assert via the client result and
            // registry contents, not this task's outcome.
            start_renderer_server(
                addr,
                &cert_dir_path,
                server_registry,
                server_shutdown,
                section,
            )
            .await
            .ok();
        });

        TestServer {
            addr,
            fingerprint,
            registry,
            shutdown,
            manager,
            _cert_dir: cert_dir,
        }
    }

    async fn client_handshake(
        server: &TestServer,
        pin: &str,
        api_key: &str,
    ) -> Result<(u8, Vec<u8>), RenderError> {
        let client_config = tls::build_client_config(pin)?;
        let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("loopback addr"))
            .expect("client endpoint");
        endpoint.set_default_client_config(client_config);

        let connection = endpoint
            .connect(server.addr, "harmonia")
            .map_err(|e| RenderError::Connection {
                message: e.to_string(),
                location: snafu::location!(),
            })?
            .await?;

        let (mut send, mut recv) = connection.open_bi().await?;
        let init = SessionInit {
            name: "test-renderer".into(),
            protocol_version: PROTOCOL_VERSION,
            api_key: api_key.to_string(),
        };
        let payload = serde_json::to_vec(&init).expect("serialize init");
        protocol::send_message(&mut send, MSG_SESSION_INIT, &payload).await?;

        tokio::time::timeout(Duration::from_secs(5), protocol::recv_message(&mut recv))
            .await
            .map_err(|_| RenderError::Connection {
                message: "timed out waiting for server response".into(),
                location: snafu::location!(),
            })?
    }

    /// A live renderer session held open by the test. Its existence keeps the
    /// server-side session — and therefore its `LiveGate` admission guard —
    /// alive; dropping it closes the QUIC connection, which the server
    /// observes as a disconnect (freeing the slot and removing the registry
    /// entry). `client_handshake` above deliberately does NOT hold its
    /// connection, so it is unsuitable for the persistence assertions below.
    struct HeldSession {
        // WHY: every one of these must stay alive to keep the session up.
        // Dropping the endpoint or connection tears down QUIC; dropping the
        // control SendStream finishes the stream, which the server's
        // `read_status_loop` sees as a clean disconnect and ends the session.
        _endpoint: quinn::Endpoint,
        _connection: quinn::Connection,
        _ctrl_send: quinn::SendStream,
        _ctrl_recv: quinn::RecvStream,
    }

    /// Connects, completes SessionInit, and RETURNS the live streams/connection
    /// so the caller keeps the session (and its server-side admission slot)
    /// alive for the duration of the test. Contrast `client_handshake`, which
    /// drops everything on return.
    async fn client_connect_and_hold(
        server: &TestServer,
        pin: &str,
        api_key: &str,
    ) -> Result<(HeldSession, u8, Vec<u8>), RenderError> {
        let client_config = tls::build_client_config(pin)?;
        let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("loopback addr"))
            .expect("client endpoint");
        endpoint.set_default_client_config(client_config);

        let connection = endpoint
            .connect(server.addr, "harmonia")
            .map_err(|e| RenderError::Connection {
                message: e.to_string(),
                location: snafu::location!(),
            })?
            .await?;

        let (mut send, mut recv) = connection.open_bi().await?;
        let init = SessionInit {
            name: "test-renderer".into(),
            protocol_version: PROTOCOL_VERSION,
            api_key: api_key.to_string(),
        };
        let payload = serde_json::to_vec(&init).expect("serialize init");
        protocol::send_message(&mut send, MSG_SESSION_INIT, &payload).await?;

        let (msg_type, body) =
            tokio::time::timeout(Duration::from_secs(5), protocol::recv_message(&mut recv))
                .await
                .map_err(|_| RenderError::Connection {
                    message: "timed out waiting for server response".into(),
                    location: snafu::location!(),
                })??;

        Ok((
            HeldSession {
                _endpoint: endpoint,
                _connection: connection,
                _ctrl_send: send,
                _ctrl_recv: recv,
            },
            msg_type,
            body,
        ))
    }

    /// Polls until the registry reaches `expected` entries (bounded), then
    /// asserts. Registry insertion happens AFTER SessionAccept is sent, so a
    /// held session is not necessarily listed the instant its handshake
    /// returns — this closes that race without masking a genuine miscount.
    async fn assert_registry_len(server: &TestServer, expected: usize, context: &str) {
        for _ in 0..100 {
            if server.registry.list().await.len() == expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(server.registry.list().await.len(), expected, "{context}");
    }

    #[tokio::test]
    async fn handshake_accepts_valid_api_key() {
        let server = spawn_test_server(Some("key-123")).await;

        let (msg_type, payload) = client_handshake(&server, &server.fingerprint, "key-123")
            .await
            .expect("authenticated handshake succeeds");

        assert_eq!(msg_type, MSG_SESSION_ACCEPT);
        let accept: SessionAccept = serde_json::from_slice(&payload).expect("decode accept");
        assert!(!accept.session_id.0.is_empty());
        assert_eq!(server.registry.list().await.len(), 1);
    }

    #[tokio::test]
    async fn handshake_rejects_wrong_api_key() {
        let server = spawn_test_server(Some("key-123")).await;

        let result = client_handshake(&server, &server.fingerprint, "wrong-key").await;

        assert!(result.is_err(), "wrong api key must not get a session");
        assert!(server.registry.list().await.is_empty());
    }

    #[tokio::test]
    async fn handshake_rejects_empty_api_key() {
        let server = spawn_test_server(Some("key-123")).await;

        let result = client_handshake(&server, &server.fingerprint, "").await;

        assert!(result.is_err(), "empty api key must not get a session");
        assert!(server.registry.list().await.is_empty());
    }

    #[tokio::test]
    async fn handshake_rejects_every_peer_when_key_unconfigured() {
        let server = spawn_test_server(None).await;

        let result = client_handshake(&server, &server.fingerprint, "anything").await;

        assert!(
            result.is_err(),
            "unconfigured server key must reject all peers"
        );
        assert!(server.registry.list().await.is_empty());
    }

    #[tokio::test]
    async fn client_rejects_mismatched_server_fingerprint() {
        let server = spawn_test_server(Some("key-123")).await;
        let wrong_pin = "0".repeat(64);

        let result = client_handshake(&server, &wrong_pin, "key-123").await;

        assert!(result.is_err(), "wrong pin must fail the TLS handshake");
        assert!(server.registry.list().await.is_empty());
    }

    // ── #411: registry cleanup covers every post-add exit path ─────────────

    #[tokio::test]
    async fn registry_entry_removed_when_client_drops_after_accept() {
        let server = spawn_test_server(Some("key-123")).await;

        // client_handshake drops its endpoint on return, closing the
        // connection; whatever exit path the handler takes afterwards
        // (open_uni failure or status-loop termination), the registry entry
        // must not leak.
        let (msg_type, _) = client_handshake(&server, &server.fingerprint, "key-123")
            .await
            .expect("authenticated handshake succeeds");
        assert_eq!(msg_type, MSG_SESSION_ACCEPT);

        let mut cleaned = false;
        for _ in 0..100 {
            if server.registry.list().await.is_empty() {
                cleaned = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            cleaned,
            "a dropped connection must not leave a registry entry behind"
        );
    }

    // ── #546: pre-auth admission is bounded ─────────────────────────────────

    #[tokio::test]
    async fn stalled_peer_is_dropped_after_session_init_timeout() {
        let server =
            spawn_test_server_with_timeout(Some("key-123"), Duration::from_millis(200)).await;

        let client_config = tls::build_client_config(&server.fingerprint).expect("client config");
        let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("loopback addr"))
            .expect("client endpoint");
        endpoint.set_default_client_config(client_config);

        let connection = endpoint
            .connect(server.addr, "harmonia")
            .expect("connect")
            .await
            .expect("QUIC handshake completes");

        // Deliberately never opens the control stream or sends SessionInit —
        // pre-fix, the server would hold this connection (and task) open
        // indefinitely.
        let closed = tokio::time::timeout(Duration::from_secs(5), connection.closed()).await;
        assert!(
            closed.is_ok(),
            "the server must drop a peer that never completes SessionInit \
             instead of holding the connection open indefinitely"
        );
        assert!(server.registry.list().await.is_empty());
    }

    #[tokio::test]
    async fn admission_cap_refuses_beyond_max_connections() {
        let cert_dir = tempfile::TempDir::new().expect("tempdir");
        let server_config =
            tls::load_or_generate_server_config(cert_dir.path()).expect("server config");
        let endpoint =
            quinn::Endpoint::server(server_config, "127.0.0.1:0".parse().expect("loopback addr"))
                .expect("server endpoint");
        let addr = endpoint.local_addr().expect("local addr");
        let cert_der = std::fs::read(cert_dir.path().join("server.der")).expect("read cert");
        let fingerprint = hex_fingerprint(&cert_der);

        let admission = Arc::new(LiveGate::new());

        let client_config = tls::build_client_config(&fingerprint).expect("client config");
        let mut client = quinn::Endpoint::client("127.0.0.1:0".parse().expect("loopback addr"))
            .expect("client endpoint");
        client.set_default_client_config(client_config);

        // Two peers dial before either Incoming is admitted, so the second is
        // guaranteed to still be pending when the server accepts it below.
        // WHY: fire-and-forget — the test only cares that the server sees the
        // initial packet, not that either handshake completes.
        let connecting1 = client.connect(addr, "harmonia").expect("client1 connect");
        tokio::spawn(async move {
            connecting1.await.ok();
        });
        let connecting2 = client.connect(addr, "harmonia").expect("client2 connect");
        tokio::spawn(async move {
            connecting2.await.ok();
        });

        let incoming1 = endpoint.accept().await.expect("first incoming");
        let admitted1 = try_admit(incoming1, &admission, 1);
        assert!(admitted1.is_some(), "first connection fits under the cap");

        let incoming2 = endpoint.accept().await.expect("second incoming");
        let admitted2 = try_admit(incoming2, &admission, 1);
        assert!(
            admitted2.is_none(),
            "a connection beyond max_connections must be refused"
        );
    }

    // ── #529 step 4: renderer_api_key / renderer_max_connections go live ───

    #[tokio::test]
    async fn rotating_api_key_rejects_old_accepts_new_keeps_prior_session() {
        let server = spawn_test_server(Some("old-key")).await;

        // Hold the pre-rotation session open for the whole test — the
        // established connection is what a later rotation must NOT disturb.
        let (prior, msg_type, _) = client_connect_and_hold(&server, &server.fingerprint, "old-key")
            .await
            .expect("pre-rotation handshake with the original key succeeds");
        assert_eq!(msg_type, MSG_SESSION_ACCEPT);
        assert_registry_len(&server, 1, "pre-rotation session must be registered").await;

        let rotated = renderer_test_config(
            Some("new-key"),
            DEFAULT_TEST_MAX_CONNECTIONS,
            Duration::from_secs(5),
        );
        server
            .manager
            .replace(rotated)
            .expect("replace applies the rotated key");

        let old_key_result = client_connect_and_hold(&server, &server.fingerprint, "old-key").await;
        assert!(
            old_key_result.is_err(),
            "a NEW registration with the OLD key must be rejected after rotation"
        );

        let (_new, msg_type_new, _) =
            client_connect_and_hold(&server, &server.fingerprint, "new-key")
                .await
                .expect("a NEW registration with the rotated key must succeed");
        assert_eq!(msg_type_new, MSG_SESSION_ACCEPT);

        // Rotation affects new registrations only — the pre-rotation session
        // is never re-challenged or evicted.
        assert_registry_len(
            &server,
            2,
            "the pre-rotation session must still be registered alongside the new one",
        )
        .await;

        // Keep `prior` alive until here so the survival assertion is genuine.
        drop(prior);
    }

    #[tokio::test]
    async fn lowering_max_connections_refuses_new_while_existing_survive() {
        let server =
            spawn_test_server_with_config(Some("key-123"), 2, Duration::from_secs(5)).await;

        let (s1, msg_type_a, _) = client_connect_and_hold(&server, &server.fingerprint, "key-123")
            .await
            .expect("first connection admitted under a cap of 2");
        assert_eq!(msg_type_a, MSG_SESSION_ACCEPT);
        let (s2, msg_type_b, _) = client_connect_and_hold(&server, &server.fingerprint, "key-123")
            .await
            .expect("second connection admitted under a cap of 2");
        assert_eq!(msg_type_b, MSG_SESSION_ACCEPT);
        assert_registry_len(&server, 2, "both pre-lowering sessions must register").await;

        let lowered = renderer_test_config(Some("key-123"), 1, Duration::from_secs(5));
        server
            .manager
            .replace(lowered)
            .expect("replace applies the lowered cap");

        // Both slots are still held, so the live cap of 1 refuses this new
        // connection — a LOWERED cap applies to new admissions only.
        let refused = client_connect_and_hold(&server, &server.fingerprint, "key-123").await;
        assert!(
            refused.is_err(),
            "a new connection beyond the lowered cap must be refused"
        );

        assert_registry_len(
            &server,
            2,
            "existing sessions must survive a cap decrease — never force-closed",
        )
        .await;

        drop((s1, s2));
    }

    #[tokio::test]
    async fn raising_max_connections_admits_a_new_connection() {
        let server =
            spawn_test_server_with_config(Some("key-123"), 1, Duration::from_secs(5)).await;

        let (s1, msg_type, _) = client_connect_and_hold(&server, &server.fingerprint, "key-123")
            .await
            .expect("first connection admitted under a cap of 1");
        assert_eq!(msg_type, MSG_SESSION_ACCEPT);
        assert_registry_len(&server, 1, "the first session must be registered").await;

        // The first slot is still held, so at the original cap of 1 a second
        // connection is refused.
        let refused_at_old_cap =
            client_connect_and_hold(&server, &server.fingerprint, "key-123").await;
        assert!(
            refused_at_old_cap.is_err(),
            "a second connection at the original cap of 1 must be refused"
        );

        let raised = renderer_test_config(Some("key-123"), 2, Duration::from_secs(5));
        server
            .manager
            .replace(raised)
            .expect("replace applies the raised cap");

        let (_s2, msg_type_second, _) =
            client_connect_and_hold(&server, &server.fingerprint, "key-123")
                .await
                .expect("a new connection must be admitted once the cap is raised");
        assert_eq!(msg_type_second, MSG_SESSION_ACCEPT);
        assert_registry_len(
            &server,
            2,
            "both sessions must be registered after the raise",
        )
        .await;

        drop(s1);
    }
}
