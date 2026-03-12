// Stub — full serve implementation in P2-13.
//
// SIGHUP wiring (to be integrated with serve.rs in P2-13):
//
//   let (config_manager, config_handle) =
//       horismos::ConfigManager::new(initial_config, config_path);
//
//   let manager_for_reload = config_manager.clone();
//   tokio::spawn(async move {
//       use tokio::signal::unix::SignalKind;
//       let mut sighup = tokio::signal::unix::signal(SignalKind::hangup())
//           .expect("failed to register SIGHUP handler");
//       loop {
//           sighup.recv().await;
//           tracing::info!("SIGHUP received — reloading configuration");
//           match manager_for_reload.reload() {
//               Ok(warnings) => {
//                   for w in warnings {
//                       tracing::warn!("config reload warning: {}", w.message);
//                   }
//               }
//               Err(e) => {
//                   tracing::error!("config reload failed: {e} — keeping current config");
//               }
//           }
//       }
//   });

fn main() {}
