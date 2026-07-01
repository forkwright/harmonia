/// Jitter buffer: reorder, buffer, and emit audio frames at playout time.
use std::collections::BTreeMap;

use crate::config::ClientConfig;
use crate::protocol::frame::AudioFrame;

#[derive(Debug)]
pub struct JitterBuffer {
    frames: BTreeMap<u64, AudioFrame>,
    depth_us: u64,
    max_frames: usize,
    next_sequence: u64,
    clock_offset_us: i64,
    gap_count: u64,
    dropped_count: u64,
}

impl JitterBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(&ClientConfig::default())
    }

    #[must_use]
    pub fn with_config(config: &ClientConfig) -> Self {
        Self {
            frames: BTreeMap::new(),
            depth_us: config.jitter_buffer_depth_ms * 1000,
            max_frames: config.jitter_buffer_max_frames,
            next_sequence: 0,
            clock_offset_us: 0,
            gap_count: 0,
            dropped_count: 0,
        }
    }

    #[must_use]
    pub fn with_depth_ms(depth_ms: u64) -> Self {
        Self::with_config(&ClientConfig {
            jitter_buffer_depth_ms: depth_ms,
            ..ClientConfig::default()
        })
    }

    /// Update the clock OFFSET used for playout timing.
    pub fn set_clock_offset(&mut self, offset_us: i64) {
        self.clock_offset_us = offset_us;
    }

    /// Insert a received frame into the buffer.
    ///
    /// At `max_frames` capacity the oldest buffered frame is evicted first,
    /// bounding memory when playout stalls while the stream keeps flowing.
    pub fn insert(&mut self, frame: AudioFrame) {
        while self.frames.len() >= self.max_frames {
            if let Some((&oldest, _)) = self.frames.first_key_value() {
                self.frames.remove(&oldest);
                self.dropped_count += 1;
            } else {
                break;
            }
        }
        self.frames.insert(frame.sequence, frame);
    }

    /// Try to emit the next frame in sequence ORDER, respecting the jitter buffer depth.
    ///
    /// `now_us` is the local clock time in microseconds.
    /// Returns the frame if it is ready for playout.
    pub fn pop_ready(&mut self, now_us: u64) -> Option<AudioFrame> {
        let (&seq, frame) = self.frames.first_key_value()?;

        // Adjust the frame timestamp by clock OFFSET to convert to local time
        let local_playout = (i64::try_from(frame.timestamp_us).unwrap_or_default() // WHY: audio timestamps fit comfortably in i64 (microseconds since epoch; ~292k year range)
            + self.clock_offset_us) as u64
            + self.depth_us;

        if now_us >= local_playout {
            // INVARIANT: seq came from first_key_value() above; no intervening mutation removes it.
            let frame = self
                .frames
                .remove(&seq)
                .expect("key just read via first_key_value()"); // INVARIANT: see comment above

            if seq > self.next_sequence {
                self.gap_count += seq - self.next_sequence;
            }
            self.next_sequence = seq + 1;

            Some(frame)
        } else {
            None
        }
    }

    /// Drain all frames that are past their playout deadline (for catchup).
    pub fn drain_ready(&mut self, now_us: u64) -> Vec<AudioFrame> {
        let mut ready = Vec::new();
        while let Some(frame) = self.pop_ready(now_us) {
            ready.push(frame);
        }
        ready
    }

    /// Current buffer depth in frames.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Number of sequence gaps detected (potential lost frames).
    #[must_use]
    pub fn gap_count(&self) -> u64 {
        self.gap_count
    }

    /// Number of frames evicted because the buffer was at capacity.
    #[must_use]
    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    /// Estimated buffer depth in milliseconds based on timestamp span.
    #[must_use]
    pub fn depth_ms(&self) -> u16 {
        if self.frames.len() < 2 {
            return 0;
        }
        // INVARIANT: len() >= 2 (guarded above), so next() and next_back() both yield Some.
        let first_ts = self
            .frames
            .values()
            .next()
            .expect("len >= 2 guard above") // INVARIANT: see comment above
            .timestamp_us;
        let last_ts = self
            .frames
            .values()
            .next_back()
            .expect("len >= 2 guard above") // INVARIANT: see comment above
            .timestamp_us;
        ((last_ts.saturating_sub(first_ts)) / 1000) as u16
    }
}

