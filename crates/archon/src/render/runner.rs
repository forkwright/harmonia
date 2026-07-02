// Renderer runner: main connection loop with auto-reconnect and signal handling.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "systemd")]
mod watchdog {
    use std::time::Duration;

    pub(super) fn notify_ready() {
        // WHY: sd_notify failure is non-fatal; running outside systemd is normal
        sd_notify::notify(&[sd_notify::NotifyState::Ready]).ok();
    }

    pub(super) fn notify_stopping() {
        // WHY: sd_notify failure is non-fatal; running outside systemd is normal
        sd_notify::notify(&[sd_notify::NotifyState::Stopping]).ok();
    }

    // WHY: WatchdogSec=30 in the unit file; we ping at half that interval so the
    // watchdog resets before the deadline regardless of scheduling jitter.
    pub(super) const WATCHDOG_INTERVAL: Duration = Duration::from_secs(15);

    pub(super) fn notify_watchdog() {
        // WHY: sd_notify failure is non-fatal; running outside systemd is normal
        sd_notify::notify(&[sd_notify::NotifyState::Watchdog]).ok();
    }

    pub(super) fn active() -> bool {
        std::env::var("NOTIFY_SOCKET").is_ok()
    }
}

#[cfg(not(feature = "systemd"))]
mod watchdog {
    use std::time::Duration;

    pub(super) fn notify_ready() {}
    pub(super) fn notify_stopping() {}
    pub(super) fn notify_watchdog() {}
    pub(super) fn active() -> bool {
        false
    }
    pub(super) const WATCHDOG_INTERVAL: Duration = Duration::from_secs(15);
}

use tokio::signal::unix::SignalKind;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, info, warn};

use super::config::{RendererConfig, load_renderer_config};
use super::error::{ProtocolSnafu, RenderError};
use super::pipeline::RenderPipeline;
use super::protocol::{
    self, AudioFrame, DeviceState, MSG_AUDIO_FRAME, MSG_SESSION_ACCEPT, MSG_SESSION_INIT,
    MSG_STATUS_REPORT, PROTOCOL_VERSION, SessionAccept, SessionInit,
};
use super::status::StatusReporter;
use super::tls;

/// Arguments for the QUIC rendering connection loop.
pub struct RunnerArgs {
    pub server_addr: SocketAddr,
    pub name: String,
    pub config_path: Option<PathBuf>,
    /// Pinned SHA-256 fingerprint (hex) of the server TLS certificate.
    pub server_fingerprint: String,
    /// Shared-secret credential presented in `SessionInit`.
    pub api_key: String,
}

/// QUIC connection loop: connects to a server, negotiates a session,
/// receives audio frames, and plays them through the local DSP pipeline.
pub async fn run_renderer_loop(args: RunnerArgs) -> Result<(), RenderError> {
    let config = load_renderer_config(args.config_path.as_deref())?;

    let client_config = tls::build_client_config(&args.server_fingerprint)?;

    let dsp_config = config.dsp_config();
    let (dsp_tx, _) = watch::channel(dsp_config);

    let shutdown = CancellationToken::new();

    spawn_sighup_handler(
        args.config_path.clone(),
        dsp_tx.clone(),
        shutdown.child_token(),
    );
    spawn_shutdown_handler(shutdown.clone());

    info!(
        name = %args.name,
        server = %args.server_addr,
        "starting renderer"
    );

    if watchdog::active() {
        let shutdown_wd = shutdown.child_token();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(watchdog::WATCHDOG_INTERVAL);
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown_wd.cancelled() => break,
                    _ = interval.tick() => watchdog::notify_watchdog(),
                }
            }
        });
    }
    watchdog::notify_ready();

    let mut backoff = ExponentialBackoff::new(
        config.reconnect.initial_backoff_ms,
        config.reconnect.max_backoff_ms,
    );

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        match connect_and_run(
            &args,
            &client_config,
            &config,
            dsp_tx.subscribe(),
            shutdown.child_token(),
        )
        .await
        {
            Ok(()) => break,
            Err(e) => {
                if !config.reconnect.enabled || shutdown.is_cancelled() {
                    return Err(e);
                }
                let delay = backoff.next_delay();
                warn!(
                    error = %e,
                    retry_ms = delay.as_millis(),
                    "connection lost, reconnecting after backoff"
                );
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    _ = tokio::time::sleep(delay) => {
                        // retry
                    }
                }
            }
        }
    }

    watchdog::notify_stopping();
    info!("renderer stopped");
    Ok(())
}

