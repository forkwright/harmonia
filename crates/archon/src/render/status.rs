// Status reporter: collects renderer metrics and produces status reports.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use super::protocol::{DeviceState, StatusReport};

pub struct StatusReporter {
    buffer_depth_ms: AtomicU64,
    latency_ms: AtomicU64,
    device_state: Mutex<DeviceState>,
    underrun_count: AtomicU64,
    /// Underrun count as of the last `report()` call, so a fresh climb can be
    /// detected against the previous report rather than the current value.
    last_reported_underrun_count: AtomicU64,
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
            // WHY: u64::MAX is the "no prior report" sentinel — the first report
            // establishes the baseline (current > MAX is always false) rather than
            // warning on a count that already existed before observation began.
            last_reported_underrun_count: AtomicU64::new(u64::MAX),
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
        // INVARIANT: compare against the PREVIOUS report's count, not another
        // load of the same atomic — two loads of `underrun_count` back-to-back
        // are equal by construction (nothing else writes it in between), which
        // silently defeated this warning regardless of how high the count climbed.
        let previous_underrun_count = self
            .last_reported_underrun_count
            .swap(underrun_count, Ordering::AcqRel);

        if latency_ms > 200.0 {
            tracing::warn!(latency_ms, "high renderer latency");
        }
        if underrun_count > previous_underrun_count {
            tracing::warn!(
                underrun_count,
                previous_underrun_count,
                "audio underruns detected"
            );
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

    // ── #553: underrun warning compares against the PREVIOUS report ────────

    #[derive(Clone)]
    struct VecWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for VecWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for VecWriter {
        type Writer = VecWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn captured_log(run: impl FnOnce()) -> String {
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer(VecWriter(std::sync::Arc::clone(&buf)))
            .without_time()
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, run);
        String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone())
            .expect("log output is valid utf8")
    }

    #[test]
    fn underrun_warning_does_not_fire_when_count_is_unchanged() {
        let reporter = StatusReporter::new();
        reporter.update_underrun_count(2);

        let log = captured_log(|| {
            let _ = reporter.report();
            let _ = reporter.report();
        });

        assert_eq!(
            log.matches("audio underruns detected").count(),
            0,
            "an unchanged count across reports must not warn"
        );
    }

    #[test]
    fn underrun_warning_fires_on_increase_since_last_report() {
        let reporter = StatusReporter::new();

        let log = captured_log(|| {
            let _ = reporter.report(); // count 0 -> 0, no warning
            reporter.update_underrun_count(3);
            let _ = reporter.report(); // count 0 -> 3, warns
            let _ = reporter.report(); // count 3 -> 3, no warning
        });

        assert_eq!(
            log.matches("audio underruns detected").count(),
            1,
            "the warning must fire exactly once, on the report where the count climbed"
        );
    }
}
