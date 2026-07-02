// Render pipeline: receives audio frames, applies local DSP, outputs via cpal.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use akouo_core::output::{AudioDataCallback, OutputBackend, OutputParams};
use akouo_core::signal_path::QualityTier;
use akouo_core::{DspConfig, DspPipeline, RingBuffer};
use tokio::sync::watch;
use tracing::{info, warn};

use super::config::RendererConfig;
use super::error::RenderError;
use super::protocol::AudioFrame;

/// Interval between ring-buffer push retries while the buffer is full.
const PUSH_RETRY_INTERVAL: Duration = Duration::from_millis(1);

/// Lower bound on the push deadline, guarding tiny rings and degenerate
/// (zero-rate) frame parameters against spurious timeouts.
const MIN_PUSH_DEADLINE_MS: u64 = 250;

pub struct RenderPipeline {
    dsp: DspPipeline,
    ring: Arc<RingBuffer>,
    ring_capacity: usize,
    backend: akouo_core::output::cpal::CpalOutputBackend,
    output_opened: bool,
    underrun_count: Arc<AtomicU64>,
    device_name: Option<String>,
    output_config: PipelineOutputConfig,
}

struct PipelineOutputConfig {
    exclusive_mode: bool,
    bit_depth: u32,
}

impl RenderPipeline {
    pub fn new(
        config: &RendererConfig,
        dsp_rx: watch::Receiver<DspConfig>,
    ) -> Result<Self, RenderError> {
        let dsp = DspPipeline::new(config.dsp_config(), dsp_rx);
        let ring_capacity = config.ring_buffer_capacity();
        let ring = Arc::new(RingBuffer::new(ring_capacity));
        let backend = akouo_core::output::cpal::CpalOutputBackend::new();
        let device_name = if config.output.device == "default" {
            None
        } else {
            Some(config.output.device.clone())
        };
        Ok(Self {
            dsp,
            ring,
            ring_capacity,
            backend,
            output_opened: false,
            underrun_count: Arc::new(AtomicU64::new(0)),
            device_name,
            output_config: PipelineOutputConfig {
                exclusive_mode: config.output.exclusive_mode,
                bit_depth: config.output.bit_depth,
            },
        })
    }

    pub async fn process_frame(&mut self, frame: AudioFrame) -> Result<(), RenderError> {
        if !self.output_opened {
            self.open_output(frame.sample_rate, frame.channels).await?;
            self.output_opened = true;
        }

        let mut samples = frame.samples;
        let _stage_metas = self
            .dsp
            .process_frame(&mut samples, frame.channels, frame.sample_rate);

        // Push to ring buffer with bounded backpressure.
        //
        // WHY: a wedged output callback (device unplugged, stream stalled)
        // leaves the ring full forever; an unbounded retry loop then blocks the
        // audio receive task silently. A deadline converts the stall into a
        // loud AudioOutput error so the runner's reconnect path engages.
        let deadline = self.push_deadline(frame.sample_rate, frame.channels);
        let ring = Arc::clone(&self.ring);
        let pushed = tokio::time::timeout(deadline, async move {
            loop {
                if ring.push_frame(&samples) {
                    break;
                }
                tokio::time::sleep(PUSH_RETRY_INTERVAL).await;
            }
        })
        .await;

        if pushed.is_err() {
            let stalled_ms = deadline.as_millis();
            tracing::error!(
                stalled_ms,
                "audio ring buffer stalled; output callback is not draining"
            );
            return Err(RenderError::AudioOutput {
                message: format!("ring buffer full for {stalled_ms} ms; audio output stalled"),
                location: snafu::location!(),
            });
        }

        Ok(())
    }

    /// Deadline for a single ring-buffer push.
    ///
    /// Twice the ring's real-time duration is the longest a healthy output
    /// callback can lag before space frees up; anything beyond that is a
    /// stalled consumer, not backpressure.
    fn push_deadline(&self, sample_rate: u32, channels: u16) -> Duration {
        let samples_per_ms = (u64::from(sample_rate) * u64::from(channels)) / 1000;
        let ring_ms = if samples_per_ms == 0 {
            0
        } else {
            self.ring_capacity as u64 / samples_per_ms
        };
        Duration::from_millis((ring_ms * 2).max(MIN_PUSH_DEADLINE_MS))
    }

