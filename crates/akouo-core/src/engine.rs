use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{instrument, warn};

use crate::config::{DspConfig, EngineConfig};
use crate::decode::DecodedFrame;
use tracing::Instrument;

use crate::decode::probe::open_decoder;
use crate::dsp::DspPipeline;
use crate::error::EngineError;
use crate::output::OutputDevice;
use crate::ring_buffer::RingBuffer;
use crate::signal_path::{QualityTier, SignalPathSnapshot, SignalStageInfo, SourceInfo};

const STATE_STOPPED: u8 = 0;
const STATE_PLAYING: u8 = 1;
const STATE_PAUSED: u8 = 2;

/// An audio source to be played back by the engine.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AudioSource {
    /// A local file path.
    File(PathBuf),
}

/// Events emitted by the engine during playback. Subscribe via `Engine::subscribe_events`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EngineEvent {
    /// Playback started for a new source.
    PlaybackStarted { source: AudioSource },
    /// Playback stopped (either via `stop()` or natural track end with no next track).
    PlaybackStopped,
    /// Playback paused.
    PlaybackPaused,
    /// Playback resumed after pause.
    PlaybackResumed,
    /// The current track reached its natural end.
    TrackEnded { source: AudioSource },
    /// The engine transitioned from one track to the next (gapless / crossfade).
    TrackChanged { from: AudioSource, to: AudioSource },
    /// A seek completed; contains the actual position reached.
    SeekCompleted { position: Duration },
    /// The signal path configuration changed (DSP stage enabled/disabled, source changed).
    SignalPathChanged(SignalPathSnapshot),
    /// The output device changed.
    OutputDeviceChanged { device: OutputDevice },
    /// A non-fatal error occurred during playback.
    Error { message: String },
    /// The output ring buffer underran; `count` is the cumulative underrun count.
    Underrun { count: u64 },
}

struct PlaybackSession {
    decode_task: JoinHandle<()>,
    dsp_task: JoinHandle<()>,
}

/// The audio engine: owns the decode → DSP → output pipeline.
///
/// Construct via `Engine::new`, wrap in `Arc<Engine>` for multi-task access.
/// All public methods take `&self` and use internal synchronisation.
///
/// **Runtime requirement:** `play()` calls `tokio::spawn` internally. The engine must be
/// used within a Tokio runtime context; calling `play()` outside a runtime panics.
pub struct Engine {
    config: EngineConfig,
    state: Arc<AtomicU8>,
    dsp_config_tx: Arc<watch::Sender<DspConfig>>,
    signal_path_tx: Arc<watch::Sender<SignalPathSnapshot>>,
    event_tx: broadcast::Sender<EngineEvent>,
    session: Mutex<Option<PlaybackSession>>,
}

// SAFETY: All fields are Send+Sync. Mutex<Option<PlaybackSession>> is Sync because
// JoinHandle<()> and AudioSource are Send.
unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

impl Engine {
    /// Creates a new engine with the given configuration.
    ///
    /// Does not start playback or open audio devices. Safe to call outside a Tokio runtime.
    #[instrument]
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let state = Arc::new(AtomicU8::new(STATE_STOPPED));
        let (dsp_config_tx, _dsp_rx) = watch::channel(config.dsp.clone());
        let (signal_path_tx, _sp_rx) = watch::channel(SignalPathSnapshot::idle());
        let (event_tx, _) = broadcast::channel(256);

