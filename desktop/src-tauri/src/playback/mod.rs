//! Playback state management for podcast and audiobook modes, plus general playback engine.
pub(crate) mod audiobook;
pub mod podcast;

pub(crate) mod commands;
pub(crate) mod queue;
pub(crate) mod signal_path;
pub(crate) mod stream;

use std::sync::Arc;
use std::time::Instant;

use akroasis_core::{AudioSource, Engine, EngineConfig, EngineEvent};
use serde::Serialize;
use snafu::{ResultExt, Snafu};
use tauri::Emitter;
use tokio::sync::Mutex;
use tracing::{instrument, warn};

pub(crate) use queue::{DesktopQueue, QueueEntry, RepeatMode};
pub(crate) use signal_path::SignalPathInfo;

#[derive(Debug, Snafu)]
pub(crate) enum PlaybackError {
    #[snafu(display("failed to create audio engine: {source}"))]
    EngineCreate {
        source: akroasis_core::EngineError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("failed to start playback: {source}"))]
    EnginePlay {
        source: akroasis_core::EngineError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("no track is currently loaded"))]
    NoTrack {
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("stream error: {source}"))]
    Stream {
        source: stream::StreamError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("queue index {index} out of bounds"))]
    QueueBounds {
        index: usize,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlaybackStatus {
    Stopped,
    Buffering,
    Playing,
    Paused,
}

/// Display metadata for the currently playing track.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct TrackInfo {
    pub track_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
}

impl From<QueueEntry> for TrackInfo {
    fn from(e: QueueEntry) -> Self {
        Self {
            track_id: e.track_id,
            title: e.title,
            artist: e.artist,
            album: e.album,
            duration_ms: e.duration_ms,
        }
    }
}

/// Snapshot of playback state for frontend polling.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PlaybackState {
    pub status: PlaybackStatus,
    pub track: Option<TrackInfo>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f64,
    pub repeat_mode: RepeatMode,
    pub shuffle: bool,
}

/// Tauri event payloads.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProgressEvent {
    pub position_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PlaybackStateEvent {
    pub status: PlaybackStatus,
    pub track: Option<TrackInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct QueueChangedEvent {
    pub entries: Vec<QueueEntry>,
    pub current_index: usize,
}

/// Snapshot of queue state for `queue_get` command.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct QueueState {
    pub entries: Vec<QueueEntry>,
    pub current_index: usize,
    pub repeat_mode: RepeatMode,
    pub shuffle: bool,
    pub source_label: String,
}

struct PlaybackInner {
    status: PlaybackStatus,
    current_track: Option<TrackInfo>,
    position_ms: u64,
    duration_ms: u64,
    volume: f64,
    /// Monotonic instant when `Playing` last started (or resumed).
    play_start: Option<Instant>,
    /// Accumulated position before the last pause.
    pause_offset_ms: u64,
    queue: DesktopQueue,
    /// Path to the currently cached stream temp file.
    current_stream_path: Option<std::path::PathBuf>,
    progress_task: Option<tokio::task::JoinHandle<()>>,
}

impl PlaybackInner {
    fn current_position_ms(&self) -> u64 {
        match self.play_start {
            Some(start) => self.pause_offset_ms + start.elapsed().as_millis() as u64,
            None => self.pause_offset_ms,
        }
    }
}

/// Manages the akroasis-core audio pipeline for the desktop app.
///
/// All public methods take `&self` and synchronise internally via `tokio::sync::Mutex`.
pub(crate) struct PlaybackEngine {
    engine: Arc<Engine>,
    inner: Arc<Mutex<PlaybackInner>>,
    http: reqwest::Client,
    /// Broadcasts state changes to subscribers (e.g. the MPRIS bridge).
    state_tx: tokio::sync::broadcast::Sender<PlaybackStateEvent>,
}

// SAFETY: Arc<Engine> is Send+Sync (Engine explicitly marks itself so).
// Arc<Mutex<PlaybackInner>> is Send+Sync when PlaybackInner: Send.
unsafe impl Send for PlaybackEngine {}
unsafe impl Sync for PlaybackEngine {}

