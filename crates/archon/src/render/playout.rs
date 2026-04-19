/// Playout scheduling: receives audio frames with playout timestamps and
/// schedules output at the correct wall-clock moment for zone-synchronized playback.
use std::collections::VecDeque;
use std::time::Duration;

use tracing::{debug, trace, warn};

use super::config::BufferSettings;

/// A frame queued for playout at a specific local time.
#[derive(Debug, Clone)]
pub struct PlayoutFrame {
    pub sequence: u64,
    pub playout_ts: u64,
    pub timestamp_us: u64,
    pub payload_len: usize,
}

/// Outcome of scheduling a frame for playout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayoutDecision {
    /// Frame is ready to play now.
    Play,
    /// Frame should be held until playout time.
    Hold { wait_us: u64 },
    /// Frame arrived too late  -  playout time already passed.
    Late { late_by_us: u64 },
}

/// Renderer-side playout pipeline: converts server playout timestamps to local
/// scheduled output times using clock OFFSET.
#[derive(Debug)]
pub struct PlayoutPipeline {
    clock_offset_us: i64,
    buffer_target_us: u64,
    pending: VecDeque<PlayoutFrame>,
    underrun_count: u64,
    early_count: u64,
    late_history: VecDeque<bool>,
    settings: BufferSettings,
}

impl PlayoutPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self::with_settings(BufferSettings::default())
    }

    #[must_use]
    pub fn with_settings(settings: BufferSettings) -> Self {
        Self {
            clock_offset_us: 0,
            buffer_target_us: settings.playout_initial_depth_ms * 1000,
            pending: VecDeque::new(),
            underrun_count: 0,
            early_count: 0,
            late_history: VecDeque::with_capacity(settings.playout_stats_window),
            settings,
        }
    }

    /// Update the clock OFFSET (server-to-local difference in microseconds).
    pub fn set_clock_offset(&mut self, offset_us: i64) {
        self.clock_offset_us = offset_us;
    }

    /// Enqueue a frame for playout scheduling.
    pub fn enqueue(&mut self, frame: PlayoutFrame) {
        self.pending.push_back(frame);
    }

    /// Evaluate the next frame against the current local time.
    ///
    /// `local_now_us` is the current monotonic time in microseconds.
    #[must_use]
    pub fn evaluate(&self, local_now_us: u64) -> Option<PlayoutDecision> {
        let frame = self.pending.front()?;

        if frame.playout_ts == 0 {
            return Some(PlayoutDecision::Play);
        }

        // WHY: Convert server playout timestamp to local time by subtracting the
        // server's clock OFFSET. If server clock is ahead (positive OFFSET),
        // local playout time is earlier.
        let local_playout =
            (i64::try_from(frame.playout_ts).unwrap_or_default() - self.clock_offset_us) as u64;

        if local_now_us >= local_playout {
            let late_by = local_now_us - local_playout;
            if late_by > self.buffer_target_us {
                Some(PlayoutDecision::Late {
                    late_by_us: late_by,
                })
            } else {
                Some(PlayoutDecision::Play)
            }
        } else {
            let wait = local_playout - local_now_us;
            Some(PlayoutDecision::Hold { wait_us: wait })
        }
    }

    /// Dequeue the next frame after deciding to play or skip it.
    pub fn dequeue(&mut self) -> Option<PlayoutFrame> {
        self.pending.pop_front()
    }

    /// Process frames: play ready ones, skip late ones, wait for early ones.
    /// Returns frames ready to play and the wait duration before checking again.
    pub fn process(&mut self, local_now_us: u64) -> (Vec<PlayoutFrame>, Option<Duration>) {
        let mut ready = Vec::new();

        loop {
            match self.evaluate(local_now_us) {
                Some(PlayoutDecision::Play) => {
                    if let Some(frame) = self.dequeue() {
                        self.record_timing(false);
                        ready.push(frame);
                    }
                }
                Some(PlayoutDecision::Late { late_by_us }) => {
                    if let Some(frame) = self.dequeue() {
                        warn!(
                            sequence = frame.sequence,
                            late_by_us, "frame underrun: skipping late frame"
                        );
                        self.underrun_count += 1;
                        self.record_timing(true);
                    }
                }
                Some(PlayoutDecision::Hold { wait_us }) => {
                    self.early_count += 1;
                    let wait = Duration::from_micros(wait_us.min(10_000));
                    return (ready, Some(wait));
                }
                None => {
                    return (ready, None);
                }
            }
        }
    }

    fn record_timing(&mut self, was_late: bool) {
        if self.late_history.len() >= self.settings.playout_stats_window {
            self.late_history.pop_front();
        }
        self.late_history.push_back(was_late);
        self.adapt_buffer();
    }

    /// Adaptive buffer depth: increase on consistent lateness, decrease on consistency.
    fn adapt_buffer(&mut self) {
        if self.late_history.len() < 20 {
            return;
        }

        let late_ratio = self.late_history.iter().filter(|&&l| l).count() as f64
            / self.late_history.len() as f64;
        let min_us = self.settings.playout_min_depth_ms * 1000;
        let max_us = self.settings.playout_max_depth_ms * 1000;

        if late_ratio > 0.1 {
            let increase = (self.buffer_target_us / 10).max(5_000);
            let new_target = (self.buffer_target_us + increase).min(max_us);
            if new_target != self.buffer_target_us {
                debug!(
                    old_ms = self.buffer_target_us / 1000,
                    new_ms = new_target / 1000,
                    late_ratio,
                    "increasing buffer target due to late frames"
                );
                self.buffer_target_us = new_target;
            }
        } else if late_ratio < 0.01 && self.buffer_target_us > min_us {
            let decrease = (self.buffer_target_us / 20).max(1_000);
            let new_target = self.buffer_target_us.saturating_sub(decrease).max(min_us);
            if new_target != self.buffer_target_us {
                trace!(
                    old_ms = self.buffer_target_us / 1000,
                    new_ms = new_target / 1000,
                    "decreasing buffer target -- running smoothly"
                );
                self.buffer_target_us = new_target;
            }
        }
    }

    #[must_use]
    pub fn underrun_count(&self) -> u64 {
        self.underrun_count
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub fn buffer_target_ms(&self) -> u64 {
        self.buffer_target_us / 1000
    }
}

