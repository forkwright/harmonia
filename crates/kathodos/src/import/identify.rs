use horismos::MediaType as LibMediaType;
use themelion::MediaType;

/// Map horismos library media type to themelion::MediaType.
pub(crate) fn resolve_media_type(lib_type: &LibMediaType) -> MediaType {
    match lib_type {
        LibMediaType::Music => MediaType::Music,
        LibMediaType::Video => MediaType::Movie,
        LibMediaType::Book => MediaType::Book,
        // WHY: horismos::MediaType is #[non_exhaustive]; fall back to Music
        // for any future variant until kathodos defines an explicit mapping.
        _ => MediaType::Music,
    }
}

/// Detect media type from file extension, given the library's expected type.
/// Library type is used to resolve ambiguous extensions.
pub fn detect_media_type(
    path: &std::path::Path,
    library_media_type: MediaType,
) -> Option<MediaType> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let ext = ext.as_str();

    // Unambiguous extensions
    let unambiguous = match ext {
        "flac" | "wav" | "aiff" | "aif" | "wv" | "alac" | "aac" => Some(MediaType::Music),
        "m4b" => Some(MediaType::Audiobook),
        "epub" | "mobi" | "azw3" => Some(MediaType::Book),
        "cbz" | "cbr" | "cb7" => Some(MediaType::Comic),
        "avi" | "m4v" => Some(MediaType::Movie),
        _ => None,
    };
    if let Some(t) = unambiguous {
        return Some(t);
    }

    // Ambiguous — resolve using library type
    match ext {
        "mp3" | "m4a" | "ogg" | "opus" => match library_media_type {
            MediaType::Podcast => Some(MediaType::Podcast),
            MediaType::Audiobook => Some(MediaType::Audiobook),
            _ => Some(MediaType::Music),
        },
        "mkv" | "mp4" => match library_media_type {
            MediaType::Tv => Some(MediaType::Tv),
            _ => Some(MediaType::Movie),
        },
        "pdf" => match library_media_type {
            MediaType::Comic => Some(MediaType::Comic),
            _ => Some(MediaType::Book),
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn flac_is_music() {
        assert_eq!(
            detect_media_type(Path::new("track.flac"), MediaType::Music),
            Some(MediaType::Music)
        );
    }

    #[test]
    fn m4b_is_audiobook() {
        assert_eq!(
            detect_media_type(Path::new("book.m4b"), MediaType::Music),
            Some(MediaType::Audiobook)
        );
    }

    #[test]
    fn mp3_in_music_library_is_music() {
        assert_eq!(
            detect_media_type(Path::new("track.mp3"), MediaType::Music),
            Some(MediaType::Music)
        );
    }

    #[test]
    fn mp3_in_podcast_library_is_podcast() {
        assert_eq!(
            detect_media_type(Path::new("episode.mp3"), MediaType::Podcast),
            Some(MediaType::Podcast)
        );
    }

    #[test]
    fn mkv_in_tv_library_is_tv() {
        assert_eq!(
            detect_media_type(Path::new("episode.mkv"), MediaType::Tv),
            Some(MediaType::Tv)
        );
    }

    #[test]
    fn mkv_in_movie_library_is_movie() {
        assert_eq!(
            detect_media_type(Path::new("movie.mkv"), MediaType::Movie),
            Some(MediaType::Movie)
        );
    }

    #[test]
    fn unknown_extension_returns_none() {
        assert_eq!(
            detect_media_type(Path::new("file.xyz"), MediaType::Music),
            None
        );
    }

    #[test]
    fn pdf_in_comic_library_is_comic() {
        assert_eq!(
            detect_media_type(Path::new("issue.pdf"), MediaType::Comic),
            Some(MediaType::Comic)
        );
    }

    #[test]
    fn pdf_in_book_library_is_book() {
        assert_eq!(
            detect_media_type(Path::new("book.pdf"), MediaType::Book),
            Some(MediaType::Book)
        );
    }

    #[test]
    fn resolve_media_type_music() {
        assert_eq!(resolve_media_type(&LibMediaType::Music), MediaType::Music);
    }

    #[test]
    fn resolve_media_type_video_is_movie() {
        assert_eq!(resolve_media_type(&LibMediaType::Video), MediaType::Movie);
    }

    #[test]
    fn resolve_media_type_book() {
        assert_eq!(resolve_media_type(&LibMediaType::Book), MediaType::Book);
    }
}