impl PlaybackEngine {
    pub(crate) fn new() -> Result<Self, PlaybackError> {
        let engine = Engine::new(EngineConfig::default()).context(EngineCreateSnafu)?;
        let (state_tx, _) = tokio::sync::broadcast::channel(32);
        Ok(Self {
            engine: Arc::new(engine),
            inner: Arc::new(Mutex::new(PlaybackInner {
                status: PlaybackStatus::Stopped,
                current_track: None,
                position_ms: 0,
                duration_ms: 0,
                volume: 1.0,
                play_start: None,
                pause_offset_ms: 0,
                queue: DesktopQueue::new(),
                current_stream_path: None,
                progress_task: None,
            })),
            http: reqwest::Client::new(),
            state_tx,
        })
    }

    /// Returns a receiver for playback state change events (used by the MPRIS bridge).
    pub(crate) fn subscribe_state(&self) -> tokio::sync::broadcast::Receiver<PlaybackStateEvent> {
        self.state_tx.subscribe()
    }

    // ---------------------------------------------------------------------------
    // Transport controls
    // ---------------------------------------------------------------------------

    /// Loads and starts playing `entry`. Fetches the audio stream from `base_url`.
    #[instrument(skip(self, app))]
    pub(crate) async fn play_entry(
        &self,
        entry: QueueEntry,
        base_url: &str,
        token: Option<&str>,
        app: tauri::AppHandle,
    ) -> Result<(), PlaybackError> {
        {
            let mut guard = self.inner.lock().await;
            guard.status = PlaybackStatus::Buffering;
            let track: TrackInfo = entry.clone().into();
            guard.current_track = Some(track.clone());
            emit_state(&app, PlaybackStatus::Buffering, Some(track), &self.state_tx);
        }

        let path = stream::fetch_stream(&self.http, base_url, &entry.track_id, token)
            .await
            .context(StreamSnafu)?;

        // Stop any current session before starting a new one.
        if let Err(e) = self.engine.stop() {
            warn!(error = %e, "engine stop before play");
        }

        self.engine
            .play(AudioSource::File(path.clone()))
            .context(EnginePlaySnafu)?;

        let duration_ms = entry.duration_ms.unwrap_or(0);
        let track: TrackInfo = entry.clone().into();

        {
            let mut guard = self.inner.lock().await;
            guard.status = PlaybackStatus::Playing;
            guard.current_track = Some(track.clone());
            guard.duration_ms = duration_ms;
            guard.position_ms = 0;
            guard.pause_offset_ms = 0;
            guard.play_start = Some(Instant::now());
            guard.current_stream_path = Some(path);

            // Cancel previous progress task.
            if let Some(h) = guard.progress_task.take() {
                h.abort();
            }

            let inner = Arc::clone(&self.inner);
            let app2 = app.clone();
            let task = tokio::spawn(async move {
                progress_task(inner, app2).await;
            });
            guard.progress_task = Some(task);
        }

        emit_state(&app, PlaybackStatus::Playing, Some(track), &self.state_tx);

        // Subscribe to engine events to handle track end.
        let inner = Arc::clone(&self.inner);
        let engine = Arc::clone(&self.engine);
        let app2 = app.clone();
        let state_tx2 = self.state_tx.clone();
        tokio::spawn(async move {
            event_listener(inner, engine, app2, state_tx2).await;
        });

        Ok(())
    }

    #[instrument(skip(self, app))]
    pub(crate) async fn pause(&self, app: &tauri::AppHandle) -> Result<(), PlaybackError> {
        let mut guard = self.inner.lock().await;
        if guard.status != PlaybackStatus::Playing {
            return Ok(());
        }
        guard.pause_offset_ms = guard.current_position_ms();
        guard.play_start = None;
        guard.status = PlaybackStatus::Paused;
        let track = guard.current_track.clone();
        drop(guard);

        if let Err(e) = self.engine.pause() {
            warn!(error = %e, "engine pause");
        }
        emit_state(app, PlaybackStatus::Paused, track, &self.state_tx);
        Ok(())
    }

    #[instrument(skip(self, app))]
    pub(crate) async fn resume(&self, app: &tauri::AppHandle) -> Result<(), PlaybackError> {
        let mut guard = self.inner.lock().await;
        if guard.status != PlaybackStatus::Paused {
            return Ok(());
        }
        guard.play_start = Some(Instant::now());
        guard.status = PlaybackStatus::Playing;
        let track = guard.current_track.clone();
        drop(guard);

        if let Err(e) = self.engine.resume() {
            warn!(error = %e, "engine resume");
        }
        emit_state(app, PlaybackStatus::Playing, track, &self.state_tx);
        Ok(())
    }

