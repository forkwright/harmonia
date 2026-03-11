// P1-11: Output format negotiation and quantization.

use crate::error::OutputError;
use crate::output::{DeviceCapabilities, OutputParams};

/// Quantization target for the output stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quantization {
    /// 16-bit signed integer (CD quality).
    I16,
    /// 24-bit signed integer packed in 32-bit (most DACs).
    I24,
    /// 32-bit signed integer.
    I32,
    /// 32-bit float.
    F32,
}

impl Quantization {
    pub fn bit_depth(self) -> u32 {
        match self {
            Quantization::I16 => 16,
            Quantization::I24 => 24,
            Quantization::I32 | Quantization::F32 => 32,
        }
    }
}

/// Negotiates the best output format from a device's capabilities and requested params.
pub fn negotiate_format(
    _caps: &DeviceCapabilities,
    _requested: &OutputParams,
) -> Result<OutputParams, OutputError> {
    todo!("P1-11: prefer requested sample rate/bit depth; fall back to nearest supported")
}

/// Converts an interleaved f64 buffer to the target quantization format.
/// Used in the output callback to convert from the internal f64 pipeline.
pub fn quantize(_samples: &[f64], _target: Quantization) -> Vec<u8> {
    todo!("P1-11: scale [-1.0, 1.0] f64 to target integer range with clipping")
}
