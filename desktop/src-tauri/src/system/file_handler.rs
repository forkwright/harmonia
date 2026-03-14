//! Handles audio files opened via OS file associations ("Open With").
use std::path::PathBuf;

use tauri::Emitter;

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "m4b", "ogg", "opus", "wav", "aac"];

/// Called when the OS passes an audio file to Harmonia (e.g. "Open With").
///
/// Emits `file-open` with the file path so the frontend can initiate playback.
/// Non-audio paths are silently ignored.
pub(crate) fn handle_file_open(app: &tauri::AppHandle, path: PathBuf) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let is_audio = ext
        .as_deref()
        .map(|e| AUDIO_EXTENSIONS.contains(&e))
        .unwrap_or(false);

    if is_audio {
        let _ = app.emit("file-open", path.to_string_lossy().to_string());
    }
}

/// Checks command-line arguments for an audio file path and emits `file-open`.
///
/// Called once at app startup to handle "Open With" invocations.
pub(crate) fn handle_startup_file(app: &tauri::AppHandle) {
    if let Some(path) = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .filter(|p| p.exists())
    {
        handle_file_open(app, path);
    }
}
