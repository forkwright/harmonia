use tokio::signal::unix::SignalKind;

pub async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    let mut sigterm =
        tokio::signal::unix::signal(SignalKind::terminate()).expect("failed to register SIGTERM");

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Ctrl+C received, shutting down");
        }
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received, shutting down");
        }
    }
}
