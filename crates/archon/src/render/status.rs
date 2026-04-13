// Status reporter: collects renderer metrics and produces status reports.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use super::protocol::{DeviceState, StatusReport};

pub struct StatusReporter {
    buffer_depth_ms: AtomicU64,
    latency_ms: AtomicU64,
    device_state: Mutex<DeviceState>,
    underrun_count: AtomicU64,
}

impl Default for StatusReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusReporter {
    pub fn new() -> Self {
        Self {
            buffer_depth_ms: AtomicU64::new(0.0f64.to_bits()),
            latency_ms: AtomicU64::new(0.0f64.to_bits()),
            device_state: Mutex::new(DeviceState::Opening),
            underrun_count: AtomicU64::new(0),
        }
    }

    pub fn update_buffer_depth(&self, depth_ms: f64) {
        self.buffer_depth_ms
            .store(depth_ms.to_bits(), Ordering::Release);
    }

    pub fn update_latency(&self, latency_ms: f64) {
        self.latency_ms
            .store(latency_ms.to_bits(), Ordering::Release);
    }

    pub fn set_device_state(&self, state: DeviceState) {
        let mut guard = self.device_state.lock().unwrap_or_else(|e| e.into_inner());
        *guard = state;
    }

    pub fn update_underrun_count(&self, count: u64) {
        self.underrun_count.store(count, Ordering::Release);
    }

    pub fn report(&self) -> StatusReport {
        let buffer_depth_ms = f64::from_bits(self.buffer_depth_ms.load(Ordering::Acquire));
        let latency_ms = f64::from_bits(self.latency_ms.load(Ordering::Acquire));
        let device_state = self
            .device_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let underrun_count = self.underrun_count.load(Ordering::Acquire);

        if latency_ms > 200.0 {
            tracing::warn!(latency_ms, "high renderer latency");
        }
        if underrun_count > 0 && underrun_count != self.underrun_count.load(Ordering::Relaxed) {
            tracing::warn!(underrun_count, "audio underruns detected");
        }

        StatusReport {
            buffer_depth_ms,
            latency_ms,
            device_state,
            underrun_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_report_has_zero_values() {
        let reporter = StatusReporter::new();
        let report = reporter.report();
        assert!((report.buffer_depth_ms - 0.0).abs() < f64::EPSILON);
        assert!((report.latency_ms - 0.0).abs() < f64::EPSILON);
        assert_eq!(report.device_state, DeviceState::Opening);
        assert_eq!(report.underrun_count, 0);
    }

    #[test]
    fn updates_are_reflected_in_report() {
        let reporter = StatusReporter::new();
        reporter.update_buffer_depth(95.0);
        reporter.update_latency(42.5);
        reporter.set_device_state(DeviceState::Playing);
        reporter.update_underrun_count(3);

        let report = reporter.report();
        assert!((report.buffer_depth_ms - 95.0).abs() < f64::EPSILON);
        assert!((report.latency_ms - 42.5).abs() < f64::EPSILON);
        assert_eq!(report.device_state, DeviceState::Playing);
        assert_eq!(report.underrun_count, 3);
    }

    #[test]
    fn device_state_transitions() {
        let reporter = StatusReporter::new();

        reporter.set_device_state(DeviceState::Playing);
        assert_eq!(reporter.report().device_state, DeviceState::Playing);

        reporter.set_device_state(DeviceState::Stopped);
        assert_eq!(reporter.report().device_state, DeviceState::Stopped);

        reporter.set_device_state(DeviceState::Error("test".into()));
        assert_eq!(
            reporter.report().device_state,
            DeviceState::Error("test".into())
        );
    }
}
