// mDNS-based harmonia server discovery for renderers
use std::net::SocketAddr;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent};
use tracing::{debug, info, warn};

/// The mDNS service type used by harmonia servers.
pub const SERVICE_TYPE: &str = "_harmonia._udp.local.";

/// A discovered harmonia server.
#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    pub instance_name: String,
    pub addr: SocketAddr,
    pub server_id: Option<String>,
    pub cert_fingerprint: Option<String>,
    pub protocol_version: Option<String>,
}

/// Discover harmonia servers via mDNS, waiting up to `timeout` for results.
///
/// If `preferred_fingerprint` is provided, a matching server is sorted first.
pub async fn discover_servers(
    timeout: Duration,
    preferred_fingerprint: Option<&str>,
) -> Result<Vec<DiscoveredServer>, String> {
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let receiver = daemon.browse(SERVICE_TYPE).map_err(|e| e.to_string())?;

    let mut servers: Vec<DiscoveredServer> = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        let event = match tokio::time::timeout(remaining, receiver.recv_async()).await {
            Ok(Ok(event)) => event,
            Ok(Err(_)) | Err(_) => break,
        };

        let ServiceEvent::ServiceResolved(info) = event else {
            continue;
        };

        debug!(
            instance = %info.get_fullname(),
            "mDNS: resolved harmonia server"
        );

        let server_id = info.get_property_val_str("server_id").map(str::to_string);
        let cert_fingerprint = info.get_property_val_str("fingerprint").map(str::to_string);
        let protocol_version = info.get_property_val_str("version").map(str::to_string);
        let port = info.get_port();

        // WHY: take the first address only; multiple addresses for the same instance
        // are handled at connection time by trying each in turn.
        if let Some(scoped) = info.get_addresses().iter().next() {
            let ip = scoped.to_ip_addr();
            let server = DiscoveredServer {
                instance_name: info.get_fullname().to_string(),
                addr: SocketAddr::new(ip, port),
                server_id: server_id.clone(),
                cert_fingerprint: cert_fingerprint.clone(),
                protocol_version: protocol_version.clone(),
            };

            // If we match the preferred fingerprint, return immediately with it first.
            if preferred_fingerprint.is_some_and(|pref| cert_fingerprint.as_deref() == Some(pref)) {
                info!(addr = %server.addr, "mDNS: found preferred server");
                servers.insert(0, server);
                let _ = daemon.shutdown();
                return Ok(servers);
            }

            servers.push(server);
        }
    }

    let _ = daemon.shutdown();

    if servers.is_empty() {
        info!("mDNS discovery: no harmonia servers found within timeout");
    } else {
        info!(
            count = servers.len(),
            "mDNS discovery: found {} server(s)",
            servers.len()
        );
    }

    Ok(servers)
}

/// Discover a single harmonia server, with optional explicit address override.
///
/// - If `explicit_addr` is provided, returns it immediately (no mDNS).
/// - Otherwise browses mDNS, preferring a server matching `preferred_fingerprint`.
/// - Returns `None` if discovery times out with no results.
pub async fn discover_server(
    explicit_addr: Option<SocketAddr>,
    preferred_fingerprint: Option<&str>,
) -> Option<DiscoveredServer> {
    if let Some(addr) = explicit_addr {
        info!(%addr, "using explicit server address (skipping mDNS)");
        return Some(DiscoveredServer {
            instance_name: addr.to_string(),
            addr,
            server_id: None,
            cert_fingerprint: None,
            protocol_version: None,
        });
    }

    let timeout = Duration::from_secs(10);
    match discover_servers(timeout, preferred_fingerprint).await {
        Ok(mut servers) if !servers.is_empty() => Some(servers.remove(0)),
        Ok(_) => {
            warn!(
                "no harmonia servers found via mDNS after {}s",
                timeout.as_secs()
            );
            None
        }
        Err(e) => {
            warn!("mDNS discovery failed: {e}");
            None
        }
    }
}
