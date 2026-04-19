use std::path::{Path, PathBuf};

use jiff::Zoned;
use snafu::ResultExt;
use walkdir::{DirEntry, WalkDir};

use crate::cli::{CliMediaType, MigrateArgs};
use crate::error::{HostError, MigrateIoSnafu, MigrateSourceMissingSnafu};

/// Summary of a completed migration run.
#[derive(Debug, Default)]
pub struct MigrationReport {
    pub processed: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// Entry point for `harmonia migrate`.
pub async fn run_migrate(args: MigrateArgs) -> Result<(), HostError> {
    if !args.source.exists() {
        return MigrateSourceMissingSnafu { path: args.source }.fail();
    }

    let media_type = args.media_type.clone();
    let source = args.source.clone();
    let target = args.target.clone();
    let dry_run = args.dry_run;
    let copy = args.copy;

    let report = tokio::task::spawn_blocking(move || {
        migrate_blocking(&source, &target, &media_type, dry_run, copy)
    })
    .await
    .map_err(|e| HostError::MigrateIo {
        operation: "spawn_blocking".into(),
        source: std::io::Error::other(e.to_string()),
        location: snafu::location!(),
    })
    .and_then(|r| r)?;

    if dry_run {
        println!("Dry run — no files were moved or copied.");
    }
    println!(
        "Migration complete: {} processed, {} skipped, {} errors",
        report.processed, report.skipped, report.errors
    );

    Ok(())
}

fn migrate_blocking(
    source: &Path,
    target: &Path,
    media_type: &CliMediaType,
    dry_run: bool,
    copy: bool,
) -> Result<MigrationReport, HostError> {
    let mut report = MigrationReport::default();
    let imported_at = Zoned::now().to_string();

    for entry in WalkDir::new(source).follow_links(false).into_iter() {
        let entry: DirEntry = entry.map_err(|e| {
            let path = e.path().unwrap_or(source).to_path_buf();
            HostError::MigrateIo {
                operation: format!("walk {}", path.display()),
                source: e.into_io_error().unwrap_or_else(|| {
                    std::io::Error::other("walkdir error without underlying IO error")
                }),
                location: snafu::location!(),
            }
        })?;

        if entry.file_type().is_dir() {
            continue;
        }

        let file_path = entry.into_path();

        if !is_media_file(&file_path, media_type) {
            report.skipped += 1;
            continue;
        }

        let rel = file_path.strip_prefix(source).unwrap_or(&file_path);

        let outcome = match media_type {
            CliMediaType::Music => migrate_music_file(source, rel, &imported_at),
            CliMediaType::Books => migrate_book_file(source, rel, &imported_at),
            CliMediaType::Audiobooks => migrate_audiobook_file(source, rel, &imported_at),
            CliMediaType::Podcasts => migrate_podcast_file(source, rel, &imported_at),
        };

        let (canonical_path, sidecar) = match outcome {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("skipping {}: {e}", file_path.display());
                report.errors += 1;
                continue;
            }
        };

        let canonical_abs = target.join(&canonical_path);
        let sidecar_abs = canonical_abs.parent().map(|p| {
            let name = sidecar_filename(media_type);
            p.join(name)
        });

        if dry_run {
            println!("{} -> {}", file_path.display(), canonical_abs.display());
            if let Some(ref sc) = sidecar_abs {
                println!("  sidecar -> {}", sc.display());
            }
            report.processed += 1;
            continue;
        }

        // Create parent directories.
        if let Some(parent) = canonical_abs.parent() {
            std::fs::create_dir_all(parent).with_context(|_| MigrateIoSnafu {
                operation: format!("create_dir_all {}", parent.display()),
            })?;
        }

        // Move or copy the file.
        if copy {
            std::fs::copy(&file_path, &canonical_abs).with_context(|_| MigrateIoSnafu {
                operation: format!(
                    "copy {} -> {}",
                    file_path.display(),
                    canonical_abs.display()
                ),
            })?;
        } else {
            match std::fs::rename(&file_path, &canonical_abs) {
                Ok(()) => {}
                Err(e) if e.raw_os_error() == Some(18) => {
                    // EXDEV: cross-device move — fall back to copy + delete.
                    std::fs::copy(&file_path, &canonical_abs).with_context(|_| MigrateIoSnafu {
                        operation: format!(
                            "copy (cross-device) {} -> {}",
                            file_path.display(),
                            canonical_abs.display()
                        ),
                    })?;
                    let _ = std::fs::remove_file(&file_path);
                }
                Err(e) => {
                    return Err(HostError::MigrateIo {
                        operation: format!(
                            "rename {} -> {}",
                            file_path.display(),
                            canonical_abs.display()
                        ),
                        source: e,
                        location: snafu::location!(),
                    });
                }
            }
        }

        // Write sidecar if it doesn't already exist.
        if let (Some(sc_path), Some(content)) = (sidecar_abs, sidecar)
            && !sc_path.exists()
        {
            std::fs::write(&sc_path, content).with_context(|_| MigrateIoSnafu {
                operation: format!("write sidecar {}", sc_path.display()),
            })?;
        }

        report.processed += 1;
    }

