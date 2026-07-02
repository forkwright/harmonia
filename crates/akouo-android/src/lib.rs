use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle as ThreadJoinHandle;
use std::time::Duration;

use akouo_core::decode::probe::open_decoder;
use akouo_core::{AudioDecoder, DspConfig, DspPipeline, RingBuffer};
use snafu::Snafu;
use tokio::runtime::{Builder, Handle};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

uniffi::setup_scaffolding!();

const STATE_STOPPED: u8 = 0;
const STATE_PLAYING: u8 = 1;
const STATE_PAUSED: u8 = 2;
const DEFAULT_RING_CAPACITY: usize = 65_536;
const DEFAULT_CALLBACK_SAMPLES: usize = 1_024;
const DRAIN_SLEEP: Duration = Duration::from_millis(2);
const PAUSE_SLEEP: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, uniffi::Record)]
pub struct AndroidEngineConfig {
    pub ring_buffer_capacity: u64,
    pub callback_frame_samples: u64,
}

impl Default for AndroidEngineConfig {
    fn default() -> Self {
        Self {
            ring_buffer_capacity: DEFAULT_RING_CAPACITY as u64,
            callback_frame_samples: DEFAULT_CALLBACK_SAMPLES as u64,
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum AndroidEngineEventKind {
    PlaybackStarted,
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped,
    TrackEnded,
    SeekCompleted,
    Error,
    Underrun,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct AndroidEngineEvent {
    pub kind: AndroidEngineEventKind,
    pub path: Option<String>,
    pub message: Option<String>,
    pub position_secs: Option<f64>,
    pub underrun_count: Option<u64>,
}

#[derive(Debug, Snafu, uniffi::Error)]
pub enum AndroidEngineError {
    #[snafu(display("playback already in progress"))]
    AlreadyPlaying,
    #[snafu(display("no playback session is active"))]
    NotPlaying,
    #[snafu(display("invalid engine configuration: {message}"))]
    InvalidConfig { message: String },
    #[snafu(display("runtime initialization failed: {message}"))]
    Runtime { message: String },
    #[snafu(display("playback failed: {message}"))]
    Playback { message: String },
}

#[uniffi::export(callback_interface)]
pub trait AudioCallback: Send + Sync {
    fn on_frame(&self, samples: Vec<f64>);
}

#[uniffi::export(callback_interface)]
pub trait EventListener: Send + Sync {
    fn on_event(&self, event: AndroidEngineEvent);
}

/// A seek request forwarded to the playback task; `reply` carries the decoder's actual
/// post-seek position in seconds, or an error message.
struct SeekCommand {
    target_secs: f64,
    reply: oneshot::Sender<Result<f64, String>>,
}

type EventListeners = Arc<Mutex<Vec<(u64, Arc<dyn EventListener>)>>>;

// WHY: the callback is stored as Arc so invokers clone the handle out and
// drop the Mutex guard BEFORE calling into foreign code — holding a
// std::sync::Mutex across an FFI on_frame call stalls every other task
// (drop, callback replace) for as long as the Android side blocks.
type SharedAudioCallback = Arc<Mutex<Option<Arc<dyn AudioCallback>>>>;

#[derive(uniffi::Object)]
pub struct AndroidEngine {
    runtime: RuntimeThread,
    state: Arc<AtomicU8>,
    audio_callback: SharedAudioCallback,
    event_listeners: EventListeners,
    next_listener_id: AtomicU64,
    playback_task: Mutex<Option<JoinHandle<()>>>,
    seek_tx: Mutex<Option<mpsc::Sender<SeekCommand>>>,
    ring_buffer_capacity: usize,
    callback_frame_samples: usize,
}

#[uniffi::export]
impl AndroidEngine {
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, AndroidEngineError> {
        Self::with_config(AndroidEngineConfig::default())
    }

    #[uniffi::constructor]
    pub fn with_config(config: AndroidEngineConfig) -> Result<Arc<Self>, AndroidEngineError> {
        let ring_buffer_capacity = checked_usize(
            config.ring_buffer_capacity,
            "ring_buffer_capacity",
            DEFAULT_RING_CAPACITY,
        )?;
        let callback_frame_samples = checked_usize(
            config.callback_frame_samples,
            "callback_frame_samples",
            DEFAULT_CALLBACK_SAMPLES,
        )?;
        let ring_usable_capacity = ring_buffer_capacity.next_power_of_two().max(2) - 1;
        if callback_frame_samples > ring_usable_capacity {
            return Err(AndroidEngineError::InvalidConfig {
                message: format!(
                    "callback_frame_samples ({callback_frame_samples}) exceeds ring buffer usable capacity ({ring_usable_capacity})"
                ),
            });
        }

        Ok(Arc::new(Self {
            runtime: RuntimeThread::start()?,
            state: Arc::new(AtomicU8::new(STATE_STOPPED)),
            audio_callback: Arc::new(Mutex::new(None)),
            event_listeners: Arc::new(Mutex::new(Vec::new())),
            next_listener_id: AtomicU64::new(0),
            playback_task: Mutex::new(None),
            seek_tx: Mutex::new(None),
            ring_buffer_capacity,
            callback_frame_samples,
        }))
    }

    pub fn register_callback(&self, callback: Box<dyn AudioCallback>) {
        let mut guard = self
            .audio_callback
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = Some(Arc::from(callback));
    }

    /// Registers an event listener and returns a subscription id for
    /// `unsubscribe_events`.
    pub fn subscribe_events(&self, listener: Box<dyn EventListener>) -> u64 {
        let id = self.next_listener_id.fetch_add(1, Ordering::SeqCst);
        let mut guard = self
            .event_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.push((id, Arc::from(listener)));
        id
    }

    /// Removes the listener registered under `id`. Returns `true` if a listener was
    /// removed, `false` if the id was unknown (already removed or never issued).
    pub fn unsubscribe_events(&self, id: u64) -> bool {
        let mut guard = self
            .event_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let count_before = guard.len();
        guard.retain(|(listener_id, _)| *listener_id != id);
        guard.len() != count_before
    }

    pub async fn play(&self, path: String) -> Result<(), AndroidEngineError> {
        self.state
            .compare_exchange(
                STATE_STOPPED,
                STATE_PLAYING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map_err(|_| AndroidEngineError::AlreadyPlaying)?;

        let (seek_tx, seek_rx) = mpsc::channel::<SeekCommand>(4);
        *self.seek_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(seek_tx);

        let source_path = PathBuf::from(path.clone());
        let task_context = PlaybackTaskContext {
            state: Arc::clone(&self.state),
            callback: Arc::clone(&self.audio_callback),
            listeners: Arc::clone(&self.event_listeners),
            seek_rx,
            ring_capacity: self.ring_buffer_capacity,
            callback_samples: self.callback_frame_samples,
        };
        let task_state = Arc::clone(&task_context.state);
        let task_listeners = Arc::clone(&task_context.listeners);

        let task = self.runtime.handle().spawn(async move {
            notify(
                &task_listeners,
                AndroidEngineEvent::for_path(AndroidEngineEventKind::PlaybackStarted, &path),
            );

            if let Err(message) = playback_task(source_path, path.clone(), task_context).await {
                task_state.store(STATE_STOPPED, Ordering::SeqCst);
                notify(
                    &task_listeners,
                    AndroidEngineEvent::message(AndroidEngineEventKind::Error, message),
                );
                notify(
                    &task_listeners,
                    AndroidEngineEvent::simple(AndroidEngineEventKind::PlaybackStopped),
                );
            }
        });

        let mut guard = self.playback_task.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(task);
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), AndroidEngineError> {
        if self
            .state
            .compare_exchange(
                STATE_PLAYING,
                STATE_PAUSED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            self.notify(AndroidEngineEvent::simple(
                AndroidEngineEventKind::PlaybackPaused,
            ));
        }
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), AndroidEngineError> {
        if self
            .state
            .compare_exchange(
                STATE_PAUSED,
                STATE_PLAYING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            self.notify(AndroidEngineEvent::simple(
                AndroidEngineEventKind::PlaybackResumed,
            ));
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), AndroidEngineError> {
        self.state.store(STATE_STOPPED, Ordering::SeqCst);
        *self.seek_tx.lock().unwrap_or_else(|e| e.into_inner()) = None;
        if let Some(task) = self
            .playback_task
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            task.abort();
        }
        self.notify(AndroidEngineEvent::simple(
            AndroidEngineEventKind::PlaybackStopped,
        ));
        Ok(())
    }

    /// Seeks to `position_secs` within the current track. The request is forwarded to
    /// the playback task, which repositions the decoder, flushes buffered audio, and
    /// emits `SeekCompleted` with the decoder's actual position — also returned here.
    pub async fn seek(&self, position_secs: f64) -> Result<f64, AndroidEngineError> {
        if !position_secs.is_finite() {
            return Err(AndroidEngineError::Playback {
                message: "seek position must be finite".to_string(),
            });
        }
        if self.state.load(Ordering::SeqCst) == STATE_STOPPED {
            return Err(AndroidEngineError::NotPlaying);
        }
        let seek_tx = self
            .seek_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let Some(seek_tx) = seek_tx else {
            return Err(AndroidEngineError::NotPlaying);
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        seek_tx
            .send(SeekCommand {
                target_secs: position_secs.max(0.0),
                reply: reply_tx,
            })
            .await
            .map_err(|_| AndroidEngineError::NotPlaying)?;

        match reply_rx.await {
            Ok(Ok(actual_secs)) => Ok(actual_secs),
            Ok(Err(message)) => Err(AndroidEngineError::Playback { message }),
            // WHY: the playback task exited before replying; never fabricate success.
            Err(_) => Err(AndroidEngineError::NotPlaying),
        }
    }

    pub fn state(&self) -> String {
        match self.state.load(Ordering::SeqCst) {
            STATE_PLAYING => "playing",
            STATE_PAUSED => "paused",
            _ => "stopped",
        }
        .to_string()
    }
}

impl AndroidEngine {
    fn notify(&self, event: AndroidEngineEvent) {
        notify(&self.event_listeners, event);
    }

    #[cfg(test)]
    fn emit_test_frame(&self, samples: Vec<f64>) {
        let callback = self
            .audio_callback
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(callback) = callback {
            callback.on_frame(samples);
        }
    }
}

impl Drop for AndroidEngine {
    fn drop(&mut self) {
        self.state.store(STATE_STOPPED, Ordering::SeqCst);
        if let Some(task) = self
            .playback_task
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            task.abort();
        }
    }
}

struct PlaybackTaskContext {
    state: Arc<AtomicU8>,
    callback: SharedAudioCallback,
    listeners: EventListeners,
    seek_rx: mpsc::Receiver<SeekCommand>,
    ring_capacity: usize,
    callback_samples: usize,
}

struct DrainTaskContext {
    ring: Arc<RingBuffer>,
    state: Arc<AtomicU8>,
    producer_done: Arc<AtomicBool>,
    underruns: Arc<AtomicU64>,
    callback: SharedAudioCallback,
    listeners: EventListeners,
    callback_samples: usize,
}

async fn playback_task(
    source_path: PathBuf,
    display_path: String,
    context: PlaybackTaskContext,
) -> Result<(), String> {
    let PlaybackTaskContext {
        state,
        callback,
        listeners,
        mut seek_rx,
        ring_capacity,
        callback_samples,
    } = context;

    let mut decoder = open_decoder(&source_path)
        .await
        .map_err(|e| e.to_string())?;
    let (_dsp_tx, dsp_rx) = watch::channel(DspConfig::default());
    let mut dsp = DspPipeline::new(DspConfig::default(), dsp_rx);
    let ring = Arc::new(RingBuffer::new(ring_capacity));
    let producer_done = Arc::new(AtomicBool::new(false));
    let underruns = Arc::new(AtomicU64::new(0));

    let drain_task = tokio::spawn(drain_callback_task(DrainTaskContext {
        ring: Arc::clone(&ring),
        state: Arc::clone(&state),
        producer_done: Arc::clone(&producer_done),
        underruns: Arc::clone(&underruns),
        callback,
        listeners: Arc::clone(&listeners),
        callback_samples,
    }));

    loop {
        // WHY(#398): seeks are serviced before the state check so they also work
        // while paused.
        while let Ok(command) = seek_rx.try_recv() {
            run_seek(decoder.as_mut(), &ring, &listeners, command).await;
        }

        match state.load(Ordering::SeqCst) {
            STATE_STOPPED => break,
            STATE_PAUSED => {
                tokio::time::sleep(PAUSE_SLEEP).await;
                continue;
            }
            _ => {}
        }

        let frame = match decoder.next_frame().await {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(e) => return Err(e.to_string()),
        };

        let mut samples = frame.samples.to_vec();
        let _stage_metas = dsp.process_frame(&mut samples, frame.channels, frame.sample_rate);
        if samples.len() >= ring.capacity() {
            return Err(format!(
                "decoded frame has {} samples, exceeding ring buffer usable capacity {}",
                samples.len(),
                ring.capacity().saturating_sub(1)
            ));
        }

        loop {
            if state.load(Ordering::SeqCst) == STATE_STOPPED {
                break;
            }
            // WHY(#398): a seek landing mid-backpressure supersedes this frame — it
            // predates the new position, so it is dropped rather than pushed.
            if let Ok(command) = seek_rx.try_recv() {
                run_seek(decoder.as_mut(), &ring, &listeners, command).await;
                break;
            }
            if ring.push_frame(&samples) {
                break;
            }
            tokio::task::yield_now().await;
        }
    }

    producer_done.store(true, Ordering::SeqCst);
    let _ = drain_task.await;

    if state.swap(STATE_STOPPED, Ordering::SeqCst) != STATE_STOPPED {
        notify(
            &listeners,
            AndroidEngineEvent::for_path(AndroidEngineEventKind::TrackEnded, &display_path),
        );
        notify(
            &listeners,
            AndroidEngineEvent::simple(AndroidEngineEventKind::PlaybackStopped),
        );
    }

    Ok(())
}

/// Executes one seek command: repositions the decoder, flushes buffered audio, emits
/// `SeekCompleted` with the actual position, and replies to the caller. On failure the
/// caller receives the error and playback continues FROM the old position.
async fn run_seek(
    decoder: &mut dyn AudioDecoder,
    ring: &RingBuffer,
    listeners: &EventListeners,
    command: SeekCommand,
) {
    let target = match Duration::try_from_secs_f64(command.target_secs) {
        Ok(target) => target,
        Err(e) => {
            let message = format!("invalid seek target {}: {e}", command.target_secs);
            notify(
                listeners,
                AndroidEngineEvent::message(AndroidEngineEventKind::Error, message.clone()),
            );
            // WHY: reply send fails only when the caller stopped waiting; intentional
            command.reply.send(Err(message)).ok();
            return;
        }
    };

    match decoder.seek(target).await {
        Ok(actual) => {
            // SAFETY: producer and consumer tasks both run on the engine's dedicated
            // single-threaded runtime, so no ring access races clear() resetting the
            // positions.
            ring.clear();
            let actual_secs = actual.as_secs_f64();
            notify(
                listeners,
                AndroidEngineEvent {
                    kind: AndroidEngineEventKind::SeekCompleted,
                    path: None,
                    message: None,
                    position_secs: Some(actual_secs),
                    underrun_count: None,
                },
            );
            // WHY: reply send fails only when the caller stopped waiting; intentional
            command.reply.send(Ok(actual_secs)).ok();
        }
        Err(e) => {
            let message = e.to_string();
            notify(
                listeners,
                AndroidEngineEvent::message(AndroidEngineEventKind::Error, message.clone()),
            );
            // WHY: reply send fails only when the caller stopped waiting; intentional
            command.reply.send(Err(message)).ok();
        }
    }
}

async fn drain_callback_task(context: DrainTaskContext) {
    let mut out = vec![0.0; context.callback_samples];
    loop {
        match context.state.load(Ordering::SeqCst) {
            STATE_STOPPED => break,
            STATE_PAUSED => {
                tokio::time::sleep(PAUSE_SLEEP).await;
                continue;
            }
            _ => {}
        }

        if context.ring.pop_frame(&mut out) {
            // WHY: clone the Arc and drop the guard before the foreign call —
            // a blocking on_frame must never pin the callback Mutex.
            let callback = context
                .callback
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            if let Some(callback) = callback {
                callback.on_frame(out.clone());
            }
            continue;
        }

        if context.producer_done.load(Ordering::SeqCst) {
            let remaining = context.ring.available_to_read();
            if remaining == 0 {
                break;
            }
            let mut tail = vec![0.0; remaining];
            if context.ring.pop_frame(&mut tail) {
                let callback = context
                    .callback
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if let Some(callback) = callback {
                    callback.on_frame(tail);
                }
                continue;
            }
        }

        let count = context.underruns.fetch_add(1, Ordering::SeqCst) + 1;
        if count == 1 || count.checked_rem(100) == Some(0) {
            notify(
                &context.listeners,
                AndroidEngineEvent {
                    kind: AndroidEngineEventKind::Underrun,
                    path: None,
                    message: None,
                    position_secs: None,
                    underrun_count: Some(count),
                },
            );
        }
        tokio::time::sleep(DRAIN_SLEEP).await;
    }
}

fn notify(listeners: &EventListeners, event: AndroidEngineEvent) {
    // WHY(#400): the lock only covers snapshotting listener handles. Foreign on_event
    // callbacks run after the guard is released — invoking them under the lock
    // deadlocks any reentrant subscribe_events/unsubscribe_events call (std::sync::Mutex
    // is not reentrant).
    let snapshot: Vec<Arc<dyn EventListener>> = listeners
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(|(_, listener)| Arc::clone(listener))
        .collect();
    for listener in snapshot {
        listener.on_event(event.clone());
    }
}

impl AndroidEngineEvent {
    fn simple(kind: AndroidEngineEventKind) -> Self {
        Self {
            kind,
            path: None,
            message: None,
            position_secs: None,
            underrun_count: None,
        }
    }

    fn for_path(kind: AndroidEngineEventKind, path: &str) -> Self {
        Self {
            kind,
            path: Some(path.to_string()),
            message: None,
            position_secs: None,
            underrun_count: None,
        }
    }

    fn message(kind: AndroidEngineEventKind, message: String) -> Self {
        Self {
            kind,
            path: None,
            message: Some(message),
            position_secs: None,
            underrun_count: None,
        }
    }
}

struct RuntimeThread {
    handle: Handle,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    thread: Mutex<Option<ThreadJoinHandle<()>>>,
}

impl RuntimeThread {
    fn start() -> Result<Self, AndroidEngineError> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AndroidEngineError::Runtime {
                message: e.to_string(),
            })?;
        let handle = runtime.handle().clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("akouo-android-runtime".to_string())
            .spawn(move || {
                runtime.block_on(async {
                    let _ = shutdown_rx.await;
                });
            })
            .map_err(|e| AndroidEngineError::Runtime {
                message: e.to_string(),
            })?;

        Ok(Self {
            handle,
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
            thread: Mutex::new(Some(thread)),
        })
    }

    fn handle(&self) -> &Handle {
        &self.handle
    }
}

impl Drop for RuntimeThread {
    fn drop(&mut self) {
        if let Some(tx) = self
            .shutdown_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = tx.send(());
        }
        if let Some(thread) = self.thread.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = thread.join();
        }
    }
}

fn checked_usize(
    value: u64,
    name: &str,
    default_value: usize,
) -> Result<usize, AndroidEngineError> {
    if value == 0 {
        return Ok(default_value);
    }
    usize::try_from(value).map_err(|_| AndroidEngineError::InvalidConfig {
        message: format!("{name} exceeds platform usize"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountingCallback {
        calls: Arc<AtomicU64>,
        samples: Arc<AtomicU64>,
    }

    impl AudioCallback for CountingCallback {
        fn on_frame(&self, samples: Vec<f64>) {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.samples
                .fetch_add(samples.len() as u64, Ordering::SeqCst);
        }
    }

    struct CountingListener {
        calls: Arc<AtomicU64>,
    }

    impl EventListener for CountingListener {
        fn on_event(&self, _event: AndroidEngineEvent) {
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn emit_test_frame_reaches_registered_callback() {
        let engine = AndroidEngine::new().unwrap();
        let calls = Arc::new(AtomicU64::new(0));
        let samples = Arc::new(AtomicU64::new(0));

        engine.register_callback(Box::new(CountingCallback {
            calls: Arc::clone(&calls),
            samples: Arc::clone(&samples),
        }));
        engine.emit_test_frame(vec![0.0, 0.5, -0.5, 1.0]);

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(samples.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn rejects_callback_samples_larger_than_ring_usable_capacity() {
        let result = AndroidEngine::with_config(AndroidEngineConfig {
            ring_buffer_capacity: 8,
            callback_frame_samples: 8,
        });

        assert!(matches!(
            result,
            Err(AndroidEngineError::InvalidConfig { .. })
        ));
    }

    #[tokio::test]
    async fn pause_resume_stop_emit_events() {
        let engine = AndroidEngine::new().unwrap();
        let calls = Arc::new(AtomicU64::new(0));
        engine.subscribe_events(Box::new(CountingListener {
            calls: Arc::clone(&calls),
        }));

        engine.state.store(STATE_PLAYING, Ordering::SeqCst);
        engine.pause().await.unwrap();
        engine.resume().await.unwrap();
        engine.stop().await.unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(engine.state(), "stopped");
    }

    // --- #399: subscription ids and unsubscribe ---

    #[tokio::test]
    async fn subscribe_events_returns_unique_ids() {
        let engine = AndroidEngine::new().unwrap();
        let calls = Arc::new(AtomicU64::new(0));
        let id_a = engine.subscribe_events(Box::new(CountingListener {
            calls: Arc::clone(&calls),
        }));
        let id_b = engine.subscribe_events(Box::new(CountingListener {
            calls: Arc::clone(&calls),
        }));
        assert_ne!(id_a, id_b, "subscription ids must be unique");
    }

    #[tokio::test]
    async fn unsubscribe_removes_listener() {
        let engine = AndroidEngine::new().unwrap();
        let calls_a = Arc::new(AtomicU64::new(0));
        let calls_b = Arc::new(AtomicU64::new(0));
        let id_a = engine.subscribe_events(Box::new(CountingListener {
            calls: Arc::clone(&calls_a),
        }));
        let id_b = engine.subscribe_events(Box::new(CountingListener {
            calls: Arc::clone(&calls_b),
        }));

        engine.state.store(STATE_PLAYING, Ordering::SeqCst);
        engine.pause().await.unwrap();
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);
        assert_eq!(calls_b.load(Ordering::SeqCst), 1);

        assert!(engine.unsubscribe_events(id_a), "id_a must be removed");
        engine.resume().await.unwrap();
        assert_eq!(
            calls_a.load(Ordering::SeqCst),
            1,
            "removed listener notified"
        );
        assert_eq!(calls_b.load(Ordering::SeqCst), 2);

        assert!(
            !engine.unsubscribe_events(id_a),
            "double unsubscribe must report nothing removed"
        );
        assert!(engine.unsubscribe_events(id_b));
        engine.pause().await.unwrap();
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);
        assert_eq!(
            calls_b.load(Ordering::SeqCst),
            2,
            "removed listener notified"
        );
    }

    // --- #400: notify must not hold the listener lock during callbacks ---

    struct NoopListener;

    impl EventListener for NoopListener {
        fn on_event(&self, _event: AndroidEngineEvent) {}
    }

    struct ReentrantSubscribeListener {
        engine: Arc<AndroidEngine>,
        reentered: Arc<AtomicU64>,
    }

    impl EventListener for ReentrantSubscribeListener {
        fn on_event(&self, _event: AndroidEngineEvent) {
            if self.reentered.fetch_add(1, Ordering::SeqCst) == 0 {
                self.engine.subscribe_events(Box::new(NoopListener));
            }
        }
    }

    struct SelfRemovingListener {
        engine: Arc<AndroidEngine>,
        own_id: Arc<AtomicU64>,
        calls: Arc<AtomicU64>,
    }

    impl EventListener for SelfRemovingListener {
        fn on_event(&self, _event: AndroidEngineEvent) {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.engine
                .unsubscribe_events(self.own_id.load(Ordering::SeqCst));
        }
    }

    /// Drives one notify (via stop()) on a helper thread so a deadlock regression
    /// surfaces as a bounded test failure instead of a hang — a thread stuck inside a
    /// std::sync::Mutex cannot be cancelled by an async timeout.
    fn stop_with_deadlock_guard(engine: &Arc<AndroidEngine>) {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let engine_for_thread = Arc::clone(engine);
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("test runtime");
            runtime
                .block_on(engine_for_thread.stop())
                .expect("stop must succeed");
            done_tx.send(()).ok();
        });
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("notify() deadlocked while a listener reentered the engine");
    }

    #[test]
    fn notify_reentrant_subscribe_does_not_deadlock() {
        let engine = AndroidEngine::new().unwrap();
        let reentered = Arc::new(AtomicU64::new(0));
        engine.subscribe_events(Box::new(ReentrantSubscribeListener {
            engine: Arc::clone(&engine),
            reentered: Arc::clone(&reentered),
        }));

        stop_with_deadlock_guard(&engine);
        assert_eq!(reentered.load(Ordering::SeqCst), 1);

        // The listener added during notify participates in subsequent notifies.
        stop_with_deadlock_guard(&engine);
        assert_eq!(reentered.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn notify_reentrant_unsubscribe_does_not_deadlock() {
        let engine = AndroidEngine::new().unwrap();
        let own_id = Arc::new(AtomicU64::new(0));
        let calls = Arc::new(AtomicU64::new(0));
        let id = engine.subscribe_events(Box::new(SelfRemovingListener {
            engine: Arc::clone(&engine),
            own_id: Arc::clone(&own_id),
            calls: Arc::clone(&calls),
        }));
        own_id.store(id, Ordering::SeqCst);

        stop_with_deadlock_guard(&engine);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        stop_with_deadlock_guard(&engine);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "listener removed itself during the first notify"
        );
    }

    /// Blocks inside on_frame until released, and flags entry — proves the
    /// callback Mutex is NOT held across the foreign call.
    struct BlockingCallback {
        entered: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
    }

    impl AudioCallback for BlockingCallback {
        fn on_frame(&self, _samples: Vec<f64>) {
            self.entered.store(true, Ordering::SeqCst);
            while !self.release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn drain_task_does_not_hold_callback_lock_across_on_frame() {
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let callback: SharedAudioCallback = Arc::new(Mutex::new(Some(Arc::new(BlockingCallback {
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        })
            as Arc<dyn AudioCallback>)));

        let ring = Arc::new(RingBuffer::new(64));
        assert!(ring.push_frame(&[0.0; 8]));
        let state = Arc::new(AtomicU8::new(STATE_PLAYING));
        let drain = tokio::spawn(drain_callback_task(DrainTaskContext {
            ring,
            state: Arc::clone(&state),
            producer_done: Arc::new(AtomicBool::new(true)),
            underruns: Arc::new(AtomicU64::new(0)),
            callback: Arc::clone(&callback),
            listeners: Arc::new(Mutex::new(Vec::new())),
            callback_samples: 8,
        }));

        // Wait until the drain task is inside the (blocking) on_frame call.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !entered.load(Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "drain task never invoked the callback"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        // While on_frame is still blocked, replacing the callback must not
        // deadlock — the guard was dropped before the foreign call.
        let replaced = tokio::task::spawn_blocking(move || {
            let mut guard = callback.lock().unwrap_or_else(|e| e.into_inner());
            *guard = None;
            true
        });
        let replaced = tokio::time::timeout(Duration::from_secs(5), replaced)
            .await
            .expect("callback lock held across on_frame — replace deadlocked")
            .expect("replace thread panicked");
        assert!(replaced);

        // Unblock and shut down.
        release.store(true, Ordering::SeqCst);
        state.store(STATE_STOPPED, Ordering::SeqCst);
        tokio::time::timeout(Duration::from_secs(5), drain)
            .await
            .expect("drain task must exit")
            .expect("drain task panicked");
    }

    #[tokio::test]
    async fn playback_task_reports_open_decoder_failure() {
        let (_seek_tx, seek_rx) = mpsc::channel::<SeekCommand>(4);
        let context = PlaybackTaskContext {
            state: Arc::new(AtomicU8::new(STATE_PLAYING)),
            callback: Arc::new(Mutex::new(None)),
            listeners: Arc::new(Mutex::new(Vec::new())),
            seek_rx,
            ring_capacity: DEFAULT_RING_CAPACITY,
            callback_samples: DEFAULT_CALLBACK_SAMPLES,
        };

        let result = playback_task(
            PathBuf::from("/nonexistent/akouo-android-test.mp3"),
            "/nonexistent/akouo-android-test.mp3".to_string(),
            context,
        )
        .await;

        let message = result.expect_err("open_decoder failure must surface as Err");
        assert!(
            !message.is_empty(),
            "error message must describe the failure"
        );
    }

    // --- #398: seek must reposition playback ---

    #[tokio::test]
    async fn seek_before_play_errors() {
        let engine = AndroidEngine::new().unwrap();
        let result = engine.seek(1.0).await;
        assert!(matches!(result, Err(AndroidEngineError::NotPlaying)));
    }

    #[tokio::test]
    async fn seek_rejects_non_finite_position() {
        let engine = AndroidEngine::new().unwrap();
        let result = engine.seek(f64::INFINITY).await;
        assert!(matches!(result, Err(AndroidEngineError::Playback { .. })));
    }

    /// Records delivered samples and paces the drain (~5ms per chunk) so the test has a
    /// stable window to issue the seek before playback finishes.
    struct PacedRecordingCallback {
        samples: Arc<Mutex<Vec<f64>>>,
    }

    impl AudioCallback for PacedRecordingCallback {
        fn on_frame(&self, samples: Vec<f64>) {
            self.samples
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(&samples);
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    struct RecordingListener {
        events: Arc<Mutex<Vec<AndroidEngineEvent>>>,
    }

    impl EventListener for RecordingListener {
        fn on_event(&self, event: AndroidEngineEvent) {
            self.events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(event);
        }
    }

    /// Mono 16-bit WAV whose sample amplitude ramps 0 → 1 over the duration, so the
    /// value of a delivered sample identifies its position in the track.
    fn ramp_wav_tempfile(seconds: u32) -> tempfile::NamedTempFile {
        use std::io::Write;

        let sample_rate = 44_100u32;
        let n_samples = sample_rate * seconds;
        let data_len = n_samples * 2;
        let byte_rate = sample_rate * 2;

        let mut v = Vec::with_capacity(44 + data_len as usize);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes()); // block align
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for i in 0..n_samples {
            let value = ((f64::from(i) / f64::from(n_samples)) * 32_767.0) as i16;
            v.extend_from_slice(&value.to_le_bytes());
        }

        let mut f = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        f.write_all(&v).unwrap();
        f
    }

    #[tokio::test]
    async fn seek_moves_playback_position() {
        let engine = AndroidEngine::new().unwrap();
        let samples = Arc::new(Mutex::new(Vec::new()));
        engine.register_callback(Box::new(PacedRecordingCallback {
            samples: Arc::clone(&samples),
        }));
        let events = Arc::new(Mutex::new(Vec::new()));
        engine.subscribe_events(Box::new(RecordingListener {
            events: Arc::clone(&events),
        }));

        // 10s ramp = 441 000 samples; paced drain gives >2s of wall-clock playback.
        let wav = ramp_wav_tempfile(10);
        engine
            .play(wav.path().to_string_lossy().into_owned())
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        let actual = tokio::time::timeout(Duration::from_secs(10), engine.seek(9.0))
            .await
            .expect("seek must not hang")
            .expect("seek must succeed");
        assert!((actual - 9.0).abs() < 0.5, "actual position {actual}");

        // Await TrackEnded (only ~1s of audio remains after a real seek).
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let ended = events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .any(|e| matches!(e.kind, AndroidEngineEventKind::TrackEnded));
            if ended {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "TrackEnded not observed after seek"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let recorded = samples.lock().unwrap_or_else(|e| e.into_inner()).clone();
        // The full 10s file is 441 000 samples; a real seek skips most of it.
        assert!(
            recorded.len() < 300_000,
            "played {} samples — seek did not skip ahead",
            recorded.len()
        );
        // Ramp values >= 0.85 exist only past the 8.5s mark.
        let max = recorded.iter().fold(0.0f64, |m, &s| m.max(s));
        assert!(
            max >= 0.85,
            "no post-seek samples delivered (max amplitude {max}) — playback did not jump"
        );

        // SeekCompleted must carry the decoder's actual position.
        let seek_positions: Vec<f64> = events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|e| matches!(e.kind, AndroidEngineEventKind::SeekCompleted))
            .filter_map(|e| e.position_secs)
            .collect();
        assert!(
            seek_positions.iter().any(|&p| (p - actual).abs() < 1e-9),
            "SeekCompleted must carry the actual position, got {seek_positions:?}"
        );
    }
}
