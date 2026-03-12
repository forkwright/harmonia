// Public API surface for akroasis-core.

pub mod config;
pub mod decode;
pub mod dsp;
pub mod engine;
pub mod error;
pub mod gapless;
pub mod output;
pub mod queue;
pub(crate) mod ring_buffer;
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

// Engine
pub use engine::{AudioSource, Engine, EngineEvent};

// Queue
pub use queue::PlayQueue;

// Errors
pub use error::{DecodeError, DspError, EngineError, OutputError};

// Output
pub use output::format::Quantization;
pub use output::resample::Resampler;
pub use output::{DeviceCapabilities, OutputBackend, OutputDevice, OutputParams};

// Signal path
pub use signal_path::tier::{propagate_tier, source_tier};
pub use signal_path::{
    OutputInfo, QualityTier, SignalPathSnapshot, SignalStageInfo, SourceInfo, StageParams,
};

// DSP pipeline (for embedding in custom engine implementations)
pub use dsp::{DspPipeline, DspStage, StageResult};

// Gapless
pub use gapless::prebuffer::PreBuffer;
pub use gapless::{
    AlbumDetector, CarryBuffer, GaplessScheduler, TransitionMode, TrimPosition, crossfade_frame,
    trim_codec_delay,
};
