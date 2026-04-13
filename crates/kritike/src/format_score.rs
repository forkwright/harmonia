/// Score an ebook format. Higher is better.
///
/// Tier order: EPUB > AZW3 > PDF > MOBI > TXT
/// Unknown extensions return 0.0.
pub fn ebook_format_score(extension: &str) -> f64 {
    match extension.to_ascii_lowercase().as_str() {
        "epub" => 1.0,
        "azw3" => 0.8,
        "pdf" => 0.6,
        "mobi" => 0.4,
        "txt" => 0.2,
        _ => 0.0,
    }
}

/// Score an audiobook format. Higher is better.
///
/// Tier order: M4B > FLAC > OGG, then MP3 by bitrate (320 > 256 > 192 > 128 > 64).
/// Unknown extensions return 0.0. Unknown bitrate for MP3 returns 0.0.
pub fn audiobook_format_score(extension: &str, bitrate_kbps: Option<u32>) -> f64 {
    match extension.to_ascii_lowercase().as_str() {
        "m4b" => 1.0,
        "flac" => 0.95,
        "ogg" => 0.85,
        "mp3" => mp3_score(bitrate_kbps),
        _ => 0.0,
    }
}

/// Score a music format. Higher is better.
///
/// Tier order: lossless (FLAC/ALAC) > high-quality OGG > MP3 320 > AAC 256 > OGG Q5 > MP3 192 > MP3 128.
/// A sample rate above 44100 Hz adds a 0.05 bonus, capped at 1.0.
/// Unknown extensions return 0.0.
pub fn music_format_score(
    extension: &str,
    bitrate_kbps: Option<u32>,
    sample_rate: Option<u32>,
) -> f64 {
    let base = match extension.to_ascii_lowercase().as_str() {
        "flac" | "alac" => 1.0,
        "ogg" => ogg_music_score(bitrate_kbps),
        "mp3" => mp3_score(bitrate_kbps),
        "aac" | "m4a" => aac_score(bitrate_kbps),
        _ => 0.0,
    };

    if base == 0.0 {
        return 0.0;
    }

    let hi_res_bonus = if sample_rate.is_some_and(|sr| sr > 44100) {
        0.05
    } else {
        0.0
    };

    (base + hi_res_bonus).min(1.0)
}

/// Overall quality score combining format and metadata completeness.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityScore {
    /// Format score in the range [0.0, 1.0].
    pub format: f64,
    /// Metadata completeness score in the range [0.0, 1.0].
    /// Computed as the fraction of provided optional fields that are non-empty.
    pub metadata: f64,
    /// Weighted combination: 0.7 * format + 0.3 * metadata.
    pub overall: f64,
}

impl QualityScore {
    /// Build a `QualityScore` from pre-computed components.
    ///
    /// `overall` is derived automatically; passing the format and metadata
    /// scores is sufficient.
    pub fn new(format: f64, metadata: f64) -> Self {
        let overall = 0.7 * format + 0.3 * metadata;
        Self {
            format,
            metadata,
            overall,
        }
    }
}

// --- private helpers ---

fn mp3_score(bitrate_kbps: Option<u32>) -> f64 {
    match bitrate_kbps {
        Some(kbps) if kbps >= 320 => 0.8,
        Some(kbps) if kbps >= 256 => 0.7,
        Some(kbps) if kbps >= 192 => 0.6,
        Some(kbps) if kbps >= 128 => 0.4,
        Some(kbps) if kbps >= 64 => 0.2,
        Some(_) => 0.1,
        None => 0.0,
    }
}

fn ogg_music_score(bitrate_kbps: Option<u32>) -> f64 {
    // OGG Q8+ maps roughly to >=256 kbps; Q5 maps roughly to >=160 kbps.
    match bitrate_kbps {
        Some(kbps) if kbps >= 256 => 0.9,
        Some(_) => 0.75,
        None => 0.85, // treat unknown OGG as mid-tier
    }
}

