use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle as ThreadJoinHandle;
use std::time::Duration;

use akouo_core::decode::probe::open_decoder;
use akouo_core::{DspConfig, DspPipeline, RingBuffer};
use snafu::Snafu;
use tokio::runtime::{Builder, Handle};
use tokio::sync::{oneshot, watch};
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

#[derive(uniffi::Object)]
pub struct AndroidEngine {
    runtime: RuntimeThread,
    state: Arc<AtomicU8>,
    audio_callback: Arc<Mutex<Option<Box<dyn AudioCallback>>>>,
    event_listeners: Arc<Mutex<Vec<Box<dyn EventListener>>>>,
    playback_task: Mutex<Option<JoinHandle<()>>>,
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
            playback_task: Mutex::new(None),
            ring_buffer_capacity,
            callback_frame_samples,
        }))
    }

    pub fn register_callback(&self, callback: Box<dyn AudioCallback>) {
        let mut guard = self
            .audio_callback
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = Some(callback);
    }

    pub fn subscribe_events(&self, listener: Box<dyn EventListener>) {
        let mut guard = self
            .event_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.push(listener);
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

        let source_path = PathBuf::from(path.clone());
        let task_context = PlaybackTaskContext {
            state: Arc::clone(&self.state),
            callback: Arc::clone(&self.audio_callback),
            listeners: Arc::clone(&self.event_listeners),
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

    pub async fn seek(&self, position_secs: f64) -> Result<f64, AndroidEngineError> {
        if self.state.load(Ordering::SeqCst) == STATE_STOPPED {
            return Err(AndroidEngineError::NotPlaying);
        }
        self.notify(AndroidEngineEvent {
            kind: AndroidEngineEventKind::SeekCompleted,
            path: None,
            message: None,
            position_secs: Some(position_secs.max(0.0)),
            underrun_count: None,
        });
        Ok(position_secs.max(0.0))
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
        if let Some(callback) = self
            .audio_callback
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
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
    callback: Arc<Mutex<Option<Box<dyn AudioCallback>>>>,
    listeners: Arc<Mutex<Vec<Box<dyn EventListener>>>>,
    ring_capacity: usize,
    callback_samples: usize,
}

struct DrainTaskContext {
    ring: Arc<RingBuffer>,
    state: Arc<AtomicU8>,
    producer_done: Arc<AtomicBool>,
    underruns: Arc<AtomicU64>,
    callback: Arc<Mutex<Option<Box<dyn AudioCallback>>>>,
    listeners: Arc<Mutex<Vec<Box<dyn EventListener>>>>,
    callback_samples: usize,
}

async fn playback_task(
    source_path: PathBuf,
    display_path: String,
    context: PlaybackTaskContext,
) -> Result<(), String> {
    let mut decoder = open_decoder(&source_path)
        .await
        .map_err(|e| e.to_string())?;
    let (_dsp_tx, dsp_rx) = watch::channel(DspConfig::default());
    let mut dsp = DspPipeline::new(DspConfig::default(), dsp_rx);
    let ring = Arc::new(RingBuffer::new(context.ring_capacity));
    let producer_done = Arc::new(AtomicBool::new(false));
    let underruns = Arc::new(AtomicU64::new(0));

    let drain_task = tokio::spawn(drain_callback_task(DrainTaskContext {
        ring: Arc::clone(&ring),
        state: Arc::clone(&context.state),
        producer_done: Arc::clone(&producer_done),
        underruns: Arc::clone(&underruns),
        callback: context.callback,
        listeners: Arc::clone(&context.listeners),
        callback_samples: context.callback_samples,
    }));

    loop {
        match context.state.load(Ordering::SeqCst) {
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
            if context.state.load(Ordering::SeqCst) == STATE_STOPPED {
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

    if context.state.swap(STATE_STOPPED, Ordering::SeqCst) != STATE_STOPPED {
        notify(
            &context.listeners,
            AndroidEngineEvent::for_path(AndroidEngineEventKind::TrackEnded, &display_path),
        );
        notify(
            &context.listeners,
            AndroidEngineEvent::simple(AndroidEngineEventKind::PlaybackStopped),
        );
    }

    Ok(())
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
            if let Some(callback) = context
                .callback
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
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
                if let Some(callback) = context
                    .callback
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                {
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

fn notify(listeners: &Arc<Mutex<Vec<Box<dyn EventListener>>>>, event: AndroidEngineEvent) {
    for listener in listeners.lock().unwrap_or_else(|e| e.into_inner()).iter() {
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
}
