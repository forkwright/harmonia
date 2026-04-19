// Render pipeline: receives audio frames, applies local DSP, outputs via cpal.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use akouo_core::output::{AudioDataCallback, OutputBackend, OutputParams};
use akouo_core::signal_path::QualityTier;
use akouo_core::{DspConfig, DspPipeline, RingBuffer};
use tokio::sync::watch;
use tracing::{info, warn};

use super::config::RendererConfig;
use super::error::RenderError;
use super::protocol::AudioFrame;

pub struct RenderPipeline {
    dsp: DspPipeline,
    ring: Arc<RingBuffer>,
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
        let ring = Arc::new(RingBuffer::new(config.ring_buffer_capacity()));
        let backend = akouo_core::output::cpal::CpalOutputBackend::new();
        let device_name = if config.output.device == "default" {
            None
        } else {
            Some(config.output.device.clone())
        };
        Ok(Self {
            dsp,
            ring,
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

        // Push to ring buffer with yield-based backpressure.
        loop {
            if self.ring.push_frame(&samples) {
                break;
            }
            tokio::task::yield_now().await;
        }

        Ok(())
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

        self.backend
            .open(self.device_name.as_deref(), params, callback)
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
}