fn aac_score(bitrate_kbps: Option<u32>) -> f64 {
    match bitrate_kbps {
        Some(kbps) if kbps >= 256 => 0.8,
        Some(kbps) if kbps >= 192 => 0.7,
        Some(kbps) if kbps >= 128 => 0.55,
        Some(_) => 0.4,
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ebook ---

    #[test]
    fn ebook_epub_scores_1() {
        assert_eq!(ebook_format_score("epub"), 1.0);
    }

    #[test]
    fn ebook_azw3_scores_0_8() {
        assert_eq!(ebook_format_score("azw3"), 0.8);
    }

    #[test]
    fn ebook_pdf_scores_0_6() {
        assert_eq!(ebook_format_score("pdf"), 0.6);
    }

    #[test]
    fn ebook_mobi_scores_0_4() {
        assert_eq!(ebook_format_score("mobi"), 0.4);
    }

    #[test]
    fn ebook_txt_scores_0_2() {
        assert_eq!(ebook_format_score("txt"), 0.2);
    }

    #[test]
    fn ebook_unknown_scores_0() {
        assert_eq!(ebook_format_score("docx"), 0.0);
        assert_eq!(ebook_format_score(""), 0.0);
    }

    #[test]
    fn ebook_case_insensitive() {
        assert_eq!(ebook_format_score("EPUB"), ebook_format_score("epub"));
        assert_eq!(ebook_format_score("PDF"), ebook_format_score("pdf"));
    }

    // --- audiobook ---

    #[test]
    fn audiobook_m4b_scores_1() {
        assert_eq!(audiobook_format_score("m4b", None), 1.0);
    }

    #[test]
    fn audiobook_flac_scores_0_95() {
        assert_eq!(audiobook_format_score("flac", None), 0.95);
    }

    #[test]
    fn audiobook_ogg_scores_0_85() {
        assert_eq!(audiobook_format_score("ogg", None), 0.85);
    }

    #[test]
    fn audiobook_mp3_bitrate_tiers() {
        assert_eq!(audiobook_format_score("mp3", Some(320)), 0.8);
        assert_eq!(audiobook_format_score("mp3", Some(256)), 0.7);
        assert_eq!(audiobook_format_score("mp3", Some(192)), 0.6);
        assert_eq!(audiobook_format_score("mp3", Some(128)), 0.4);
        assert_eq!(audiobook_format_score("mp3", Some(64)), 0.2);
    }

    #[test]
    fn audiobook_mp3_no_bitrate_scores_0() {
        assert_eq!(audiobook_format_score("mp3", None), 0.0);
    }

    #[test]
    fn audiobook_unknown_scores_0() {
        assert_eq!(audiobook_format_score("wma", None), 0.0);
    }

    #[test]
    fn audiobook_case_insensitive() {
        assert_eq!(
            audiobook_format_score("M4B", None),
            audiobook_format_score("m4b", None)
        );
        assert_eq!(
            audiobook_format_score("FLAC", None),
            audiobook_format_score("flac", None)
        );
    }

    // --- music ---

    #[test]
    fn music_flac_scores_1() {
        assert_eq!(music_format_score("flac", None, None), 1.0);
    }

    #[test]
    fn music_alac_scores_1() {
        assert_eq!(music_format_score("alac", None, None), 1.0);
    }

    #[test]
    fn music_mp3_bitrate_tiers() {
        assert_eq!(music_format_score("mp3", Some(320), None), 0.8);
        assert_eq!(music_format_score("mp3", Some(256), None), 0.7);
        assert_eq!(music_format_score("mp3", Some(192), None), 0.6);
        assert_eq!(music_format_score("mp3", Some(128), None), 0.4);
    }

    #[test]
    fn music_hi_res_bonus_applied() {
        // FLAC at 96 kHz: 1.0 + 0.05 = 1.0 (capped)
        assert_eq!(music_format_score("flac", None, Some(96_000)), 1.0);
        // MP3 320 at 96 kHz: 0.8 + 0.05 = 0.85
        assert!((music_format_score("mp3", Some(320), Some(96_000)) - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn music_hi_res_bonus_not_applied_at_44100() {
        assert_eq!(
            music_format_score("flac", None, Some(44100)),
            music_format_score("flac", None, None)
        );
    }

    #[test]
    fn music_hi_res_bonus_not_applied_below_44100() {
        assert_eq!(
            music_format_score("flac", None, Some(22050)),
            music_format_score("flac", None, None)
        );
    }

    #[test]
    fn music_unknown_scores_0() {
        assert_eq!(music_format_score("wma", None, None), 0.0);
        assert_eq!(music_format_score("", None, None), 0.0);
    }

    #[test]
    fn music_case_insensitive() {
        assert_eq!(
            music_format_score("FLAC", None, None),
            music_format_score("flac", None, None)
        );
        assert_eq!(
            music_format_score("ALAC", None, None),
            music_format_score("alac", None, None)
        );
    }

    // --- QualityScore ---

    #[test]
    fn quality_score_overall_weighted() {
        let qs = QualityScore::new(1.0, 0.5);
        let expected = 0.7 * 1.0 + 0.3 * 0.5;
        assert!((qs.overall - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn quality_score_all_zeros() {
        let qs = QualityScore::new(0.0, 0.0);
        assert_eq!(qs.overall, 0.0);
    }

    #[test]
    fn quality_score_perfect() {
        let qs = QualityScore::new(1.0, 1.0);
        assert_eq!(qs.overall, 1.0);
    }
}
