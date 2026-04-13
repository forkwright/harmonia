//! Canonical path templates for all media types.
//!
//! Implements the one-answer-per-item rule from `docs/data/storage-layout.md`.
//! Given typed metadata, each function returns the single correct relative path
//! (directory or filename) for that item in its library.
//!
//! # Sanitization
//!
//! All string inputs pass through [`sanitize_component`] before being embedded
//! in paths. A dedicated `sanitize.rs` module (issue #160) will replace the
//! inline copy here once implemented.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ── Release type ──────────────────────────────────────────────────────────────

/// Music release type.
///
/// Controls the bracketed tag in the release directory name. `Album` is the
/// default — studio albums carry no tag. All other variants insert a `[Tag]`
/// between the year and the title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    /// Studio album — no tag in directory name.
    Album,
    /// Extended play — `[EP]`.
    EP,
    /// Single — `[Single]`.
    Single,
    /// Live recording — `[Live]`.
    Live,
    /// Compilation / various artists — `[Comp]`.
    Compilation,
    /// Original soundtrack — `[OST]`.
    Soundtrack,
}

impl ReleaseType {
    /// Returns the bracketed directory tag, or an empty string for `Album`.
    ///
    /// ```
    /// use kathodos::template::ReleaseType;
    /// assert_eq!(ReleaseType::Album.tag(), "");
    /// assert_eq!(ReleaseType::EP.tag(), "[EP]");
    /// assert_eq!(ReleaseType::Soundtrack.tag(), "[OST]");
    /// ```
    pub fn tag(self) -> &'static str {
        match self {
            Self::Album => "",
            Self::EP => "[EP]",
            Self::Single => "[Single]",
            Self::Live => "[Live]",
            Self::Compilation => "[Comp]",
            Self::Soundtrack => "[OST]",
        }
    }
}

// ── Music ─────────────────────────────────────────────────────────────────────

/// Canonical relative directory path for a music release.
///
/// Format: `{Artist Name}/[{YYYY}] {Title}/`
/// Format with tag: `{Artist Name}/[{YYYY}] [Tag] {Title}/`
///
/// Per the storage layout spec, `Album` releases carry no type tag; all other
/// release types insert the tag between the year bracket and the title.
///
/// # Arguments
///
/// * `artist` — artist name (sanitized before use)
/// * `year` — four-digit release year
/// * `title` — release title (sanitized before use)
/// * `release_type` — controls the optional `[Tag]` in the directory name
pub fn music_release_path(
    artist: &str,
    year: u16,
    title: &str,
    release_type: ReleaseType,
) -> PathBuf {
    let artist = sanitize_component(artist);
    let title = sanitize_component(title);

    let dir_name = match release_type.tag() {
        "" => format!("[{year}] {title}"),
        tag => format!("[{year}] {tag} {title}"),
    };

    PathBuf::from(artist).join(dir_name)
}

/// Canonical filename for a single music track.
///
/// Format: `{disc:02}-{track:02} - {Title}.{ext}`
///
/// Both disc and track numbers are zero-padded to two digits. Single-disc
/// releases still include the `01-` disc prefix.
///
/// # Arguments
///
/// * `disc` — disc number (1-based)
/// * `track` — track number within the disc (1-based)
/// * `title` — track title (sanitized before use)
/// * `ext` — file extension without leading dot (e.g. `"flac"`)
pub fn music_track_filename(disc: u8, track: u8, title: &str, ext: &str) -> String {
    let title = sanitize_component(title);
    format!("{disc:02}-{track:02} - {title}.{ext}")
}

// ── Books ─────────────────────────────────────────────────────────────────────

/// Canonical relative directory path for an ebook.
///
/// Format: `{Author Name}/[{YYYY}] {Title}/`
///
/// # Arguments
///
/// * `author` — author name (sanitized before use)
/// * `year` — publication year
/// * `title` — book title (sanitized before use)
pub fn book_path(author: &str, year: u16, title: &str) -> PathBuf {
    let author = sanitize_component(author);
    let title = sanitize_component(title);
    PathBuf::from(author).join(format!("[{year}] {title}"))
}

// ── Audiobooks ────────────────────────────────────────────────────────────────

/// Canonical relative directory path for an audiobook.
///
/// Format: `{Author Name}/[{YYYY}] {Title}/`
///
/// Identical structure to `book_path`; the two are separate functions because
/// they belong to distinct library roots and the type distinction aids future
/// divergence (e.g. narrator directory level).
///
/// # Arguments
///
/// * `author` — author name (sanitized before use)
/// * `year` — publication year
/// * `title` — book title (sanitized before use)
pub fn audiobook_path(author: &str, year: u16, title: &str) -> PathBuf {
    let author = sanitize_component(author);
    let title = sanitize_component(title);
    PathBuf::from(author).join(format!("[{year}] {title}"))
}

// ── Podcasts ──────────────────────────────────────────────────────────────────

