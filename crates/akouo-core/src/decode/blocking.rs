// Dedicated-thread decoder wrapper: keeps blocking decode I/O off the async executor.

use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use tokio::sync::oneshot;

use crate::decode::{AudioDecoder, DecodedFrame, GaplessInfo, StreamParams, SyncAudioDecoder};
use crate::error::DecodeError;

enum Command {
    NextFrame(oneshot::Sender<Result<Option<DecodedFrame>, DecodeError>>),
    Seek(Duration, oneshot::Sender<Result<Duration, DecodeError>>),
}

/// Runs a `SyncAudioDecoder` on a dedicated OS thread and exposes the async
/// `AudioDecoder` interface.
///
/// Every decode and seek call — including their blocking file reads — executes on the
/// decode thread; the async side only awaits a `oneshot` reply, so the Tokio executor is
/// never blocked regardless of runtime flavor (works on `current_thread` runtimes where
/// `block_in_place` would panic).
///
/// Dropping the wrapper closes the command channel; the decode thread drains any queued
/// command and exits on its own. No join occurs on drop, so dropping from async context
/// never blocks the runtime.
pub struct BlockingDecoder {
    command_tx: std_mpsc::Sender<Command>,
    stream_params: StreamParams,
    gapless_info: Option<GaplessInfo>,
}

impl BlockingDecoder {
    /// Moves `inner` onto a dedicated decode thread and returns the async wrapper.
    pub fn spawn(inner: Box<dyn SyncAudioDecoder>) -> Result<Self, DecodeError> {
        let stream_params = inner.stream_params();
        let gapless_info = inner.gapless_info();
        let (command_tx, command_rx) = std_mpsc::channel::<Command>();

        std::thread::Builder::new()
            .name("akouo-decode".to_string())
            .spawn(move || decode_thread(inner, command_rx))
            .map_err(|e| DecodeError::TaskJoin {
                message: format!("failed to spawn decode thread: {e}"),
                location: snafu::location!(),
            })?;

        Ok(Self {
            command_tx,
            stream_params,
            gapless_info,
        })
    }
}

fn decode_thread(mut inner: Box<dyn SyncAudioDecoder>, command_rx: std_mpsc::Receiver<Command>) {
    while let Ok(command) = command_rx.recv() {
        match command {
            // WHY: reply send fails only when the caller dropped its receiver
            // (request cancelled); the result is discarded intentionally.
            Command::NextFrame(reply) => {
                reply.send(inner.next_frame()).ok();
            }
            Command::Seek(position, reply) => {
                reply.send(inner.seek(position)).ok();
            }
        }
    }
}

fn thread_terminated() -> DecodeError {
    DecodeError::TaskJoin {
        message: "decode thread terminated unexpectedly".to_string(),
        location: snafu::location!(),
    }
}

impl AudioDecoder for BlockingDecoder {
    fn next_frame(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DecodedFrame>, DecodeError>> + Send + '_>> {
        Box::pin(async move {
            let (reply_tx, reply_rx) = oneshot::channel();
            self.command_tx
                .send(Command::NextFrame(reply_tx))
                .map_err(|_| thread_terminated())?;
            reply_rx.await.unwrap_or_else(|_| Err(thread_terminated()))
        })
    }

    fn seek(
        &mut self,
        position: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Duration, DecodeError>> + Send + '_>> {
        Box::pin(async move {
            let (reply_tx, reply_rx) = oneshot::channel();
            self.command_tx
                .send(Command::Seek(position, reply_tx))
                .map_err(|_| thread_terminated())?;
            reply_rx.await.unwrap_or_else(|_| Err(thread_terminated()))
        })
    }

    fn stream_params(&self) -> StreamParams {
        self.stream_params.clone()
    }

    fn gapless_info(&self) -> Option<GaplessInfo> {
        self.gapless_info.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Codec;

    /// Sync decoder stub producing `frames_left` one-sample frames, then EOS.
    struct StubDecoder {
        frames_left: u32,
        fail_next: bool,
    }

    impl SyncAudioDecoder for StubDecoder {
        fn next_frame(&mut self) -> Result<Option<DecodedFrame>, DecodeError> {
            if self.fail_next {
                return Err(DecodeError::SymphoniaDecode {
                    message: "stub failure".to_string(),
                    location: snafu::location!(),
                });
            }
            if self.frames_left == 0 {
                return Ok(None);
            }
            self.frames_left -= 1;
            Ok(Some(DecodedFrame {
                samples: vec![0.25].into_boxed_slice(),
                channels: 1,
                sample_rate: 44100,
                timestamp: 0,
            }))
        }

        fn seek(&mut self, position: Duration) -> Result<Duration, DecodeError> {
            Ok(position / 2)
        }

        fn stream_params(&self) -> StreamParams {
            StreamParams {
                codec: Codec::Wav,
                sample_rate: 44100,
                channels: 1,
                bit_depth: Some(16),
                duration: None,
                bitrate: None,
            }
        }

        fn gapless_info(&self) -> Option<GaplessInfo> {
            None
        }
    }

    #[tokio::test]
    async fn blocking_decoder_streams_frames_then_eos() {
        let mut dec = BlockingDecoder::spawn(Box::new(StubDecoder {
            frames_left: 2,
            fail_next: false,
        }))
        .unwrap();

        assert!(dec.next_frame().await.unwrap().is_some());
        assert!(dec.next_frame().await.unwrap().is_some());
        assert!(dec.next_frame().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn blocking_decoder_propagates_decode_errors() {
        let mut dec = BlockingDecoder::spawn(Box::new(StubDecoder {
            frames_left: 0,
            fail_next: true,
        }))
        .unwrap();

        let err = dec.next_frame().await.expect_err("stub must fail");
        assert!(matches!(err, DecodeError::SymphoniaDecode { .. }));
    }

    #[tokio::test]
    async fn blocking_decoder_seek_returns_inner_actual_position() {
        let mut dec = BlockingDecoder::spawn(Box::new(StubDecoder {
            frames_left: 0,
            fail_next: false,
        }))
        .unwrap();

        let actual = dec.seek(Duration::from_secs(2)).await.unwrap();
        assert_eq!(actual, Duration::from_secs(1), "stub halves the target");
    }

    #[tokio::test]
    async fn blocking_decoder_caches_params_and_gapless() {
        let dec = BlockingDecoder::spawn(Box::new(StubDecoder {
            frames_left: 0,
            fail_next: false,
        }))
        .unwrap();

        assert_eq!(dec.stream_params().sample_rate, 44100);
        assert!(dec.gapless_info().is_none());
    }
}
