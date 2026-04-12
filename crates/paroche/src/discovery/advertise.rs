// mDNS service advertisement for the harmonia server
use std::collections::HashMap;

use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceInfo};
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

/// The mDNS service type used by harmonia servers.
pub const SERVICE_TYPE: &str = "_harmonia._udp.local.";

/// Protocol version advertised in TXT records.
const PROTOCOL_VERSION: &str = "1";

/// Parameters for registering the mDNS advertisement.
pub struct AdvertiseParams {
    /// Human-readable server instance name (e.g. "Harmonia").
    pub instance_name: String,
    /// QUIC listen port.
    pub port: u16,
    /// Stable server ID (UUID as string).
    pub server_id: String,
    /// Server TLS cert fingerprint (SHA-256 hex).
    pub cert_fingerprint: String,
}

/// A running mDNS advertisement that unregisters when dropped.
pub struct AdvertisedService {
    daemon: ServiceDaemon,
    service_fullname: String,
}

impl AdvertisedService {
    /// Register the service and wait for the first `DaemonEvent::Announce` confirmation.
    pub async fn start(params: AdvertiseParams) -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;

        let mut props = HashMap::new();
        props.insert("version".to_string(), PROTOCOL_VERSION.to_string());
        props.insert("server_id".to_string(), params.server_id.clone());
        props.insert("fingerprint".to_string(), params.cert_fingerprint.clone());

        // WHY: hostname must be unique within .local domain; use sanitized instance name.
        let hostname = format!(
            "{}.local.",
            params.instance_name.replace(' ', "-").to_lowercase()
        );

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &params.instance_name,
            &hostname,
            "",
            params.port,
            props,
        )
        .map_err(|e| e.to_string())?;

        let service_fullname = service_info.get_fullname().to_string();

        let monitor = daemon.monitor().map_err(|e| e.to_string())?;

        daemon.register(service_info).map_err(|e| e.to_string())?;

        // Wait for Announce confirmation, indicating the service is being advertised.
        let (tx, rx) = oneshot::channel::<()>();
        let fullname_check = service_fullname.clone();
        let mut tx_opt = Some(tx);

        tokio::spawn(async move {
            loop {
                match monitor.recv_async().await {
                    Ok(DaemonEvent::Announce(name, intf)) => {
                        debug!(name, intf, "mDNS daemon: announce");
                        if name == fullname_check {
                            if let Some(tx) = tx_opt.take() {
                                let _ = tx.send(());
                            }
                            break;
                        }
                    }
                    Ok(DaemonEvent::Error(e)) => {
                        warn!("mDNS daemon error: {e}");
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        warn!("mDNS monitor channel closed: {e}");
                        break;
                    }
                }
            }
        });

        tokio::time::timeout(std::time::Duration::from_secs(5), rx)
            .await
            .map_err(|_| "mDNS registration timed out".to_string())?
            .map_err(|_| "mDNS registration channel dropped".to_string())?;

        info!(
            instance = %params.instance_name,
            port = params.port,
            "mDNS service registered: {SERVICE_TYPE}"
        );

        Ok(Self {
            daemon,
            service_fullname,
        })
    }

    /// Unregister the service FROM the mDNS daemon.
    pub fn stop(self) {
        if let Err(e) = self.daemon.unregister(&self.service_fullname) {
            error!("mDNS unregister failed: {e}");
        } else {
            info!(
                service = %self.service_fullname,
                "mDNS service unregistered"
            );
        }
        let _ = self.daemon.shutdown();
    }
}
