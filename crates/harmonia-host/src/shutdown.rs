use tokio::signal::unix::SignalKind;

pub async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    let sigterm = tokio::signal::unix::signal(SignalKind::terminate());

    match sigterm {
        Ok(mut sigterm) => {
            tokio::select! {
                _ = ctrl_c => {
                    tracing::info!("Ctrl+C received, shutting down");
                }
                _ = sigterm.recv() => {
                    tracing::info!("SIGTERM received, shutting down");
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to register SIGTERM handler: {e}; relying on Ctrl+C only");
            ctrl_c.await.ok();
        }
    }
}
