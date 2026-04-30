// Play subcommand — plays a local audio file via akouo-core.

use std::io::Write;
use std::sync::Arc;

use akouo_core::{AudioSource, Engine, EngineConfig, EngineEvent};
use snafu::ResultExt;

use crate::cli::PlayArgs;
use crate::error::{AudioEngineSnafu, HostError};

pub async fn run_play(args: PlayArgs, out: &mut impl Write) -> Result<(), HostError> {
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
                let _ = writeln!(out, "playback error: {message}");
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    engine.stop().context(AudioEngineSnafu)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_play_output_param_accepted() {
        let mut out = Vec::new();
        let args = PlayArgs {
            file: std::path::PathBuf::from("/nonexistent/file.flac"),
            device: None,
        };
        // Verify the function accepts a Vec<u8> writer and completes without panic.
        let result = run_play(args, &mut out).await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
    }
}