impl Default for PlayoutPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(seq: u64, playout: u64) -> PlayoutFrame {
        PlayoutFrame {
            sequence: seq,
            playout_ts: playout,
            timestamp_us: playout.saturating_sub(10_000),
            payload_len: 1024,
        }
    }

    #[test]
    fn play_when_on_time() {
        let mut pipe = PlayoutPipeline::new();
        pipe.enqueue(frame(0, 1_000_000));
        let decision = pipe.evaluate(1_000_000);
        assert_eq!(decision, Some(PlayoutDecision::Play));
    }

    #[test]
    fn hold_when_early() {
        let mut pipe = PlayoutPipeline::new();
        pipe.enqueue(frame(0, 1_000_000));
        let decision = pipe.evaluate(900_000);
        assert!(matches!(decision, Some(PlayoutDecision::Hold { .. })));
        if let Some(PlayoutDecision::Hold { wait_us }) = decision {
            assert_eq!(wait_us, 100_000);
        }
    }

    #[test]
    fn late_when_very_late() {
        let mut pipe = PlayoutPipeline::new();
        pipe.set_clock_offset(0);
        pipe.enqueue(frame(0, 100_000));
        let decision = pipe.evaluate(500_000);
        assert!(matches!(decision, Some(PlayoutDecision::Late { .. })));
    }

    #[test]
    fn zero_playout_ts_plays_immediately() {
        let mut pipe = PlayoutPipeline::new();
        pipe.enqueue(PlayoutFrame {
            sequence: 0,
            playout_ts: 0,
            timestamp_us: 0,
            payload_len: 128,
        });
        assert_eq!(pipe.evaluate(0), Some(PlayoutDecision::Play));
    }

    #[test]
    fn clock_offset_adjusts_playout() {
        let mut pipe = PlayoutPipeline::new();
        pipe.set_clock_offset(1000);
        pipe.enqueue(frame(0, 1_001_000));

        // Local time 1_000_000: server says play at 1_001_000, but server is 1000us
        // ahead, so local playout = 1_001_000 - 1000 = 1_000_000
        assert_eq!(pipe.evaluate(1_000_000), Some(PlayoutDecision::Play));
    }

    #[test]
    fn process_returns_ready_frames() {
        let mut pipe = PlayoutPipeline::new();
        pipe.enqueue(frame(0, 100));
        pipe.enqueue(frame(1, 200));
        pipe.enqueue(frame(2, 1_000_000));

        let (ready, wait) = pipe.process(500);
        assert_eq!(ready.len(), 2);
        assert_eq!(ready[0].sequence, 0);
        assert_eq!(ready[1].sequence, 1);
        assert!(wait.is_some());
    }

    #[test]
    fn empty_pipeline_returns_none() {
        let pipe = PlayoutPipeline::new();
        assert_eq!(pipe.evaluate(1_000_000), None);
    }

    #[test]
    fn adaptive_buffer_increases_on_late_frames() {
        let mut pipe = PlayoutPipeline::new();
        let initial = pipe.buffer_target_ms();

        for i in 0..30 {
            pipe.enqueue(frame(i, 100));
        }
        let _ = pipe.process(500_000);

        assert!(
            pipe.buffer_target_ms() >= initial,
            "buffer should increase or stay after late frames"
        );
    }

    #[test]
    fn custom_initial_depth_observed() {
        // WHY: non-default settings must observably change the initial buffer target.
        let settings = BufferSettings {
            playout_initial_depth_ms: 37,
            ..BufferSettings::default()
        };
        let pipe = PlayoutPipeline::with_settings(settings);
        assert_eq!(pipe.buffer_target_ms(), 37);
    }
}
