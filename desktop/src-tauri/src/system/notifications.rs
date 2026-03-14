//! Desktop notification helpers.
use tauri_plugin_notification::NotificationExt;
use tracing::warn;

use crate::config::NotificationConfig;

/// Sends a track-change notification.
///
/// Silently skips if notifications or `track_change` are disabled in `config`.
pub(crate) fn notify_track_change(
    app: &tauri::AppHandle,
    config: &NotificationConfig,
    title: &str,
    artist: Option<&str>,
) {
    if !config.enabled || !config.track_change {
        return;
    }
    let body = artist.map(|a| format!("by {a}")).unwrap_or_default();
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        warn!(error = %e, "track-change notification failed");
    }
}

/// Notifies the user that a download has completed.
#[allow(dead_code)]
pub(crate) fn notify_download_complete(
    app: &tauri::AppHandle,
    config: &NotificationConfig,
    title: &str,
) {
    if !config.enabled || !config.downloads {
        return;
    }
    if let Err(e) = app
        .notification()
        .builder()
        .title("Download Complete")
        .body(format!("\"{title}\" has been downloaded"))
        .show()
    {
        warn!(error = %e, "download-complete notification failed");
    }
}

/// Notifies the user that a media request has been approved.
#[allow(dead_code)]
pub(crate) fn notify_request_approved(
    app: &tauri::AppHandle,
    config: &NotificationConfig,
    title: &str,
) {
    if !config.enabled {
        return;
    }
    if let Err(e) = app
        .notification()
        .builder()
        .title("Request Approved")
        .body(format!("Your request for \"{title}\" has been approved"))
        .show()
    {
        warn!(error = %e, "request-approved notification failed");
    }
}

/// Notifies the user that a library scan has completed.
#[allow(dead_code)]
pub(crate) fn notify_library_scan_complete(
    app: &tauri::AppHandle,
    config: &NotificationConfig,
    count: u64,
) {
    if !config.enabled {
        return;
    }
    if let Err(e) = app
        .notification()
        .builder()
        .title("Library Scan Complete")
        .body(format!("Scan complete: {count} items added"))
        .show()
    {
        warn!(error = %e, "library-scan-complete notification failed");
    }
}
