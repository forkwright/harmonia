mod commands;
mod config;
mod dsp;
mod playback;
mod system;

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tracing::warn;

use config::{AppConfig, NotificationConfig, load_config, save_config};
use playback::{PlaybackEngine, PlaybackStatus};

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

/// Thread-safe handle to the persisted application configuration.
pub(crate) struct AppConfigState(pub Mutex<AppConfig>);

// ---------------------------------------------------------------------------
// System IPC commands
// ---------------------------------------------------------------------------

/// Returns a copy of the full application configuration.
#[tauri::command]
async fn get_app_config(state: State<'_, AppConfigState>) -> Result<AppConfig, String> {
    Ok(state.0.lock().unwrap().clone())
}

/// Replaces the full application configuration and persists it to disk.
#[tauri::command]
async fn set_app_config(
    new_config: AppConfig,
    state: State<'_, AppConfigState>,
) -> Result<(), String> {
    *state.0.lock().unwrap() = new_config.clone();
    save_config(&new_config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn tray_get_minimize_to_tray(state: State<'_, AppConfigState>) -> Result<bool, String> {
    Ok(state.0.lock().unwrap().minimize_to_tray)
}

#[tauri::command]
async fn tray_set_minimize_to_tray(
    enabled: bool,
    state: State<'_, AppConfigState>,
) -> Result<(), String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.minimize_to_tray = enabled;
    save_config(&cfg).map_err(|e| e.to_string())
}

#[tauri::command]
async fn autostart_is_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(system::autostart::is_enabled(&app))
}

#[tauri::command]
async fn autostart_set(
    enabled: bool,
    app: tauri::AppHandle,
    state: State<'_, AppConfigState>,
) -> Result<(), String> {
    {
        let mut cfg = state.0.lock().unwrap();
        cfg.auto_start = enabled;
        save_config(&cfg).map_err(|e| e.to_string())?;
    }
    if enabled {
        system::autostart::enable(&app);
    } else {
        system::autostart::disable(&app);
    }
    Ok(())
}

#[tauri::command]
async fn notifications_get_config(
    state: State<'_, AppConfigState>,
) -> Result<NotificationConfig, String> {
    Ok(state.0.lock().unwrap().notifications.clone())
}

#[tauri::command]
async fn notifications_set_config(
    new_config: NotificationConfig,
    state: State<'_, AppConfigState>,
) -> Result<(), String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.notifications = new_config;
    save_config(&cfg).map_err(|e| e.to_string())
}

/// Returns version and platform information about the running app.
#[tauri::command]
async fn get_app_info() -> Result<AppInfo, String> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: "2".to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    })
}

#[derive(Serialize, Deserialize)]
struct AppInfo {
    version: String,
    tauri_version: String,
    os: String,
    arch: String,
}

// ---------------------------------------------------------------------------
// Window state helpers
// ---------------------------------------------------------------------------

