//! Behavioral tuning knobs for syndesis.
//!
//! Every field here was previously a hardcoded `const` scattered across the
//! crate. Centralising them in `SyndesisConfig` lets operators tune the
//! transport + clock subsystem without code changes and lets agents discover
//! the parameter surface through serde.
//!
//! Defaults preserve historical behaviour exactly. See [`Default`] impls for
//! each sub-config. Only tuning knobs live here — protocol-spec constants
//! (wire format sizes, frame type opcodes) stay hardcoded next to the code
//! that relies on them.
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Top-level syndesis tuning config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SyndesisConfig {
    pub clock: ClockConfig,
    pub client: ClientConfig,
    pub server: ServerConfig,
}

/// Clock estimation + sync-scheduler tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    /// Sliding window size (number of NTP samples retained) for the estimator.
    pub window_size: usize,
    /// Samples with RTT greater than `median_rtt * outlier_factor` are
    /// excluded from the offset estimate.
    pub outlier_factor: u64,
    /// Absolute offset (microseconds) above which a warning is logged.
    pub warn_offset_us: i64,
    /// Absolute offset (microseconds) above which an error is logged.
    pub error_offset_us: i64,
    /// Interval used while the estimator is converging or after a drift
    /// re-acceleration.
    pub initial_interval_secs: u64,
    /// Interval the scheduler backs off to once the estimator is stable.
    pub stable_interval_secs: u64,
    /// Offset change (microseconds) between stable probes that forces a
    /// return to `initial_interval`.
    pub drift_threshold_us: i64,
    /// Extra jitter margin (microseconds) added to the worst-case per-zone
    /// offset when computing zone-wide playout timestamps.
    pub buffer_margin_us: i64,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            window_size: 50,
            outlier_factor: 2,
            warn_offset_us: 5_000,
            error_offset_us: 20_000,
            initial_interval_secs: 5,
            stable_interval_secs: 30,
            drift_threshold_us: 500,
            buffer_margin_us: 10_000,
        }
    }
}

impl ClockConfig {
    #[must_use]
    pub fn initial_interval(&self) -> Duration {
        Duration::from_secs(self.initial_interval_secs)
    }

    #[must_use]
    pub fn stable_interval(&self) -> Duration {
        Duration::from_secs(self.stable_interval_secs)
    }
}

/// Client-side tuning: jitter buffer and status reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientConfig {
    /// Default jitter-buffer depth (milliseconds). Governs playout latency.
    pub jitter_buffer_depth_ms: u64,
    /// How often the client sends a `StatusReport` datagram to the server.
    pub status_report_interval_ms: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            jitter_buffer_depth_ms: 100,
            status_report_interval_ms: 1000,
        }
    }
}

/// Server-side tuning: flow control + zone fan-out.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Per-renderer encoded-frame mpsc channel capacity in a zone fan-out.
    pub frame_channel_capacity: usize,
    /// High watermark (ms) above which the server pauses the stream.
    pub buffer_high_watermark_ms: u16,
    /// Low watermark (ms) below which the server resumes a paused stream.
    pub buffer_low_watermark_ms: u16,
    /// Zone low watermark (ms) for declaring a renderer behind.
    pub zone_low_watermark_ms: u16,
    /// Consecutive low-buffer status reports before a renderer is marked
    /// degraded.
    pub degraded_lag_count: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            frame_channel_capacity: 256,
            buffer_high_watermark_ms: 200,
            buffer_low_watermark_ms: 80,
            zone_low_watermark_ms: 50,
            degraded_lag_count: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preserves_legacy_values() {
        let c = SyndesisConfig::default();
        assert_eq!(c.clock.window_size, 50);
        assert_eq!(c.clock.outlier_factor, 2);
        assert_eq!(c.clock.warn_offset_us, 5_000);
        assert_eq!(c.clock.error_offset_us, 20_000);
        assert_eq!(c.clock.initial_interval(), Duration::from_secs(5));
        assert_eq!(c.clock.stable_interval(), Duration::from_secs(30));
        assert_eq!(c.clock.drift_threshold_us, 500);
        assert_eq!(c.clock.buffer_margin_us, 10_000);
        assert_eq!(c.client.jitter_buffer_depth_ms, 100);
        assert_eq!(c.client.status_report_interval_ms, 1000);
        assert_eq!(c.server.frame_channel_capacity, 256);
        assert_eq!(c.server.buffer_high_watermark_ms, 200);
        assert_eq!(c.server.buffer_low_watermark_ms, 80);
        assert_eq!(c.server.zone_low_watermark_ms, 50);
        assert_eq!(c.server.degraded_lag_count, 10);
    }

    #[test]
    fn serde_round_trip() {
        let original = SyndesisConfig {
            clock: ClockConfig {
                window_size: 77,
                outlier_factor: 3,
                warn_offset_us: 1_234,
                error_offset_us: 9_876,
                initial_interval_secs: 7,
                stable_interval_secs: 42,
                drift_threshold_us: 321,
                buffer_margin_us: 11_000,
            },
            client: ClientConfig {
                jitter_buffer_depth_ms: 250,
                status_report_interval_ms: 500,
            },
            server: ServerConfig {
                frame_channel_capacity: 1024,
                buffer_high_watermark_ms: 300,
                buffer_low_watermark_ms: 60,
                zone_low_watermark_ms: 40,
                degraded_lag_count: 5,
            },
        };
        let toml = toml::to_string(&original).expect("serialize");
        let parsed: SyndesisConfig = toml::from_str(&toml).expect("deserialize");
        assert_eq!(parsed.clock.window_size, 77);
        assert_eq!(parsed.clock.outlier_factor, 3);
        assert_eq!(parsed.clock.initial_interval_secs, 7);
        assert_eq!(parsed.client.jitter_buffer_depth_ms, 250);
        assert_eq!(parsed.server.frame_channel_capacity, 1024);
        assert_eq!(parsed.server.zone_low_watermark_ms, 40);
    }

    #[test]
    fn partial_toml_uses_defaults() {
        let toml = r#"
            [clock]
            window_size = 99
        "#;
        let parsed: SyndesisConfig = toml::from_str(toml).expect("deserialize partial");
        assert_eq!(parsed.clock.window_size, 99);
        // Other clock fields keep defaults.
        assert_eq!(parsed.clock.outlier_factor, 2);
        // Other sections keep full defaults.
        assert_eq!(parsed.client.jitter_buffer_depth_ms, 100);
        assert_eq!(parsed.server.frame_channel_capacity, 256);
    }
}
