//! Auto-start on login via XDG autostart (Linux) or platform equivalents.
use tauri_plugin_autostart::ManagerExt;
use tracing::warn;

/// Registers Harmonia to launch at login.
pub(crate) fn enable(app: &tauri::AppHandle) {
    if let Err(e) = app.autolaunch().enable() {
        warn!(error = %e, "failed to enable autostart");
    }
}

/// Removes the auto-start registration.
pub(crate) fn disable(app: &tauri::AppHandle) {
    if let Err(e) = app.autolaunch().disable() {
        warn!(error = %e, "failed to disable autostart");
    }
}

/// Returns `true` if auto-start is currently registered.
pub(crate) fn is_enabled(app: &tauri::AppHandle) -> bool {
    match app.autolaunch().is_enabled() {
        Ok(enabled) => enabled,
        Err(e) => {
            warn!(error = %e, "failed to query autostart status");
            false
        }
    }
}
