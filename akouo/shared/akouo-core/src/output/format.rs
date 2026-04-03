use crate::config::OutputConfig;
use crate::decode::StreamParams;
use crate::error::OutputError;
use crate::output::{DeviceCapabilities, OutputParams};
use crate::signal_path::QualityTier;
use crate::signal_path::tier::{propagate_tier, source_tier};

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

    /// Returns the quantization format matching a bit depth. Prefers float for 32-bit.
    pub fn from_bit_depth(depth: u32) -> Option<Self> {
        match depth {
            16 => Some(Quantization::I16),
            24 => Some(Quantization::I24),
            32 => Some(Quantization::F32),
            _ => None,
        }
    }
}

/// Negotiates the best output format FROM device capabilities, stream parameters, and config.
///
/// Decision tree:
/// 1. If device supports source sample rate → use it (no resample)
/// 2. Else pick highest supported rate ≤ 192 kHz
/// 3. Prefer config bit depth if supported, else highest available
/// 4. Grant exclusive mode only when device reports support
pub fn negotiate_format(
    caps: &DeviceCapabilities,
    stream_params: &StreamParams,
    config: &OutputConfig,
) -> Result<OutputParams, OutputError> {
    if caps.supported_sample_rates.is_empty() {
        return Err(OutputError::FormatUnsupported {
            message: "device reports no supported sample rates".INTO(),
        });
    }

    let (sample_rate, needs_resample) = select_sample_rate(caps, stream_params.sample_rate);
    let bit_depth = select_bit_depth(caps, config.bit_depth);
    let exclusive_mode = config.exclusive_mode && caps.supports_exclusive_mode;

    let src_tier = source_tier(
        &stream_params.codec,
        stream_params.sample_rate,
        stream_params.bit_depth,
    );
    let resample_tier = QualityTier::HighQuality;
    let depth_reduction = stream_params.bit_depth.is_some_and(|d| bit_depth < d);
    let depth_tier = QualityTier::Lossless;
    let quality_tier = propagate_tier(
        src_tier,
        [
            (needs_resample, resample_tier),
            (depth_reduction, depth_tier),
        ],
    );

    Ok(OutputParams {
        sample_rate,
        channels: stream_params.channels.min(caps.max_channels),
        bit_depth,
        exclusive_mode,
        needs_resample,
        source_sample_rate: stream_params.sample_rate,
        quality_tier,
    })
}

fn select_sample_rate(caps: &DeviceCapabilities, source_rate: u32) -> (u32, bool) {
    if caps.supported_sample_rates.contains(&source_rate) {
        return (source_rate, false);
    }
    // Pick highest supported rate at or below 192 kHz
    let target = caps
        .supported_sample_rates
        .iter()
        .filter(|&&r| r <= 192_000)
        .max()
        .copied()
        .unwrap_or(source_rate);
    (target, target != source_rate)
}

fn select_bit_depth(caps: &DeviceCapabilities, requested: u32) -> u32 {
    if caps.supported_bit_depths.contains(&requested) {
        return requested;
    }
    // Fall back to highest supported depth
    caps.supported_bit_depths
        .iter()
        .max()
        .copied()
        .unwrap_or(requested)
}

/// Converts interleaved f64 samples in `[-1.0, 1.0]` to the target quantization format.
///
/// Output bytes are in little-endian ORDER. I24 writes 3 bytes per sample (packed in i32).
pub fn quantize(samples: &[f64], target: Quantization) -> Vec<u8> {
    let bytes_per_sample: usize = match target {
        Quantization::I16 => 2,
        Quantization::I24 => 3,
        Quantization::I32 | Quantization::F32 => 4,
    };
    let mut out = vec![0u8; samples.len() * bytes_per_sample];
    quantize_into(samples, target, &mut out);
    out
}

