// Play subcommand — plays a local audio file via akouo-core.

use std::io::Write;
use std::sync::Arc;

use akouo_core::{AudioSource, Engine, EngineConfig, EngineEvent};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;

use crate::cli::PlayArgs;
use crate::error::{AudioEngineSnafu, HostError};

pub async fn run_play(
    args: PlayArgs,
    out: &mut impl Write,
    cancel: CancellationToken,
) -> Result<(), HostError> {
    run_play_with_config(args, EngineConfig::default(), out, cancel).await
}

/// Plays `args.file` until the track ends, the engine reports an error, or
/// `cancel` fires. Cancellation is cooperative: the event loop selects on
/// the token, and the existing `engine.stop()` teardown below is the real
/// stop path — it aborts the decode/DSP tasks and awaits their joins, so a
/// cancelled call releases the audio device instead of lingering.
async fn run_play_with_config(
    args: PlayArgs,
    config: EngineConfig,
    out: &mut impl Write,
    cancel: CancellationToken,
) -> Result<(), HostError> {
    let engine = Arc::new(Engine::new(config).context(AudioEngineSnafu)?);
    let mut events = engine.subscribe_events();

    let source = AudioSource::File(args.file);
    engine.play(source).context(AudioEngineSnafu)?;

    // WHY: block until playback finishes or an error occurs — or the caller
    // cancels (MCP `notifications/cancelled` lands on this token). A fresh,
    // never-cancelled token from the CLI pends forever and changes nothing.
    let cancelled = loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break true,
            event = events.recv() => match event {
                Ok(EngineEvent::PlaybackStopped | EngineEvent::TrackEnded { .. }) => break false,
                Ok(EngineEvent::Error { message }) => {
                    // WHY: writeln! to stdout is non-fatal; broken pipe on exit is expected behavior
                    writeln!(out, "playback error: {message}").ok();
                    break false;
                }
                Ok(_) => {}
                Err(_) => break false,
            },
        }
    };

    engine.stop().await.context(AudioEngineSnafu)?;
    if cancelled {
        // WHY: names the stop cause so an MCP client can tell a cancelled
        // playback apart from a track that merely ran to completion.
        writeln!(out, "playback stopped (cancelled)").ok();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::time::Duration;

    use super::*;

    /// Writes a silent 16-bit PCM WAV; mirrored on akouo-core's own
    /// `make_wav` test helper, kept local so this test owns its fixture.
    fn write_wav(
        dir: &std::path::Path,
        channels: u16,
        sample_rate: u32,
        secs: f32,
    ) -> std::path::PathBuf {
        let n_samples = (sample_rate as f32 * secs) as u32 * u32::from(channels);
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
        v.extend_from_slice(&16u16.to_le_bytes()); // bit depth
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.extend(std::iter::repeat_n(0u8, data_len as usize));

        let path = dir.join("tone.wav");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&v).unwrap();
        path
    }

    #[tokio::test]
    async fn run_play_output_param_accepted() {
        let mut out = Vec::new();
        let args = PlayArgs {
            file: std::path::PathBuf::from("/nonexistent/file.flac"),
            device: None,
        };
        // Verify the function accepts a Vec<u8> writer and completes without panic.
        let result = run_play(args, &mut out, CancellationToken::new()).await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
    }

    #[tokio::test]
    async fn cancellation_triggers_the_engine_stop_path() {
        // WHY: the #652 PR-3 falsifier for playback — a cancelled call must
        // STOP the engine, not just return. With output disabled the engine
        // decodes into a ring no hardware drains: once the ring fills, the
        // 300s track never reaches TrackEnded on its own, so a prompt return
        // after cancel() can only come from the ct branch + engine.stop().
        let dir = tempfile::tempdir().unwrap();
        let wav = write_wav(dir.path(), 1, 8000, 300.0);

        let mut config = EngineConfig::default();
        config.output.enabled = false;
        let args = PlayArgs {
            file: wav,
            device: None,
        };
        let cancel = CancellationToken::new();
        let mut out = Vec::new();

        // WHY the inner scope: `pin!` stores the future in a hidden local
        // dropped at scope end; the scope ends the `&mut out` borrow before
        // the output is inspected below.
        {
            let mut run =
                std::pin::pin!(run_play_with_config(args, config, &mut out, cancel.clone()));
            // Let playback start and the ring backpressure engage.
            tokio::time::sleep(Duration::from_millis(300)).await;
            assert!(
                tokio::time::timeout(Duration::from_millis(1), &mut run)
                    .await
                    .is_err(),
                "the 300s track must still be playing before the cancel lands"
            );

            cancel.cancel();
            tokio::time::timeout(Duration::from_secs(5), &mut run)
                .await
                .expect("a cancelled play must return promptly, not at track end")
                .expect("cancelled play returns Ok");
        }

        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("cancelled"),
            "the stop cause must be named in the output: {text:?}"
        );
    }
}