async fn connect_and_run(
    args: &RunnerArgs,
    client_config: &quinn::ClientConfig,
    config: &RendererConfig,
    dsp_rx: watch::Receiver<akouo_core::DspConfig>,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    let mut endpoint =
        quinn::Endpoint::client(SocketAddr::from(([0, 0, 0, 0], 0))).map_err(|e| {
            RenderError::Connection {
                message: e.to_string(),
                location: snafu::location!(),
            }
        })?;
    endpoint.set_default_client_config(client_config.clone());

    info!(server = %args.server_addr, "connecting");
    let connection = endpoint
        .connect(args.server_addr, "harmonia")
        .map_err(|e| RenderError::Connection {
            message: e.to_string(),
            location: snafu::location!(),
        })?
        .await?;
    info!("QUIC connection established");

    // Session establishment on bidirectional stream.
    let (mut ctrl_send, mut ctrl_recv) = connection.open_bi().await?;

    let init = SessionInit {
        name: args.name.clone(),
        protocol_version: PROTOCOL_VERSION,
        api_key: args.api_key.clone(),
    };
    let init_payload = serde_json::to_vec(&init).map_err(|e| RenderError::Protocol {
        message: e.to_string(),
        location: snafu::location!(),
    })?;
    protocol::send_message(&mut ctrl_send, MSG_SESSION_INIT, &init_payload).await?;

    let (msg_type, payload) = protocol::recv_message(&mut ctrl_recv).await?;
    if msg_type != MSG_SESSION_ACCEPT {
        return ProtocolSnafu {
            message: format!("expected SessionAccept (0x02), got 0x{msg_type:02x}"),
        }
        .fail();
    }
    let accept: SessionAccept =
        serde_json::from_slice(&payload).map_err(|e| RenderError::Protocol {
            message: e.to_string(),
            location: snafu::location!(),
        })?;
    info!(
        session_id = %accept.session_id,
        sample_rate = accept.sample_rate,
        channels = accept.channels,
        "session established"
    );

    let mut pipeline = RenderPipeline::new(config, dsp_rx)?;
    let status = Arc::new(StatusReporter::new());
    status.set_device_state(DeviceState::Opening);

    // Accept unidirectional audio stream FROM server.
    let audio_recv = connection.accept_uni().await?;
    status.set_device_state(DeviceState::Playing);

    let status_audio = Arc::clone(&status);
    let shutdown_audio = shutdown.child_token();
    let stream_sample_rate = accept.sample_rate;
    let stream_channels = accept.channels;

    let mut audio_task = tokio::spawn(
        async move {
            receive_audio(
                audio_recv,
                &mut pipeline,
                &status_audio,
                stream_sample_rate,
                stream_channels,
                shutdown_audio,
            )
            .await
        }
        .instrument(tracing::info_span!("audio_receive")),
    );

    let status_report = Arc::clone(&status);
    let shutdown_status = shutdown.child_token();

    let mut status_task = tokio::spawn(
        async move { send_status_reports(ctrl_send, &status_report, shutdown_status).await }
            .instrument(tracing::info_span!("status_report")),
    );

    tokio::select! {
        biased;
        _ = shutdown.cancelled() => {
            info!("shutdown requested, draining");
        }
        result = &mut audio_task => {
            // WHY: the sibling task would otherwise outlive this connection
            // attempt as a detached leak; the reconnect loop spawns fresh tasks.
            status_task.abort();
            if let Err(e) = join_outcome("audio", result) {
                status.set_device_state(DeviceState::Stopped);
                return Err(e);
            }
        }
        result = &mut status_task => {
            audio_task.abort();
            if let Err(e) = join_outcome("status", result) {
                status.set_device_state(DeviceState::Stopped);
                return Err(e);
            }
        }
    }

    status.set_device_state(DeviceState::Stopped);
    Ok(())
}

