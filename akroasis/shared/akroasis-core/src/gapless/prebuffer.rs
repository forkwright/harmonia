// P1-10: PreBuffer task — decodes ahead into a secondary ring buffer for gapless transitions.

use crate::ring_buffer::RingBuffer;

/// Manages a secondary decode buffer for the upcoming track.
///
/// When the current track's ring buffer crosses below `threshold_samples`, `PreBuffer`
/// begins decoding the next source into `buffer` so the transition is seamless.
pub struct PreBuffer {
    buffer: RingBuffer,
    threshold_samples: usize,
}

impl PreBuffer {
    pub fn new(capacity: usize, threshold_samples: usize) -> Self {
        Self {
            buffer: RingBuffer::new(capacity),
            threshold_samples,
        }
    }

    /// Starts the background decode task for the next source.
    pub async fn start_prefetch(&self) {
        todo!("P1-10: spawn tokio task: open_decoder(next_source), fill self.buffer")
    }

    /// Returns a reference to the pre-buffered sample store.
    pub fn buffer(&self) -> &RingBuffer {
        &self.buffer
    }

    /// Returns the threshold in samples below which pre-fetching begins.
    pub fn threshold_samples(&self) -> usize {
        self.threshold_samples
    }
}
