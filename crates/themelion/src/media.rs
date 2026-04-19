use std::fmt;

use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Music,
    Audiobook,
    Book,
    Comic,
    Podcast,
    News,
    Movie,
    Tv,
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Music => "music",
            Self::Audiobook => "audiobook",
            Self::Book => "book",
            Self::Comic => "comic",
            Self::Podcast => "podcast",
            Self::News => "news",
            Self::Movie => "movie",
            Self::Tv => "tv",
        };
        f.write_str(s)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaItemState {
    Discovered,
    Wanted,
    Downloading,
    Imported,
    Fingerprinting,
    ChapterExtracted,
    Enriched,
    Organized,
    Available,
}

/// Represents the minimum quality threshold for a media item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QualityProfile {
    pub score: u32,
}

impl QualityProfile {
    pub fn new(score: u32) -> Self {
        Self { score }
    }
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn media_type_serde_roundtrip() {
        let variants = [
            MediaType::Music,
            MediaType::Audiobook,
            MediaType::Book,
            MediaType::Comic,
            MediaType::Podcast,
            MediaType::News,
            MediaType::Movie,
            MediaType::Tv,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let recovered: MediaType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, recovered);
        }
    }

    #[test]
    fn media_type_display() {
        assert_eq!(MediaType::Music.to_string(), "music");
        assert_eq!(MediaType::Audiobook.to_string(), "audiobook");
        assert_eq!(MediaType::Book.to_string(), "book");
        assert_eq!(MediaType::Comic.to_string(), "comic");
        assert_eq!(MediaType::Podcast.to_string(), "podcast");
        assert_eq!(MediaType::News.to_string(), "news");
        assert_eq!(MediaType::Movie.to_string(), "movie");
        assert_eq!(MediaType::Tv.to_string(), "tv");
    }

    #[test]
    fn media_type_serde_snake_case() {
        let json = serde_json::to_string(&MediaType::Tv).unwrap();
        assert_eq!(json, "\"tv\"");
        let json = serde_json::to_string(&MediaType::Audiobook).unwrap();
        assert_eq!(json, "\"audiobook\"");
    }

    #[test]
    fn media_item_state_serde_roundtrip() {
        let variants = [
            MediaItemState::Discovered,
            MediaItemState::Wanted,
            MediaItemState::Downloading,
            MediaItemState::Imported,
            MediaItemState::Fingerprinting,
            MediaItemState::ChapterExtracted,
            MediaItemState::Enriched,
            MediaItemState::Organized,
            MediaItemState::Available,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let recovered: MediaItemState = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, recovered);
        }
    }

    #[test]
    fn quality_profile_serde_roundtrip() {
        let qp = QualityProfile::new(320);
        let json = serde_json::to_string(&qp).unwrap();
        let recovered: QualityProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(qp, recovered);
    }
}
