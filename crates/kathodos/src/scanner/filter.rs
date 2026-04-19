use std::path::{Path, PathBuf};

use themelion::MediaType;

static MUSIC_EXTENSIONS: &[&str] = &[
    "flac", "wav", "mp3", "m4a", "ogg", "opus", "aiff", "aif", "wv", "alac", "aac",
];
static AUDIOBOOK_EXTENSIONS: &[&str] = &["m4b", "mp3", "m4a", "flac"];
static BOOK_EXTENSIONS: &[&str] = &["epub", "pdf", "mobi", "azw3", "cbz", "cbr"];
static COMIC_EXTENSIONS: &[&str] = &["cbz", "cbr", "cb7", "pdf"];
static MOVIE_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "m4v"];
static TV_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "m4v"];
static PODCAST_EXTENSIONS: &[&str] = &["mp3", "m4a", "ogg", "opus"];

/// Returns true if this file extension is supported for the given media type.
pub(crate) fn is_supported_extension(path: &Path, media_type: MediaType) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_ascii_lowercase(),
        None => return false,
    };
    let exts: &[&str] = match media_type {
        MediaType::Music => MUSIC_EXTENSIONS,
        MediaType::Audiobook => AUDIOBOOK_EXTENSIONS,
        MediaType::Book => BOOK_EXTENSIONS,
        MediaType::Comic => COMIC_EXTENSIONS,
        MediaType::Movie => MOVIE_EXTENSIONS,
        MediaType::Tv => TV_EXTENSIONS,
        MediaType::Podcast => PODCAST_EXTENSIONS,
        MediaType::News => &[],
        _ => &[],
    };
    exts.contains(&ext.as_str())
}

/// Returns all media types for which this extension is supported.
pub fn extension_media_types(path: &Path) -> Vec<MediaType> {
    if path.extension().is_none() {
        return vec![];
    }
    let mut types = Vec::new();
    for mt in [
        MediaType::Music,
        MediaType::Audiobook,
        MediaType::Book,
        MediaType::Comic,
        MediaType::Movie,
        MediaType::Tv,
        MediaType::Podcast,
    ] {
        if is_supported_extension(path, mt) {
            types.push(mt);
        }
    }
    types
}

/// Represents loaded .harmoniaignore rules for a library root.
pub(crate) struct HarmoniaIgnore {
    rules: Vec<IgnoreRule>,
    root: PathBuf,
}

#[derive(Debug)]
struct IgnoreRule {
    pattern: IgnorePattern,
    negated: bool,
}

#[derive(Debug)]
enum IgnorePattern {
    Glob {
        prefix: Option<String>,
        suffix: Option<String>,
        is_dir: bool,
    },
    Exact(String),
    All,
}

impl HarmoniaIgnore {
    /// Load .harmoniaignore from root directory only.
    pub(crate) fn load(root: &Path) -> Self {
        let rules = Self::load_file(&root.join(".harmoniaignore")).unwrap_or_default();
        Self {
            rules,
            root: root.to_path_buf(),
        }
    }

    fn load_file(path: &Path) -> Option<Vec<IgnoreRule>> {
        let content = std::fs::read_to_string(path).ok()?;
        let rules = content
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .filter_map(|line| parse_rule(line.trim()))
            .collect();
        Some(rules)
    }

    pub(crate) fn is_ignored(&self, path: &Path) -> bool {
        let rel = path.strip_prefix(&self.root).unwrap_or(path);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_dir = path.is_dir();

        let mut ignored = false;
        for rule in &self.rules {
            if rule.negated {
                if matches_rule(&rule.pattern, rel, name, is_dir) {
                    ignored = false;
                }
            } else if matches_rule(&rule.pattern, rel, name, is_dir) {
                ignored = true;
            }
        }
        ignored
    }
}

fn parse_rule(line: &str) -> Option<IgnoreRule> {
    let (negated, line) = if let Some(rest) = line.strip_prefix('!') {
        (true, rest)
    } else {
        (false, line)
    };
    let is_dir = line.ends_with('/');
    let line = line.trim_end_matches('/');

    if line.is_empty() {
        return None;
    }

    let pattern = if line == "*" {
        IgnorePattern::All
    } else if let Some(suffix) = line.strip_prefix('*') {
        IgnorePattern::Glob {
            prefix: None,
            suffix: Some(suffix.to_string()),
            is_dir,
        }
    } else if let Some(prefix) = line.strip_suffix('*') {
        IgnorePattern::Glob {
            prefix: Some(prefix.to_string()),
            suffix: None,
            is_dir,
        }
    } else {
        IgnorePattern::Exact(line.to_string())
    };

    Some(IgnoreRule { pattern, negated })
}