    #[instrument(skip(self, app))]
    pub(crate) async fn stop(&self, app: &tauri::AppHandle) {
        let mut guard = self.inner.lock().await;
        guard.status = PlaybackStatus::Stopped;
        guard.current_track = None;
        guard.position_ms = 0;
        guard.pause_offset_ms = 0;
        guard.play_start = None;
        if let Some(h) = guard.progress_task.take() {
            h.abort();
        }
        // Clean up temp stream file.
        if let Some(path) = guard.current_stream_path.take() {
            let _ = std::fs::remove_file(path);
        }
        drop(guard);

        if let Err(e) = self.engine.stop() {
            warn!(error = %e, "engine stop");
        }
        emit_state(app, PlaybackStatus::Stopped, None, &self.state_tx);
    }

    #[instrument(skip(self))]
    pub(crate) async fn seek(&self, position_ms: u64) -> Result<(), PlaybackError> {
        let mut guard = self.inner.lock().await;
        if guard.current_track.is_none() {
            return Err(NoTrackSnafu.build());
        }
        let was_playing = guard.status == PlaybackStatus::Playing;
        guard.pause_offset_ms = position_ms;
        guard.play_start = if was_playing {
            Some(Instant::now())
        } else {
            None
        };
        guard.position_ms = position_ms;
        drop(guard);

        let pos = std::time::Duration::from_millis(position_ms);
        if let Err(e) = self.engine.seek(pos) {
            warn!(error = %e, "engine seek");
        }
        Ok(())
    }

    pub(crate) async fn set_volume(&self, level: f64) {
        let mut guard = self.inner.lock().await;
        guard.volume = level.clamp(0.0, 1.0);
        drop(guard);

        // Apply via DSP volume stage.
        let mut dsp = akroasis_core::DspConfig::default();
        dsp.volume.level_db = volume_to_db(level);
        self.engine.configure_dsp(dsp);
    }

    pub(crate) async fn volume(&self) -> f64 {
        self.inner.lock().await.volume
    }

    // ---------------------------------------------------------------------------
    // Queue management
    // ---------------------------------------------------------------------------

    pub(crate) async fn queue_add(&self, entries: Vec<QueueEntry>, app: &tauri::AppHandle) {
        let mut guard = self.inner.lock().await;
        guard.queue.append(entries);
        emit_queue_changed(&guard.queue, app);
    }

    pub(crate) async fn queue_remove(
        &self,
        index: usize,
        app: &tauri::AppHandle,
    ) -> Result<(), PlaybackError> {
        let mut guard = self.inner.lock().await;
        if index >= guard.queue.display_entries().len() {
            return Err(QueueBoundsSnafu { index }.build());
        }
        guard.queue.remove(index);
        emit_queue_changed(&guard.queue, app);
        Ok(())
    }

    pub(crate) async fn queue_clear(&self, app: &tauri::AppHandle) {
        let mut guard = self.inner.lock().await;
        guard.queue.clear();
        emit_queue_changed(&guard.queue, app);
    }

    pub(crate) async fn queue_move(
        &self,
        from: usize,
        to: usize,
        app: &tauri::AppHandle,
    ) -> Result<(), PlaybackError> {
        let mut guard = self.inner.lock().await;
        let len = guard.queue.display_entries().len();
        if from >= len || to >= len {
            return Err(QueueBoundsSnafu {
                index: from.max(to),
            }
            .build());
        }
        guard.queue.move_entry(from, to);
        emit_queue_changed(&guard.queue, app);
        Ok(())
    }

    pub(crate) async fn queue_state(&self) -> QueueState {
        let guard = self.inner.lock().await;
        QueueState {
            entries: guard.queue.display_entries().into_iter().cloned().collect(),
            current_index: guard.queue.current_display_index(),
            repeat_mode: guard.queue.repeat,
            shuffle: guard.queue.shuffle_enabled(),
            source_label: guard.queue.source_label.clone(),
        }
    }

    pub(crate) async fn set_repeat_mode(&self, mode: RepeatMode) {
        let mut guard = self.inner.lock().await;
        guard.queue.repeat = mode;
    }

    pub(crate) async fn set_shuffle(&self, enabled: bool, app: &tauri::AppHandle) {
        let mut guard = self.inner.lock().await;
        guard.queue.set_shuffle(enabled);
        emit_queue_changed(&guard.queue, app);
    }

    // ---------------------------------------------------------------------------
    // State query
    // ---------------------------------------------------------------------------