impl Default for JitterBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::protocol::AudioCodec;

    fn test_frame(seq: u64, timestamp_us: u64) -> AudioFrame {
        AudioFrame {
            sequence: seq,
            timestamp_us,
            playout_ts: 0,
            codec: AudioCodec::Pcm,
            channels: 2,
            sample_rate: 48000,
            payload: Bytes::from_static(b"test"),
        }
    }

    #[test]
    fn emits_frames_in_sequence_order() {
        let mut buf = JitterBuffer::with_depth_ms(0);
        buf.insert(test_frame(2, 2000));
        buf.insert(test_frame(0, 0));
        buf.insert(test_frame(1, 1000));

        let frames = buf.drain_ready(u64::MAX);
        let seqs: Vec<u64> = frames.iter().map(|f| f.sequence).collect();
        assert_eq!(seqs, vec![0, 1, 2]);
    }

    #[test]
    fn respects_jitter_depth() {
        let mut buf = JitterBuffer::with_depth_ms(100);
        buf.insert(test_frame(0, 1_000_000));

        assert!(buf.pop_ready(1_000_000).is_none(), "too early for playout");
        assert!(
            buf.pop_ready(1_100_000).is_some(),
            "should be ready after depth"
        );
    }

    #[test]
    fn detects_sequence_gaps() {
        let mut buf = JitterBuffer::with_depth_ms(0);
        buf.insert(test_frame(0, 0));
        buf.insert(test_frame(3, 3000));

        buf.drain_ready(u64::MAX);
        assert_eq!(buf.gap_count(), 2, "missed sequences 1 and 2");
    }

    #[test]
    fn reports_buffer_depth() {
        let mut buf = JitterBuffer::new();
        buf.insert(test_frame(0, 0));
        buf.insert(test_frame(1, 50_000));
        assert_eq!(buf.depth_ms(), 50);
    }

    #[test]
    fn jitter_buffer_evicts_oldest_when_full() {
        let config = ClientConfig {
            jitter_buffer_max_frames: 3,
            ..ClientConfig::default()
        };
        let mut buf = JitterBuffer::with_config(&config);
        for seq in 0..4u64 {
            buf.insert(test_frame(seq, seq * 1000));
        }

        assert_eq!(buf.len(), 3, "capacity cap must hold");
        assert_eq!(buf.dropped_count(), 1, "one eviction expected");
        // Oldest (seq 0) evicted; the next playable frame is seq 1.
        let first = buf.pop_ready(u64::MAX).map(|f| f.sequence);
        assert_eq!(first, Some(1));
    }

    #[test]
    fn jitter_buffer_length_never_exceeds_cap_without_playout() {
        let config = ClientConfig {
            jitter_buffer_max_frames: 8,
            ..ClientConfig::default()
        };
        let mut buf = JitterBuffer::with_config(&config);
        for seq in 0..1000u64 {
            buf.insert(test_frame(seq, seq * 1000));
            assert!(buf.len() <= 8, "buffer exceeded cap at seq {seq}");
        }
        assert_eq!(buf.dropped_count(), 992);
    }

    #[test]
    fn applies_clock_offset() {
        let mut buf = JitterBuffer::with_depth_ms(0);
        buf.set_clock_offset(-500_000);
        buf.insert(test_frame(0, 1_000_000));

        // Frame timestamp 1_000_000 - OFFSET 500_000 = local playout at 500_000
        assert!(buf.pop_ready(499_999).is_none());
        assert!(buf.pop_ready(500_000).is_some());
    }
}
