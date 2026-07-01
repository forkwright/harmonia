// Renderer mode: headless audio endpoint receiving streams via QUIC.

pub mod config;
pub mod credentials;
pub mod discovery;
pub mod error;
pub mod pipeline;
pub mod playout;
pub mod protocol;
pub mod runner;
mod secret;
pub mod server;
pub mod status;
pub mod tls;

use std::net::SocketAddr;
use std::path::PathBuf;

pub use server::RendererRegistry;
use tracing::info;

use crate::error::HostError;

/// Arguments for the `render` subcommand.
pub struct RenderArgs {
    /// Explicit server address (skips mDNS discovery if provided).
    pub server: Option<SocketAddr>,
    /// Directory for storing TLS certs and pairing credentials.
    pub cert_dir: PathBuf,
    /// Renderer display name (defaults to hostname if not SET).
    pub name: Option<String>,
    /// Path to renderer TOML config file.
    pub config_path: Option<PathBuf>,
}

fn default_renderer_name() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "harmonia-renderer".to_string())
}

/// Entry point for the renderer process:
/// discovers the server, loads existing credentials, and prepares for connection.
///
/// On first run (no credentials), initiates pairing with the discovered server.
/// On subsequent runs, reconnects using the stored API key.
pub async fn run_render(args: RenderArgs) -> Result<(), HostError> {
    let name = args.name.unwrap_or_else(default_renderer_name);

    let creds = credentials::load_credentials(&args.cert_dir).map_err(|e| HostError::Render {
        message: e,
        location: snafu::location!(),
    })?;

    let preferred_fp = creds.as_ref().map(|c| c.server_fingerprint.as_str());

    let server = discovery::discover_server(args.server, preferred_fp).await;

    match server {
        Some(s) => {
            info!(
                addr = %s.addr,
                instance = %s.instance_name,
                server_id = ?s.server_id,
                fingerprint = ?s.cert_fingerprint,
                version = ?s.protocol_version,
                "renderer: found server"
            );

            let creds = match creds {
                Some(c) => c,
                None => pin_first_seen_server(&args.cert_dir, &s)?,
            };

            // INVARIANT: the TLS client trusts only the pinned fingerprint;
            // an empty pin must fail here, never widen to accept-any.
            if creds.server_fingerprint.is_empty() {
                return Err(HostError::Render {
                    message: format!(
                        "stored credentials at {} lack a server fingerprint; delete \
                         credentials.toml and re-pair via mDNS discovery",
                        args.cert_dir.display()
                    ),
                    location: snafu::location!(),
                });
            }
            if creds.api_key.is_empty() {
                tracing::warn!(
                    "credentials.toml has an empty api_key; the server rejects registration \
                     until a key is provisioned"
                );
            }

            runner::run_renderer_loop(runner::RunnerArgs {
                server_addr: s.addr,
                name,
                config_path: args.config_path,
                server_fingerprint: creds.server_fingerprint,
                api_key: creds.api_key,
            })
            .await
            .map_err(|e| HostError::Render {
                message: e.to_string(),
                location: snafu::location!(),
            })?;
        }
        None => {
            tracing::warn!("renderer: no server found -- check network or use --server");
        }
    }

    Ok(())
}

/// First run: pin the discovered server's certificate fingerprint (trust-on-first-use)
/// and persist it so every later connection enforces the pin.
///
/// Fails closed when the discovered server advertises no fingerprint (for example an
/// explicit `--server` address, which skips mDNS): connecting without a pin would
/// accept any certificate.
fn pin_first_seen_server(
    cert_dir: &std::path::Path,
    s: &discovery::DiscoveredServer,
) -> Result<credentials::RendererCredentials, HostError> {
    let Some(fp) = s.cert_fingerprint.clone() else {
        return Err(HostError::Render {
            message: "no stored credentials and the server advertises no certificate \
                      fingerprint; pair via mDNS discovery first so the fingerprint \
                      can be pinned"
                .to_string(),
            location: snafu::location!(),
        });
    };
    let new_creds = credentials::RendererCredentials {
        api_key: String::new(),
        server_fingerprint: fp,
        server_name: s.instance_name.clone(),
        paired_at: jiff::Zoned::now()
            .strftime("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    };
    credentials::save_credentials(cert_dir, &new_creds).map_err(|e| HostError::Render {
        message: e,
        location: snafu::location!(),
    })?;
    info!("pinned server certificate fingerprint on first discovery (trust-on-first-use)");
    Ok(new_creds)
}
