//! Audiobook playback state: chapter tracking, speed control, sleep timer, position sync.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const SPEED_MIN: f64 = 0.5;
const SPEED_MAX: f64 = 3.0;
const SYNC_INTERVAL_SECS: u64 = 30;
const SLEEP_FADE_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChapterInfo {
    pub(crate) position: usize,
    pub(crate) title: String,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
}

#[derive(Debug, Clone)]
struct SleepTimerConfig {
    end_of_chapter: bool,
    duration_secs: u64,
    started_at: Instant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SleepTimerState {
    pub(crate) end_of_chapter: bool,
    pub(crate) total_secs: u64,
    pub(crate) elapsed_secs: u64,
    pub(crate) remaining_secs: u64,
    pub(crate) fading: bool,
}

#[derive(Debug)]
struct PlaybackState {
    audiobook_id: String,
    chapters: Vec<ChapterInfo>,
    current_chapter_index: usize,
    chapter_offset_ms: u64,
    playback_speed: f64,
    is_playing: bool,
    sleep_timer: Option<SleepTimerConfig>,
    server_url: String,
    token: String,
}

impl PlaybackState {
    fn current_chapter(&self) -> Option<&ChapterInfo> {
        self.chapters.get(self.current_chapter_index)
    }
}

pub(crate) struct AudiobookController {
    state: Arc<Mutex<Option<PlaybackState>>>,
    sync_handle: Mutex<Option<JoinHandle<()>>>,
}

impl AudiobookController {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
            sync_handle: Mutex::new(None),
        }
    }

    async fn start_sync_task(&self, state_arc: Arc<Mutex<Option<PlaybackState>>>) {
        let mut handle_guard = self.sync_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(SYNC_INTERVAL_SECS));
            // first tick fires immediately; skip it
            interval.tick().await;
            loop {
                interval.tick().await;
                let snapshot = {
                    let guard = state_arc.lock().await;
                    guard.as_ref().map(|s| {
                        (
                            s.audiobook_id.clone(),
                            s.current_chapter_index,
                            s.chapter_offset_ms,
                            s.is_playing,
                            s.server_url.clone(),
                            s.token.clone(),
                        )
                    })
                };
                if let Some((id, chapter, offset, is_playing, url, token)) = snapshot {
                    if !is_playing {
                        break;
                    }
                    sync_progress(&id, chapter, offset, &url, &token).await;
                } else {
                    break;
                }
            }
        });
        *handle_guard = Some(handle);
    }

    async fn stop_sync_task(&self) {
        let mut handle_guard = self.sync_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
    }
}

async fn sync_progress(id: &str, chapter: usize, offset_ms: u64, server_url: &str, token: &str) {
    let url = format!(
        "{}/api/audiobooks/{}/progress",
        server_url.trim_end_matches('/'),
        id
    );
    let body = serde_json::json!({
        "chapterPosition": chapter,
        "offsetMs": offset_ms,
    });
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    // WHY: Log failure but don't propagate — sync errors are non-fatal.
    let _ = client.put(&url).bearer_auth(token).json(&body).send().await;
}