    pub(crate) async fn playback_state(&self) -> PlaybackState {
        let mut guard = self.inner.lock().await;
        let pos = guard.current_position_ms();
        guard.position_ms = pos;
        PlaybackState {
            status: guard.status,
            track: guard.current_track.clone(),
            position_ms: pos,
            duration_ms: guard.duration_ms,
            volume: guard.volume,
            repeat_mode: guard.queue.repeat,
            shuffle: guard.queue.shuffle_enabled(),
        }
    }

    /// Returns the previous queue entry using the `back` strategy on the live queue.
    ///
    /// Passing `position_ms` allows the queue to decide whether to restart the current
    /// track (when > 3 s) or go to the preceding track.
    pub(crate) async fn go_previous(&self, position_ms: u64) -> Option<QueueEntry> {
        let mut guard = self.inner.lock().await;
        guard.queue.back(position_ms).cloned()
    }

    pub(crate) fn signal_path(&self) -> SignalPathInfo {
        self.engine.signal_path().into()
    }
}

// ---------------------------------------------------------------------------
// Background tasks
// ---------------------------------------------------------------------------

/// Emits `playback-progress` events every 250 ms while playing.
async fn progress_task(inner: Arc<Mutex<PlaybackInner>>, app: tauri::AppHandle) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(250));
    loop {
        interval.tick().await;
        let mut guard = inner.lock().await;
        if guard.status == PlaybackStatus::Stopped {
            break;
        }
        if guard.status == PlaybackStatus::Playing {
            guard.position_ms = guard.current_position_ms();
        }
        let pos = guard.position_ms;
        let dur = guard.duration_ms;
        drop(guard);

        let _ = app.emit(
            "playback-progress",
            ProgressEvent {
                position_ms: pos,
                duration_ms: dur,
            },
        );
    }
}

/// Listens to engine events to handle natural track end -> advance queue.
async fn event_listener(
    inner: Arc<Mutex<PlaybackInner>>,
    _engine: Arc<Engine>,
    app: tauri::AppHandle,
    state_tx: tokio::sync::broadcast::Sender<PlaybackStateEvent>,
) {
    let mut rx = { _engine.subscribe_events() };
    loop {
        match rx.recv().await {
            Ok(EngineEvent::TrackEnded { .. }) => {
                let mut guard = inner.lock().await;
                if guard.queue.is_empty() {
                    guard.status = PlaybackStatus::Stopped;
                    guard.current_track = None;
                    guard.pause_offset_ms = 0;
                    guard.play_start = None;
                    drop(guard);
                    emit_state(&app, PlaybackStatus::Stopped, None, &state_tx);
                    break;
                }
                let next = guard.queue.advance().cloned();
                if let Some(entry) = next {
                    let track: TrackInfo = entry.clone().into();
                    guard.current_track = Some(track.clone());
                    emit_queue_changed(&guard.queue, &app);
                    drop(guard);
                    emit_state(&app, PlaybackStatus::Stopped, None, &state_tx);
                } else {
                    guard.status = PlaybackStatus::Stopped;
                    guard.current_track = None;
                    guard.pause_offset_ms = 0;
                    guard.play_start = None;
                    drop(guard);
                    emit_state(&app, PlaybackStatus::Stopped, None, &state_tx);
                    break;
                }
            }
            Ok(EngineEvent::PlaybackStopped) => break,
            Ok(EngineEvent::Error { message }) => {
                warn!(message, "engine error during playback");
            }
            Err(_) => break,
            Ok(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn emit_state(
    app: &tauri::AppHandle,
    status: PlaybackStatus,
    track: Option<TrackInfo>,
    state_tx: &tokio::sync::broadcast::Sender<PlaybackStateEvent>,
) {
    let event = PlaybackStateEvent { status, track };
    let _ = app.emit("playback-state-changed", &event);
    // Notify MPRIS bridge and other internal subscribers.
    let _ = state_tx.send(event);
}

fn emit_queue_changed(queue: &DesktopQueue, app: &tauri::AppHandle) {
    let entries: Vec<QueueEntry> = queue.display_entries().into_iter().cloned().collect();
    let _ = app.emit(
        "queue-changed",
        QueueChangedEvent {
            entries,
            current_index: queue.current_display_index(),
        },
    );
}

fn volume_to_db(linear: f64) -> f64 {
    if linear <= 0.0 {
        -144.0
    } else {
        20.0 * linear.log10()
    }
}
