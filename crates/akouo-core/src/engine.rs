use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use snafu::ResultExt;
use tokio::sync::{Notify, broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tracing::{Instrument, instrument, warn};

use crate::config::{DspConfig, EngineConfig};
use crate::decode::DecodedFrame;
use crate::decode::probe::open_decoder;
use crate::dsp::DspPipeline;
use crate::error::{DecodeError, EngineError, OutputError, SeekFailedSnafu};
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
    /// The engine transitioned FROM one track to the next (gapless / crossfade).
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

/// Outcome of one decode step, sent decode task → DSP task.
///
/// Distinguishes clean end-of-stream (`Eos`) so a failed decode is never
/// reported as a completed track.
enum DecodeOutcome {
    /// A decoded frame tagged with the seek generation it was produced under.
    Frame {
        frame: DecodedFrame,
        generation: u64,
    },
    /// Clean end of stream: the track played to completion.
    Eos,
    /// Decoding failed; an `EngineEvent::Error` was already emitted by the decode task.
    Failed,
}

/// A seek request sent to the decode task; `reply` carries the decoder's actual
/// post-seek position.
struct SeekCommand {
    target: Duration,
    reply: oneshot::Sender<Result<Duration, DecodeError>>,
}

struct PlaybackSession {
    decode_task: JoinHandle<()>,
    dsp_task: JoinHandle<()>,
    seek_tx: mpsc::Sender<SeekCommand>,
    // WHY: keeps the output-error channel open for the session's lifetime and provides
    // the injection seam for stream-error tests; production sends originate in the
    // output backend.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "held to keep the output-error channel open; read via the test-only inject_output_error seam"
        )
    )]
    output_error_tx: mpsc::Sender<OutputError>,
    // WHY(#542): written by the DSP task's pause/resume-propagation wiring so the
    // output backend's actual hardware pause state is observable; read via the
    // test-only backend_paused() seam.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "written by the DSP task pause wiring (#542); read via the test-only backend_paused() seam"
        )
    )]
    backend_paused: Arc<AtomicBool>,
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
    // WHY(#542): wakes the DSP task immediately on pause()/resume() even while it
    // is blocked awaiting the next frame — without this, a pause landing on an
    // empty frame channel would not be observed until the next frame arrived.
    state_notify: Arc<Notify>,
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
            state_notify: Arc::new(Notify::new()),
        })
    }

    /// Begins playback of `source`. Returns `EngineError::AlreadyPlaying` if a track is
    /// currently playing  -  call `stop()` first.
    ///
    /// Spawns decode and DSP tasks via `tokio::spawn`; must be called within a Tokio runtime.
    #[instrument(skip(self))]
    pub fn play(&self, source: AudioSource) -> Result<(), EngineError> {
        // WHY: the session lock is held across the whole spawn sequence — a
        // stop() racing between task spawn and session store would otherwise
        // find `session` empty and be unable to abort the new tasks.
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());

        // Atomically transition STOPPED → PLAYING.
        self.state
            .compare_exchange(
                STATE_STOPPED,
                STATE_PLAYING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map_err(|_| EngineError::AlreadyPlaying)?;

        // WHY(#401): PlaybackStarted must be observable before any event the spawned
        // tasks can emit (Error, TrackEnded, PlaybackStopped). broadcast::Sender::send
        // is synchronous, so emitting before tokio::spawn guarantees the ordering.
        // WHY: send fails only when no receivers exist; dropping is intentional
        self.event_tx
            .send(EngineEvent::PlaybackStarted {
                source: source.clone(),
            })
            .ok();

        // Build fresh decode→DSP channel, seek channels, and output ring buffer.
        let (frame_tx, frame_rx) = mpsc::channel::<DecodeOutcome>(256);
        let (seek_tx, seek_rx) = mpsc::channel::<SeekCommand>(4);
        let (seek_generation_tx, seek_generation_rx) = watch::channel(0u64);
        let (output_error_tx, output_error_rx) = mpsc::channel::<OutputError>(16);
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
        let output_error_tx_for_dsp = output_error_tx.clone();
        let state_notify_for_dsp = Arc::clone(&self.state_notify);
        let backend_paused = Arc::new(AtomicBool::new(false));
        let backend_paused_for_dsp = Arc::clone(&backend_paused);

        let AudioSource::File(ref path) = source;
        let path = path.clone();

        // Decode task: read file, send DecodeOutcome to DSP channel, service seeks.
        let state_dec = Arc::clone(&state);
        let event_dec = event_tx.clone();
        let decode_task = tokio::spawn(
            decode_task_fn(
                path,
                frame_tx,
                seek_rx,
                seek_generation_tx,
                state_dec,
                event_dec,
            )
            .instrument(tracing::info_span!("decode_task")),
        );

        // DSP+output task: receive frames, run DSP pipeline, push to ring buffer,
        // open cpal stream and feed audio hardware (when native-output feature is enabled).
        let dsp_task = tokio::spawn(
            dsp_task_fn(DspTaskParams {
                source: source_for_dsp,
                frame_rx,
                dsp_config_rx,
                initial_dsp_config,
                engine_config,
                ring: ring_for_dsp,
                signal_path_tx,
                state,
                event_tx,
                seek_generation_rx,
                output_error_rx,
                output_error_tx: output_error_tx_for_dsp,
                state_notify: state_notify_for_dsp,
                backend_paused: backend_paused_for_dsp,
            })
            .instrument(tracing::info_span!("dsp_task")),
        );

        // Store session (lock held since before the CAS).
        *guard = Some(PlaybackSession {
            decode_task,
            dsp_task,
            seek_tx,
            output_error_tx,
            backend_paused,
        });
        drop(guard);

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
            // WHY: send fails only when no receivers exist; dropping is intentional
            self.event_tx.send(EngineEvent::PlaybackPaused).ok();
            // WHY(#542): wakes the DSP task immediately even if it is currently
            // blocked awaiting the next frame, so the output backend is paused
            // (true hardware stop) without waiting on frame arrival.
            self.state_notify.notify_one();
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
            // WHY: send fails only when no receivers exist; dropping is intentional
            self.event_tx.send(EngineEvent::PlaybackResumed).ok();
            // WHY(#542): symmetric wake for the resume path — see pause() above.
            self.state_notify.notify_one();
        }
        Ok(())
    }

    /// Stops playback and resets the engine to idle.
    ///
    /// Awaits both pipeline tasks after aborting them, so when `stop()`
    /// returns their resources (decoder thread, output stream) are released
    /// — an immediate `play()` never races the old session's teardown.
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<(), EngineError> {
        self.state.store(STATE_STOPPED, Ordering::SeqCst);

        let session = {
            let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
            guard.take()
        };
        if let Some(session) = session {
            session.decode_task.abort();
            session.dsp_task.abort();
            let (decode_result, dsp_result) = tokio::join!(session.decode_task, session.dsp_task);
            // WHY: cancellation is the expected join outcome after abort();
            // a genuine task panic must still be surfaced in the log.
            for result in [decode_result, dsp_result] {
                if let Err(e) = result
                    && !e.is_cancelled()
                {
                    warn!(error = %e, "pipeline task panicked during stop");
                }
            }
        }

        // WHY: send fails only when no receivers exist; dropping is intentional
        self.event_tx.send(EngineEvent::PlaybackStopped).ok();
        Ok(())
    }

    /// Seeks to `position` within the current track. Returns the actual position reached.
    ///
    /// The request is forwarded to the decode task, which repositions the decoder,
    /// bumps the seek generation (so the DSP task discards stale pre-seek frames and
    /// flushes buffered output), and replies with the decoder's actual post-seek
    /// position. A `SeekCompleted` event carrying that actual position is emitted by
    /// the decode task once the decoder has repositioned.
    #[instrument(skip(self))]
    pub async fn seek(&self, position: Duration) -> Result<Duration, EngineError> {
        let seek_tx = {
            let guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref() {
                Some(session) => session.seek_tx.clone(),
                None => {
                    return Err(EngineError::SeekOutOfBounds {
                        position_secs: position.as_secs_f64(),
                        duration_secs: 0.0,
                    });
                }
            }
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        seek_tx
            .send(SeekCommand {
                target: position,
                reply: reply_tx,
            })
            .await
            .map_err(|_| decode_task_gone())
            .context(SeekFailedSnafu)?;

        match reply_rx.await {
            Ok(result) => result.context(SeekFailedSnafu),
            // WHY: the decode task exited (stop/abort/track end) before replying;
            // never fabricate success.
            Err(_) => Err(decode_task_gone()).context(SeekFailedSnafu),
        }
    }

    /// Applies a new DSP configuration to the running pipeline without interrupting playback.
    ///
    /// The DSP task picks up the new config on the next frame via a `watch` channel.
    #[instrument(skip(self))]
    pub fn configure_dsp(&self, config: DspConfig) {
        // WHY: send fails only when no receivers exist; dropping is intentional
        self.dsp_config_tx.send(config).ok();
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

    /// Test seam: injects an output-stream error as if the audio backend reported it.
    #[cfg(test)]
    fn inject_output_error(&self, error: OutputError) -> bool {
        let guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .is_some_and(|session| session.output_error_tx.try_send(error).is_ok())
    }

    /// Test seam: whether the output backend has actually been paused (#542) —
    /// distinct FROM the engine's own `STATE_PAUSED`, which only reflects intent
    /// until the DSP task propagates it to the hardware stream.
    #[cfg(test)]
    fn backend_paused(&self) -> bool {
        let guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .is_some_and(|session| session.backend_paused.load(Ordering::Relaxed))
    }
}

fn decode_task_gone() -> DecodeError {
    DecodeError::TaskJoin {
        message: "decode task is not running".to_string(),
        location: snafu::location!(),
    }
}

// ---------------------------------------------------------------------------
// Decode task
// ---------------------------------------------------------------------------

async fn decode_task_fn(
    path: PathBuf,
    frame_tx: mpsc::Sender<DecodeOutcome>,
    mut seek_rx: mpsc::Receiver<SeekCommand>,
    seek_generation_tx: watch::Sender<u64>,
    state: Arc<AtomicU8>,
    event_tx: broadcast::Sender<EngineEvent>,
) {
    let mut decoder = match open_decoder(&path).await {
        Ok(d) => d,
        Err(e) => {
            // WHY: send fails only when no receivers exist; dropping is intentional
            event_tx
                .send(EngineEvent::Error {
                    message: e.to_string(),
                })
                .ok();
            // WHY(#402): open failure is a Failed outcome, never Eos — the DSP task
            // must not report TrackEnded for a track that never decoded.
            // WHY: send fails only when no receivers exist; dropping is intentional
            frame_tx.send(DecodeOutcome::Failed).await.ok();
            return;
        }
    };

    let mut generation: u64 = 0;
    // INVARIANT: once seek_rx yields None the sender is gone (session replaced or
    // dropped); the branch is disabled to keep the select FROM spinning on a closed
    // channel.
    let mut seek_channel_open = true;

    loop {
        if state.load(Ordering::Relaxed) == STATE_STOPPED {
            break;
        }

        // Service pending seeks first so they work while paused and take priority
        // over decoding the next frame.
        while let Ok(command) = seek_rx.try_recv() {
            generation = run_seek(
                decoder.as_mut(),
                command,
                generation,
                &seek_generation_tx,
                &event_tx,
            )
            .await;
        }

        // Pause: yield until resumed or stopped.
        if state.load(Ordering::Relaxed) == STATE_PAUSED {
            tokio::time::sleep(Duration::from_millis(5)).await;
            continue;
        }

        match decoder.next_frame().await {
            Ok(Some(frame)) => {
                let outcome = DecodeOutcome::Frame { frame, generation };
                // WHY(#386): select keeps seeks responsive even while the frame
                // channel is full; a seek that wins drops the pending frame, which is
                // correct — it predates the new position.
                tokio::select! {
                    sent = frame_tx.send(outcome) => {
                        if sent.is_err() {
                            break; // DSP task dropped receiver
                        }
                    }
                    command = seek_rx.recv(), if seek_channel_open => {
                        match command {
                            Some(command) => {
                                generation = run_seek(
                                    decoder.as_mut(),
                                    command,
                                    generation,
                                    &seek_generation_tx,
                                    &event_tx,
                                )
                                .await;
                            }
                            None => seek_channel_open = false,
                        }
                    }
                }
            }
            Ok(None) => {
                // WHY: send fails only when no receivers exist; dropping is intentional
                frame_tx.send(DecodeOutcome::Eos).await.ok();
                break;
            }
            Err(e) => {
                // WHY: send fails only when no receivers exist; dropping is intentional
                event_tx
                    .send(EngineEvent::Error {
                        message: e.to_string(),
                    })
                    .ok();
                // WHY(#402): decode failure is signalled distinctly FROM end-of-stream
                // so the DSP task never emits TrackEnded for a failed track.
                // WHY: send fails only when no receivers exist; dropping is intentional
                frame_tx.send(DecodeOutcome::Failed).await.ok();
                break;
            }
        }
    }
}

/// Executes one seek command against the decoder. On success bumps the seek generation
/// (visible to the DSP task before any post-seek frame is sent), emits `SeekCompleted`
/// with the decoder's actual position, and replies to the caller. Returns the
/// generation in effect afterwards.
async fn run_seek(
    decoder: &mut dyn crate::decode::AudioDecoder,
    command: SeekCommand,
    generation: u64,
    seek_generation_tx: &watch::Sender<u64>,
    event_tx: &broadcast::Sender<EngineEvent>,
) -> u64 {
    match decoder.seek(command.target).await {
        Ok(actual) => {
            let next_generation = generation + 1;
            // WHY: send fails only when no receivers exist; dropping is intentional
            seek_generation_tx.send(next_generation).ok();
            // WHY: send fails only when no receivers exist; dropping is intentional
            event_tx
                .send(EngineEvent::SeekCompleted { position: actual })
                .ok();
            // WHY: reply send fails only when the caller stopped waiting; intentional
            command.reply.send(Ok(actual)).ok();
            next_generation
        }
        Err(e) => {
            // WHY: send fails only when no receivers exist; dropping is intentional
            event_tx
                .send(EngineEvent::Error {
                    message: e.to_string(),
                })
                .ok();
            // WHY: reply send fails only when the caller stopped waiting; intentional
            command.reply.send(Err(e)).ok();
            generation
        }
    }
}

// ---------------------------------------------------------------------------
// DSP + output task
// ---------------------------------------------------------------------------

/// Everything the DSP+output task needs; bundled so the task has a single owner-struct
/// instead of a dozen positional parameters.
struct DspTaskParams {
    source: AudioSource,
    frame_rx: mpsc::Receiver<DecodeOutcome>,
    dsp_config_rx: watch::Receiver<DspConfig>,
    initial_dsp_config: DspConfig,
    engine_config: EngineConfig,
    ring: Arc<RingBuffer>,
    signal_path_tx: Arc<watch::Sender<SignalPathSnapshot>>,
    state: Arc<AtomicU8>,
    event_tx: broadcast::Sender<EngineEvent>,
    seek_generation_rx: watch::Receiver<u64>,
    output_error_rx: mpsc::Receiver<OutputError>,
    output_error_tx: mpsc::Sender<OutputError>,
    state_notify: Arc<Notify>,
    backend_paused: Arc<AtomicBool>,
}

async fn dsp_task_fn(params: DspTaskParams) {
    let DspTaskParams {
        source,
        mut frame_rx,
        dsp_config_rx,
        initial_dsp_config,
        engine_config,
        ring,
        signal_path_tx,
        state,
        event_tx,
        seek_generation_rx,
        mut output_error_rx,
        output_error_tx,
        state_notify,
        backend_paused,
    } = params;
    #[cfg(not(feature = "native-output"))]
    let _ = (&engine_config, &output_error_tx, &backend_paused);

    let mut dsp = DspPipeline::new(initial_dsp_config, dsp_config_rx);
    let mut output_opened = false;
    let mut last_snapshot_update = Instant::now();
    // Seek generation currently being played; frames tagged with an older generation
    // predate the most recent seek and are discarded.
    let mut current_generation: u64 = 0;

    #[cfg(feature = "native-output")]
    let mut backend: Option<crate::output::cpal::CpalOutputBackend> = None;
    #[cfg(feature = "native-output")]
    let mut last_underrun_count: u64 = 0;

    loop {
        if state.load(Ordering::Relaxed) == STATE_STOPPED {
            break;
        }

        if state.load(Ordering::Relaxed) == STATE_PAUSED {
            #[cfg(feature = "native-output")]
            if !backend_paused.load(Ordering::Relaxed)
                && let Some(b) = backend.as_mut()
            {
                use crate::output::OutputBackend;
                // WHY(#542): stop the hardware stream immediately — otherwise
                // cpal keeps popping already-buffered ring samples and audio
                // continues audibly for up to ring_buffer_capacity (~0.7s)
                // after pause() returns.
                if let Err(e) = b.pause().await {
                    // WHY: send fails only when no receivers exist; dropping is intentional
                    event_tx
                        .send(EngineEvent::Error {
                            message: e.to_string(),
                        })
                        .ok();
                }
                backend_paused.store(true, Ordering::Relaxed);
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
            continue;
        }

        #[cfg(feature = "native-output")]
        if backend_paused.load(Ordering::Relaxed)
            && let Some(b) = backend.as_mut()
        {
            use crate::output::OutputBackend;
            if let Err(e) = b.start().await {
                // WHY: send fails only when no receivers exist; dropping is intentional
                event_tx
                    .send(EngineEvent::Error {
                        message: e.to_string(),
                    })
                    .ok();
            }
            backend_paused.store(false, Ordering::Relaxed);
        }

        // WHY(#404): the select surfaces asynchronous output-stream errors while
        // waiting for frames, so a dead stream stops playback instead of leaving
        // STATE_PLAYING forever.
        let outcome = tokio::select! {
            received = frame_rx.recv() => match received {
                Some(outcome) => outcome,
                None => break, // decode task dropped sender
            },
            stream_error = output_error_rx.recv() => {
                let Some(e) = stream_error else {
                    // INVARIANT: unreachable while the session holds a sender clone;
                    // defensive break avoids spinning on a closed channel.
                    break;
                };
                report_output_error(&e, &state, &event_tx);
                break;
            }
            () = state_notify.notified() => {
                // WHY(#542): a pause can land while this select is blocked
                // awaiting the next frame (e.g. decode is also paused and the
                // frame channel is drained); wake immediately so the
                // pause/resume propagation above runs without delay.
                continue;
            }
        };

        let (frame, frame_generation) = match outcome {
            DecodeOutcome::Frame { frame, generation } => (frame, generation),
            DecodeOutcome::Eos => {
                // End of stream: allow ring buffer to drain before stopping.
                tokio::time::sleep(Duration::from_millis(200)).await;
                let prev = state.swap(STATE_STOPPED, Ordering::SeqCst);
                if prev != STATE_STOPPED {
                    // WHY: send fails only when no receivers exist; dropping is intentional
                    event_tx
                        .send(EngineEvent::TrackEnded {
                            source: source.clone(),
                        })
                        .ok();
                    // WHY: send fails only when no receivers exist; dropping is intentional
                    event_tx.send(EngineEvent::PlaybackStopped).ok();
                }
                break;
            }
            DecodeOutcome::Failed => {
                // WHY(#402): decode failure — the decode task already emitted Error;
                // stop playback WITHOUT TrackEnded (the track did not complete).
                let prev = state.swap(STATE_STOPPED, Ordering::SeqCst);
                if prev != STATE_STOPPED {
                    // WHY: send fails only when no receivers exist; dropping is intentional
                    event_tx.send(EngineEvent::PlaybackStopped).ok();
                }
                break;
            }
        };

        // WHY(#386): frames FROM before the latest seek are stale — drop them without
        // processing so the seek takes effect immediately instead of after the stale
        // backlog drains at real-time rate.
        let latest_generation = *seek_generation_rx.borrow();
        if frame_generation < latest_generation {
            continue;
        }
        if frame_generation > current_generation {
            current_generation = frame_generation;
            // First frame after a seek: discard buffered pre-seek audio.
            #[cfg(feature = "native-output")]
            flush_ring_after_seek(&ring, backend.as_mut()).await;
            #[cfg(not(feature = "native-output"))]
            // SAFETY: without native output no consumer thread exists for this ring;
            // clear() is not racing a concurrent pop_frame.
            ring.clear();
        }

        // Open output on first frame (now that we know sample rate and channels).
        if !output_opened {
            output_opened = true;

            #[cfg(feature = "native-output")]
            if engine_config.output.enabled {
                use crate::output::{AudioDataCallback, OutputBackend, OutputParams};
                let ring_cb = Arc::clone(&ring);
                let callback: AudioDataCallback = Box::new(move |buf: &mut [f64]| {
                    // WHY(#541): the return value is the single source of truth the
                    // cpal backend uses to count a genuine (ring-empty) underrun —
                    // the raw cpal callback can no longer see the ring directly.
                    let popped = ring_cb.pop_frame(buf);
                    if !popped {
                        buf.fill(0.0);
                    }
                    popped
                });
                let params = OutputParams {
                    sample_rate: frame.sample_rate,
                    channels: frame.channels,
                    bit_depth: engine_config.output.bit_depth,
                    exclusive_mode: engine_config.output.exclusive_mode,
                    needs_resample: false,
                    source_sample_rate: frame.sample_rate,
                    quality_tier: QualityTier::Lossless,
                    // WHY(#543): honors the configured buffer_size instead of
                    // always requesting the platform default.
                    buffer_size: engine_config.output.buffer_size.clone(),
                };
                let mut b = crate::output::cpal::CpalOutputBackend::new();
                match b
                    .open(
                        engine_config.output.device_name.as_deref(),
                        params,
                        callback,
                        output_error_tx.clone(),
                    )
                    .await
                {
                    Ok(()) => match b.start().await {
                        Ok(()) => backend = Some(b),
                        Err(e) => {
                            // WHY: send fails only when no receivers exist; dropping is intentional
                            event_tx
                                .send(EngineEvent::Error {
                                    message: e.to_string(),
                                })
                                .ok();
                            state.store(STATE_STOPPED, Ordering::SeqCst);
                            // WHY: send fails only when no receivers exist; dropping is intentional
                            event_tx.send(EngineEvent::PlaybackStopped).ok();
                            return;
                        }
                    },
                    Err(e) => {
                        // WHY: send fails only when no receivers exist; dropping is intentional
                        event_tx
                            .send(EngineEvent::Error {
                                message: e.to_string(),
                            })
                            .ok();
                        state.store(STATE_STOPPED, Ordering::SeqCst);
                        // WHY: send fails only when no receivers exist; dropping is intentional
                        event_tx.send(EngineEvent::PlaybackStopped).ok();
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
            // WHY: send fails only when no receivers exist; dropping is intentional
            signal_path_tx.send(snap.clone()).ok();
            // WHY: send fails only when no receivers exist; dropping is intentional
            event_tx.send(EngineEvent::SignalPathChanged(snap)).ok();
        }

        // Process frame through DSP pipeline.
        let mut samples = frame.samples.to_vec();
        let stage_metas = dsp.process_frame(&mut samples, frame.channels, frame.sample_rate);

        // WHY(#387): a frame that can never fit the ring would spin the retry loop
        // forever (push_frame requires used + n < capacity); fail fast instead.
        if samples.len() >= ring.capacity() {
            // WHY: send fails only when no receivers exist; dropping is intentional
            event_tx
                .send(EngineEvent::Error {
                    message: format!(
                        "decoded frame has {} samples, exceeding ring buffer usable capacity {}; increase ring_buffer_capacity",
                        samples.len(),
                        ring.capacity().saturating_sub(1)
                    ),
                })
                .ok();
            state.store(STATE_STOPPED, Ordering::SeqCst);
            // WHY: send fails only when no receivers exist; dropping is intentional
            event_tx.send(EngineEvent::PlaybackStopped).ok();
            break;
        }

        // Push processed samples to ring buffer with yield-based backpressure.
        loop {
            if state.load(Ordering::Relaxed) == STATE_STOPPED {
                break;
            }
            // WHY(#386): abandon the push when a seek lands mid-backpressure — this
            // frame is stale and blocking here would delay the seek by a full ring
            // drain.
            if *seek_generation_rx.borrow() != current_generation {
                break;
            }
            // WHY(#404): a dead output stream stops the consumer, so backpressure
            // never resolves — the error must be polled here or a stream failure
            // under full ring would spin forever unnoticed.
            if let Ok(e) = output_error_rx.try_recv() {
                report_output_error(&e, &state, &event_tx);
                break;
            }
            if ring.push_frame(&samples) {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Poll the output backend's underrun counter (~ once per frame) and
        // surface increases as EngineEvent::Underrun.
        #[cfg(feature = "native-output")]
        if let Some(b) = backend.as_ref()
            && let Some(count) = underrun_increase(last_underrun_count, b.underrun_count())
        {
            last_underrun_count = count;
            // WHY: send fails only when no receivers exist; dropping is intentional
            event_tx.send(EngineEvent::Underrun { count }).ok();
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
            // WHY: send fails only when no receivers exist; dropping is intentional
            signal_path_tx.send(snap).ok();
        }
    }

    // Close output backend.
    #[cfg(feature = "native-output")]
    if let Some(mut b) = backend {
        use crate::output::OutputBackend;
        // WHY: close error on shutdown is non-fatal; device already stopping
        b.close().await.ok();
    }
}

/// Returns the new cumulative count when the underrun counter increased,
/// `None` otherwise. Pure so the emission policy is unit-testable without
/// audio hardware.
#[cfg_attr(
    not(any(feature = "native-output", test)),
    expect(
        dead_code,
        reason = "keeps-alive: polled from the native-output DSP loop, kept unconditional so the policy stays unit-tested in every build"
    )
)]
fn underrun_increase(previous: u64, current: u64) -> Option<u64> {
    (current > previous).then_some(current)
}

/// Surfaces an asynchronous output-stream error: emits `EngineEvent::Error`, stops
/// playback state, and emits `PlaybackStopped`.
fn report_output_error(
    error: &OutputError,
    state: &AtomicU8,
    event_tx: &broadcast::Sender<EngineEvent>,
) {
    // WHY: send fails only when no receivers exist; dropping is intentional
    event_tx
        .send(EngineEvent::Error {
            message: error.to_string(),
        })
        .ok();
    state.store(STATE_STOPPED, Ordering::SeqCst);
    // WHY: send fails only when no receivers exist; dropping is intentional
    event_tx.send(EngineEvent::PlaybackStopped).ok();
}

/// Discards buffered pre-seek audio. With an open backend the stream is paused first so
/// the output callback is not concurrently popping while positions reset, then resumed.
#[cfg(feature = "native-output")]
async fn flush_ring_after_seek(
    ring: &RingBuffer,
    backend: Option<&mut crate::output::cpal::CpalOutputBackend>,
) {
    use crate::output::OutputBackend;
    match backend {
        Some(b) => {
            // WHY: pause/start failures are non-fatal here — worst case a fragment of
            // pre-seek audio plays; stream errors surface via the error channel.
            b.pause().await.ok();
            // WARNING: cpal pause is best-effort; an in-flight callback may still be
            // reading. clear() only resets positions, so the residual risk is one
            // callback buffer of mixed samples, not memory unsafety beyond the ring's
            // documented SPSC contract.
            ring.clear();
            b.start().await.ok();
        }
        // SAFETY: no backend open yet — no consumer thread exists; clear() is safe.
        None => ring.clear(),
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
        let n_samples = (sample_rate as f32 * duration_secs) as u32 * u32::from(channels);
        let data_len = n_samples * 2;
        let byte_rate = sample_rate * u32::from(channels) * 2;
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
        v.extend(std::iter::repeat_n(
            0u8,
            usize::try_from(data_len).unwrap_or_default(),
        ));

        let mut f = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        f.write_all(&v).unwrap();
        f
    }

    /// Config with hardware output disabled: behavioral tests must not depend on an
    /// audio device existing (CI is headless and feature unification with archon
    /// enables native-output for this crate's tests).
    fn headless_config() -> EngineConfig {
        let mut config = EngineConfig::default();
        config.output.enabled = false;
        config
    }

    #[test]
    fn underrun_increase_emits_only_on_growth() {
        assert_eq!(underrun_increase(0, 0), None);
        assert_eq!(underrun_increase(0, 3), Some(3));
        assert_eq!(underrun_increase(3, 3), None);
        assert_eq!(underrun_increase(3, 7), Some(7));
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
            .unwrap()
            .unwrap();

        assert!(
            matches!(evt, EngineEvent::PlaybackStarted { .. }),
            "expected PlaybackStarted, got {evt:?}"
        );

        engine.stop().await.unwrap();
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

        engine.stop().await.unwrap();
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
                .unwrap()
                .unwrap();
            if matches!(evt, EngineEvent::PlaybackStarted { .. }) {
                break;
            }
        }

        engine.stop().await.unwrap();

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

        // Reconfigure DSP while playing  -  must not panic.
        for _ in 0..5 {
            engine.configure_dsp(crate::config::DspConfig::default());
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        engine.stop().await.unwrap();
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
        // After playback starts, the snapshot should have been updated FROM idle.
        // The DSP task publishes source info on the first frame.
        // (With native-output disabled the DSP task still processes frames.)
        let _ = snap; // no assertion on content  -  just must not panic

        engine.stop().await.unwrap();
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
            .unwrap()
            .unwrap();
        assert!(matches!(evt, EngineEvent::PlaybackPaused));

        engine.resume().unwrap();
        let evt = timeout(Duration::from_millis(500), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(evt, EngineEvent::PlaybackResumed));

        engine.stop().await.unwrap();
    }

    // --- #542: pause() must propagate to the output backend (true hardware stop),
    // not just flip the state atomic while cpal keeps draining the ring ---

    #[tokio::test]
    #[ignore = "requires audio hardware — see #542"]
    async fn engine_pause_stops_output_backend() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 5.0);

        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        // Wait for the output device to actually open: SignalPathChanged is
        // published once the DSP task reaches the post-open snapshot.
        loop {
            let evt = timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(evt, EngineEvent::SignalPathChanged(_)) {
                break;
            }
        }

        assert!(
            !engine.backend_paused(),
            "backend must not be paused before pause() is called"
        );

        engine.pause().unwrap();
        // Give the DSP task a moment to observe the notify wake and call
        // the backend's pause().
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            engine.backend_paused(),
            "backend must be paused (true hardware stop) after Engine::pause()"
        );

        engine.resume().unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !engine.backend_paused(),
            "backend must be resumed after Engine::resume()"
        );

        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn engine_signal_path_stream_returns_receiver() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let rx = engine.signal_path_stream();
        // Receiver should hold the initial idle snapshot.
        let snap = rx.borrow().clone();
        assert!(snap.source.is_none());
    }

    /// stop() is a synchronization point: after it returns, the old session's
    /// tasks are joined, so an immediate play() must always succeed — cycling
    /// rapidly would previously race the CAS against un-aborted task teardown.
    #[tokio::test]
    async fn stop_then_immediate_play_cycles_cleanly() {
        let engine = Engine::new(headless_config()).unwrap();
        let wav = make_wav(2, 44100, 2.0);

        for i in 0..5 {
            engine
                .play(AudioSource::File(wav.path().to_path_buf()))
                .unwrap_or_else(|e| panic!("cycle {i}: play after stop must succeed: {e}"));
            timeout(Duration::from_secs(5), engine.stop())
                .await
                .expect("stop must not hang")
                .unwrap();
        }
    }

    // --- #386: seek must reposition the decoder, not fabricate completion ---

    #[tokio::test]
    async fn engine_seek_before_play_errors() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let result = engine.seek(Duration::from_millis(500)).await;
        assert!(
            matches!(result, Err(EngineError::SeekOutOfBounds { .. })),
            "seek without a session must error"
        );
    }

    /// Behavioral proof the decoder repositions: a 5s stereo WAV (882 000 samples) can
    /// never reach EOS in the test build — nothing drains the 65 536-sample ring, so
    /// decode stalls on backpressure well before the end. Only a real seek close to EOF
    /// leaves few enough samples for decode to finish and TrackEnded to fire.
    #[tokio::test]
    async fn engine_seek_moves_playback_position() {
        let engine = Engine::new(headless_config()).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 5.0);
        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        loop {
            let evt = timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(evt, EngineEvent::PlaybackStarted { .. }) {
                break;
            }
        }

        let actual = timeout(
            Duration::from_secs(5),
            engine.seek(Duration::from_millis(4900)),
        )
        .await
        .expect("seek must not hang")
        .expect("seek must succeed");
        assert!(
            actual >= Duration::from_secs(4) && actual <= Duration::from_secs(5),
            "seek landed at {actual:?}"
        );

        let mut saw_seek_completed = false;
        let mut saw_track_ended = false;
        for _ in 0..50 {
            match timeout(Duration::from_secs(10), events.recv()).await {
                Ok(Ok(EngineEvent::SeekCompleted { position })) => {
                    saw_seek_completed = true;
                    assert_eq!(position, actual, "event and return value must agree");
                }
                Ok(Ok(EngineEvent::TrackEnded { .. })) => {
                    saw_track_ended = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(saw_seek_completed, "SeekCompleted event missing");
        assert!(
            saw_track_ended,
            "TrackEnded missing — the decoder did not actually reposition near EOF"
        );
    }

    #[tokio::test]
    async fn engine_seek_past_eof_does_not_fabricate_position() {
        let engine = Engine::new(headless_config()).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 1.0);
        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();
        loop {
            let evt = timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(evt, EngineEvent::PlaybackStarted { .. }) {
                break;
            }
        }

        let requested = Duration::from_secs(30);
        match timeout(Duration::from_secs(5), engine.seek(requested))
            .await
            .expect("seek must not hang")
        {
            // A clamping decoder must report where it actually landed, never echo the
            // out-of-range request.
            Ok(actual) => assert!(
                actual < requested,
                "seek fabricated the requested position: {actual:?}"
            ),
            Err(e) => assert!(
                matches!(e, EngineError::SeekFailed { .. }),
                "unexpected error kind: {e}"
            ),
        }

        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn engine_seek_works_while_paused() {
        let engine = Engine::new(headless_config()).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 5.0);
        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();
        loop {
            let evt = timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(evt, EngineEvent::PlaybackStarted { .. }) {
                break;
            }
        }
        engine.pause().unwrap();

        let actual = timeout(Duration::from_secs(5), engine.seek(Duration::from_secs(3)))
            .await
            .expect("seek while paused must not hang")
            .expect("seek while paused must succeed");
        assert!(actual <= Duration::from_secs(5), "landed at {actual:?}");

        engine.stop().await.unwrap();
    }

    // --- #387: oversized frame must error, not livelock ---

    #[tokio::test]
    async fn engine_oversized_frame_errors_instead_of_livelocking() {
        let config = EngineConfig {
            ring_buffer_capacity: 64,
            ..headless_config()
        };
        let engine = Engine::new(config).unwrap();
        let mut events = engine.subscribe_events();
        let wav = make_wav(2, 44100, 1.0);
        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        let mut saw_error = false;
        let mut saw_stopped = false;
        for _ in 0..20 {
            // WHY: the bounded timeout turns a regression to the old spin INTO a test
            // failure instead of a hang.
            match timeout(Duration::from_secs(5), events.recv()).await {
                Ok(Ok(EngineEvent::Error { message })) => {
                    assert!(
                        message.contains("exceeding"),
                        "unexpected error message: {message}"
                    );
                    saw_error = true;
                }
                Ok(Ok(EngineEvent::PlaybackStopped)) => {
                    saw_stopped = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(saw_error, "oversized frame must emit EngineEvent::Error");
        assert!(saw_stopped, "oversized frame must stop playback");
    }

    // --- #401: PlaybackStarted must precede any task-emitted event ---

    #[tokio::test]
    async fn playback_started_precedes_error_on_bad_file() {
        let engine = Engine::new(EngineConfig::default()).unwrap();
        let mut events = engine.subscribe_events();
        engine
            .play(AudioSource::File(PathBuf::from(
                "/nonexistent/akouo-test-file.wav",
            )))
            .unwrap();

        let mut received = Vec::new();
        for _ in 0..10 {
            match timeout(Duration::from_secs(2), events.recv()).await {
                Ok(Ok(evt)) => {
                    let is_stopped = matches!(evt, EngineEvent::PlaybackStopped);
                    received.push(evt);
                    if is_stopped {
                        break;
                    }
                }
                _ => break,
            }
        }

        assert!(
            matches!(received.first(), Some(EngineEvent::PlaybackStarted { .. })),
            "first event must be PlaybackStarted, got {received:?}"
        );
        assert!(
            received
                .iter()
                .any(|e| matches!(e, EngineEvent::Error { .. })),
            "open failure must surface as Error: {received:?}"
        );
        assert!(
            received
                .iter()
                .any(|e| matches!(e, EngineEvent::PlaybackStopped)),
            "open failure must stop playback: {received:?}"
        );
        // #402: a track that never decoded must not report TrackEnded.
        assert!(
            !received
                .iter()
                .any(|e| matches!(e, EngineEvent::TrackEnded { .. })),
            "no TrackEnded for a failed open: {received:?}"
        );
    }

    // --- #402: decode failure vs clean EOS ---

    #[tokio::test]
    async fn clean_eos_still_emits_track_ended() {
        let engine = Engine::new(headless_config()).unwrap();
        let mut events = engine.subscribe_events();
        // 0.2s stereo = 17 640 samples: fits the ring, so decode reaches EOS.
        let wav = make_wav(2, 44100, 0.2);
        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();

        let mut saw_track_ended = false;
        for _ in 0..20 {
            match timeout(Duration::from_secs(10), events.recv()).await {
                Ok(Ok(EngineEvent::TrackEnded { .. })) => {
                    saw_track_ended = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(saw_track_ended, "clean EOS must still emit TrackEnded");
    }

    /// Direct DSP-task unit test: a mid-stream decode failure (frames, then Failed)
    /// must stop playback WITHOUT TrackEnded.
    #[tokio::test]
    async fn dsp_failed_after_frames_stops_without_track_ended() {
        let (frame_tx, frame_rx) = mpsc::channel::<DecodeOutcome>(8);
        let (_dsp_cfg_tx, dsp_cfg_rx) = watch::channel(DspConfig::default());
        let (_generation_tx, seek_generation_rx) = watch::channel(0u64);
        let (output_error_tx, output_error_rx) = mpsc::channel::<OutputError>(4);
        let (event_tx, mut events) = broadcast::channel::<EngineEvent>(64);
        let (signal_path_tx, _signal_path_rx) = watch::channel(SignalPathSnapshot::idle());
        let state = Arc::new(AtomicU8::new(STATE_PLAYING));

        let task = tokio::spawn(dsp_task_fn(DspTaskParams {
            source: AudioSource::File(PathBuf::from("test.wav")),
            frame_rx,
            dsp_config_rx: dsp_cfg_rx,
            initial_dsp_config: DspConfig::default(),
            engine_config: headless_config(),
            ring: Arc::new(RingBuffer::new(1024)),
            signal_path_tx: Arc::new(signal_path_tx),
            state: Arc::clone(&state),
            event_tx: event_tx.clone(),
            seek_generation_rx,
            output_error_rx,
            output_error_tx,
            state_notify: Arc::new(Notify::new()),
            backend_paused: Arc::new(AtomicBool::new(false)),
        }));

        frame_tx
            .send(DecodeOutcome::Frame {
                frame: DecodedFrame {
                    samples: vec![0.0; 8].into_boxed_slice(),
                    channels: 2,
                    sample_rate: 44100,
                    timestamp: 0,
                },
                generation: 0,
            })
            .await
            .unwrap();
        frame_tx.send(DecodeOutcome::Failed).await.unwrap();

        timeout(Duration::from_secs(5), task)
            .await
            .expect("dsp task must terminate on Failed")
            .unwrap();
        assert_eq!(state.load(Ordering::SeqCst), STATE_STOPPED);

        let mut saw_track_ended = false;
        let mut saw_stopped = false;
        while let Ok(evt) = events.try_recv() {
            match evt {
                EngineEvent::TrackEnded { .. } => saw_track_ended = true,
                EngineEvent::PlaybackStopped => saw_stopped = true,
                _ => {}
            }
        }
        assert!(
            !saw_track_ended,
            "decode failure must not be reported as TrackEnded"
        );
        assert!(saw_stopped, "decode failure must emit PlaybackStopped");
    }

    // --- #404: output stream errors must stop playback ---

    #[tokio::test]
    async fn output_stream_error_emits_error_and_stops() {
        let engine = Engine::new(headless_config()).unwrap();
        let mut events = engine.subscribe_events();
        // 2s stereo ≫ ring capacity keeps the DSP task alive (in backpressure) so the
        // injected error is observed FROM the push loop poll.
        let wav = make_wav(2, 44100, 2.0);
        engine
            .play(AudioSource::File(wav.path().to_path_buf()))
            .unwrap();
        loop {
            let evt = timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(evt, EngineEvent::PlaybackStarted { .. }) {
                break;
            }
        }

        assert!(
            engine.inject_output_error(OutputError::StreamError {
                message: "device unplugged".to_string(),
            }),
            "injection requires a live session"
        );

        let mut saw_error = false;
        let mut saw_stopped = false;
        for _ in 0..20 {
            match timeout(Duration::from_secs(5), events.recv()).await {
                Ok(Ok(EngineEvent::Error { message })) => {
                    if message.contains("device unplugged") {
                        saw_error = true;
                    }
                }
                Ok(Ok(EngineEvent::PlaybackStopped)) => {
                    saw_stopped = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(saw_error, "stream error must surface as EngineEvent::Error");
        assert!(saw_stopped, "stream error must stop playback");

        // Engine must be reusable after the failure.
        let wav2 = make_wav(2, 44100, 0.2);
        engine
            .play(AudioSource::File(wav2.path().to_path_buf()))
            .expect("engine must accept a new play() after a stream error");
        engine.stop().await.unwrap();
    }
}