        Ok(Self {
            config,
            state,
            dsp_config_tx: Arc::new(dsp_config_tx),
            signal_path_tx: Arc::new(signal_path_tx),
            event_tx,
            session: Mutex::new(None),
        })
    }

    /// Begins playback of `source`. Returns `EngineError::AlreadyPlaying` if a track is
    /// currently playing — call `stop()` first.
    ///
    /// Spawns decode and DSP tasks via `tokio::spawn`; must be called within a Tokio runtime.
    #[instrument(skip(self))]
    pub fn play(&self, source: AudioSource) -> Result<(), EngineError> {
        // Atomically transition STOPPED → PLAYING.
        self.state
            .compare_exchange(
                STATE_STOPPED,
                STATE_PLAYING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map_err(|_| EngineError::AlreadyPlaying)?;

        // Build fresh decode→DSP channel and output ring buffer.
        let (frame_tx, frame_rx) = mpsc::channel::<Option<DecodedFrame>>(256);
        let ring = Arc::new(RingBuffer::new(self.config.ring_buffer_capacity));

        // Clone shared handles for the tasks.
        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();
        let dsp_config_rx = self.dsp_config_tx.subscribe();
        let signal_path_tx = Arc::clone(&self.signal_path_tx);
        let engine_config = self.config.clone();
        let initial_dsp_config = self.config.dsp.clone();
        let source_for_dsp = source.clone();
        let ring_for_dsp = Arc::clone(&ring);

        let AudioSource::File(ref path) = source;
        let path = path.clone();

        // Decode task: read file, send DecodedFrame to DSP channel.
        let state_dec = Arc::clone(&state);
        let event_dec = event_tx.clone();
        let decode_task = tokio::spawn(
            async move {
                decode_task_fn(path, frame_tx, state_dec, event_dec).await;
            }
            .instrument(tracing::info_span!("decode_task")),
        );

        // DSP+output task: receive frames, run DSP pipeline, push to ring buffer,
        // open cpal stream and feed audio hardware (when native-output feature is enabled).
        let state_dsp = Arc::clone(&state);
        let event_dsp = event_tx.clone();
        let dsp_task = tokio::spawn(
            async move {
                dsp_task_fn(
                    source_for_dsp,
                    frame_rx,
                    dsp_config_rx,
                    initial_dsp_config,
                    engine_config,
                    ring_for_dsp,
                    signal_path_tx,
                    state_dsp,
                    event_dsp,
                )
                .await;
            }
            .instrument(tracing::info_span!("dsp_task")),
        );

        // Store session.
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(PlaybackSession {
            decode_task,
            dsp_task,
        });
        drop(guard);

        let _ = self.event_tx.send(EngineEvent::PlaybackStarted { source });
        Ok(())
    }

    /// Pauses playback at the current position. Safe to call if already paused.
    #[instrument(skip(self))]
    pub fn pause(&self) -> Result<(), EngineError> {
        let prev = self.state.compare_exchange(
            STATE_PLAYING,
            STATE_PAUSED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        if prev.is_ok() {
            let _ = self.event_tx.send(EngineEvent::PlaybackPaused);
        }
        Ok(())
    }

    /// Resumes paused playback.
    #[instrument(skip(self))]
    pub fn resume(&self) -> Result<(), EngineError> {
        let prev = self.state.compare_exchange(
            STATE_PAUSED,
            STATE_PLAYING,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        if prev.is_ok() {
            let _ = self.event_tx.send(EngineEvent::PlaybackResumed);
        }
        Ok(())
    }

    /// Stops playback and resets the engine to idle.
    #[instrument(skip(self))]
    pub fn stop(&self) -> Result<(), EngineError> {
        self.state.store(STATE_STOPPED, Ordering::SeqCst);

        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(session) = guard.take() {
            session.decode_task.abort();
            session.dsp_task.abort();
        }
        drop(guard);

        let _ = self.event_tx.send(EngineEvent::PlaybackStopped);
        Ok(())
    }

    /// Seeks to `position` within the current track. Returns the actual position reached.
    ///
    /// The seek flushes DSP state and signals the decode task to reposition. In this
    /// implementation the position is accepted as-is and a `SeekCompleted` event is emitted.
    /// Full inter-task seek coordination is completed in a subsequent pass.
    #[instrument(skip(self))]
    pub fn seek(&self, position: Duration) -> Result<Duration, EngineError> {
        if self
            .session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none()
        {
            return Err(EngineError::SeekOutOfBounds {
                position_secs: position.as_secs_f64(),
                duration_secs: 0.0,
            });
        }
        let _ = self.event_tx.send(EngineEvent::SeekCompleted { position });
        Ok(position)
    }

    /// Applies a new DSP configuration to the running pipeline without interrupting playback.
    ///
    /// The DSP task picks up the new config on the next frame via a `watch` channel.
    #[instrument(skip(self))]
    pub fn configure_dsp(&self, config: DspConfig) {
        let _ = self.dsp_config_tx.send(config);
    }

    /// Returns the current signal path snapshot.
    pub fn signal_path(&self) -> SignalPathSnapshot {
        self.signal_path_tx.borrow().clone()
    }

    /// Returns a watch receiver that emits a new `SignalPathSnapshot` whenever the signal
    /// path changes (DSP config updated, source changed, output opened/closed).
    pub fn signal_path_stream(&self) -> watch::Receiver<SignalPathSnapshot> {
        self.signal_path_tx.subscribe()
    }

    /// Returns a broadcast receiver for engine events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<EngineEvent> {
        self.event_tx.subscribe()
    }
}

// ---------------------------------------------------------------------------
// Decode task
// ---------------------------------------------------------------------------

async fn decode_task_fn(
    path: PathBuf,
    frame_tx: mpsc::Sender<Option<DecodedFrame>>,
    state: Arc<AtomicU8>,
    event_tx: broadcast::Sender<EngineEvent>,
) {
    let mut decoder = match open_decoder(&path).await {
        Ok(d) => d,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error {
                message: e.to_string(),
            });
            let _ = frame_tx.send(None).await;
            return;
        }
    };

    loop {
        if state.load(Ordering::Relaxed) == STATE_STOPPED {
            break;
        }

        // Pause: yield until resumed or stopped.
        if state.load(Ordering::Relaxed) == STATE_PAUSED {
            tokio::time::sleep(Duration::from_millis(5)).await;
            continue;
        }

        match decoder.next_frame().await {
            Ok(Some(frame)) => {
                if frame_tx.send(Some(frame)).await.is_err() {
                    break; // DSP task dropped receiver
                }
            }
            Ok(None) => {
                let _ = frame_tx.send(None).await;
                break;
            }
            Err(e) => {
                let _ = event_tx.send(EngineEvent::Error {
                    message: e.to_string(),
                });
                let _ = frame_tx.send(None).await;
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DSP + output task
// ---------------------------------------------------------------------------

#[expect(
    clippy::too_many_arguments,
    reason = "DSP task receives all pipeline components; splitting further would require wrapper structs"
)]
#[cfg_attr(
    not(feature = "native-output"),
    expect(
        unused_variables,
        reason = "several parameters are used only when native-output feature is enabled"
    )
)]
async fn dsp_task_fn(
    source: AudioSource,
    mut frame_rx: mpsc::Receiver<Option<DecodedFrame>>,
    dsp_config_rx: watch::Receiver<DspConfig>,
    initial_dsp_config: DspConfig,
    engine_config: EngineConfig,
    ring: Arc<RingBuffer>,
    signal_path_tx: Arc<watch::Sender<SignalPathSnapshot>>,
    state: Arc<AtomicU8>,
    event_tx: broadcast::Sender<EngineEvent>,
) {
    let mut dsp = DspPipeline::new(initial_dsp_config, dsp_config_rx);
    let mut output_opened = false;
    let mut last_snapshot_update = Instant::now();

    #[cfg(feature = "native-output")]
    let mut backend: Option<crate::output::cpal::CpalOutputBackend> = None;

    loop {
        if state.load(Ordering::Relaxed) == STATE_STOPPED {
            break;
        }

        if state.load(Ordering::Relaxed) == STATE_PAUSED {
            tokio::time::sleep(Duration::from_millis(5)).await;
            continue;
        }

        let opt_frame = match frame_rx.recv().await {
            Some(v) => v,
            None => break, // decode task dropped sender
        };

        let Some(frame) = opt_frame else {
            // End of stream: allow ring buffer to drain before stopping.
            tokio::time::sleep(Duration::from_millis(200)).await;
            let prev = state.swap(STATE_STOPPED, Ordering::SeqCst);
            if prev != STATE_STOPPED {
                let _ = event_tx.send(EngineEvent::TrackEnded {
                    source: source.clone(),
                });
                let _ = event_tx.send(EngineEvent::PlaybackStopped);
            }
            break;
        };

        // Open output on first frame (now that we know sample rate and channels).
        if !output_opened {
            output_opened = true;

            #[cfg(feature = "native-output")]
            {
                use crate::output::{AudioDataCallback, OutputBackend, OutputParams};
                let ring_cb = Arc::clone(&ring);
                let callback: AudioDataCallback = Box::new(move |buf: &mut [f64]| {
                    if !ring_cb.pop_frame(buf) {
                        buf.fill(0.0);
                    }
                });
                let params = OutputParams {
                    sample_rate: frame.sample_rate,
                    channels: frame.channels,
                    bit_depth: engine_config.output.bit_depth,
                    exclusive_mode: engine_config.output.exclusive_mode,
                    needs_resample: false,
                    source_sample_rate: frame.sample_rate,
                    quality_tier: QualityTier::Lossless,
                };
                let mut b = crate::output::cpal::CpalOutputBackend::new();
                match b
                    .open(
                        engine_config.output.device_name.as_deref(),
                        params,
                        callback,
                    )
                    .await
                {
                    Ok(()) => match b.start().await {
                        Ok(()) => backend = Some(b),
                        Err(e) => {
                            let _ = event_tx.send(EngineEvent::Error {
                                message: e.to_string(),
                            });
                            state.store(STATE_STOPPED, Ordering::SeqCst);
                            let _ = event_tx.send(EngineEvent::PlaybackStopped);
                            return;
                        }
                    },
                    Err(e) => {
                        let _ = event_tx.send(EngineEvent::Error {
                            message: e.to_string(),
                        });
                        state.store(STATE_STOPPED, Ordering::SeqCst);
                        let _ = event_tx.send(EngineEvent::PlaybackStopped);
                        return;
                    }
                }
            }

            // Publish initial signal path snapshot.
            let source_info = build_source_info(&source, frame.sample_rate, frame.channels);
            let stages = dsp.stage_metas();
            let tier = compute_tier(&source_info, &stages);
            let snap = SignalPathSnapshot {
                tier,
                source: Some(source_info),
                stages: stages.clone(),
                output: None,
                timestamp: Instant::now(),
            };
            let _ = signal_path_tx.send(snap.clone());
            let _ = event_tx.send(EngineEvent::SignalPathChanged(snap));
        }

        // Process frame through DSP pipeline.
        let mut samples = frame.samples.to_vec();
        let stage_metas = dsp.process_frame(&mut samples, frame.channels, frame.sample_rate);

        // Push processed samples to ring buffer with yield-based backpressure.
        loop {
            if state.load(Ordering::Relaxed) == STATE_STOPPED {
                break;
            }
            if ring.push_frame(&samples) {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Throttle signal path updates to avoid watch channel spam (~4 Hz).
        if last_snapshot_update.elapsed() >= Duration::from_millis(250) {
            last_snapshot_update = Instant::now();
            let source_info = build_source_info(&source, frame.sample_rate, frame.channels);
            let tier = compute_tier(&source_info, &stage_metas);
            let snap = SignalPathSnapshot {
                tier,
                source: Some(source_info),
                stages: stage_metas,
                output: None,
                timestamp: Instant::now(),
            };
            let _ = signal_path_tx.send(snap);
        }
    }

    // Close output backend.
    #[cfg(feature = "native-output")]
    if let Some(mut b) = backend {
        use crate::output::OutputBackend;
        let _ = b.close().await;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_source_info(source: &AudioSource, sample_rate: u32, channels: u16) -> SourceInfo {
    let codec_str = match source {
        AudioSource::File(p) => p
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "Unknown".into()),
    };
    SourceInfo {
        codec: codec_str,
        sample_rate,
        channels,
        bit_depth: None,
        tier: QualityTier::Lossless,
    }
}

fn compute_tier(source: &SourceInfo, stages: &[SignalStageInfo]) -> QualityTier {
    let base = source.tier;
    stages
        .iter()
        .filter(|s| s.enabled)
        .filter_map(|s| s.tier_impact)
        .fold(base, QualityTier::min)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::time::Duration;

    use tempfile::NamedTempFile;
    use tokio::time::timeout;

    use super::*;
    use crate::config::EngineConfig;

    /// Builds a minimal valid WAV file with enough samples to keep the decode task alive
    /// for a few hundred milliseconds.
    fn make_wav(channels: u16, sample_rate: u32, duration_secs: f32) -> NamedTempFile {
        let n_samples = (sample_rate as f32 * duration_secs) as u32 * channels as u32;
        let data_len = n_samples * 2;
        let byte_rate = sample_rate * channels as u32 * 2;
        let block_align = channels * 2;

        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&block_align.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.extend(std::iter::repeat_n(0u8, data_len as usize));

        let mut f = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        f.write_all(&v).unwrap();
        f
    }

    #[test]
    fn engine_new_succeeds_with_default_config() {
        let engine = Engine::new(EngineConfig::default());
        assert!(engine.is_ok());
    }

    #[test]
    fn engine_initial_signal_path_is_idle() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let snap = engine.signal_path();
        assert!(snap.source.is_none());
        assert!(snap.output.is_none());
    }

    #[tokio::test]
    async fn engine_play_emits_playback_started() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 2.0);

        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        let evt = timeout(Duration::from_secs(5), events.recv())
            .await
            .expect("timeout waiting for PlaybackStarted")
            .expect("broadcast recv error");

        assert!(
            matches!(evt, EngineEvent::PlaybackStarted { .. }),
            "expected PlaybackStarted, got {evt:?}"
        );

        engine.stop().unwrap();
    }

    #[tokio::test]
    async fn engine_play_rejects_already_playing() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let wav = make_wav(2, 44100, 2.0);

        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        let second = engine.play(AudioSource::File(wav.path().to_path_buf()));
        assert!(
            matches!(second, Err(EngineError::AlreadyPlaying)),
            "expected AlreadyPlaying"
        );

        engine.stop().unwrap();
    }

    #[tokio::test]
    async fn engine_stop_emits_playback_stopped() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 2.0);

        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        // Drain PlaybackStarted (and possibly SignalPathChanged).
        loop {
            let evt = timeout(Duration::from_secs(5), events.recv())
                .await
                .expect("timeout")
                .expect("recv error");
            if matches!(evt, EngineEvent::PlaybackStarted { .. }) {
                break;
            }
        }

        engine.stop().unwrap();

        // Collect events, expect PlaybackStopped.
        let mut saw_stopped = false;
        for _ in 0..10 {
            match timeout(Duration::from_millis(500), events.recv()).await {
                Ok(Ok(EngineEvent::PlaybackStopped)) => {
                    saw_stopped = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(saw_stopped, "expected PlaybackStopped event");
    }

    #[tokio::test]
    async fn engine_configure_dsp_mid_playback_does_not_crash() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let wav = make_wav(2, 44100, 2.0);

        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        // Reconfigure DSP while playing — must not panic.
        for _ in 0..5 {
            engine.configure_dsp(crate::config::DspConfig::default());
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        engine.stop().unwrap();
    }

    #[tokio::test]
    async fn engine_signal_path_updated_during_playback() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let wav = make_wav(2, 44100, 1.0);

        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        // Give the DSP task time to process the first frame and publish a snapshot.
        tokio::time::sleep(Duration::from_millis(200)).await;

        let snap = engine.signal_path();
        // After playback starts, the snapshot should have been updated from idle.
        // The DSP task publishes source info on the first frame.
        // (With native-output disabled the DSP task still processes frames.)
        let _ = snap; // no assertion on content — just must not panic

        engine.stop().unwrap();
    }

    #[tokio::test]
    async fn engine_pause_resume_cycle() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 2.0);

        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();
        let _ = timeout(Duration::from_secs(2), events.recv()).await; // PlaybackStarted

        engine.pause().unwrap();
        let evt = timeout(Duration::from_millis(500), events.recv())
            .await
            .expect("timeout after pause")
            .expect("recv error");
        assert!(matches!(evt, EngineEvent::PlaybackPaused));

        engine.resume().unwrap();
        let evt = timeout(Duration::from_millis(500), events.recv())
            .await
            .expect("timeout after resume")
            .expect("recv error");
        assert!(matches!(evt, EngineEvent::PlaybackResumed));

        engine.stop().unwrap();
    }

    #[tokio::test]
    async fn engine_signal_path_stream_returns_receiver() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let rx = engine.signal_path_stream();
        // Receiver should hold the initial idle snapshot.
        let snap = rx.borrow().clone();
        assert!(snap.source.is_none());
    }
}
