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

impl MediaType {
    /// Canonical string form of `apotheke`'s `wants.media_type` CHECK
    /// constraint (`music_album`, `audiobook`, `book`, `comic`, `podcast`,
    /// `movie`, `tv_series`). Distinct FROM [`Display`](fmt::Display), which
    /// is human-readable and covers `News` (which has no want
    /// representation — `None` here).
    ///
    /// The single source of truth for this mapping — callers that need it
    /// (e.g. archon's want/release wiring) should delegate here instead of
    /// hand-matching the CHECK values, which drifts silently from the schema.
    pub fn as_want_str(&self) -> Option<&'static str> {
        match self {
            Self::Music => Some("music_album"),
            Self::Audiobook => Some("audiobook"),
            Self::Book => Some("book"),
            Self::Comic => Some("comic"),
            Self::Podcast => Some("podcast"),
            Self::Movie => Some("movie"),
            Self::Tv => Some("tv_series"),
            Self::News => None,
        }
    }

    /// Parses a `wants.media_type` CHECK-constraint value back into a
    /// [`MediaType`]. Inverse of [`MediaType::as_want_str`]; `None` for any
    /// string outside the 7 CHECK values.
    pub fn parse_want_str(s: &str) -> Option<Self> {
        match s {
            "music_album" => Some(Self::Music),
            "audiobook" => Some(Self::Audiobook),
            "book" => Some(Self::Book),
            "comic" => Some(Self::Comic),
            "podcast" => Some(Self::Podcast),
            "movie" => Some(Self::Movie),
            "tv_series" => Some(Self::Tv),
            _ => None,
        }
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

/// One viewing or listening event reported by an external media server's
/// watch history (Plex `/status/sessions/history/all`), mapped into
/// Harmonia's domain so serving-layer stats stay server-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchRecord {
    /// The server's own item key (Plex `ratingKey`).
    pub source_ref: String,
    /// Item title as the server reports it.
    pub title: String,
    /// Series or artist context for episodes and tracks (Plex
    /// `grandparentTitle`); absent for films and loose items.
    pub grandparent_title: Option<String>,
    /// The server's item kind (`movie`, `episode`, `track`).
    pub media_kind: String,
    /// The server account that viewed the item (Plex `accountID`).
    pub account_id: Option<i64>,
    /// Epoch-seconds timestamp of the view (Plex `viewedAt`).
    pub viewed_at: Option<i64>,
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
    fn want_str_round_trips_all_check_values() {
        for mt in [
            MediaType::Music,
            MediaType::Audiobook,
            MediaType::Book,
            MediaType::Comic,
            MediaType::Podcast,
            MediaType::Movie,
            MediaType::Tv,
        ] {
            let s = mt
                .as_want_str()
                .unwrap_or_else(|| panic!("{mt:?} must have a want string"));
            assert_eq!(
                MediaType::parse_want_str(s),
                Some(mt),
                "round-trip failed for {mt:?} via {s:?}"
            );
        }
    }

    #[test]
    fn as_want_str_matches_schema_check_values() {
        assert_eq!(MediaType::Music.as_want_str(), Some("music_album"));
        assert_eq!(MediaType::Audiobook.as_want_str(), Some("audiobook"));
        assert_eq!(MediaType::Book.as_want_str(), Some("book"));
        assert_eq!(MediaType::Comic.as_want_str(), Some("comic"));
        assert_eq!(MediaType::Podcast.as_want_str(), Some("podcast"));
        assert_eq!(MediaType::Movie.as_want_str(), Some("movie"));
        assert_eq!(MediaType::Tv.as_want_str(), Some("tv_series"));
    }

    #[test]
    fn news_has_no_want_representation() {
        assert_eq!(MediaType::News.as_want_str(), None);
    }

    #[test]
    fn parse_want_str_rejects_unknown_values() {
        assert_eq!(MediaType::parse_want_str("bogus"), None);
        assert_eq!(MediaType::parse_want_str(""), None);
        assert_eq!(MediaType::parse_want_str("Music"), None);
        assert_eq!(MediaType::parse_want_str("news"), None);
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

    #[test]
    fn watch_record_serde_roundtrip() {
        let record = WatchRecord {
            source_ref: "101".to_string(),
            title: "Gantz Graf".to_string(),
            grandparent_title: Some("Autechre".to_string()),
            media_kind: "track".to_string(),
            account_id: Some(1),
            viewed_at: Some(1_700_000_000),
        };
        let json = serde_json::to_string(&record).unwrap();
        let recovered: WatchRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, recovered);
    }
}
