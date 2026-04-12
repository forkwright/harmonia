// Play subcommand — plays a local audio file via akouo-core.

use std::sync::Arc;

use akouo_core::{AudioSource, Engine, EngineConfig, EngineEvent};
use snafu::ResultExt;

use crate::cli::PlayArgs;
use crate::error::{AudioEngineSnafu, HostError};

pub async fn run_play(args: PlayArgs) -> Result<(), HostError> {
    let config = EngineConfig::default();
    let engine = Arc::new(Engine::new(config).context(AudioEngineSnafu)?);
    let mut events = engine.subscribe_events();

    let source = AudioSource::File(args.file);
    engine.play(source).context(AudioEngineSnafu)?;

    // WHY: block until playback finishes or an error occurs.
    loop {
        match events.recv().await {
            Ok(EngineEvent::PlaybackStopped | EngineEvent::TrackEnded { .. }) => break,
            Ok(EngineEvent::Error { message }) => {
                eprintln!("playback error: {message}");
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    engine.stop().context(AudioEngineSnafu)?;
    Ok(())
}
