//! Podcast-specific playback state and Tauri IPC commands.
//!
//! Manages speed, position tracking, and trim-silence state for podcast
//! playback. Actual audio rendering is delegated to the akroasis-core engine
//! when P3-11 integration is complete; this module owns only the
//! podcast-specific metadata layer.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::State;

const MIN_SPEED: f64 = 0.5;
const MAX_SPEED: f64 = 3.0;
const DEFAULT_SPEED: f64 = 1.0;
// WHY: 30 s is the industry-standard sync interval for podcast progress.
const DEFAULT_SYNC_INTERVAL: Duration = Duration::from_secs(30);

struct PodcastPlayback {
    episode_id: String,
    playback_speed: f64,
    position_sync_interval: Duration,
    last_sync: Instant,
    trim_silence: bool,
    position_ms: u64,
}

impl PodcastPlayback {
    fn new(episode_id: String, speed: f64, trim_silence: bool, position_ms: u64) -> Self {
        Self {
            episode_id,
            playback_speed: speed,
            position_sync_interval: DEFAULT_SYNC_INTERVAL,
            last_sync: Instant::now(),
            trim_silence,
            position_ms,
        }
    }

    fn sync_due(&self) -> bool {
        self.last_sync.elapsed() >= self.position_sync_interval
    }

    fn mark_synced(&mut self) {
        self.last_sync = Instant::now();
    }
}

/// Snapshot of podcast playback state returned to the frontend.
#[derive(Debug, Serialize)]
pub struct PlaybackSnapshot {
    pub episode_id: String,
    pub position_ms: u64,
    pub playback_speed: f64,
    pub trim_silence: bool,
    pub sync_due: bool,
}

/// Managed state for podcast playback, held by the Tauri application.
pub struct PodcastController {
    state: Mutex<Option<PodcastPlayback>>,
    speed: Mutex<f64>,
    trim_silence: Mutex<bool>,
}

impl Default for PodcastController {
    fn default() -> Self {
        Self {
            state: Mutex::new(None),
            speed: Mutex::new(DEFAULT_SPEED),
            trim_silence: Mutex::new(false),
        }
    }
}

impl PodcastController {
    pub fn new() -> Self {
        Self::default()
    }
}

#[tauri::command]
pub fn podcast_play_episode(
    episode_id: String,
    controller: State<'_, PodcastController>,
) -> Result<(), String> {
    let speed = *controller.speed.lock().expect("speed lock poisoned");
    let trim = *controller.trim_silence.lock().expect("trim_silence lock poisoned");
    let mut state = controller.state.lock().expect("state lock poisoned");
    *state = Some(PodcastPlayback::new(episode_id, speed, trim, 0));
    Ok(())
}

#[tauri::command]
pub fn podcast_resume_episode(
    episode_id: String,
    position_ms: u64,
    controller: State<'_, PodcastController>,
) -> Result<(), String> {
    let speed = *controller.speed.lock().expect("speed lock poisoned");
    let trim = *controller.trim_silence.lock().expect("trim_silence lock poisoned");
    let mut state = controller.state.lock().expect("state lock poisoned");
    *state = Some(PodcastPlayback::new(episode_id, speed, trim, position_ms));
    Ok(())
}

#[tauri::command]
pub fn podcast_set_speed(
    speed: f64,
    controller: State<'_, PodcastController>,
) -> Result<(), String> {
    if !(MIN_SPEED..=MAX_SPEED).contains(&speed) {
        return Err(format!("speed must be between {MIN_SPEED} and {MAX_SPEED}"));
    }
    *controller.speed.lock().expect("speed lock poisoned") = speed;
    if let Some(pb) = controller.state.lock().expect("state lock poisoned").as_mut() {
        pb.playback_speed = speed;
    }
    Ok(())
}

#[tauri::command]
pub fn podcast_get_speed(controller: State<'_, PodcastController>) -> Result<f64, String> {
    Ok(*controller.speed.lock().expect("speed lock poisoned"))
}

#[tauri::command]
pub fn podcast_skip_forward(
    seconds: u64,
    controller: State<'_, PodcastController>,
) -> Result<(), String> {
    let mut state = controller.state.lock().expect("state lock poisoned");
    match state.as_mut() {
        Some(pb) => {
            pb.position_ms = pb.position_ms.saturating_add(seconds.saturating_mul(1_000));
            Ok(())
        }
        None => Err("no episode is playing".to_string()),
    }
}

#[tauri::command]
pub fn podcast_skip_backward(
    seconds: u64,
    controller: State<'_, PodcastController>,
) -> Result<(), String> {
    let mut state = controller.state.lock().expect("state lock poisoned");
    match state.as_mut() {
        Some(pb) => {
            pb.position_ms = pb.position_ms.saturating_sub(seconds.saturating_mul(1_000));
            Ok(())
        }
        None => Err("no episode is playing".to_string()),
    }
}

#[tauri::command]
pub fn podcast_set_trim_silence(
    enabled: bool,
    controller: State<'_, PodcastController>,
) -> Result<(), String> {
    *controller.trim_silence.lock().expect("trim_silence lock poisoned") = enabled;
    if let Some(pb) = controller.state.lock().expect("state lock poisoned").as_mut() {
        pb.trim_silence = enabled;
    }
    Ok(())
}

/// Returns a snapshot of the current playback state; `None` if nothing is playing.
/// The `sync_due` field signals whether the frontend should push a progress update.
#[tauri::command]
pub fn podcast_get_playback_snapshot(
    controller: State<'_, PodcastController>,
) -> Result<Option<PlaybackSnapshot>, String> {
    let mut state = controller.state.lock().expect("state lock poisoned");
    let snapshot = state.as_mut().map(|pb| {
        let sync_due = pb.sync_due();
        if sync_due {
            pb.mark_synced();
        }
        PlaybackSnapshot {
            episode_id: pb.episode_id.clone(),
            position_ms: pb.position_ms,
            playback_speed: pb.playback_speed,
            trim_silence: pb.trim_silence,
            sync_due,
        }
    });
    Ok(snapshot)
}
