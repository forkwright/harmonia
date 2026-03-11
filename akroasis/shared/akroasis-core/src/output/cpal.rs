// P1-11: cpal audio output backend.

use crate::error::OutputError;
use crate::output::{DeviceCapabilities, OutputBackend, OutputDevice, OutputParams};

/// cpal-backed output: works on Linux (ALSA/PulseAudio/PipeWire), macOS, and Windows.
pub struct CpalOutputBackend {
    _private: (),
}

impl CpalOutputBackend {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for CpalOutputBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputBackend for CpalOutputBackend {
    fn available_devices(&self) -> Result<Vec<OutputDevice>, OutputError> {
        todo!("P1-11: enumerate cpal output devices")
    }

    fn device_capabilities(
        &self,
        _device_id: Option<&str>,
    ) -> Result<DeviceCapabilities, OutputError> {
        todo!("P1-11: query supported configs from cpal SupportedOutputConfigs")
    }

    async fn open(
        &mut self,
        _device_id: Option<&str>,
        _params: OutputParams,
        _data_callback: Box<dyn FnMut(&mut [f64]) + Send + 'static>,
    ) -> Result<(), OutputError> {
        todo!("P1-11: build cpal stream with f64 SampleFormat; wire data_callback")
    }

    async fn start(&mut self) -> Result<(), OutputError> {
        todo!("P1-11: call cpal Stream::play()")
    }

    async fn pause(&mut self) -> Result<(), OutputError> {
        todo!("P1-11: call cpal Stream::pause()")
    }

    async fn close(&mut self) -> Result<(), OutputError> {
        todo!("P1-11: drop cpal Stream, release device handle")
    }
}
