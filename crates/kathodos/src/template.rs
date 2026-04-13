use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::sanitize::sanitize_component;

// ── Release type ──────────────────────────────────────────────────────────────

/// Discriminates music release types for directory naming.
///
/// The tag returned by [`ReleaseType::tag`] is the bracketed token inserted
/// into a release directory name, e.g. `[EP]`, `[Single]`.  Studio albums
/// have no tag (empty string) so their directories read as
/// `[{YYYY}] {Title}` without any type annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    /// Standard studio album — no type tag in the directory name.
    Album,
    /// Extended play.
    EP,
    /// Single release.
    Single,
    /// Live recording.
    Live,
    /// Compilation or various-artists release.
    Compilation,
    /// Original soundtrack.
    Soundtrack,
}

impl ReleaseType {
    /// Returns the bracketed tag for this release type, or `""` for albums.
    ///
    /// Used as an infix when building a release directory name:
    /// `[{YYYY}] {tag} {Title}` where the tag (including surrounding spaces)
    /// is omitted entirely for studio albums.
    pub fn tag(&self) -> &'static str {
        match self {
            ReleaseType::Album => "",
            ReleaseType::EP => "[EP]",
            ReleaseType::Single => "[Single]",
            ReleaseType::Live => "[Live]",
            ReleaseType::Compilation => "[Comp]",
            ReleaseType::Soundtrack => "[OST]",
        }
    }
}

// ── Music templates ───────────────────────────────────────────────────────────

/// Canonical path for a music release directory (relative, two components).
///
/// Structure: `{Artist Name}/[{YYYY}] {Title}` for albums, or
/// `{Artist Name}/[{YYYY}] [{Type}] {Title}` for other release types.
///
/// Both `artist` and `title` are passed through [`sanitize_component`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use kathodos::template::{music_release_path, ReleaseType};
///
/// let path = music_release_path("Nirvana", 1991, "Nevermind", ReleaseType::Album);
/// assert_eq!(path, PathBuf::from("Nirvana/[1991] Nevermind"));
///
/// let ep = music_release_path("Radiohead", 2001, "I Might Be Wrong", ReleaseType::EP);
/// assert_eq!(ep, PathBuf::from("Radiohead/[2001] [EP] I Might Be Wrong"));
/// ```
pub fn music_release_path(artist: &str, year: u16, title: &str, release_type: ReleaseType) -> PathBuf {
    let artist_dir = sanitize_component(artist);
    let title_san = sanitize_component(title);

    let release_dir = match release_type.tag() {
        "" => format!("[{year}] {title_san}"),
        tag => format!("[{year}] {tag} {title_san}"),
    };

    PathBuf::from(artist_dir).join(release_dir)
}

/// Canonical filename for a music track.
///
/// Format: `{disc:02}-{track:02} - {Title}.{ext}`
///
/// Both disc and track numbers are zero-padded to two digits.  Single-disc
/// releases still carry the `01-` disc prefix for consistency.  `title` is
/// passed through [`sanitize_component`]; `ext` is used as-is (caller is
/// responsible for passing a clean extension without a leading dot).
///
/// # Example
///
/// ```
/// use kathodos::template::music_track_filename;
///
/// assert_eq!(
///     music_track_filename(1, 3, "Come as You Are", "flac"),
///     "01-03 - Come as You Are.flac"
/// );
/// assert_eq!(
///     music_track_filename(2, 14, "Track Name", "opus"),
///     "02-14 - Track Name.opus"
/// );
/// ```
pub fn music_track_filename(disc: u8, track: u8, title: &str, ext: &str) -> String {
    let title_san = sanitize_component(title);
    format!("{disc:02}-{track:02} - {title_san}.{ext}")
}

// ── Book template ─────────────────────────────────────────────────────────────

/// Canonical path for a book (ebook) directory (relative, two components).
///
/// Structure: `{Author Name}/[{YYYY}] {Title}`
///
/// Both `author` and `title` are passed through [`sanitize_component`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use kathodos::template::book_path;
///
/// let path = book_path("Frank Herbert", 1965, "Dune");
/// assert_eq!(path, PathBuf::from("Frank Herbert/[1965] Dune"));
/// ```
pub fn book_path(author: &str, year: u16, title: &str) -> PathBuf {
    let author_dir = sanitize_component(author);
    let title_san = sanitize_component(title);
    PathBuf::from(author_dir).join(format!("[{year}] {title_san}"))
}

// ── Audiobook template ────────────────────────────────────────────────────────

/// Canonical path for an audiobook directory (relative, two components).
///
/// Structure: `{Author Name}/[{YYYY}] {Title}`
///
/// Mirrors [`book_path`] — the distinction is which sidecar file lives inside
/// (`audiobook.toml` vs `book.toml`).  Both `author` and `title` are passed
/// through [`sanitize_component`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use kathodos::template::audiobook_path;
///
/// let path = audiobook_path("Frank Herbert", 1965, "Dune");
/// assert_eq!(path, PathBuf::from("Frank Herbert/[1965] Dune"));
/// ```
pub fn audiobook_path(author: &str, year: u16, title: &str) -> PathBuf {
    let author_dir = sanitize_component(author);
    let title_san = sanitize_component(title);
    PathBuf::from(author_dir).join(format!("[{year}] {title_san}"))
}