fn save_window_state<R: tauri::Runtime>(window: &tauri::Window<R>) {
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.inner_size() else {
        return;
    };
    let maximized = window.is_maximized().unwrap_or(false);

    let mut cfg = load_config();
    cfg.window.x = pos.x;
    cfg.window.y = pos.y;
    cfg.window.width = size.width;
    cfg.window.height = size.height;
    cfg.window.maximized = maximized;
    if let Err(e) = save_config(&cfg) {
        warn!(error = %e, "failed to save window state");
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() {
    let initial_config = load_config();
    let start_minimized = initial_config.start_minimized;

    let engine = PlaybackEngine::new().unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppConfigState(Mutex::new(initial_config)))
        .manage(dsp::DspController::new())
        .manage(playback::podcast::PodcastController::new())
        .manage(playback::audiobook::AudiobookController::new())
        .manage(engine)
        .setup(move |app| {
            // Tray icon
            system::tray::setup_tray(app.handle())?;

            // Restore saved window geometry
            if let Some(window) = app.get_webview_window("main") {
                let cfg = load_config();
                let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: cfg.window.width,
                    height: cfg.window.height,
                }));
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: cfg.window.x,
                    y: cfg.window.y,
                }));
                if cfg.window.maximized {
                    let _ = window.maximize();
                }
                if start_minimized {
                    let _ = window.hide();
                }
            }

            // Handle any audio file passed on the command line (file associations).
            system::file_handler::handle_startup_file(app.handle());

            let engine: State<'_, PlaybackEngine> = app.state();
            let state_rx = engine.subscribe_state();

            // Background task: keep tray track info and notifications in sync.
            let handle = app.handle().clone();
            tokio::spawn(async move {
                let mut rx = state_rx;
                let mut last_track_id: Option<String> = None;
                loop {
                    match rx.recv(.instrument(tracing::info_span!("spawned_task"))).await {
                        Ok(event) => {
                            let title = event.track.as_ref().map(|t| t.title.as_str());
                            let artist = event.track.as_ref().and_then(|t| t.artist.as_deref());
                            system::tray::update_tray_track(&handle, title, artist);

                            if event.status == PlaybackStatus::Playing {
                                if let Some(track) = &event.track {
                                    let new_id = Some(track.track_id.clone());
                                    if new_id != last_track_id {
                                        last_track_id = new_id;
                                        let cfg = load_config();
                                        system::notifications::notify_track_change(
                                            &handle,
                                            &cfg.notifications,
                                            &track.title,
                                            track.artist.as_deref(),
                                        );
                                    }
                                }
                            } else if event.track.is_none() {
                                last_track_id = None;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            // MPRIS D-Bus service (Linux only).
            #[cfg(target_os = "linux")]
            {
                let mpris_rx = app.state::<PlaybackEngine>().subscribe_state();
                let mpris_handle = app.handle().clone();
                tokio::spawn(async move {
                    if let Err(e.instrument(tracing::info_span!("spawned_task"))) = system::mpris::start(mpris_handle, mpris_rx).await {
                        warn!(error = %e, "MPRIS service failed to start");
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                save_window_state(window);
                let cfg = load_config();
                if cfg.minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                save_window_state(window);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            // General
            commands::health_check,
            commands::get_server_url,
            commands::set_server_url,
            // System
            get_app_config,
            set_app_config,
            tray_get_minimize_to_tray,
            tray_set_minimize_to_tray,
            autostart_is_enabled,
            autostart_set,
            notifications_get_config,
            notifications_set_config,
            get_app_info,
            // DSP
            dsp::commands::get_dsp_config,
            dsp::commands::set_eq_enabled,
            dsp::commands::set_eq_preamp,
            dsp::commands::set_eq_band,
            dsp::commands::set_eq_bands,
            dsp::commands::add_eq_band,
            dsp::commands::remove_eq_band,
            dsp::commands::load_eq_preset,
            dsp::commands::get_eq_presets,
            dsp::commands::reset_eq,
            dsp::commands::set_crossfeed,
            dsp::commands::set_crossfeed_preset,
            dsp::commands::set_replaygain,
            dsp::commands::set_compressor,
            dsp::commands::set_volume,
            dsp::commands::list_output_devices,
            dsp::commands::set_output_device,
            // Podcast
            playback::podcast::podcast_play_episode,
            playback::podcast::podcast_resume_episode,
            playback::podcast::podcast_set_speed,
            playback::podcast::podcast_get_speed,
            playback::podcast::podcast_skip_forward,
            playback::podcast::podcast_skip_backward,
            playback::podcast::podcast_set_trim_silence,
            playback::podcast::podcast_get_playback_snapshot,
            // Audiobook
            playback::audiobook::audiobook_play,
            playback::audiobook::audiobook_play_from_chapter,
            playback::audiobook::audiobook_resume,
            playback::audiobook::audiobook_pause,
            playback::audiobook::audiobook_stop,
            playback::audiobook::audiobook_next_chapter,
            playback::audiobook::audiobook_prev_chapter,
            playback::audiobook::audiobook_go_to_chapter,
            playback::audiobook::audiobook_skip_forward,
            playback::audiobook::audiobook_skip_backward,
            playback::audiobook::audiobook_set_speed,
            playback::audiobook::audiobook_get_speed,
            playback::audiobook::sleep_timer_set,
            playback::audiobook::sleep_timer_set_end_of_chapter,
            playback::audiobook::sleep_timer_cancel,
            playback::audiobook::sleep_timer_extend,
            playback::audiobook::sleep_timer_get,
            playback::audiobook::audiobook_get_position,
            playback::audiobook::audiobook_update_offset,
            // Playback
            playback::commands::play_track,
            playback::commands::pause,
            playback::commands::resume,
            playback::commands::stop,
            playback::commands::seek,
            playback::commands::next_track,
            playback::commands::previous_track,
            playback::commands::playback_set_volume,
            playback::commands::playback_get_volume,
            playback::commands::queue_add,
            playback::commands::queue_remove,
            playback::commands::queue_clear,
            playback::commands::queue_move,
            playback::commands::queue_get,
            playback::commands::set_repeat_mode,
            playback::commands::set_shuffle,
            playback::commands::get_playback_state,
            playback::commands::get_signal_path,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_default();
}