/// Classifies a joined renderer task result at the handling site.
///
/// A task that returned `Err(RenderError)` propagates so the caller's
/// backoff-reconnect path engages; a clean exit and a panicked task both
/// resolve to `Ok(())` (the panic is logged, matching prior behavior).
fn join_outcome(
    task: &'static str,
    result: Result<Result<(), RenderError>, tokio::task::JoinError>,
) -> Result<(), RenderError> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            warn!(task, error = %e, "renderer task failed");
            Err(e)
        }
        Err(e) => {
            warn!(task, error = %e, "renderer task panicked");
            Ok(())
        }
    }
}

async fn receive_audio(
    mut stream: quinn::RecvStream,
    pipeline: &mut RenderPipeline,
    status: &StatusReporter,
    sample_rate: u32,
    channels: u16,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    loop {
        let result = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            r = protocol::recv_message(&mut stream) => r,
        };

        let (msg_type, payload) = result?;
        if msg_type != MSG_AUDIO_FRAME {
            continue;
        }

        let frame = AudioFrame::decode_payload(&payload)?;
        pipeline.process_frame(frame).await?;

        let depth = pipeline.buffer_depth_ms(sample_rate, channels);
        status.update_buffer_depth(depth);
        status.update_underrun_count(pipeline.underrun_count());
    }

    pipeline.drain().await;
    pipeline.close().await;
    Ok(())
}

async fn send_status_reports(
    mut stream: quinn::SendStream,
    status: &StatusReporter,
    shutdown: CancellationToken,
) -> Result<(), RenderError> {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = interval.tick() => {
                let report = status.report();
                let payload = serde_json::to_vec(&report).map_err(|e| RenderError::Protocol {
                    message: e.to_string(),
                    location: snafu::location!(),
                })?;
                protocol::send_message(&mut stream, MSG_STATUS_REPORT, &payload).await?;
            }
        }
    }
    Ok(())
}

fn spawn_sighup_handler(
    config_path: Option<PathBuf>,
    dsp_tx: watch::Sender<akouo_core::DspConfig>,
    shutdown: CancellationToken,
) {
    tokio::spawn(
        async move {
            let mut sighup = match tokio::signal::unix::signal(SignalKind::hangup()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(error = %e, "failed to register SIGHUP handler");
                    return;
                }
            };
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    _ = sighup.recv() => {
                        info!("SIGHUP received, reloading DSP config");
                        match load_renderer_config(config_path.as_deref()) {
                            Ok(config) => {
                                // WHY: send fails only when no receivers exist; dropping is intentional during shutdown
                                dsp_tx.send(config.dsp_config()).ok();
                                info!("DSP config reloaded");
                            }
                            Err(e) => {
                                warn!(error = %e, "config reload failed, keeping current config");
                            }
                        }
                    }
                }
            }
        }
        .instrument(tracing::info_span!("sighup_handler")),
    );
}

fn spawn_shutdown_handler(shutdown: CancellationToken) {
    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();
        let sigterm = tokio::signal::unix::signal(SignalKind::terminate());

        match sigterm {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = ctrl_c => info!("Ctrl+C received"),
                    _ = sigterm.recv() => info!("SIGTERM received"),
                }
            }
            Err(_) => {
                if let Err(e) = ctrl_c.await {
                    tracing::warn!(error = %e, "operation failed");
                }
            }
        }
        shutdown.cancel();
    });
}

struct ExponentialBackoff {
    current_ms: u64,
    max_ms: u64,
    initial_ms: u64,
}

impl ExponentialBackoff {
    fn new(initial_ms: u64, max_ms: u64) -> Self {
        Self {
            current_ms: initial_ms,
            max_ms,
            initial_ms,
        }
    }

