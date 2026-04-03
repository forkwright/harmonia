//! System tray icon with playback controls and minimize-to-tray support.
use tauri::{
    Emitter, Manager,
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tracing::warn;

/// ID used when building and later looking up the system tray icon.
const TRAY_ID: &str = "harmonia";

/// Builds and registers the system tray icon with its context menu.
pub(crate) fn setup_tray<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("now_playing", "Not Playing")
        .separator()
        .text("play_pause", "Play / Pause")
        .text("next", "Next Track")
        .text("prev", "Previous Track")
        .separator()
        .text("show", "Open Harmonia")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::InvalidWindowHandle)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip("Harmonia")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    match window.is_visible() {
                        Ok(true) => {
                            let _ = window.hide();
                        }
                        _ => {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "play_pause" => {
                let _ = app.emit("tray-play-pause", ());
            }
            "next" => {
                let _ = app.emit("tray-next", ());
            }
            "prev" => {
                let _ = app.emit("tray-prev", ());
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Rebuilds and sets the tray menu with updated "Now Playing" text.
pub(crate) fn update_tray_track<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    title: Option<&str>,
    artist: Option<&str>,
) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let now_playing = match (title, artist) {
        (Some(t), Some(a)) => format!("{t} — {a}"),
        (Some(t), None) => t.to_string(),
        _ => "Not Playing".to_string(),
    };

    let menu = match MenuBuilder::new(app)
        .text("now_playing", &now_playing)
        .separator()
        .text("play_pause", "Play / Pause")
        .text("next", "Next Track")
        .text("prev", "Previous Track")
        .separator()
        .text("show", "Open Harmonia")
        .separator()
        .text("quit", "Quit")
        .build()
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "failed to build updated tray menu");
            return;
        }
    };

    if let Err(e) = tray.set_menu(Some(menu)) {
        warn!(error = %e, "failed to UPDATE tray menu");
    }

    if let Some(t) = title {
        let _ = tray.set_tooltip(Some(t));
    }
}
