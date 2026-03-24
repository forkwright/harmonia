/// Audio source trait: abstraction for feeding audio data into the stream.
use crate::protocol::frame::AudioFrame;

pub trait AudioSource: Send + Sync + 'static {
    /// Fetch the next audio frame. Returns None when the source is exhausted.
    fn next_frame(&mut self) -> impl std::future::Future<Output = Option<AudioFrame>> + Send;
}

/// Test audio source backed by a pre-loaded Vec of frames.
#[derive(Debug, Clone)]
pub struct VecAudioSource {
    frames: Vec<AudioFrame>,
    index: usize,
}

impl VecAudioSource {
    #[must_use]
    pub fn new(frames: Vec<AudioFrame>) -> Self {
        Self { frames, index: 0 }
    }
}

impl AudioSource for VecAudioSource {
    async fn next_frame(&mut self) -> Option<AudioFrame> {
        if self.index < self.frames.len() {
            let frame = self.frames[self.index].clone();
            self.index += 1;
            Some(frame)
        } else {
            None
        }
    }
}
