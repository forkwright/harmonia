use crate::decode::Codec;

/// Quality tier for the audio signal path, from lowest to highest.
///
/// The `Ord` ordering is ascending quality: `Degraded < Standard < HighQuality < Lossless < BitPerfect`.
/// Tier propagation takes the `min` across all active stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum QualityTier {
    /// Significant processing artifacts or lossy re-encoding applied.
    Degraded = 0,
    /// Standard lossy source (MP3, AAC < 192 kbps) with no additional degradation.
    Standard = 1,
    /// High-quality lossy source (Opus, Vorbis, AAC ≥ 192 kbps) or resampled lossless.
    HighQuality = 2,
    /// Losslessly decoded, DSP applied (EQ, crossfeed, ReplayGain).
    Lossless = 3,
    /// Bit-perfect: lossless source, no resampling, no DSP, output matches source depth.
    BitPerfect = 4,
}

impl std::fmt::Display for QualityTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityTier::Degraded => f.write_str("Degraded"),
            QualityTier::Standard => f.write_str("Standard"),
            QualityTier::HighQuality => f.write_str("High Quality"),
            QualityTier::Lossless => f.write_str("Lossless"),
            QualityTier::BitPerfect => f.write_str("Bit-Perfect"),
        }
    }
}

/// Determines the quality tier ceiling of a source based on codec, sample rate, and bit depth.
/// DSP stages may only lower the tier below this ceiling via `tier.min(stage_tier)`.
pub fn source_tier(codec: &Codec, sample_rate: u32, bit_depth: Option<u32>) -> QualityTier {
    match codec {
        Codec::Flac | Codec::Alac | Codec::Wav | Codec::Aiff => {
            // High-resolution lossless (Hi-Res Audio spec: > 44.1 kHz or > 16-bit).
            if bit_depth.is_some_and(|d| d >= 24) || sample_rate > 44100 {
                QualityTier::BitPerfect
            } else {
                QualityTier::Lossless
            }
        }
        Codec::Vorbis | Codec::Opus => QualityTier::HighQuality,
        Codec::Aac => QualityTier::HighQuality,
        Codec::Mp3 | Codec::Other(_) => QualityTier::Standard,
    }
}

/// Propagates the tier through a sequence of (enabled, stage_tier) pairs.
/// Disabled stages do not affect the tier.
pub fn propagate_tier(
    source: QualityTier,
    stages: impl IntoIterator<Item = (bool, QualityTier)>,
) -> QualityTier {
    stages
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, tier)| tier)
        .fold(source, QualityTier::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_tier_ordering() {
        assert!(QualityTier::Degraded < QualityTier::Standard);
        assert!(QualityTier::Standard < QualityTier::HighQuality);
        assert!(QualityTier::HighQuality < QualityTier::Lossless);
        assert!(QualityTier::Lossless < QualityTier::BitPerfect);
    }

    #[test]
    fn source_tier_flac_16bit_44k_is_lossless() {
        assert_eq!(
            source_tier(&Codec::Flac, 44100, Some(16)),
            QualityTier::Lossless
        );
    }

    #[test]
    fn source_tier_flac_24bit_96k_is_bitperfect() {
        assert_eq!(
            source_tier(&Codec::Flac, 96000, Some(24)),
            QualityTier::BitPerfect
        );
    }

    #[test]
    fn source_tier_wav_hires_sample_rate_is_bitperfect() {
        assert_eq!(
            source_tier(&Codec::Wav, 88200, Some(16)),
            QualityTier::BitPerfect
        );
    }

    #[test]
    fn source_tier_mp3_is_standard() {
        assert_eq!(
            source_tier(&Codec::Mp3, 44100, None),
            QualityTier::Standard
        );
    }

    #[test]
    fn source_tier_opus_is_high_quality() {
        assert_eq!(
            source_tier(&Codec::Opus, 48000, None),
            QualityTier::HighQuality
        );
    }

    #[test]
    fn source_tier_alac_24bit_is_bitperfect() {
        assert_eq!(
            source_tier(&Codec::Alac, 44100, Some(24)),
            QualityTier::BitPerfect
        );
    }

    #[test]
    fn propagate_tier_no_active_stages_preserves_source() {
        let tier = propagate_tier(
            QualityTier::BitPerfect,
            [(false, QualityTier::Degraded), (false, QualityTier::Standard)],
        );
        assert_eq!(tier, QualityTier::BitPerfect);
    }

    #[test]
    fn propagate_tier_active_stage_lowers_tier() {
        let tier = propagate_tier(
            QualityTier::BitPerfect,
            [(true, QualityTier::Lossless)],
        );
        assert_eq!(tier, QualityTier::Lossless);
    }

    #[test]
    fn propagate_tier_takes_minimum() {
        let tier = propagate_tier(
            QualityTier::BitPerfect,
            [
                (true, QualityTier::Lossless),
                (true, QualityTier::HighQuality),
                (true, QualityTier::Lossless),
            ],
        );
        assert_eq!(tier, QualityTier::HighQuality);
    }

    #[test]
    fn quality_tier_display() {
        assert_eq!(QualityTier::BitPerfect.to_string(), "Bit-Perfect");
        assert_eq!(QualityTier::Lossless.to_string(), "Lossless");
        assert_eq!(QualityTier::HighQuality.to_string(), "High Quality");
        assert_eq!(QualityTier::Standard.to_string(), "Standard");
        assert_eq!(QualityTier::Degraded.to_string(), "Degraded");
    }
}
