use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{broadcast, watch};
use tracing::instrument;

use crate::config::{DspConfig, EngineConfig};
use crate::error::EngineError;
use crate::signal_path::SignalPathSnapshot;

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
    /// The engine is filling its pre-buffer before starting playback.
    Buffering { progress: f32 },
    /// Playback started or resumed.
    Playing,
    /// Playback paused.
    Paused,
    /// Playback stopped and the engine is idle.
    Stopped,
    /// The current track finished and the engine moved to the next (gapless) or stopped.
    TrackEnded,
    /// The signal path configuration changed (e.g. DSP stage enabled/disabled).
    SignalPathChanged(SignalPathSnapshot),
}

/// The audio engine: owns the decode → DSP → output pipeline.
///
/// Construct via `Engine::new`, wrap in `Arc<Engine>` for multi-task access.
/// All methods take `&self` and use internal synchronisation.
///
/// Full implementation is in P1-10. All method bodies are `todo!()` stubs here.
#[allow(dead_code)]
pub struct Engine {
    config: EngineConfig,
    signal_path_tx: watch::Sender<SignalPathSnapshot>,
    signal_path_rx: watch::Receiver<SignalPathSnapshot>,
    event_tx: broadcast::Sender<EngineEvent>,
}

impl Engine {
    /// Creates a new engine with the given configuration.
    #[instrument]
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let _ = config;
        todo!("P1-10: initialise ring buffer, DSP pipeline, output backend")
    }

    /// Begins playback of `source`. Returns `EngineError::AlreadyPlaying` if a track is
    /// currently playing — call `stop()` first.
    #[instrument(skip(self))]
    pub fn play(&self, source: AudioSource) -> Result<(), EngineError> {
        let _ = source;
        todo!("P1-10: open decoder, fill pre-buffer, start output stream")
    }

    /// Pauses playback at the current position. Returns immediately if already paused.
    #[instrument(skip(self))]
    pub fn pause(&self) -> Result<(), EngineError> {
        todo!("P1-10: signal the output task to pause")
    }

    /// Resumes paused playback.
    #[instrument(skip(self))]
    pub fn resume(&self) -> Result<(), EngineError> {
        todo!("P1-10: signal the output task to resume")
    }

    /// Stops playback and resets the engine to idle.
    #[instrument(skip(self))]
    pub fn stop(&self) -> Result<(), EngineError> {
        todo!("P1-10: signal decode + output tasks to shut down; drain ring buffer")
    }

    /// Seeks to `position` within the current track. Returns the actual position reached
    /// (may differ due to frame boundaries or container seek granularity).
    ///
    /// Returns `EngineError::SeekOutOfBounds` if `position` exceeds the track duration.
    #[instrument(skip(self))]
    pub fn seek(&self, _position: Duration) -> Result<Duration, EngineError> {
        todo!("P1-10: forward seek request to decode task; flush ring buffer")
    }

    /// Applies a new DSP configuration to the running pipeline without interrupting playback.
    #[instrument(skip(self))]
    pub fn configure_dsp(&self, config: DspConfig) {
        let _ = config;
        todo!("P1-10: send config update via watch::Sender<DspConfig>")
    }

    /// Returns the current signal path snapshot.
    pub fn signal_path(&self) -> SignalPathSnapshot {
        todo!("P1-10: read current value from watch::Receiver<SignalPathSnapshot>")
    }

    /// Returns a watch receiver that emits a new `SignalPathSnapshot` whenever the signal
    /// path changes (DSP config updated, source changed, output opened/closed).
    pub fn signal_path_stream(&self) -> watch::Receiver<SignalPathSnapshot> {
        todo!("P1-10: return clone of watch::Receiver<SignalPathSnapshot>")
    }

    /// Returns a broadcast receiver for playback events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<EngineEvent> {
        todo!("P1-10: return self.event_tx.subscribe()")
    }
}