    fn next_delay(&mut self) -> Duration {
        let delay = Duration::from_millis(self.current_ms);
        self.current_ms = (self.current_ms * 2).min(self.max_ms);
        delay
    }

    #[cfg_attr(not(test), expect(dead_code))]
    fn reset(&mut self) {
        self.current_ms = self.initial_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_inactive_without_notify_socket() {
        // WHY: NOTIFY_SOCKET must not be SET in CI or dev environments; verify the guard.
        // SAFETY: single-threaded test; no concurrent env access.
        unsafe { std::env::remove_var("NOTIFY_SOCKET") };
        assert!(!watchdog::active());
    }

    #[test]
    fn watchdog_interval_is_half_of_watchdog_sec() {
        // WatchdogSec=30 in the unit file; interval must be ≤ 15s.
        assert!(watchdog::WATCHDOG_INTERVAL.as_secs() <= 15);
    }

    #[test]
    fn watchdog_noop_when_no_socket() {
        // SAFETY: single-threaded test; no concurrent env access.
        unsafe { std::env::remove_var("NOTIFY_SOCKET") };
        // These must not panic when no socket is SET.
        watchdog::notify_ready();
        watchdog::notify_watchdog();
        watchdog::notify_stopping();
    }

    #[cfg(feature = "systemd")]
    #[test]
    fn watchdog_active_with_notify_socket() {
        use std::os::unix::net::UnixListener;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("notify.sock");
        let _listener = UnixListener::bind(&socket_path).unwrap();
        // SAFETY: single-threaded test; no concurrent env access.
        unsafe { std::env::set_var("NOTIFY_SOCKET", socket_path.to_str().unwrap()) };
        assert!(watchdog::active());
        watchdog::notify_ready();
        watchdog::notify_watchdog();
        watchdog::notify_stopping();
        // SAFETY: single-threaded test; no concurrent env access.
        unsafe { std::env::remove_var("NOTIFY_SOCKET") };
    }

    #[test]
    fn exponential_backoff_increases() {
        let mut backoff = ExponentialBackoff::new(1000, 30000);
        assert_eq!(backoff.next_delay(), Duration::from_millis(1000));
        assert_eq!(backoff.next_delay(), Duration::from_millis(2000));
        assert_eq!(backoff.next_delay(), Duration::from_millis(4000));
        assert_eq!(backoff.next_delay(), Duration::from_millis(8000));
        assert_eq!(backoff.next_delay(), Duration::from_millis(16000));
        assert_eq!(backoff.next_delay(), Duration::from_millis(30000));
        assert_eq!(backoff.next_delay(), Duration::from_millis(30000));
    }

    #[test]
    fn exponential_backoff_reset() {
        let mut backoff = ExponentialBackoff::new(500, 10000);
        backoff.next_delay();
        backoff.next_delay();
        backoff.reset();
        assert_eq!(backoff.next_delay(), Duration::from_millis(500));
    }

    // ── #388: inner task Results are handled, not discarded ────────────────

    #[tokio::test]
    async fn join_outcome_propagates_inner_task_error() {
        let handle = tokio::spawn(async {
            Err::<(), RenderError>(RenderError::Protocol {
                message: "stream corrupted".to_string(),
                location: snafu::location!(),
            })
        });
        let result = join_outcome("audio", handle.await);
        assert!(
            matches!(result, Err(RenderError::Protocol { .. })),
            "an inner task Err must propagate to trigger reconnect"
        );
    }

    #[tokio::test]
    async fn join_outcome_clean_exit_is_ok() {
        let handle = tokio::spawn(async { Ok::<(), RenderError>(()) });
        assert!(join_outcome("audio", handle.await).is_ok());
    }

    #[tokio::test]
    async fn join_outcome_panic_is_logged_not_propagated() {
        let handle = tokio::spawn(async { panic!("injected panic") });
        // WHY: a panicked JoinHandle yields Err(JoinError); the handler keeps
        // the pre-fix warn-only behavior for panics.
        assert!(join_outcome("status", handle.await).is_ok());
    }

    #[tokio::test]
    async fn join_outcome_abort_is_not_propagated() {
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok::<(), RenderError>(())
        });
        handle.abort();
        assert!(join_outcome("audio", handle.await).is_ok());
    }
}