/// Canonical filename for a podcast episode.
///
/// Format: `[{YYYY-MM-DD}] {Episode Title}.{ext}`
///
/// Episodes live directly under the show directory — no per-season
/// subdirectories. The ISO date prefix gives chronological sort in file
/// managers.
///
/// # Arguments
///
/// * `date` — ISO 8601 date string (`"YYYY-MM-DD"`) — passed through as-is
/// * `title` — episode title (sanitized before use)
/// * `ext` — file extension without leading dot (e.g. `"mp3"`)
pub fn podcast_episode_filename(date: &str, title: &str, ext: &str) -> String {
    let title = sanitize_component(title);
    format!("[{date}] {title}.{ext}")
}

// ── Sanitization ──────────────────────────────────────────────────────────────

/// Sanitize a single path component for safe filesystem use.
///
/// Rules (per `docs/data/storage-layout.md` § Path Sanitization):
///
/// 1. Unicode NFC normalization.
/// 2. Replace `/ \ : * ? " < > |` with `-`.
/// 3. Collapse runs of whitespace to a single space.
/// 4. Trim leading and trailing whitespace and dots (also eliminates hidden-file
///    names — a leading dot is always stripped at this step).
/// 5. Truncate to 255 bytes (filesystem limit).
///
/// TODO(#160): replace this inline copy with `crate::sanitize::sanitize_component`
/// once the dedicated sanitize module is implemented.
pub fn sanitize_component(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization as _;

    const UNSAFE: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

    // Step 1: NFC normalize.
    let s: String = s.nfc().collect();

    // Step 2: Replace unsafe characters.
    let s: String = s
        .chars()
        .map(|c| if UNSAFE.contains(&c) { '-' } else { c })
        .collect();

    // Step 3: Collapse whitespace.
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch == ' ' || ch == '\t' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }

    // Step 4: Trim whitespace and dots (this also guarantees no leading dot).
    let result = result.trim_matches(|c: char| c == ' ' || c == '.').to_string();

    // Step 5: Truncate to 255 bytes.
    truncate_to_bytes(result, 255)
}