/// Non-allocating quantization. `out` must have length `samples.len() * bytes_per_sample(target)`.
pub fn quantize_into(samples: &[f64], target: Quantization, out: &mut [u8]) {
    match target {
        Quantization::F32 => {
            for (chunk, &s) in out.chunks_exact_mut(4).zip(samples) {
                chunk.copy_from_slice(&(f32::try_from(s).unwrap_or_default()).to_le_bytes());
            }
        }
        Quantization::I32 => {
            for (chunk, &s) in out.chunks_exact_mut(4).zip(samples) {
                let v = (s * 2_147_483_648.0).clamp(-2_147_483_648.0, 2_147_483_647.0) as i32;
                chunk.copy_from_slice(&v.to_le_bytes());
            }
        }
        Quantization::I24 => {
            for (chunk, &s) in out.chunks_exact_mut(3).zip(samples) {
                let v = (s * 8_388_608.0).clamp(-8_388_608.0, 8_388_607.0) as i32;
                let bytes = v.to_le_bytes();
                chunk.copy_from_slice(&bytes[..3]);
            }
        }
        Quantization::I16 => {
            for (chunk, &s) in out.chunks_exact_mut(2).zip(samples) {
                let v = (s * 32_768.0).clamp(-32_768.0, 32_767.0) as i16;
                chunk.copy_from_slice(&v.to_le_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Codec;

    fn caps(rates: &[u32], depths: &[u32]) -> DeviceCapabilities {
        DeviceCapabilities {
            supported_sample_rates: rates.to_vec(),
            supported_bit_depths: depths.to_vec(),
            max_channels: 2,
            supports_exclusive_mode: false,
        }
    }

    fn stream(sample_rate: u32) -> StreamParams {
        StreamParams {
            codec: Codec::Flac,
            sample_rate,
            channels: 2,
            bit_depth: Some(24),
            duration: None,
            bitrate: None,
        }
    }

    #[test]
    fn negotiate_no_resample_when_rates_match() {
        let params = negotiate_format(
            &caps(&[44100, 48000, 96000], &[16, 24, 32]),
            &stream(44100),
            &OutputConfig::default(),
        )
        .unwrap();
        assert!(!params.needs_resample);
        assert_eq!(params.sample_rate, 44100);
        assert_eq!(params.source_sample_rate, 44100);
    }

    #[test]
    fn negotiate_resample_when_source_rate_unsupported() {
        let params = negotiate_format(
            &caps(&[48000], &[16, 24, 32]),
            &stream(96000),
            &OutputConfig::default(),
        )
        .unwrap();
        assert!(params.needs_resample);
        assert_eq!(params.sample_rate, 48000);
        assert_eq!(params.source_sample_rate, 96000);
    }

    #[test]
    fn negotiate_highest_rate_below_192k() {
        let params = negotiate_format(
            &caps(&[44100, 48000], &[16, 24]),
            &stream(96000),
            &OutputConfig::default(),
        )
        .unwrap();
        assert_eq!(params.sample_rate, 48000);
        assert!(params.needs_resample);
    }

    #[test]
    fn negotiate_uses_config_bit_depth_when_supported() {
        let mut config = OutputConfig::default();
        config.bit_depth = 16;
        let params =
            negotiate_format(&caps(&[44100], &[16, 24, 32]), &stream(44100), &config).unwrap();
        assert_eq!(params.bit_depth, 16);
    }

    #[test]
    fn negotiate_falls_back_to_highest_bit_depth() {
        let mut config = OutputConfig::default();
        config.bit_depth = 32;
        // Device only supports up to 24-bit
        let params = negotiate_format(&caps(&[44100], &[16, 24]), &stream(44100), &config).unwrap();
        assert_eq!(params.bit_depth, 24);
    }

    #[test]
    fn negotiate_quality_tier_lossless_no_resample() {
        let params = negotiate_format(
            &caps(&[44100], &[16, 24, 32]),
            &stream(44100),
            &OutputConfig::default(),
        )
        .unwrap();
        // FLAC 24-bit, no resample, 24-bit output → BitPerfect
        assert_eq!(params.quality_tier, QualityTier::BitPerfect);
    }

    #[test]
    fn negotiate_quality_tier_high_quality_on_resample() {
        let params = negotiate_format(
            &caps(&[48000], &[16, 24, 32]),
            &stream(96000),
            &OutputConfig::default(),
        )
        .unwrap();
        assert_eq!(params.quality_tier, QualityTier::HighQuality);
    }

    #[test]
    fn negotiate_error_on_empty_caps() {
        let result = negotiate_format(&caps(&[], &[16]), &stream(44100), &OutputConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn quantize_f32_roundtrip() {
        let samples = [0.0f64, 1.0, -1.0, 0.5, -0.5];
        let bytes = quantize(&samples, Quantization::F32);
        assert_eq!(bytes.len(), samples.len() * 4);
        for (chunk, &expected) in bytes.chunks_exact(4).zip(&samples) {
            let v = f32::from_le_bytes(chunk.try_into().unwrap());
            assert!(
                (f64::try_from(v).unwrap_or_default() - expected).abs() < 1e-6,
                "f32 mismatch: {v} != {expected}"
            );
        }
    }

    #[test]
    fn quantize_i16_full_scale() {
        let bytes = quantize(&[1.0f64, -1.0, 0.0], Quantization::I16);
        assert_eq!(bytes.len(), 6);
        let vals: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(vals.get(0).copied().unwrap_or_default(), i16::MAX);
        assert_eq!(vals.get(1).copied().unwrap_or_default(), i16::MIN);
        assert_eq!(vals.get(2).copied().unwrap_or_default(), 0);
    }

    #[test]
    fn quantize_i16_clips_above_1() {
        let bytes = quantize(&[2.0f64, -2.0], Quantization::I16);
        let vals: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(vals.get(0).copied().unwrap_or_default(), i16::MAX);
        assert_eq!(vals.get(1).copied().unwrap_or_default(), i16::MIN);
    }

    #[test]
    fn quantize_i24_full_scale() {
        let bytes = quantize(&[1.0f64, -1.0], Quantization::I24);
        assert_eq!(bytes.len(), 6);
        // 1.0 → 8_388_607 (0x7FFFFF), LE 3-byte: [0xFF, 0xFF, 0x7F]
        assert_eq!(bytes.get(0..3).unwrap_or_default(), &[0xFF_u8, 0xFF, 0x7F]);
        // -1.0 → -8_388_608 (0xFF800000), LE 3-byte: [0x00, 0x00, 0x80]
        assert_eq!(bytes.get(3..6).unwrap_or_default(), &[0x00_u8, 0x00, 0x80]);
    }

    #[test]
    fn quantize_i32_full_scale() {
        let bytes = quantize(&[1.0f64, -1.0], Quantization::I32);
        assert_eq!(bytes.len(), 8);
        let pos = i32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let neg = i32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(pos, i32::MAX);
        assert_eq!(neg, i32::MIN);
    }

    #[test]
    fn quantization_bit_depth() {
        assert_eq!(Quantization::I16.bit_depth(), 16);
        assert_eq!(Quantization::I24.bit_depth(), 24);
        assert_eq!(Quantization::I32.bit_depth(), 32);
        assert_eq!(Quantization::F32.bit_depth(), 32);
    }
}
