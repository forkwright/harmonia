//! OS integration: MPRIS, system tray, desktop notifications, file associations, autostart.
pub(crate) mod autostart;
pub(crate) mod file_handler;
pub(crate) mod notifications;
pub(crate) mod tray;

#[cfg(target_os = "linux")]
pub(crate) mod mpris;