/// Truncate `s` to at most `max_bytes` without splitting a UTF-8 codepoint.
fn truncate_to_bytes(s: String, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ReleaseType::tag ──────────────────────────────────────────────────────

    #[test]
    fn release_type_album_has_no_tag() {
        assert_eq!(ReleaseType::Album.tag(), "");
    }

    #[test]
    fn release_type_all_tags() {
        assert_eq!(ReleaseType::EP.tag(), "[EP]");
        assert_eq!(ReleaseType::Single.tag(), "[Single]");
        assert_eq!(ReleaseType::Live.tag(), "[Live]");
        assert_eq!(ReleaseType::Compilation.tag(), "[Comp]");
        assert_eq!(ReleaseType::Soundtrack.tag(), "[OST]");
    }

    // ── music_release_path ────────────────────────────────────────────────────

    #[test]
    fn music_release_album_no_tag() {
        let path = music_release_path("Radiohead", 1997, "OK Computer", ReleaseType::Album);
        assert_eq!(path, PathBuf::from("Radiohead/[1997] OK Computer"));
    }

    #[test]
    fn music_release_ep() {
        let path = music_release_path("Radiohead", 1994, "My Iron Lung", ReleaseType::EP);
        assert_eq!(path, PathBuf::from("Radiohead/[1994] [EP] My Iron Lung"));
    }

    #[test]
    fn music_release_single() {
        let path = music_release_path("Björk", 1993, "Human Behaviour", ReleaseType::Single);
        assert_eq!(path, PathBuf::from("Björk/[1993] [Single] Human Behaviour"));
    }

    #[test]
    fn music_release_live() {
        let path = music_release_path("Nirvana", 1996, "From the Muddy Banks", ReleaseType::Live);
        assert_eq!(
            path,
            PathBuf::from("Nirvana/[1996] [Live] From the Muddy Banks")
        );
    }

    #[test]
    fn music_release_compilation() {
        let path =
            music_release_path("Various Artists", 2000, "Now That's Music 45", ReleaseType::Compilation);
        assert_eq!(
            path,
            PathBuf::from("Various Artists/[2000] [Comp] Now That's Music 45")
        );
    }

    #[test]
    fn music_release_soundtrack() {
        let path =
            music_release_path("Ennio Morricone", 1966, "The Good the Bad and the Ugly", ReleaseType::Soundtrack);
        assert_eq!(
            path,
            PathBuf::from(
                "Ennio Morricone/[1966] [OST] The Good the Bad and the Ugly"
            )
        );
    }

    #[test]
    fn music_release_unsafe_chars_in_artist() {
        // Colons and slashes are sanitized to dashes.
        let path = music_release_path("AC/DC", 1980, "Back in Black", ReleaseType::Album);
        assert_eq!(path, PathBuf::from("AC-DC/[1980] Back in Black"));
    }

    #[test]
    fn music_release_unsafe_chars_in_title() {
        let path =
            music_release_path("Artist", 2000, "Song: The Remix", ReleaseType::Album);
        assert_eq!(path, PathBuf::from("Artist/[2000] Song- The Remix"));
    }

    // ── music_track_filename ──────────────────────────────────────────────────

    #[test]
    fn music_track_single_digit_zero_padded() {
        let name = music_track_filename(1, 3, "Come as You Are", "flac");
        assert_eq!(name, "01-03 - Come as You Are.flac");
    }

    #[test]
    fn music_track_two_digit_numbers() {
        let name = music_track_filename(2, 14, "Paranoid Android", "flac");
        assert_eq!(name, "02-14 - Paranoid Android.flac");
    }

    #[test]
    fn music_track_disc_one_prefix_on_single_disc() {
        // Single-disc releases still include the 01- prefix per spec.
        let name = music_track_filename(1, 1, "Intro", "opus");
        assert_eq!(name, "01-01 - Intro.opus");
    }

    #[test]
    fn music_track_unsafe_title_chars() {
        let name = music_track_filename(1, 5, "A/B: Test", "mp3");
        assert_eq!(name, "01-05 - A-B- Test.mp3");
    }

    // ── book_path ─────────────────────────────────────────────────────────────

    #[test]
    fn book_standard_path() {
        let path = book_path("Ursula K. Le Guin", 1969, "The Left Hand of Darkness");
        assert_eq!(
            path,
            PathBuf::from("Ursula K. Le Guin/[1969] The Left Hand of Darkness")
        );
    }

    #[test]
    fn book_special_chars_author() {
        let path = book_path("Cormac McCarthy", 2006, "The Road");
        assert_eq!(path, PathBuf::from("Cormac McCarthy/[2006] The Road"));
    }

    #[test]
    fn book_unsafe_chars_sanitized() {
        let path = book_path("Author: Name", 2000, "Title / Subtitle");
        assert_eq!(
            path,
            PathBuf::from("Author- Name/[2000] Title - Subtitle")
        );
    }

    // ── audiobook_path ────────────────────────────────────────────────────────

    #[test]
    fn audiobook_standard_path() {
        let path = audiobook_path("Frank Herbert", 1965, "Dune");
        assert_eq!(path, PathBuf::from("Frank Herbert/[1965] Dune"));
    }

    #[test]
    fn audiobook_special_chars_in_title() {
        let path = audiobook_path("Author", 2010, "Title: A Novel");
        assert_eq!(path, PathBuf::from("Author/[2010] Title- A Novel"));
    }

    // ── podcast_episode_filename ──────────────────────────────────────────────

    #[test]
    fn podcast_episode_standard() {
        let name = podcast_episode_filename("2026-04-12", "The State of AI", "mp3");
        assert_eq!(name, "[2026-04-12] The State of AI.mp3");
    }

    #[test]
    fn podcast_episode_unsafe_title() {
        let name = podcast_episode_filename("2026-01-01", "Q&A: Your Questions", "mp3");
        assert_eq!(name, "[2026-01-01] Q&A- Your Questions.mp3");
    }

    // ── sanitize_component ────────────────────────────────────────────────────

    #[test]
    fn sanitize_replaces_all_unsafe_with_dash() {
        for ch in &['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
            let input = format!("pre{ch}post");
            let result = sanitize_component(&input);
            assert!(
                !result.contains(*ch),
                "unsafe char {ch:?} should be replaced; got: {result}"
            );
            assert!(result.contains('-'), "should contain dash; got: {result}");
        }
    }

    #[test]
    fn sanitize_collapses_whitespace() {
        assert_eq!(sanitize_component("hello   world"), "hello world");
        assert_eq!(sanitize_component("  trim me  "), "trim me");
    }

    #[test]
    fn sanitize_trims_leading_trailing_dots() {
        // Leading dots are trimmed (not hidden-file preserved).
        assert_eq!(sanitize_component(".hidden"), "hidden");
        assert_eq!(sanitize_component("file."), "file");
        assert_eq!(sanitize_component("...dots..."), "dots");
    }

    #[test]
    fn sanitize_no_leading_dot_in_output() {
        // The trim step guarantees no leading dot survives.
        let result = sanitize_component(".dotfile");
        assert!(!result.starts_with('.'), "should not start with dot; got: {result}");
    }

    #[test]
    fn sanitize_max_component_length_255_bytes() {
        let long: String = "a".repeat(300);
        let result = sanitize_component(&long);
        assert!(
            result.len() <= 255,
            "should be truncated to 255 bytes; len={}", result.len()
        );
    }

    #[test]
    fn sanitize_nfc_normalization_preserved() {
        // "é" as decomposed (e + combining acute) should normalize to precomposed.
        let decomposed = "e\u{0301}"; // e + combining acute accent
        let result = sanitize_component(decomposed);
        assert_eq!(result, "\u{00e9}"); // é precomposed
    }

    #[test]
    fn sanitize_empty_string_stays_empty() {
        assert_eq!(sanitize_component(""), "");
    }

    // ── year formatting ───────────────────────────────────────────────────────

    #[test]
    fn year_formatted_as_four_digits_in_brackets() {
        let path = music_release_path("Artist", 500, "Title", ReleaseType::Album);
        // u16 formats as-is; year 500 becomes [500] not [0500].
        assert!(path.to_string_lossy().contains("[500]"));
    }

    #[test]
    fn year_2026_formats_correctly() {
        let path = book_path("Author", 2026, "Title");
        assert!(path.to_string_lossy().contains("[2026]"));
    }
}
