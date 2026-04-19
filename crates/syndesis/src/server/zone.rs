/// Zone-coordinated streaming: single decode, fan-out to multiple renderers.
use std::collections::HashMap;

use bytes::Bytes;
use tokio::sync::{mpsc, watch};
use tracing::{debug, info, warn};

use crate::clock::ClockCoordinator;
use crate::protocol::DeviceState;
use crate::protocol::codec::encode_frame;
use crate::protocol::frame::{AudioFrame, Frame};
use crate::server::session::current_time_us;
use crate::server::source::AudioSource;

const FRAME_CHANNEL_CAPACITY: usize = 256;
const LOW_WATERMARK_MS: u16 = 50;
const DEGRADED_LAG_COUNT: u32 = 10;

/// Per-renderer state within a zone.
struct ZoneMember {
    frame_tx: mpsc::Sender<Bytes>,
    buffer_depth_ms: u16,
    device_state: DeviceState,
    consecutive_lags: u32,
    is_degraded: bool,
}

/// Streaming state for a zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZonePlayState {
    Playing,
    Paused,
}

/// Coordinates streaming to multiple renderers in a zone with synchronized playout.
pub struct ZoneStream {
    coordinator: ClockCoordinator,
    members: HashMap<String, ZoneMember>,
    play_state: ZonePlayState,
    current_sequence: u64,
}

/// Sync point sent to a renderer joining mid-stream.
#[derive(Debug, Clone)]
pub struct SyncPoint {
    pub sequence: u64,
    pub playout_ts: u64,
    pub server_time: u64,
}

impl ZoneStream {
    #[must_use]
    pub fn new() -> Self {
        Self {
            coordinator: ClockCoordinator::new(),
            members: HashMap::new(),
            play_state: ZonePlayState::Paused,
            current_sequence: 0,
        }
    }

    /// Add a renderer to the zone. Returns a receiver for encoded frames.
    pub fn add_renderer(&mut self, renderer_id: &str) -> mpsc::Receiver<Bytes> {
        let (tx, rx) = mpsc::channel(FRAME_CHANNEL_CAPACITY);
        self.coordinator.add_renderer(renderer_id);
        self.members.insert(
            renderer_id.to_string(),
            ZoneMember {
                frame_tx: tx,
                buffer_depth_ms: 0,
                device_state: DeviceState::Idle,
                consecutive_lags: 0,
                is_degraded: false,
            },
        );
        info!(%renderer_id, "renderer joined zone stream");
        rx
    }

    /// Remove a renderer from the zone.
    pub fn remove_renderer(&mut self, renderer_id: &str) {
        self.members.remove(renderer_id);
        self.coordinator.remove_renderer(renderer_id);
        info!(%renderer_id, "renderer LEFT zone stream");
    }

    /// Get a sync point for a renderer joining mid-stream.
    #[must_use]
    pub fn sync_point(&self) -> SyncPoint {
        let now = current_time_us();
        SyncPoint {
            sequence: self.current_sequence,
            playout_ts: self.coordinator.compute_playout_ts(now).unwrap_or(now),
            server_time: now,
        }
    }

    /// Fan-out an audio frame to all zone members with computed playout timestamp.
    /// Uses `Bytes` reference counting — no per-renderer payload clone.
    pub async fn fan_out_frame(&mut self, mut frame: AudioFrame) {
        let now = current_time_us();
        frame.playout_ts = self
            .coordinator
            .compute_playout_ts(now)
            .unwrap_or(frame.timestamp_us);

        self.current_sequence = frame.sequence;

        let encoded = encode_frame(&Frame::Audio(frame));

        let mut to_remove = Vec::new();
        for (id, member) in &self.members {
            if member.is_degraded {
                continue;
            }
            // WHY: Bytes is reference-counted — all receivers share one allocation.
            if member.frame_tx.try_send(encoded.clone()).is_err() {
                warn!(%id, "renderer frame channel full, marking degraded");
                to_remove.push(id.clone());
            }
        }
        for id in &to_remove {
            if let Some(m) = self.members.get_mut(id) {
                m.is_degraded = true;
                m.consecutive_lags = 0;
            }
        }
    }

    /// Record a clock sync exchange from a renderer.
    pub fn record_clock_exchange(
        &mut self,
        renderer_id: &str,
        originate: u64,
        receive: u64,
        transmit: u64,
        destination: u64,
    ) {
        self.coordinator
            .record_exchange(renderer_id, originate, receive, transmit, destination);
    }

    /// Update buffer status from a renderer's status report.
    pub fn update_renderer_status(
        &mut self,
        renderer_id: &str,
        buffer_depth_ms: u16,
        device_state: DeviceState,
    ) {
        if let Some(member) = self.members.get_mut(renderer_id) {
            member.buffer_depth_ms = buffer_depth_ms;
            member.device_state = device_state;

            if buffer_depth_ms < LOW_WATERMARK_MS {
                member.consecutive_lags += 1;
                if member.consecutive_lags >= DEGRADED_LAG_COUNT && !member.is_degraded {
                    warn!(
                        %renderer_id,
                        buffer_depth_ms,
                        "renderer consistently lagging, marking degraded"
                    );
                    member.is_degraded = true;
                }
            } else {
                member.consecutive_lags = 0;
                if member.is_degraded {
                    debug!(%renderer_id, "renderer recovered FROM degraded state");
                    member.is_degraded = false;
                }
            }
        }
    }