fn matches_rule(pattern: &IgnorePattern, rel_path: &Path, name: &str, is_dir: bool) -> bool {
    match pattern {
        IgnorePattern::All => true,
        IgnorePattern::Exact(s) => {
            name == s.as_str() || rel_path.to_str().is_some_and(|p| p == s.as_str())
        }
        IgnorePattern::Glob {
            prefix,
            suffix,
            is_dir: pat_is_dir,
        } => {
            if *pat_is_dir && !is_dir {
                return false;
            }
            match (prefix, suffix) {
                (None, Some(suf)) => name.ends_with(suf.as_str()),
                (Some(pre), None) => name.starts_with(pre.as_str()),
                (Some(pre), Some(suf)) => {
                    name.starts_with(pre.as_str()) && name.ends_with(suf.as_str())
                }
                (None, None) => true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn make_path(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn music_extensions_accepted() {
        for ext in &["flac", "mp3", "wav", "m4a", "ogg", "opus"] {
            let path = make_path(&format!("track.{ext}"));
            assert!(
                is_supported_extension(&path, MediaType::Music),
                "{ext} should be accepted for Music"
            );
        }
    }

    #[test]
    fn unknown_extension_rejected_for_music() {
        let path = make_path("track.xyz");
        assert!(!is_supported_extension(&path, MediaType::Music));
    }

    #[test]
    fn audiobook_extensions_accepted() {
        for ext in &["m4b", "mp3", "m4a", "flac"] {
            let path = make_path(&format!("book.{ext}"));
            assert!(
                is_supported_extension(&path, MediaType::Audiobook),
                "{ext} should be accepted for Audiobook"
            );
        }
    }

    #[test]
    fn book_extensions_accepted() {
        for ext in &["epub", "pdf", "mobi", "azw3"] {
            let path = make_path(&format!("book.{ext}"));
            assert!(
                is_supported_extension(&path, MediaType::Book),
                "{ext} should be accepted for Book"
            );
        }
    }

    #[test]
    fn comic_extensions_accepted() {
        for ext in &["cbz", "cbr", "cb7"] {
            let path = make_path(&format!("issue.{ext}"));
            assert!(
                is_supported_extension(&path, MediaType::Comic),
                "{ext} should be accepted for Comic"
            );
        }
    }

    #[test]
    fn video_extensions_accepted() {
        for ext in &["mkv", "mp4", "avi", "m4v"] {
            let path = make_path(&format!("movie.{ext}"));
            assert!(
                is_supported_extension(&path, MediaType::Movie),
                "{ext} should be accepted for Movie"
            );
        }
    }

    #[test]
    fn news_type_has_no_extensions() {
        let path = make_path("article.mp3");
        assert!(!is_supported_extension(&path, MediaType::News));
    }

    #[test]
    fn extension_case_insensitive() {
        let path = make_path("track.FLAC");
        assert!(is_supported_extension(&path, MediaType::Music));
        let path = make_path("track.Mp3");
        assert!(is_supported_extension(&path, MediaType::Music));
    }

    #[test]
    fn harmoniaignore_matches_glob_pattern() {
        let dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(dir.path().join(".harmoniaignore")).unwrap();
        writeln!(f, "*.part").unwrap();
        writeln!(f, "*.tmp").unwrap();

        let ignore = HarmoniaIgnore::load(dir.path());
        assert!(ignore.is_ignored(&dir.path().join("download.part")));
        assert!(ignore.is_ignored(&dir.path().join("file.tmp")));
        assert!(!ignore.is_ignored(&dir.path().join("track.flac")));
    }

    #[test]
    fn harmoniaignore_exact_match() {
        let dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(dir.path().join(".harmoniaignore")).unwrap();
        writeln!(f, "Thumbs.db").unwrap();

        let ignore = HarmoniaIgnore::load(dir.path());
        assert!(ignore.is_ignored(&dir.path().join("Thumbs.db")));
        assert!(!ignore.is_ignored(&dir.path().join("track.flac")));
    }

    #[test]
    fn harmoniaignore_comments_ignored() {
        let dir = TempDir::new().unwrap();
        let mut f = std::fs::File::create(dir.path().join(".harmoniaignore")).unwrap();
        writeln!(f, "# this is a comment").unwrap();
        writeln!(f, "*.part").unwrap();

        let ignore = HarmoniaIgnore::load(dir.path());
        assert!(ignore.is_ignored(&dir.path().join("file.part")));
        assert!(!ignore.is_ignored(&dir.path().join("# this is a comment")));
    }

    #[test]
    fn harmoniaignore_missing_file_no_error() {
        let dir = TempDir::new().unwrap();
        let ignore = HarmoniaIgnore::load(dir.path());
        assert!(!ignore.is_ignored(&dir.path().join("track.flac")));
    }

    #[test]
    fn extension_media_types_returns_all_applicable() {
        let path = make_path("episode.mp3");
        let types = extension_media_types(&path);
        assert!(types.contains(&MediaType::Music));
        assert!(types.contains(&MediaType::Podcast));
    }
}