    Ok(report)
}

// ── Per-type migration functions ──────────────────────────────────────────────

/// Returns `(canonical_relative_path, Option<sidecar_toml_content>)`.
///
/// Music canonical layout: `{Artist}/[{YYYY}] {Album}/{disc}-{track} - {Title}.{ext}`
fn migrate_music_file(
    source_root: &Path,
    rel: &Path,
    imported_at: &str,
) -> Result<(PathBuf, Option<String>), HostError> {
    let components: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let filename = *components.last().ok_or_else(|| HostError::MigratePathUnparseable {
        path: source_root.join(rel),
        location: snafu::location!(),
    })?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("flac");

    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    let (track_num, track_title) = parse_track_stem(stem);

    let (artist, album, year) = match components.len() {
        1 => ("Unknown Artist".to_string(), "Unknown Album".to_string(), None),
        2 => (sanitize_owned(components[0]), "Unknown Album".to_string(), None),
        _ => {
            let artist = sanitize_owned(components[0]);
            let (album, year) = extract_year_from_str(&sanitize_owned(components[1]));
            (artist, album, year)
        }
    };

    let dir = match year {
        Some(ref y) => format!("{artist}/[{y}] {album}"),
        None => format!("{artist}/{album}"),
    };

    let canonical_filename = match track_num {
        Some(n) => format!("01-{n:0>2} - {track_title}.{ext}"),
        None => format!("{track_title}.{ext}"),
    };

    let canonical_path = PathBuf::from(format!("{dir}/{canonical_filename}"));

    let album_toml = format!(
        "[meta]\n\
         source = \"path-infer\"\n\
         imported_at = \"{imported_at}\"\n\
         \n\
         [album]\n\
         artist = {artist:?}\n\
         title = {album:?}\n"
    );

    Ok((canonical_path, Some(album_toml)))
}

/// Book canonical layout: `{Author}/[{YYYY}] {Title}/{Title}.{ext}`
fn migrate_book_file(
    source_root: &Path,
    rel: &Path,
    imported_at: &str,
) -> Result<(PathBuf, Option<String>), HostError> {
    let components: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let filename = *components.last().ok_or_else(|| HostError::MigratePathUnparseable {
        path: source_root.join(rel),
        location: snafu::location!(),
    })?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("epub");

    let (author, title, year) = match components.len() {
        1 => {
            let stem = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(filename);
            ("Unknown Author".to_string(), sanitize_owned(stem), None)
        }
        2 => {
            let author = sanitize_owned(components[0]);
            let stem = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(filename);
            let (title, year) = extract_year_from_str(&sanitize_owned(stem));
            (author, title, year)
        }
        _ => {
            let author = sanitize_owned(components[0]);
            let (title, year) = extract_year_from_str(&sanitize_owned(components[1]));
            (author, title, year)
        }
    };

    let dir = match year {
        Some(ref y) => format!("{author}/[{y}] {title}"),
        None => format!("{author}/{title}"),
    };

    let canonical_path = PathBuf::from(format!("{dir}/{title}.{ext}"));

    let toml = format!(
        "[meta]\n\
         source = \"path-infer\"\n\
         imported_at = \"{imported_at}\"\n\
         \n\
         [book]\n\
         author = {author:?}\n\
         title = {title:?}\n"
    );

    Ok((canonical_path, Some(toml)))
}