#[cfg(test)]
mod connect_and_run_tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::render::protocol::{MSG_AUDIO_FRAME, MSG_SESSION_INIT};

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

    /// Spawns a QUIC server that completes the session handshake, then sends a
    /// deliberately oversized message header on the audio stream so the
    /// client's `receive_audio` fails with a protocol error (not a panic).
    async fn spawn_poisoned_audio_server(api_key: &str) -> (SocketAddr, String, tempfile::TempDir) {
        let cert_dir = tempfile::TempDir::new().expect("tempdir");
        let server_config =
            tls::load_or_generate_server_config(cert_dir.path()).expect("server config");
        let cert_der = std::fs::read(cert_dir.path().join("server.der")).expect("read cert");
        let fingerprint = hex_fingerprint(&cert_der);

        let endpoint =
            quinn::Endpoint::server(server_config, "127.0.0.1:0".parse().expect("loopback addr"))
                .expect("server endpoint");
        let addr = endpoint.local_addr().expect("local addr");
        let expected_key = api_key.to_string();

        tokio::spawn(async move {
            let Some(incoming) = endpoint.accept().await else {
                return;
            };
            let connection = incoming.await.expect("accept connection");
            let (mut ctrl_send, mut ctrl_recv) =
                connection.accept_bi().await.expect("accept control stream");

            let (msg_type, payload) = protocol::recv_message(&mut ctrl_recv)
                .await
                .expect("read SessionInit");
            assert_eq!(msg_type, MSG_SESSION_INIT);
            let init: SessionInit = serde_json::from_slice(&payload).expect("decode init");
            assert_eq!(init.api_key, expected_key);

            let accept = SessionAccept {
                session_id: crate::render::protocol::RendererSessionId("test-session".into()),
                sample_rate: 44100,
                channels: 2,
            };
            let accept_payload = serde_json::to_vec(&accept).expect("encode accept");
            protocol::send_message(&mut ctrl_send, MSG_SESSION_ACCEPT, &accept_payload)
                .await
                .expect("send accept");

            let mut audio_send = connection.open_uni().await.expect("open audio stream");
            // A 32 MiB length prefix exceeds recv_message's 16 MiB cap, so the
            // client's audio task returns Err(RenderError::Protocol).
            let poison_len = 32u32 * 1024 * 1024;
            audio_send
                .write_all(&[MSG_AUDIO_FRAME])
                .await
                .expect("write poisoned type");
            audio_send
                .write_all(&poison_len.to_le_bytes())
                .await
                .expect("write poisoned length");

            // Keep the connection open so the failure is the message error,
            // not a connection close racing it.
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(connection);
        });

        (addr, fingerprint, cert_dir)
    }

    #[tokio::test]
    async fn connect_and_run_returns_err_when_audio_stream_fails() {
        let (addr, fingerprint, _cert_dir) = spawn_poisoned_audio_server("key-388").await;

        let args = RunnerArgs {
            server_addr: addr,
            name: "test-renderer".to_string(),
            config_path: None,
            server_fingerprint: fingerprint.clone(),
            api_key: "key-388".to_string(),
        };
        let client_config = tls::build_client_config(&fingerprint).expect("client config");
        let config = RendererConfig::default();
        let (_dsp_tx, dsp_rx) = watch::channel(config.dsp_config());
        let shutdown = CancellationToken::new();

        let result = tokio::time::timeout(
            Duration::from_secs(10),
            connect_and_run(&args, &client_config, &config, dsp_rx, shutdown),
        )
        .await
        .expect("connect_and_run must not hang on a failed audio task");

        // Pre-fix, the Ok(Err(RenderError)) arm was discarded and this
        // returned Ok(()) — silently ending the renderer loop.
        assert!(
            result.is_err(),
            "a failed audio task must propagate so the reconnect backoff engages"
        );
    }
}
