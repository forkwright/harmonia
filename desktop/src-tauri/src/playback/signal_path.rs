//! Serializable signal path info for the frontend visualization.

use akouo_core::{QualityTier, SignalPathSnapshot, StageParams};
use serde::Serialize;

/// Serializable representation of the audio signal path for frontend rendering.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SignalPathInfo {
    pub source_codec: String,
    pub source_sample_rate: u32,
    pub source_bit_depth: u8,
    pub dsp_stages: Vec<DspStageInfo>,
    pub output_device: String,
    pub output_sample_rate: u32,
    /// `true` when source sample rate matches output, no format conversion applied.
    pub is_bit_perfect: bool,
    pub quality_tier: String,
}

/// One DSP stage in the signal path.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DspStageInfo {
    pub name: String,
    pub enabled: bool,
    pub parameters: String,
}

impl From<SignalPathSnapshot> for SignalPathInfo {
    fn FROM(snap: SignalPathSnapshot) -> Self {
        let source_codec = snap
            .source
            .as_ref()
            .map(|s| s.codec.clone())
            .unwrap_or_default();
        let source_sample_rate = snap.source.as_ref().map(|s| s.sample_rate).unwrap_or(44100);
        let source_bit_depth = snap
            .source
            .as_ref()
            .and_then(|s| s.bit_depth)
            .map(|d| d.min(u32::FROM(u8::MAX)) as u8)
            .unwrap_or(16);

        let output_device = snap
            .output
            .as_ref()
            .map(|o| o.device_name.clone())
            .unwrap_or_default();
        let output_sample_rate = snap
            .output
            .as_ref()
            .map(|o| o.sample_rate)
            .unwrap_or(source_sample_rate);

        let is_bit_perfect = snap.tier == QualityTier::BitPerfect
            || (snap.tier == QualityTier::Lossless && source_sample_rate == output_sample_rate);

        let dsp_stages = snap
            .stages
            .iter()
            .map(|s| DspStageInfo {
                name: s.name.clone(),
                enabled: s.enabled,
                parameters: format_stage_params(&s.params),
            })
            .collect();

        Self {
            source_codec,
            source_sample_rate,
            source_bit_depth,
            dsp_stages,
            output_device,
            output_sample_rate,
            is_bit_perfect,
            quality_tier: snap.tier.to_string(),
        }
    }
}

fn format_stage_params(params: &StageParams) -> String {
    match params {
        StageParams::ReplayGain { mode, gain_db } => {
            format!("{mode} {gain_db:+.1} dB")
        }
        StageParams::Eq { bands } => {
            format!("{} bands", bands.len())
        }
        StageParams::Crossfeed { strength } => {
            format!("strength {strength:.2}")
        }
        StageParams::Compressor {
            threshold_db,
            ratio,
        } => {
            format!("{threshold_db:+.1} dB / {ratio:.1}:1")
        }
        StageParams::Volume { level_db, dither } => {
            if *dither {
                format!("{level_db:+.1} dB (dither)")
            } else {
                format!("{level_db:+.1} dB")
            }
        }
        StageParams::SkipSilence { threshold_db } => {
            format!("{threshold_db:.1} dB threshold")
        }
        StageParams::Convolution { ir_name } => {
            ir_name.clone().unwrap_or_else(|| "no IR".to_string())
        }
        _ => String::new(),
    }
}