/// Audiobook canonical layout: `{Author}/[{YYYY}] {Title}/{Title}.{ext}`
fn migrate_audiobook_file(
    source_root: &Path,
    rel: &Path,
    imported_at: &str,
) -> Result<(PathBuf, Option<String>), HostError> {
    let components: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let filename = *components.last().ok_or_else(|| HostError::MigratePathUnparseable {
        path: source_root.join(rel),
        location: snafu::location!(),
    })?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("m4b");

    let (author, title, year) = match components.len() {
        1 => {
            let stem = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(filename);
            ("Unknown Author".to_string(), sanitize_owned(stem), None)
        }
        2 => {
            let author = sanitize_owned(components[0]);
            let stem = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(filename);
            let (title, year) = extract_year_from_str(&sanitize_owned(stem));
            (author, title, year)
        }
        _ => {
            let author = sanitize_owned(components[0]);
            let (title, year) = extract_year_from_str(&sanitize_owned(components[1]));
            (author, title, year)
        }
    };

    let dir = match year {
        Some(ref y) => format!("{author}/[{y}] {title}"),
        None => format!("{author}/{title}"),
    };

    let canonical_path = PathBuf::from(format!("{dir}/{title}.{ext}"));

    let toml = format!(
        "[meta]\n\
         source = \"path-infer\"\n\
         imported_at = \"{imported_at}\"\n\
         \n\
         [audiobook]\n\
         author = {author:?}\n\
         title = {title:?}\n"
    );

    Ok((canonical_path, Some(toml)))
}

/// Podcast canonical layout: `{Show}/[{YYYY-MM-DD}] {Episode Title}.{ext}`
fn migrate_podcast_file(
    source_root: &Path,
    rel: &Path,
    imported_at: &str,
) -> Result<(PathBuf, Option<String>), HostError> {
    let components: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let filename = *components.last().ok_or_else(|| HostError::MigratePathUnparseable {
        path: source_root.join(rel),
        location: snafu::location!(),
    })?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3");

    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    let show = if components.len() >= 2 {
        sanitize_owned(components[0])
    } else {
        "Unknown Show".to_string()
    };

    let (date, episode_title) = parse_date_stem(stem);

    let canonical_filename = match date {
        Some(ref d) => format!("[{d}] {episode_title}.{ext}"),
        None => format!("{episode_title}.{ext}"),
    };

    let canonical_path = PathBuf::from(format!("{show}/{canonical_filename}"));

    let toml = format!(
        "[meta]\n\
         source = \"path-infer\"\n\
         imported_at = \"{imported_at}\"\n\
         \n\
         [show]\n\
         title = {show:?}\n"
    );

    Ok((canonical_path, Some(toml)))
}

// ── Path parsing helpers ──────────────────────────────────────────────────────

