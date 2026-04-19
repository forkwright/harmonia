use std::collections::VecDeque;

use tokio::task::JoinHandle;
use tracing::instrument;

/// Pre-decoded frame buffer for the upcoming track.
///
/// When the current track's remaining samples fall below `threshold_secs`, the engine
/// spawns a decode task (P1-10) that fills this buffer ahead of time so the transition
/// is seamless. Frames are stored as interleaved f64 slices, already processed by the
/// DSP pipeline.
pub struct PreBuffer {
    frames: VecDeque<Box<[f64]>>,
    threshold_secs: f64,
    max_frames: usize,
    task: Option<JoinHandle<()>>,
}

impl PreBuffer {
    pub(crate) fn new(threshold_secs: f64, max_frames: usize) -> Self {
        Self {
            frames: VecDeque::new(),
            threshold_secs,
            max_frames,
            task: None,
        }
    }

    /// Returns how many seconds before track end to begin pre-buffering.
    pub(crate) fn threshold_secs(&self) -> f64 {
        self.threshold_secs
    }

    /// Returns the maximum number of frames this buffer will hold.
    pub fn max_frames(&self) -> usize {
        self.max_frames
    }

    /// Pushes a decoded frame INTO the buffer. Returns `false` if the buffer is full.
    pub fn push_frame(&mut self, frame: Box<[f64]>) -> bool {
        if self.frames.len() >= self.max_frames {
            return false;
        }
        self.frames.push_back(frame);
        true
    }

    /// Pops the next pre-buffered frame. Returns `None` when the buffer is empty.
    pub fn pop_frame(&mut self) -> Option<Box<[f64]>> {
        self.frames.pop_front()
    }

    /// Number of frames currently in the buffer.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Registers the background decode task handle so it can be cancelled later.
    pub fn set_task(&mut self, handle: JoinHandle<()>) {
        self.task = Some(handle);
    }

    /// Cancels the background decode task if one is running.
    #[instrument(skip(self))]
    pub(crate) fn cancel(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    /// Cancels any background task and discards all buffered frames.
    pub(crate) fn clear(&mut self) {
        self.cancel();
        self.frames.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pop_frames() {
        let mut buf = PreBuffer::new(10.0, 4);
        let frame: Box<[f64]> = vec![0.1, 0.2, 0.3, 0.4].into_boxed_slice();
        assert!(buf.push_frame(frame.clone()));
        assert_eq!(buf.len(), 1);
        let out = buf.pop_frame().unwrap();
        assert_eq!(&*out, &[0.1, 0.2, 0.3, 0.4]);
        assert!(buf.is_empty());
    }

    #[test]
    fn buffer_respects_max_frames() {
        let mut buf = PreBuffer::new(10.0, 2);
        let frame: Box<[f64]> = vec![0.0].into_boxed_slice();
        assert!(buf.push_frame(frame.clone()));
        assert!(buf.push_frame(frame.clone()));
        assert!(!buf.push_frame(frame)); // full
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn clear_empties_buffer() {
        let mut buf = PreBuffer::new(10.0, 8);
        buf.push_frame(vec![1.0, 2.0].into_boxed_slice());
        buf.push_frame(vec![3.0, 4.0].into_boxed_slice());
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn pop_from_empty_returns_none() {
        let mut buf = PreBuffer::new(10.0, 8);
        assert!(buf.pop_frame().is_none());
    }
}
