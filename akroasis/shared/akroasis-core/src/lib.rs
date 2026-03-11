// Public API surface for akroasis-core.

pub mod config;
pub mod decode;
pub mod dsp;
pub mod engine;
pub mod error;
pub mod gapless;
pub mod output;
pub mod signal_path;
pub(crate) mod ring_buffer;

// Config
pub use config::{
    BufferSize, CompressorConfig, ConvolutionConfig, CrossfeedConfig, DspConfig, EqBand, EqConfig,
    EngineConfig, OutputConfig, ReplayGainConfig, ReplayGainMode, SkipSilenceConfig, VolumeConfig,
};

// Decode types
pub use decode::{AudioDecoder, Codec, DecodedFrame, GaplessInfo, StreamParams};

// Engine
pub use engine::{AudioSource, Engine, EngineEvent};

// Errors
pub use error::{DecodeError, DspError, EngineError, OutputError};

// Output
pub use output::{DeviceCapabilities, OutputBackend, OutputDevice, OutputParams};

// Signal path
pub use signal_path::{
    OutputInfo, QualityTier, SignalPathSnapshot, SignalStageInfo, SourceInfo, StageParams,
};
pub use signal_path::tier::{propagate_tier, source_tier};

// DSP pipeline (for embedding in custom engine implementations)
pub use dsp::{DspPipeline, DspStage, StageResult};

// Gapless
pub use gapless::{GaplessScheduler, TransitionMode};