/// Parse track number and title from a stem.
///
/// Handles:
/// - `"01 - Come as You Are"` → `(Some(1), "Come as You Are")`
/// - `"03. Tremolo"` → `(Some(3), "Tremolo")`
/// - `"Track 03 - Name"` → `(Some(3), "Name")`
/// - `"Just a Title"` → `(None, "Just a Title")`
fn parse_track_stem(stem: &str) -> (Option<u32>, String) {
    let stem = stem
        .strip_prefix("Track ")
        .or_else(|| stem.strip_prefix("track "))
        .unwrap_or(stem);

    let mut chars = stem.chars().peekable();
    let mut num_str = String::new();
    while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
        num_str.push(chars.next().unwrap());
    }

    if !num_str.is_empty() {
        let rest: String = chars.collect();
        let rest = rest
            .strip_prefix(" - ")
            .or_else(|| rest.strip_prefix(". "))
            .or_else(|| rest.strip_prefix(' '))
            .unwrap_or(rest.as_str());
        if !rest.is_empty()
            && let Ok(n) = num_str.parse::<u32>()
        {
            return (Some(n), sanitize_owned(rest));
        }
    }

    (None, sanitize_owned(stem))
}

/// Parse a date prefix from a podcast episode stem.
///
/// Handles:
/// - `"2024-03-15 - Title"` → `(Some("2024-03-15"), "Title")`
/// - `"20240315 - Title"` → `(Some("2024-03-15"), "Title")`
/// - `"Episode Title"` → `(None, "Episode Title")`
fn parse_date_stem(stem: &str) -> (Option<String>, String) {
    if stem.len() >= 10 && looks_like_iso_date(&stem[..10]) {
        let rest = stem[10..]
            .strip_prefix(" - ")
            .or_else(|| stem[10..].strip_prefix(' '))
            .unwrap_or(stem[10..].trim());
        let title = if rest.is_empty() { stem } else { rest };
        return (Some(stem[..10].to_string()), sanitize_owned(title));
    }

    if stem.len() >= 8 && stem[..8].chars().all(|c| c.is_ascii_digit()) {
        let formatted = format!(
            "{}-{}-{}",
            &stem[..4],
            &stem[4..6],
            &stem[6..8]
        );
        let rest = stem[8..]
            .strip_prefix(" - ")
            .or_else(|| stem[8..].strip_prefix(' '))
            .unwrap_or(stem[8..].trim());
        let title = if rest.is_empty() { stem } else { rest };
        return (Some(formatted), sanitize_owned(title));
    }

    (None, sanitize_owned(stem))
}

fn looks_like_iso_date(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let b = s.as_bytes();
    b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..10].iter().all(|c| c.is_ascii_digit())
}

/// Extract a 4-digit year from common label patterns, returning `(cleaned_label, Option<year>)`.
///
/// Handles:
/// - `"[2020] Album Title"` → `("Album Title", Some("2020"))`
/// - `"Album Title (2020)"` → `("Album Title", Some("2020"))`
/// - `"Album Title - 2020"` → `("Album Title", Some("2020"))`
fn extract_year_from_str(label: &str) -> (String, Option<String>) {
    // Pattern: "[YYYY] rest"
    if let Some(rest) = label.strip_prefix('[')
        && let Some(bracket_end) = rest.find(']')
    {
        let year_candidate = &rest[..bracket_end];
        if is_year(year_candidate) {
            return (
                rest[bracket_end + 1..].trim().to_string(),
                Some(year_candidate.to_string()),
            );
        }
    }

    // Pattern: "rest (YYYY)"
    if label.ends_with(')')
        && let Some(paren_start) = label.rfind('(')
    {
        let year_candidate = &label[paren_start + 1..label.len() - 1];
        if is_year(year_candidate) {
            return (
                label[..paren_start].trim().to_string(),
                Some(year_candidate.to_string()),
            );
        }
    }

    // Pattern: "rest - YYYY"
    if let Some(dash_pos) = label.rfind(" - ") {
        let year_candidate = label[dash_pos + 3..].trim();
        if is_year(year_candidate) {
            return (label[..dash_pos].trim().to_string(), Some(year_candidate.to_string()));
        }
    }

    (label.to_string(), None)
}

fn is_year(s: &str) -> bool {
    s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) && ("1000"..="2100").contains(&s)
}

