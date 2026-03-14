//! Tauri IPC commands for playback control.

use tauri::State;

use super::{PlaybackEngine, PlaybackState, QueueEntry, QueueState, RepeatMode, SignalPathInfo};

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

#[tauri::command]
pub(crate) async fn play_track(
    entry: QueueEntry,
    base_url: String,
    token: Option<String>,
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine
        .play_entry(entry, &base_url, token.as_deref(), app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn pause(
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.pause(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn resume(
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.resume(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn stop(
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.stop(&app).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn seek(
    position_ms: u64,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.seek(position_ms).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn next_track(
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    let queue = engine.queue_state().await;
    let next = queue.entries.get(queue.current_index + 1).cloned();

    if let Some(entry) = next {
        engine
            .play_entry(entry, "", None, app)
            .await
            .map_err(|e| e.to_string())
    } else {
        engine.stop(&app).await;
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn previous_track(
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    let state = engine.playback_state().await;
    let prev = engine.go_previous(state.position_ms).await;

    if let Some(entry) = prev {
        engine
            .play_entry(entry, "", None, app)
            .await
            .map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Volume
// ---------------------------------------------------------------------------

#[tauri::command]
pub(crate) async fn playback_set_volume(
    level: f64,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.set_volume(level).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn playback_get_volume(engine: State<'_, PlaybackEngine>) -> Result<f64, String> {
    Ok(engine.volume().await)
}

// ---------------------------------------------------------------------------
// Queue management
// ---------------------------------------------------------------------------

#[tauri::command]
pub(crate) async fn queue_add(
    entries: Vec<QueueEntry>,
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.queue_add(entries, &app).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn queue_remove(
    index: usize,
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine
        .queue_remove(index, &app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn queue_clear(
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.queue_clear(&app).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn queue_move(
    from: usize,
    to: usize,
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine
        .queue_move(from, to, &app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn queue_get(engine: State<'_, PlaybackEngine>) -> Result<QueueState, String> {
    Ok(engine.queue_state().await)
}

#[tauri::command]
pub(crate) async fn set_repeat_mode(
    mode: RepeatMode,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.set_repeat_mode(mode).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn set_shuffle(
    enabled: bool,
    app: tauri::AppHandle,
    engine: State<'_, PlaybackEngine>,
) -> Result<(), String> {
    engine.set_shuffle(enabled, &app).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// State query
// ---------------------------------------------------------------------------

#[tauri::command]
pub(crate) async fn get_playback_state(
    engine: State<'_, PlaybackEngine>,
) -> Result<PlaybackState, String> {
    Ok(engine.playback_state().await)
}

#[tauri::command]
pub(crate) async fn get_signal_path(
    engine: State<'_, PlaybackEngine>,
) -> Result<SignalPathInfo, String> {
    Ok(engine.signal_path())
}