// ── Podcast template ──────────────────────────────────────────────────────────

/// Canonical filename for a podcast episode.
///
/// Format: `[{YYYY-MM-DD}] {Episode Title}.{ext}`
///
/// The ISO date prefix enables chronological sort within the show directory.
/// `date` must be a pre-formatted `YYYY-MM-DD` string; `title` is passed
/// through [`sanitize_component`]; `ext` is used as-is.
///
/// # Example
///
/// ```
/// use kathodos::template::podcast_episode_filename;
///
/// assert_eq!(
///     podcast_episode_filename("2026-04-12", "The State of Rust", "mp3"),
///     "[2026-04-12] The State of Rust.mp3"
/// );
/// ```
pub fn podcast_episode_filename(date: &str, title: &str, ext: &str) -> String {
    let title_san = sanitize_component(title);
    format!("[{date}] {title_san}.{ext}")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ReleaseType::tag ──────────────────────────────────────────────────────

    #[test]
    fn album_tag_is_empty() {
        assert_eq!(ReleaseType::Album.tag(), "");
    }

    #[test]
    fn ep_tag() {
        assert_eq!(ReleaseType::EP.tag(), "[EP]");
    }

    #[test]
    fn single_tag() {
        assert_eq!(ReleaseType::Single.tag(), "[Single]");
    }

    #[test]
    fn live_tag() {
        assert_eq!(ReleaseType::Live.tag(), "[Live]");
    }

    #[test]
    fn compilation_tag() {
        assert_eq!(ReleaseType::Compilation.tag(), "[Comp]");
    }

    #[test]
    fn soundtrack_tag() {
        assert_eq!(ReleaseType::Soundtrack.tag(), "[OST]");
    }

    // ── music_release_path ────────────────────────────────────────────────────

    #[test]
    fn album_path_has_no_type_tag() {
        let path = music_release_path("Nirvana", 1991, "Nevermind", ReleaseType::Album);
        assert_eq!(path, PathBuf::from("Nirvana/[1991] Nevermind"));
    }

    #[test]
    fn ep_path_has_ep_tag() {
        let path = music_release_path("Radiohead", 2001, "I Might Be Wrong", ReleaseType::EP);
        assert_eq!(path, PathBuf::from("Radiohead/[2001] [EP] I Might Be Wrong"));
    }

    #[test]
    fn single_path_has_single_tag() {
        let path = music_release_path("Adele", 2010, "Rolling in the Deep", ReleaseType::Single);
        assert_eq!(
            path,
            PathBuf::from("Adele/[2010] [Single] Rolling in the Deep")
        );
    }

    #[test]
    fn live_path_has_live_tag() {
        let path = music_release_path("Nirvana", 1996, "From the Muddy Banks", ReleaseType::Live);
        assert_eq!(
            path,
            PathBuf::from("Nirvana/[1996] [Live] From the Muddy Banks")
        );
    }

    #[test]
    fn compilation_path_has_comp_tag() {
        let path = music_release_path("Various Artists", 2000, "Now That's Music", ReleaseType::Compilation);
        assert_eq!(
            path,
            PathBuf::from("Various Artists/[2000] [Comp] Now That's Music")
        );
    }

    #[test]
    fn soundtrack_path_has_ost_tag() {
        let path = music_release_path("Hans Zimmer", 2010, "Inception", ReleaseType::Soundtrack);
        assert_eq!(
            path,
            PathBuf::from("Hans Zimmer/[2010] [OST] Inception")
        );
    }

    #[test]
    fn artist_with_unsafe_chars_is_sanitized() {
        // AC/DC would be split by sanitize_component — the slash is replaced with dash
        let path = music_release_path("AC/DC", 1980, "Back in Black", ReleaseType::Album);
        let s = path.to_string_lossy();
        assert!(!s.contains("AC/DC"), "raw slash must not survive: {s}");
        assert!(s.contains("AC-DC") || s.contains("AC"), "artist component preserved: {s}");
    }

    #[test]
    fn title_with_colon_is_sanitized() {
        let path = music_release_path("Arcade Fire", 2004, "Funeral: Deluxe", ReleaseType::Album);
        let s = path.to_string_lossy();
        assert!(!s.contains(':'), "colon must not survive: {s}");
    }

    #[test]
    fn unicode_artist_preserved() {
        let path = music_release_path("山田太郎", 2020, "春の歌", ReleaseType::Album);
        assert_eq!(path, PathBuf::from("山田太郎/[2020] 春の歌"));
    }

    // ── music_track_filename ──────────────────────────────────────────────────

    #[test]
    fn single_disc_track_01_01() {
        assert_eq!(
            music_track_filename(1, 1, "Come as You Are", "flac"),
            "01-01 - Come as You Are.flac"
        );
    }

    #[test]
    fn single_disc_track_zero_padded() {
        assert_eq!(
            music_track_filename(1, 9, "Track Nine", "flac"),
            "01-09 - Track Nine.flac"
        );
    }

    #[test]
    fn multi_disc_track() {
        assert_eq!(
            music_track_filename(2, 14, "The Great Gig in the Sky", "opus"),
            "02-14 - The Great Gig in the Sky.opus"
        );
    }

    #[test]
    fn track_title_with_unsafe_chars_sanitized() {
        let filename = music_track_filename(1, 1, "Song: A/B Test", "mp3");
        assert!(!filename.contains(':'));
        // slash in title is replaced
        assert!(!filename.contains('/'));
        assert!(filename.starts_with("01-01 - "));
        assert!(filename.ends_with(".mp3"));
    }

    #[test]
    fn track_extension_preserved_as_is() {
        assert!(music_track_filename(1, 1, "Track", "flac").ends_with(".flac"));
        assert!(music_track_filename(1, 1, "Track", "opus").ends_with(".opus"));
        assert!(music_track_filename(1, 1, "Track", "mp3").ends_with(".mp3"));
    }

    // ── book_path ─────────────────────────────────────────────────────────────

    #[test]
    fn book_path_standard() {
        let path = book_path("Frank Herbert", 1965, "Dune");
        assert_eq!(path, PathBuf::from("Frank Herbert/[1965] Dune"));
    }

    #[test]
    fn book_path_author_with_unsafe_chars_sanitized() {
        let path = book_path("Author: First/Last", 2000, "Title");
        let s = path.to_string_lossy();
        assert!(!s.starts_with("Author: First"), "colon in author must be sanitized: {s}");
    }

    #[test]
    fn book_path_title_with_colon_sanitized() {
        let path = book_path("Author Name", 2020, "Book: A Story");
        let s = path.to_string_lossy();
        assert!(!s.contains(':'), "colon in title must be sanitized: {s}");
    }

    #[test]
    fn book_path_unicode() {
        let path = book_path("村上春樹", 1987, "ノルウェイの森");
        assert_eq!(path, PathBuf::from("村上春樹/[1987] ノルウェイの森"));
    }

    // ── audiobook_path ────────────────────────────────────────────────────────

    #[test]
    fn audiobook_path_standard() {
        let path = audiobook_path("Frank Herbert", 1965, "Dune");
        assert_eq!(path, PathBuf::from("Frank Herbert/[1965] Dune"));
    }

    #[test]
    fn audiobook_path_matches_book_path_structure() {
        // Same structure — different sidecar file inside the directory.
        let book = book_path("Author", 2020, "Title");
        let audio = audiobook_path("Author", 2020, "Title");
        assert_eq!(book, audio);
    }

    #[test]
    fn audiobook_path_sanitizes_author() {
        let path = audiobook_path("Author/Name", 2000, "Title");
        let s = path.to_string_lossy();
        assert!(!s.contains("Author/Name"), "slash in author must be replaced: {s}");
    }

    // ── podcast_episode_filename ──────────────────────────────────────────────

    #[test]
    fn podcast_episode_standard() {
        assert_eq!(
            podcast_episode_filename("2026-04-12", "The State of Rust", "mp3"),
            "[2026-04-12] The State of Rust.mp3"
        );
    }

    #[test]
    fn podcast_episode_title_with_unsafe_chars_sanitized() {
        let filename = podcast_episode_filename("2026-01-01", "Episode: A/B", "mp3");
        assert!(!filename.contains(':'));
        assert!(filename.starts_with("[2026-01-01] "));
        assert!(filename.ends_with(".mp3"));
    }

    #[test]
    fn podcast_episode_extension_preserved() {
        assert!(podcast_episode_filename("2026-01-01", "Title", "opus").ends_with(".opus"));
        assert!(podcast_episode_filename("2026-01-01", "Title", "mp3").ends_with(".mp3"));
    }

    #[test]
    fn podcast_episode_date_bracket_format() {
        let filename = podcast_episode_filename("2026-04-12", "Episode Title", "mp3");
        assert!(filename.starts_with("[2026-04-12] "), "date must be in brackets: {filename}");
    }

    // ── serde round-trip ──────────────────────────────────────────────────────

    #[test]
    fn release_type_serde_round_trip() {
        use serde::{Deserialize, Serialize};

        // TOML requires a table at the document root; wrap the enum in a struct.
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Wrapper {
            kind: ReleaseType,
        }

        let variants = [
            ReleaseType::Album,
            ReleaseType::EP,
            ReleaseType::Single,
            ReleaseType::Live,
            ReleaseType::Compilation,
            ReleaseType::Soundtrack,
        ];
        for variant in variants {
            let wrapped = Wrapper { kind: variant };
            let encoded = toml::to_string(&wrapped)
                .unwrap_or_else(|e| panic!("serialize {variant:?}: {e}"));
            let back: Wrapper = toml::from_str(&encoded)
                .unwrap_or_else(|e| panic!("deserialize {variant:?} from {encoded:?}: {e}"));
            assert_eq!(back.kind, variant);
        }
    }
}