/// Sanitize a path component: replace filesystem-unsafe characters, collapse whitespace.
///
/// Mirrors `kathodos::import::template::sanitize_path_segment`.
fn sanitize_owned(s: &str) -> String {
    const UNSAFE: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let replaced: String = s
        .chars()
        .map(|c| if UNSAFE.contains(&c) { '_' } else { c })
        .collect();
    let mut result = String::with_capacity(replaced.len());
    let mut prev_space = false;
    for ch in replaced.trim().chars() {
        if ch == ' ' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result
}

/// Returns true if the file extension is supported for the given media type.
fn is_media_file(path: &Path, media_type: &CliMediaType) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_ascii_lowercase(),
        None => return false,
    };
    let supported: &[&str] = match media_type {
        CliMediaType::Music => &[
            "flac", "wav", "mp3", "m4a", "ogg", "opus", "aiff", "aif", "wv", "alac", "aac",
        ],
        CliMediaType::Books => &["epub", "pdf", "mobi", "azw3"],
        CliMediaType::Audiobooks => &["m4b", "mp3", "m4a", "flac"],
        CliMediaType::Podcasts => &["mp3", "m4a", "ogg", "opus"],
    };
    supported.contains(&ext.as_str())
}

/// Return the sidecar filename for the given media type.
fn sidecar_filename(media_type: &CliMediaType) -> &'static str {
    match media_type {
        CliMediaType::Music => "album.toml",
        CliMediaType::Books => "book.toml",
        CliMediaType::Audiobooks => "audiobook.toml",
        CliMediaType::Podcasts => "show.toml",
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    // ── Unit tests for parsing helpers ──────────────────────────────────────

    #[test]
    fn parse_track_stem_with_dash() {
        let (num, title) = parse_track_stem("01 - Come as You Are");
        assert_eq!(num, Some(1));
        assert_eq!(title, "Come as You Are");
    }

    #[test]
    fn parse_track_stem_with_dot() {
        let (num, title) = parse_track_stem("03. Tremolo");
        assert_eq!(num, Some(3));
        assert_eq!(title, "Tremolo");
    }

    #[test]
    fn parse_track_stem_no_number() {
        let (num, title) = parse_track_stem("Just a Title");
        assert!(num.is_none());
        assert_eq!(title, "Just a Title");
    }

    #[test]
    fn parse_track_stem_track_prefix() {
        let (num, title) = parse_track_stem("Track 03 - Name");
        assert_eq!(num, Some(3));
        assert_eq!(title, "Name");
    }

    #[test]
    fn extract_year_bracket_prefix() {
        let (label, year) = extract_year_from_str("[2020] Elisabeth");
        assert_eq!(label, "Elisabeth");
        assert_eq!(year, Some("2020".into()));
    }

    #[test]
    fn extract_year_trailing_paren() {
        let (label, year) = extract_year_from_str("Led Zeppelin IV (1971)");
        assert_eq!(label, "Led Zeppelin IV");
        assert_eq!(year, Some("1971".into()));
    }

    #[test]
    fn extract_year_dash_suffix() {
        let (label, year) = extract_year_from_str("Nevermind - 1991");
        assert_eq!(label, "Nevermind");
        assert_eq!(year, Some("1991".into()));
    }

    #[test]
    fn extract_year_no_year() {
        let (label, year) = extract_year_from_str("Unknown Album");
        assert_eq!(label, "Unknown Album");
        assert!(year.is_none());
    }

    #[test]
    fn is_year_valid() {
        assert!(is_year("2020"));
        assert!(is_year("1991"));
        assert!(!is_year("999"));
        assert!(!is_year("2101"));
        assert!(!is_year("abcd"));
    }

    #[test]
    fn looks_like_iso_date_valid() {
        assert!(looks_like_iso_date("2024-03-15"));
        assert!(!looks_like_iso_date("20240315"));
        assert!(!looks_like_iso_date("not-a-date"));
    }

    #[test]
    fn parse_date_stem_iso() {
        let (date, title) = parse_date_stem("2024-03-15 - My Episode");
        assert_eq!(date, Some("2024-03-15".into()));
        assert_eq!(title, "My Episode");
    }

    #[test]
    fn parse_date_stem_compact() {
        let (date, title) = parse_date_stem("20240315 - My Episode");
        assert_eq!(date, Some("2024-03-15".into()));
        assert_eq!(title, "My Episode");
    }

    #[test]
    fn parse_date_stem_no_date() {
        let (date, title) = parse_date_stem("Episode Title");
        assert!(date.is_none());
        assert_eq!(title, "Episode Title");
    }

    #[test]
    fn sanitize_owned_replaces_unsafe() {
        let result = sanitize_owned("AC/DC: Rock & Roll");
        assert!(!result.contains('/'));
        assert!(!result.contains(':'));
    }

    #[test]
    fn sanitize_owned_collapses_spaces() {
        assert_eq!(sanitize_owned("  hello   world  "), "hello world");
    }

    #[test]
    fn is_media_file_music() {
        assert!(is_media_file(Path::new("track.flac"), &CliMediaType::Music));
        assert!(is_media_file(Path::new("track.mp3"), &CliMediaType::Music));
        assert!(!is_media_file(Path::new("cover.jpg"), &CliMediaType::Music));
    }

    #[test]
    fn is_media_file_books() {
        assert!(is_media_file(Path::new("book.epub"), &CliMediaType::Books));
        assert!(!is_media_file(Path::new("book.m4b"), &CliMediaType::Books));
    }

    #[test]
    fn sidecar_filename_by_type() {
        assert_eq!(sidecar_filename(&CliMediaType::Music), "album.toml");
        assert_eq!(sidecar_filename(&CliMediaType::Books), "book.toml");
        assert_eq!(sidecar_filename(&CliMediaType::Audiobooks), "audiobook.toml");
        assert_eq!(sidecar_filename(&CliMediaType::Podcasts), "show.toml");
    }

    // ── Per-type migration unit tests ───────────────────────────────────────

    #[test]
    fn migrate_music_two_level_path() {
        let source = Path::new("/music");
        let rel = Path::new("Nirvana/01 - Come as You Are.flac");
        let (canonical, sidecar) =
            migrate_music_file(source, rel, "2026-01-01T00:00:00Z").unwrap();
        let s = canonical.to_string_lossy();
        assert!(s.starts_with("Nirvana/"), "got: {s}");
        assert!(s.ends_with("Come as You Are.flac"), "got: {s}");
        let toml = sidecar.unwrap();
        assert!(toml.contains("artist = \"Nirvana\""));
        assert!(toml.contains("source = \"path-infer\""));
    }

    #[test]
    fn migrate_music_three_level_path_with_year() {
        let source = Path::new("/music");
        let rel = Path::new("Nirvana/Nevermind (1991)/01 - Smells Like Teen Spirit.flac");
        let (canonical, sidecar) =
            migrate_music_file(source, rel, "2026-01-01T00:00:00Z").unwrap();
        let s = canonical.to_string_lossy();
        assert!(s.contains("1991"), "year should appear in path, got: {s}");
        assert!(s.contains("Nevermind"), "album should appear, got: {s}");
        let toml = sidecar.unwrap();
        assert!(toml.contains("source = \"path-infer\""));
    }

    #[test]
    fn migrate_podcast_iso_date_prefix() {
        let source = Path::new("/podcasts");
        let rel = Path::new("My Show/2024-03-15 - Great Episode.mp3");
        let (canonical, _) =
            migrate_podcast_file(source, rel, "2026-01-01T00:00:00Z").unwrap();
        let s = canonical.to_string_lossy();
        assert!(s.contains("[2024-03-15]"), "date should be bracketed, got: {s}");
        assert!(s.contains("Great Episode"), "title should be preserved, got: {s}");
    }

    #[test]
    fn migrate_book_author_title() {
        let source = Path::new("/books");
        let rel = Path::new("Tolkien/The Fellowship of the Ring.epub");
        let (canonical, sidecar) =
            migrate_book_file(source, rel, "2026-01-01T00:00:00Z").unwrap();
        let s = canonical.to_string_lossy();
        assert!(s.starts_with("Tolkien/"), "got: {s}");
        assert!(s.ends_with(".epub"), "got: {s}");
        let toml = sidecar.unwrap();
        assert!(toml.contains("[book]"));
    }

    // ── Integration tests against the filesystem ────────────────────────────

    #[test]
    fn migration_dry_run_does_not_write() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        let artist_dir = src.path().join("Radiohead").join("OK Computer (1997)");
        std::fs::create_dir_all(&artist_dir).unwrap();
        std::fs::write(artist_dir.join("01 - Airbag.flac"), b"FAKE").unwrap();

        let report = migrate_blocking(
            src.path(),
            dst.path(),
            &CliMediaType::Music,
            true,
            false,
        )
        .unwrap();

        assert_eq!(report.processed, 1);
        assert_eq!(report.errors, 0);
        // Dry run: nothing written to dst.
        assert!(!dst.path().join("Radiohead").exists());
    }

    #[test]
    fn migration_copy_mode_preserves_source() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        let artist_dir = src.path().join("Radiohead").join("OK Computer (1997)");
        std::fs::create_dir_all(&artist_dir).unwrap();
        let src_file = artist_dir.join("01 - Airbag.flac");
        std::fs::write(&src_file, b"FAKE").unwrap();

        let report = migrate_blocking(
            src.path(),
            dst.path(),
            &CliMediaType::Music,
            false,
            true,
        )
        .unwrap();

        assert_eq!(report.processed, 1);
        assert_eq!(report.errors, 0);
        // Source preserved in copy mode.
        assert!(src_file.exists(), "source file should still exist after copy");
        // Destination has a flac file.
        let dst_has_flac = WalkDir::new(dst.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("flac"));
        assert!(dst_has_flac, "expected flac file in dst");
    }

    #[test]
    fn migration_move_mode_removes_source() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        let artist_dir = src.path().join("Radiohead");
        std::fs::create_dir_all(&artist_dir).unwrap();
        let src_file = artist_dir.join("01 - Karma Police.flac");
        std::fs::write(&src_file, b"FAKE").unwrap();

        let report = migrate_blocking(
            src.path(),
            dst.path(),
            &CliMediaType::Music,
            false,
            false,
        )
        .unwrap();

        assert_eq!(report.processed, 1);
        assert_eq!(report.errors, 0);
        // Source removed after move.
        assert!(!src_file.exists(), "source file should be gone after move");
    }

    #[test]
    fn migration_writes_sidecar() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        let artist_dir = src.path().join("Radiohead").join("OK Computer (1997)");
        std::fs::create_dir_all(&artist_dir).unwrap();
        std::fs::write(artist_dir.join("01 - Airbag.flac"), b"FAKE").unwrap();

        migrate_blocking(
            src.path(),
            dst.path(),
            &CliMediaType::Music,
            false,
            true,
        )
        .unwrap();

        // Find album.toml in dst
        let sidecar_found = WalkDir::new(dst.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name() == "album.toml");
        assert!(sidecar_found, "album.toml sidecar should be written");
    }

    #[test]
    fn migration_skips_non_media_files() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        std::fs::write(src.path().join("cover.jpg"), b"IMG").unwrap();
        std::fs::write(src.path().join("track.flac"), b"FLAC").unwrap();

        let report = migrate_blocking(
            src.path(),
            dst.path(),
            &CliMediaType::Music,
            false,
            true,
        )
        .unwrap();

        assert_eq!(report.skipped, 1);
        assert_eq!(report.processed, 1);
    }
}