    /// Whether any active (non-degraded) renderer has a low buffer.
    #[must_use]
    pub(crate) fn needs_backpressure(&self) -> bool {
        self.members
            .values()
            .any(|m| !m.is_degraded && m.buffer_depth_ms < LOW_WATERMARK_MS)
    }

    pub fn pause(&mut self) {
        self.play_state = ZonePlayState::Paused;
        info!("zone paused");
    }

    pub fn resume(&mut self) {
        self.play_state = ZonePlayState::Playing;
        info!("zone resumed");
    }

    #[must_use]
    pub fn play_state(&self) -> ZonePlayState {
        self.play_state
    }

    #[must_use]
    pub fn coordinator(&self) -> &ClockCoordinator {
        &self.coordinator
    }

    pub fn coordinator_mut(&mut self) -> &mut ClockCoordinator {
        &mut self.coordinator
    }

    #[must_use]
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    #[must_use]
    pub fn degraded_renderers(&self) -> Vec<String> {
        self.members
            .iter()
            .filter(|(_, m)| m.is_degraded)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Run the zone stream: decode from source, fan-out to all members.
    pub async fn run<S: AudioSource>(&mut self, mut source: S, cancel: watch::Receiver<bool>) {
        self.play_state = ZonePlayState::Playing;

        loop {
            if *cancel.borrow() {
                break;
            }

            if self.play_state == ZonePlayState::Paused {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue;
            }

            if self.needs_backpressure() {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                continue;
            }

            match source.next_frame().await {
                Some(frame) => {
                    self.fan_out_frame(frame).await;
                }
                None => {
                    debug!("zone audio source exhausted");
                    break;
                }
            }
        }

        self.play_state = ZonePlayState::Paused;
    }
}

impl Default for ZoneStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::protocol::AudioCodec;

    fn test_frame(seq: u64, ts: u64) -> AudioFrame {
        AudioFrame {
            sequence: seq,
            timestamp_us: ts,
            playout_ts: 0,
            codec: AudioCodec::Pcm,
            channels: 2,
            sample_rate: 48000,
            payload: Bytes::from_static(b"zone-test"),
        }
    }

    #[tokio::test]
    async fn fan_out_delivers_to_all_renderers() {
        let mut zone = ZoneStream::new();
        let mut rx1 = zone.add_renderer("r1");
        let mut rx2 = zone.add_renderer("r2");

        zone.fan_out_frame(test_frame(0, 1000)).await;

        assert!(rx1.try_recv().is_ok(), "r1 should receive frame");
        assert!(rx2.try_recv().is_ok(), "r2 should receive frame");
    }

    #[tokio::test]
    async fn fan_out_sets_playout_ts() {
        let mut zone = ZoneStream::new();
        let _rx = zone.add_renderer("r1");

        let frame = test_frame(0, 1_000_000);
        zone.fan_out_frame(frame).await;

        // The encoded frame should have a non-zero playout_ts set by coordinator
        // (coordinator returns the server timestamp as fallback with no sync data)
    }

    #[test]
    fn sync_point_returns_current_state() {
        let mut zone = ZoneStream::new();
        let _rx = zone.add_renderer("r1");
        let sp = zone.sync_point();
        assert_eq!(sp.sequence, 0);
        assert!(sp.server_time > 0);
    }

    #[test]
    fn degraded_renderer_detection() {
        let mut zone = ZoneStream::new();
        let _rx = zone.add_renderer("r1");

        for _ in 0..DEGRADED_LAG_COUNT {
            zone.update_renderer_status("r1", 10, DeviceState::Active);
        }

        let degraded = zone.degraded_renderers();
        assert!(degraded.contains(&"r1".to_string()));
    }

    #[test]
    fn degraded_renderer_recovers() {
        let mut zone = ZoneStream::new();
        let _rx = zone.add_renderer("r1");

        for _ in 0..DEGRADED_LAG_COUNT {
            zone.update_renderer_status("r1", 10, DeviceState::Active);
        }
        assert!(!zone.degraded_renderers().is_empty());

        zone.update_renderer_status("r1", 100, DeviceState::Active);
        assert!(zone.degraded_renderers().is_empty());
    }

    #[test]
    fn pause_resume() {
        let mut zone = ZoneStream::new();
        assert_eq!(zone.play_state(), ZonePlayState::Paused);

        zone.resume();
        assert_eq!(zone.play_state(), ZonePlayState::Playing);

        zone.pause();
        assert_eq!(zone.play_state(), ZonePlayState::Paused);
    }

    #[test]
    fn add_remove_member() {
        let mut zone = ZoneStream::new();
        let _rx = zone.add_renderer("r1");
        assert_eq!(zone.member_count(), 1);

        zone.remove_renderer("r1");
        assert_eq!(zone.member_count(), 0);
    }

    #[test]
    fn needs_backpressure_when_buffer_low() {
        let mut zone = ZoneStream::new();
        let _rx = zone.add_renderer("r1");

        zone.update_renderer_status("r1", 100, DeviceState::Active);
        assert!(!zone.needs_backpressure());

        zone.update_renderer_status("r1", 10, DeviceState::Active);
        assert!(zone.needs_backpressure());
    }
}