// ── Playback commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub(crate) async fn audiobook_play(
    audiobook_id: String,
    chapters: Vec<ChapterInfo>,
    server_url: String,
    token: String,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let playback = PlaybackState {
        audiobook_id,
        chapters,
        current_chapter_index: 0,
        chapter_offset_ms: 0,
        playback_speed: 1.0,
        is_playing: true,
        sleep_timer: None,
        server_url,
        token,
    };
    *guard = Some(playback);
    drop(guard);
    state.start_sync_task(Arc::clone(&state.state)).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_play_from_chapter(
    audiobook_id: String,
    chapter: usize,
    chapters: Vec<ChapterInfo>,
    server_url: String,
    token: String,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let playback = PlaybackState {
        audiobook_id,
        chapters,
        current_chapter_index: chapter,
        chapter_offset_ms: 0,
        playback_speed: 1.0,
        is_playing: true,
        sleep_timer: None,
        server_url,
        token,
    };
    *guard = Some(playback);
    drop(guard);
    state.start_sync_task(Arc::clone(&state.state)).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_resume(
    audiobook_id: String,
    chapter: usize,
    offset_ms: u64,
    chapters: Vec<ChapterInfo>,
    server_url: String,
    token: String,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let playback = PlaybackState {
        audiobook_id,
        chapters,
        current_chapter_index: chapter,
        chapter_offset_ms: offset_ms,
        playback_speed: 1.0,
        is_playing: true,
        sleep_timer: None,
        server_url,
        token,
    };
    *guard = Some(playback);
    drop(guard);
    state.start_sync_task(Arc::clone(&state.state)).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_pause(state: State<'_, AudiobookController>) -> Result<(), String> {
    let snapshot = {
        let mut guard = state.state.lock().await;
        if let Some(s) = guard.as_mut() {
            s.is_playing = false;
            Some((
                s.audiobook_id.clone(),
                s.current_chapter_index,
                s.chapter_offset_ms,
                s.server_url.clone(),
                s.token.clone(),
            ))
        } else {
            None
        }
    };
    state.stop_sync_task().await;
    if let Some((id, chapter, offset, url, token)) = snapshot {
        sync_progress(&id, chapter, offset, &url, &token).await;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_stop(state: State<'_, AudiobookController>) -> Result<(), String> {
    let snapshot = {
        let mut guard = state.state.lock().await;
        let snap = guard.as_ref().map(|s| {
            (
                s.audiobook_id.clone(),
                s.current_chapter_index,
                s.chapter_offset_ms,
                s.server_url.clone(),
                s.token.clone(),
            )
        });
        *guard = None;
        snap
    };
    state.stop_sync_task().await;
    if let Some((id, chapter, offset, url, token)) = snapshot {
        sync_progress(&id, chapter, offset, &url, &token).await;
    }
    Ok(())
}

// ── Chapter navigation ────────────────────────────────────────────────────────

#[tauri::command]
pub(crate) async fn audiobook_next_chapter(
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let snapshot = {
        let mut guard = state.state.lock().await;
        let s = guard.as_mut().ok_or("no audiobook playing")?;
        let next = s.current_chapter_index + 1;
        if next >= s.chapters.len() {
            return Err("already at last chapter".into());
        }
        s.current_chapter_index = next;
        s.chapter_offset_ms = 0;
        Some((
            s.audiobook_id.clone(),
            s.current_chapter_index,
            s.chapter_offset_ms,
            s.server_url.clone(),
            s.token.clone(),
        ))
    };
    if let Some((id, chapter, offset, url, token)) = snapshot {
        sync_progress(&id, chapter, offset, &url, &token).await;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_prev_chapter(
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let snapshot = {
        let mut guard = state.state.lock().await;
        let s = guard.as_mut().ok_or("no audiobook playing")?;
        if s.current_chapter_index == 0 {
            return Err("already at first chapter".into());
        }
        s.current_chapter_index -= 1;
        s.chapter_offset_ms = 0;
        Some((
            s.audiobook_id.clone(),
            s.current_chapter_index,
            s.chapter_offset_ms,
            s.server_url.clone(),
            s.token.clone(),
        ))
    };
    if let Some((id, chapter, offset, url, token)) = snapshot {
        sync_progress(&id, chapter, offset, &url, &token).await;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_go_to_chapter(
    chapter: usize,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let snapshot = {
        let mut guard = state.state.lock().await;
        let s = guard.as_mut().ok_or("no audiobook playing")?;
        if chapter >= s.chapters.len() {
            return Err(format!("chapter {} out of range", chapter));
        }
        s.current_chapter_index = chapter;
        s.chapter_offset_ms = 0;
        Some((
            s.audiobook_id.clone(),
            s.current_chapter_index,
            s.chapter_offset_ms,
            s.server_url.clone(),
            s.token.clone(),
        ))
    };
    if let Some((id, chapter_idx, offset, url, token)) = snapshot {
        sync_progress(&id, chapter_idx, offset, &url, &token).await;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_skip_forward(
    seconds: u64,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    let add_ms = seconds * 1000;
    let chapter = s.current_chapter().ok_or("no current chapter")?;
    let chapter_duration_ms = chapter.end_ms - chapter.start_ms;
    let new_offset = s.chapter_offset_ms + add_ms;
    if new_offset >= chapter_duration_ms {
        let next = s.current_chapter_index + 1;
        if next < s.chapters.len() {
            s.current_chapter_index = next;
            s.chapter_offset_ms = 0;
        } else {
            s.chapter_offset_ms = chapter_duration_ms.saturating_sub(1);
        }
    } else {
        s.chapter_offset_ms = new_offset;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_skip_backward(
    seconds: u64,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    let sub_ms = seconds * 1000;
    if sub_ms >= s.chapter_offset_ms {
        if s.current_chapter_index > 0 {
            s.current_chapter_index -= 1;
            s.chapter_offset_ms = 0;
        } else {
            s.chapter_offset_ms = 0;
        }
    } else {
        s.chapter_offset_ms -= sub_ms;
    }
    Ok(())
}

// ── Speed control ─────────────────────────────────────────────────────────────

#[tauri::command]
pub(crate) async fn audiobook_set_speed(
    speed: f64,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    if !(SPEED_MIN..=SPEED_MAX).contains(&speed) {
        return Err(format!(
            "speed must be between {} and {}",
            SPEED_MIN, SPEED_MAX
        ));
    }
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    s.playback_speed = speed;
    Ok(())
}

#[tauri::command]
pub(crate) async fn audiobook_get_speed(
    state: State<'_, AudiobookController>,
) -> Result<f64, String> {
    let guard = state.state.lock().await;
    let s = guard.as_ref().ok_or("no audiobook playing")?;
    Ok(s.playback_speed)
}

// ── Sleep timer ───────────────────────────────────────────────────────────────

#[tauri::command]
pub(crate) async fn sleep_timer_set(
    minutes: u64,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    s.sleep_timer = Some(SleepTimerConfig {
        end_of_chapter: false,
        duration_secs: minutes * 60,
        started_at: Instant::now(),
    });
    Ok(())
}

#[tauri::command]
pub(crate) async fn sleep_timer_set_end_of_chapter(
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    s.sleep_timer = Some(SleepTimerConfig {
        end_of_chapter: true,
        duration_secs: 0,
        started_at: Instant::now(),
    });
    Ok(())
}

#[tauri::command]
pub(crate) async fn sleep_timer_cancel(
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    s.sleep_timer = None;
    Ok(())
}

#[tauri::command]
pub(crate) async fn sleep_timer_extend(
    minutes: u64,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    if let Some(timer) = s.sleep_timer.as_mut() {
        timer.duration_secs += minutes * 60;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn sleep_timer_get(
    state: State<'_, AudiobookController>,
) -> Result<Option<SleepTimerState>, String> {
    let guard = state.state.lock().await;
    let s = match guard.as_ref() {
        Some(s) => s,
        None => return Ok(None),
    };
    let timer = match s.sleep_timer.as_ref() {
        Some(t) => t,
        None => return Ok(None),
    };
    let elapsed_secs = timer.started_at.elapsed().as_secs();
    let (total_secs, remaining_secs) = if timer.end_of_chapter {
        let chapter_remaining = s
            .current_chapter()
            .map(|c| {
                let duration = c.end_ms - c.start_ms;
                duration.saturating_sub(s.chapter_offset_ms) / 1000
            })
            .unwrap_or(0);
        (chapter_remaining, chapter_remaining)
    } else {
        let remaining = timer.duration_secs.saturating_sub(elapsed_secs);
        (timer.duration_secs, remaining)
    };
    let fading = remaining_secs <= SLEEP_FADE_SECS;
    Ok(Some(SleepTimerState {
        end_of_chapter: timer.end_of_chapter,
        total_secs,
        elapsed_secs,
        remaining_secs,
        fading,
    }))
}

// ── Position query ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlaybackPosition {
    pub(crate) audiobook_id: String,
    pub(crate) chapter_index: usize,
    pub(crate) chapter_offset_ms: u64,
    pub(crate) chapter_title: String,
    pub(crate) playback_speed: f64,
    pub(crate) is_playing: bool,
}

#[tauri::command]
pub(crate) async fn audiobook_get_position(
    state: State<'_, AudiobookController>,
) -> Result<Option<PlaybackPosition>, String> {
    let guard = state.state.lock().await;
    let s = match guard.as_ref() {
        Some(s) => s,
        None => return Ok(None),
    };
    let chapter_title = s
        .current_chapter()
        .map(|c| c.title.clone())
        .unwrap_or_default();
    Ok(Some(PlaybackPosition {
        audiobook_id: s.audiobook_id.clone(),
        chapter_index: s.current_chapter_index,
        chapter_offset_ms: s.chapter_offset_ms,
        chapter_title,
        playback_speed: s.playback_speed,
        is_playing: s.is_playing,
    }))
}

#[tauri::command]
pub(crate) async fn audiobook_update_offset(
    offset_ms: u64,
    state: State<'_, AudiobookController>,
) -> Result<(), String> {
    let mut guard = state.state.lock().await;
    let s = guard.as_mut().ok_or("no audiobook playing")?;
    s.chapter_offset_ms = offset_ms;
    Ok(())
}
