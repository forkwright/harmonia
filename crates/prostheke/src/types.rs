//! Domain types for subtitle management.

use std::path::PathBuf;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use themelion::MediaId;
use uuid::Uuid;

/// A subtitle track acquired and stored for a media item.
pub struct SubtitleTrack {
    pub id: Uuid,
    pub media_id: MediaId,
    /// BCP 47 language tag, e.g. "en", "fr", "pt-BR".
    pub language: String,
    pub format: SubtitleFormat,
    /// Path relative to the media file.
    pub file_path: PathBuf,
    /// Provider name, e.g. "opensubtitles", "addic7ed".
    pub provider: String,
    /// Provider-specific subtitle identifier.
    pub provider_id: String,
    pub hearing_impaired: bool,
    /// True for forced subtitles (e.g. foreign language signs).
    pub forced: bool,
    /// Match quality score in the range 0.0–1.0.
    pub score: f64,
    pub acquired_at: Timestamp,
}

/// Subtitle file format.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleFormat {
    Srt,
    Ass,
    Sub,
    Vtt,
}

impl SubtitleFormat {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Ass => "ass",
            Self::Sub => "sub",
            Self::Vtt => "vtt",
        }
    }

    /// Detect format from a file extension. Case-insensitive.
    pub(crate) fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "srt" => Some(Self::Srt),
            "ass" | "ssa" => Some(Self::Ass),
            "sub" => Some(Self::Sub),
            "vtt" => Some(Self::Vtt),
            _ => None,
        }
    }
}

/// A candidate subtitle match returned by a provider search.
#[derive(Clone)]
pub struct SubtitleMatch {
    pub provider: String,
    pub provider_id: String,
    /// BCP 47 language tag.
    pub language: String,
    pub hearing_impaired: bool,
    pub forced: bool,
    /// Match quality score in the range 0.0–1.0.
    pub score: f64,
    pub download_url: String,
}

/// User language preferences for subtitle acquisition.
pub struct LanguagePreference {
    /// BCP 47 tags in preference order: ["en", "fr", "ja"].
    pub languages: Vec<String>,
    pub include_hearing_impaired: bool,
    pub include_forced: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_format_as_str_matches_serde() {
        assert_eq!(SubtitleFormat::Srt.as_str(), "srt");
        assert_eq!(SubtitleFormat::Ass.as_str(), "ass");
        assert_eq!(SubtitleFormat::Sub.as_str(), "sub");
        assert_eq!(SubtitleFormat::Vtt.as_str(), "vtt");
    }

    #[test]
    fn subtitle_format_from_extension_case_insensitive() {
        assert_eq!(
            SubtitleFormat::from_extension("SRT"),
            Some(SubtitleFormat::Srt)
        );
        assert_eq!(
            SubtitleFormat::from_extension("ass"),
            Some(SubtitleFormat::Ass)
        );
        assert_eq!(
            SubtitleFormat::from_extension("SSA"),
            Some(SubtitleFormat::Ass)
        );
        assert_eq!(
            SubtitleFormat::from_extension("vtt"),
            Some(SubtitleFormat::Vtt)
        );
        assert_eq!(SubtitleFormat::from_extension("mp4"), None);
    }

    #[test]
    fn subtitle_format_serde_roundtrip() {
        let formats = [
            SubtitleFormat::Srt,
            SubtitleFormat::Ass,
            SubtitleFormat::Sub,
            SubtitleFormat::Vtt,
        ];
        for fmt in formats {
            let json = serde_json::to_string(&fmt).unwrap();
            let restored: SubtitleFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(fmt, restored);
        }
    }
}
