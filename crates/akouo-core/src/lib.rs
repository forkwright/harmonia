// Public API surface for akouo-core.

pub mod config;
pub mod decode;
pub mod dsp;
pub mod engine;
pub mod error;
pub mod gapless;
pub mod output;
pub mod queue;
pub mod ring_buffer;
pub mod signal_path;

// Config
pub use config::{
    BufferSize, CompressorConfig, ConvolutionConfig, CrossfeedConfig, DspConfig, EngineConfig,
    EqBand, EqConfig, OutputConfig, ReplayGainConfig, ReplayGainMode, SkipSilenceConfig,
    VolumeConfig,
};
// Decode types
pub use decode::metadata::TrackMetadata;
pub use decode::{AudioDecoder, Codec, DecodedFrame, GaplessInfo, StreamParams};
// DSP pipeline (for embedding in custom engine implementations)
pub use dsp::{DspPipeline, DspStage, StageResult};
// Engine
pub use engine::{AudioSource, Engine, EngineEvent};
// Errors
pub use error::{DecodeError, DspError, EngineError, OutputError};
// Gapless
pub use gapless::prebuffer::PreBuffer;
pub use gapless::{
    AlbumDetector, CarryBuffer, GaplessScheduler, TransitionMode, TrimPosition, crossfade_frame,
    trim_codec_delay,
};
// Output
pub use output::format::Quantization;
pub use output::resample::Resampler;
pub use output::{DeviceCapabilities, OutputBackend, OutputDevice, OutputParams};
// Queue
pub use queue::PlayQueue;
// Ring buffer (for custom engine/renderer implementations)
pub use ring_buffer::RingBuffer;
// Signal path
pub use signal_path::tier::{propagate_tier, source_tier};
pub use signal_path::{
    OutputInfo, QualityTier, SignalPathSnapshot, SignalStageInfo, SourceInfo, StageParams,
};