    async fn open_output(&mut self, sample_rate: u32, channels: u16) -> Result<(), RenderError> {
        let ring_cb = Arc::clone(&self.ring);
        let underrun_cb = Arc::clone(&self.underrun_count);
        let callback: AudioDataCallback = Box::new(move |buf: &mut [f64]| {
            if !ring_cb.pop_frame(buf) {
                buf.fill(0.0);
                underrun_cb.fetch_add(1, Ordering::Relaxed);
            }
        });

        let params = OutputParams {
            sample_rate,
            channels,
            bit_depth: self.output_config.bit_depth,
            exclusive_mode: self.output_config.exclusive_mode,
            needs_resample: false,
            source_sample_rate: sample_rate,
            quality_tier: QualityTier::Lossless,
        };

        if let Ok(devices) = self.backend.available_devices() {
            info!(
                available_devices = devices.len(),
                requested = ?self.device_name,
                "enumerating audio output devices"
            );
            for d in &devices {
                info!(name = %d.name, is_default = d.is_default, "  device");
            }
        }

        let (stream_error_tx, mut stream_error_rx) =
            tokio::sync::mpsc::channel::<akouo_core::OutputError>(16);
        tokio::spawn(async move {
            while let Some(e) = stream_error_rx.recv().await {
                warn!("audio output stream error: {e}");
            }
        });

        self.backend
            .open(
                self.device_name.as_deref(),
                params,
                callback,
                stream_error_tx,
            )
            .await
            .map_err(|e| RenderError::AudioOutput {
                message: e.to_string(),
                location: snafu::location!(),
            })?;

        self.backend
            .start()
            .await
            .map_err(|e| RenderError::AudioOutput {
                message: e.to_string(),
                location: snafu::location!(),
            })?;

        info!(
            sample_rate,
            channels,
            device = ?self.device_name,
            "audio output opened"
        );
        Ok(())
    }

    /// Returns the approximate buffer depth in milliseconds.
    pub fn buffer_depth_ms(&self, sample_rate: u32, channels: u16) -> f64 {
        if sample_rate == 0 || channels == 0 {
            return 0.0;
        }
        let samples = self.ring.available_to_read();
        let frames = samples / usize::from(channels);
        (frames as f64 / sample_rate as f64) * 1000.0
    }

    pub fn underrun_count(&self) -> u64 {
        self.underrun_count.load(Ordering::Relaxed)
    }

    /// Drains remaining audio FROM the ring buffer before shutdown.
    pub async fn drain(&self) {
        let remaining = self.ring.available_to_read();
        if remaining > 0 {
            info!(remaining_samples = remaining, "draining audio buffer");
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    pub async fn close(&mut self) {
        if self.output_opened {
            if let Err(e) = self.backend.close().await {
                warn!(error = %e, "error closing audio output");
            }
            self.output_opened = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_depth_calculation() {
        let config = RendererConfig::default();
        let (_tx, rx) = watch::channel(config.dsp_config());
        let pipeline = RenderPipeline::new(&config, rx).unwrap();

        let depth = pipeline.buffer_depth_ms(44100, 2);
        assert!((depth - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pipeline_reports_zero_underruns_initially() {
        let config = RendererConfig::default();
        let (_tx, rx) = watch::channel(config.dsp_config());
        let pipeline = RenderPipeline::new(&config, rx).unwrap();
        assert_eq!(pipeline.underrun_count(), 0);
    }

    // ── #410: the push loop is deadline-bounded, not an unbounded retry ────

    fn test_pipeline_with_open_output() -> RenderPipeline {
        let config = RendererConfig::default();
        let (_tx, rx) = watch::channel(config.dsp_config());
        let mut pipeline = RenderPipeline::new(&config, rx).unwrap();
        // WHY: marking the output opened skips open_output so the test never
        // touches a real audio device; the ring then has no consumer, which is
        // exactly the wedged-callback scenario under test.
        pipeline.output_opened = true;
        pipeline
    }

    fn test_frame(samples: usize) -> AudioFrame {
        AudioFrame {
            sample_rate: 44100,
            channels: 2,
            timestamp: 0,
            samples: vec![0.25; samples],
        }
    }

    #[tokio::test(start_paused = true)]
    async fn process_frame_errors_when_ring_stalls() {
        let mut pipeline = test_pipeline_with_open_output();

        // Fill the ring completely; no consumer ever pops.
        while pipeline.ring.push_frame(&[0.5; 1024]) {}

        let result = pipeline.process_frame(test_frame(1024)).await;

        assert!(
            matches!(result, Err(RenderError::AudioOutput { .. })),
            "a stalled ring must surface as AudioOutput, got: {result:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn process_frame_pushes_when_ring_has_space() {
        let mut pipeline = test_pipeline_with_open_output();

        pipeline
            .process_frame(test_frame(1024))
            .await
            .expect("a frame that fits must push without timing out");
        assert_eq!(pipeline.ring.available_to_read(), 1024);
    }

    #[test]
    fn push_deadline_scales_with_ring_duration_and_has_floor() {
        let config = RendererConfig::default();
        let (_tx, rx) = watch::channel(config.dsp_config());
        let pipeline = RenderPipeline::new(&config, rx).unwrap();

        // Degenerate parameters fall back to the floor instead of dividing by zero.
        assert_eq!(
            pipeline.push_deadline(0, 0),
            Duration::from_millis(MIN_PUSH_DEADLINE_MS)
        );

        // At real parameters the deadline is 2x the ring's duration, never
        // below the floor.
        let deadline = pipeline.push_deadline(44100, 2);
        let ring_ms = pipeline.ring_capacity as u64 / ((44100 * 2) / 1000);
        assert_eq!(
            deadline,
            Duration::from_millis((ring_ms * 2).max(MIN_PUSH_DEADLINE_MS))
        );
    }
}
