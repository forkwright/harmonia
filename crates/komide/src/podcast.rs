use crate::parser::{Enclosure, NormalizedEntry};

/// Audio MIME type prefixes that identify podcast episode enclosures.
const AUDIO_PREFIXES: &[&str] = &["audio/", "video/mp4", "video/x-m4v"];

/// Returns true if the MIME type string represents audio or podcast-compatible video.
pub fn is_audio_enclosure(content_type: &str) -> bool {
    AUDIO_PREFIXES
        .iter()
        .any(|prefix| content_type.starts_with(prefix))
}

/// Extract the primary audio enclosure from a normalized entry, if present.
pub fn extract_audio_enclosure(entry: &NormalizedEntry) -> Option<&Enclosure> {
    // Prefer typed audio enclosures
    entry
        .enclosures
        .iter()
        .find(|e| {
            e.content_type
                .as_deref()
                .map(is_audio_enclosure)
                .unwrap_or(false)
        })
        // Fall back to any enclosure (untyped RSS enclosures)
        .or_else(|| entry.enclosures.first())
}

/// Returns true if a normalized entry looks like a podcast episode (has an audio enclosure).
pub fn is_podcast_episode(entry: &NormalizedEntry) -> bool {
    extract_audio_enclosure(entry).is_some()
}

/// Determine how many episodes to auto-download for a newly subscribed feed.
///
/// `auto_download_latest_n` = 0 → download nothing
/// otherwise → download the N most recent episodes
pub fn episodes_to_download(total_episodes: usize, auto_download_latest_n: u64) -> usize {
    if auto_download_latest_n == 0 {
        return 0;
    }
    total_episodes.min(auto_download_latest_n as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Enclosure, NormalizedEntry};

    fn entry_with_enclosure(url: &str, content_type: Option<&str>) -> NormalizedEntry {
        NormalizedEntry {
            guid: "guid".to_string(),
            title: "Title".to_string(),
            published: None,
            summary: None,
            content: None,
            enclosures: vec![Enclosure {
                url: url.to_string(),
                content_type: content_type.map(str::to_owned),
                length: None,
            }],
            link: None,
        }
    }

    fn entry_no_enclosure() -> NormalizedEntry {
        NormalizedEntry {
            guid: "guid".to_string(),
            title: "Title".to_string(),
            published: None,
            summary: None,
            content: None,
            enclosures: vec![],
            link: None,
        }
    }

    #[test]
    fn audio_mpeg_is_audio_enclosure() {
        assert!(is_audio_enclosure("audio/mpeg"));
    }

    #[test]
    fn audio_mp4_is_audio_enclosure() {
        assert!(is_audio_enclosure("audio/mp4"));
    }

    #[test]
    fn image_jpeg_is_not_audio_enclosure() {
        assert!(!is_audio_enclosure("image/jpeg"));
    }

    #[test]
    fn extract_typed_audio_enclosure() {
        let entry = entry_with_enclosure("https://example.com/ep.mp3", Some("audio/mpeg"));
        let enc = extract_audio_enclosure(&entry).expect("should have enclosure");
        assert_eq!(enc.url, "https://example.com/ep.mp3");
    }

    #[test]
    fn extract_falls_back_to_untyped_enclosure() {
        let entry = entry_with_enclosure("https://example.com/ep.mp3", None);
        assert!(extract_audio_enclosure(&entry).is_some());
    }

    #[test]
    fn entry_without_enclosure_is_not_episode() {
        assert!(!is_podcast_episode(&entry_no_enclosure()));
    }

    #[test]
    fn entry_with_audio_enclosure_is_episode() {
        let entry = entry_with_enclosure("https://x.com/e.mp3", Some("audio/mpeg"));
        assert!(is_podcast_episode(&entry));
    }

    #[test]
    fn auto_download_zero_means_nothing() {
        assert_eq!(episodes_to_download(10, 0), 0);
    }

    #[test]
    fn auto_download_respects_latest_n() {
        assert_eq!(episodes_to_download(10, 3), 3);
    }

    #[test]
    fn auto_download_capped_at_available_count() {
        assert_eq!(episodes_to_download(2, 5), 2);
    }
}
