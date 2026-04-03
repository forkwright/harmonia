use std::path::PathBuf;

use harmonia_common::{MediaId, MediaType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct UnidentifiedItem {
    pub media_id: MediaId,
    pub media_type: MediaType,
    pub file_path: PathBuf,
    pub filename_hint: Option<String>,
    pub tags: Option<EmbeddedTags>,
}

#[derive(Debug, Clone, Default)]
pub struct EmbeddedTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub isrc: Option<String>,
    pub mb_recording_id: Option<String>,
    pub mb_release_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaIdentity {
    pub media_id: MediaId,
    pub media_type: MediaType,
    pub provider: String,
    pub provider_id: String,
    pub canonical_title: String,
    pub canonical_artist: Option<String>,
    pub year: Option<u32>,
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedMetadata {
    pub identity: MediaIdentity,
    pub enrichments: Vec<ProviderEnrichment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEnrichment {
    pub provider: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintResult {
    pub fingerprint: String,
    pub duration_secs: f64,
    pub acoustid_id: Option<String>,
    pub mb_recording_ids: Vec<String>,
    pub confidence: f64,
}

/// Minimum AcoustID confidence score to accept a match without ambiguity.
pub const FINGERPRINT_ACCEPT_THRESHOLD: f64 = 0.8;

/// Minimum AcoustID confidence score to consider a match at all.
pub const FINGERPRINT_AMBIGUOUS_THRESHOLD: f64 = 0.5;

/// Parse a filename stem INTO metadata hints.
///
/// Supports patterns:
/// - `"Artist - Album - 01 - Track.flac"` → all four fields
/// - `"Album - 01 - Track.flac"` → album + track_number + title
/// - `"Artist - Album - Title.flac"` → artist + album + title
/// - `"01 - Track.flac"` → track_number + title
/// - `"Track.flac"` → title only
pub fn parse_filename(path: &std::path::Path) -> ParsedFilename {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    let parts: Vec<&str> = stem.splitn(4, " - ").collect();

    match parts.as_slice() {
        [artist, album, track_num_str, title] => ParsedFilename {
            artist: Some(artist.trim().to_string()),
            album: Some(album.trim().to_string()),
            track_number: track_num_str.trim().parse().ok(),
            title: title.trim().to_string(),
        },
        [first, second, third] => {
            if let Ok(num) = second.trim().parse::<u32>() {
                ParsedFilename {
                    artist: None,
                    album: Some(first.trim().to_string()),
                    track_number: Some(num),
                    title: third.trim().to_string(),
                }
            } else {
                ParsedFilename {
                    artist: Some(first.trim().to_string()),
                    album: Some(second.trim().to_string()),
                    track_number: None,
                    title: third.trim().to_string(),
                }
            }
        }
        [prefix, title] => {
            if let Ok(num) = prefix.trim().parse::<u32>() {
                ParsedFilename {
                    artist: None,
                    album: None,
                    track_number: Some(num),
                    title: title.trim().to_string(),
                }
            } else {
                ParsedFilename {
                    artist: None,
                    album: None,
                    track_number: None,
                    title: stem.to_string(),
                }
            }
        }
        _ => ParsedFilename {
            artist: None,
            album: None,
            track_number: None,
            title: stem.to_string(),
        },
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParsedFilename {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub title: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_four_part_filename() {
        let result = parse_filename(Path::new("Artist - Album - 01 - Track.flac"));
        assert_eq!(result.artist.as_deref(), Some("Artist"));
        assert_eq!(result.album.as_deref(), Some("Album"));
        assert_eq!(result.track_number, Some(1));
        assert_eq!(result.title, "Track");
    }

    #[test]
    fn parse_track_number_and_title() {
        let result = parse_filename(Path::new("01 - Track.flac"));
        assert_eq!(result.artist, None);
        assert_eq!(result.album, None);
        assert_eq!(result.track_number, Some(1));
        assert_eq!(result.title, "Track");
    }

    #[test]
    fn parse_title_only() {
        let result = parse_filename(Path::new("Track.flac"));
        assert_eq!(result.artist, None);
        assert_eq!(result.album, None);
        assert_eq!(result.track_number, None);
        assert_eq!(result.title, "Track");
    }

    #[test]
    fn parse_three_part_with_track_number() {
        let result = parse_filename(Path::new("Album - 03 - Title.flac"));
        assert_eq!(result.artist, None);
        assert_eq!(result.album.as_deref(), Some("Album"));
        assert_eq!(result.track_number, Some(3));
        assert_eq!(result.title, "Title");
    }

    #[test]
    fn parse_three_part_without_track_number() {
        let result = parse_filename(Path::new("Artist - Album - Title.flac"));
        assert_eq!(result.artist.as_deref(), Some("Artist"));
        assert_eq!(result.album.as_deref(), Some("Album"));
        assert_eq!(result.track_number, None);
        assert_eq!(result.title, "Title");
    }

    #[test]
    fn fingerprint_accept_threshold_value() {
        assert_eq!(FINGERPRINT_ACCEPT_THRESHOLD, 0.8);
    }

    #[test]
    fn fingerprint_ambiguous_threshold_value() {
        assert_eq!(FINGERPRINT_AMBIGUOUS_THRESHOLD, 0.5);
    }

    #[test]
    #[expect(
        clippy::assertions_on_constants,
        reason = "intentional  -  validates threshold ordering"
    )]
    fn fingerprint_accept_above_ambiguous() {
        assert!(FINGERPRINT_ACCEPT_THRESHOLD > FINGERPRINT_AMBIGUOUS_THRESHOLD);
    }
}
