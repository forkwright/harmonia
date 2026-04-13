use std::collections::HashMap;
use std::path::PathBuf;

use themelion::MediaType;

use crate::error::TaxisError;
use crate::sanitize::sanitize_component;

/// Valid tokens per media type.
fn valid_tokens(media_type: MediaType) -> &'static [&'static str] {
    match media_type {
        MediaType::Music => &[
            "Artist Name",
            "Album Title",
            "Year",
            "Track Number",
            "Track Title",
            "Disc Number",
            "Quality",
            "Extension",
        ],
        MediaType::Movie => &["Movie Title", "Year", "Quality", "Edition", "Extension"],
        MediaType::Tv => &[
            "Series Title",
            "Season Number",
            "Episode Number",
            "Episode Title",
            "Quality",
            "Extension",
        ],
        MediaType::Audiobook => &[
            "Author Name",
            "Title",
            "Year",
            "Narrator",
            "Series",
            "Series Position",
            "Extension",
        ],
        MediaType::Book => &["Author Name", "Title", "Year", "Extension"],
        MediaType::Comic => &[
            "Series Name",
            "Volume Number",
            "Issue Number",
            "Issue Title",
            "Year",
            "Extension",
        ],
        MediaType::Podcast => &[
            "Podcast Title",
            "Episode Title",
            "Publication Date",
            "Episode Number",
            "Extension",
        ],
        _ => &["Extension"],
    }
}

#[derive(Debug, Clone)]
pub enum TemplateSegment {
    Literal(String),
    Token {
        name: String,
        padding: Option<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct TemplateEngine {
    segments: Vec<TemplateSegment>,
    media_type: MediaType,
}

impl TemplateEngine {
    /// Parse and validate a template string.
    /// Returns error if any token is unknown for the given media type.
    pub fn parse(template: &str, media_type: MediaType) -> Result<Self, TaxisError> {
        let segments = parse_template(template, media_type)?;
        Ok(Self {
            segments,
            media_type,
        })
    }

    /// Resolve the template against metadata tokens, returning a relative PathBuf.
    pub fn resolve(&self, metadata: &HashMap<String, String>) -> Result<PathBuf, TaxisError> {
        let mut output = String::new();

        for segment in &self.segments {
            match segment {
                TemplateSegment::Literal(s) => {
                    output.push_str(s);
                }
                TemplateSegment::Token { name, padding } => {
                    if let Some(value) = metadata.get(name.as_str()) {
                        let sanitized = sanitize_component(value);
                        if sanitized != "unnamed" {
                            let formatted = match padding {
                                Some(width) => {
                                    if let Ok(n) = sanitized.parse::<u64>() {
                                        format!("{n:0>width$}")
                                    } else {
                                        sanitized
                                    }
                                }
                                None => sanitized,
                            };
                            output.push_str(&formatted);
                        }
                    }
                    // Missing token: skip silently
                }
            }
        }

        let output = collapse_whitespace(&output);
        let output = clean_empty_groups(&output);

        let path: PathBuf = output.split('/').filter(|s| !s.is_empty()).collect();
        Ok(path)
    }

    pub fn media_type(&self) -> MediaType {
        self.media_type
    }
}

fn parse_template(
    template: &str,
    media_type: MediaType,
) -> Result<Vec<TemplateSegment>, TaxisError> {
    let tokens = valid_tokens(media_type);
    let mut segments = Vec::new();
    let mut chars = template.chars().peekable();
    let mut literal = String::new();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if !literal.is_empty() {
                segments.push(TemplateSegment::Literal(std::mem::take(&mut literal)));
            }
            let mut token_str = String::new();
            let mut closed = false;
            for tc in chars.by_ref() {
                if tc == '}' {
                    closed = true;
                    break;
                }
                token_str.push(tc);
            }
            if !closed {
                literal.push('{');
                literal.push_str(&token_str);
                continue;
            }
            let (name, padding) = if let Some(colon_pos) = token_str.find(':') {
                let name = token_str[..colon_pos].trim().to_string();
                let pad_str = &token_str[colon_pos + 1..];
                let padding = if pad_str.chars().all(|c| c == '0') && !pad_str.is_empty() {
                    Some(pad_str.len())
                } else {
                    None
                };
                (name, padding)
            } else {
                (token_str.trim().to_string(), None)
            };

            if !tokens.contains(&name.as_str()) {
                return Err(TaxisError::UnknownToken {
                    token: name,
                    media_type: format!("{media_type:?}"),
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }

            segments.push(TemplateSegment::Token { name, padding });
        } else {
            literal.push(ch);
        }
    }

    if !literal.is_empty() {
        segments.push(TemplateSegment::Literal(literal));
    }

    Ok(segments)
}

fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
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

/// Remove empty parenthetical groups like ` ()` that arise FROM missing optional tokens.
fn clean_empty_groups(s: &str) -> String {
    let s = s.replace(" ()", "");
    let s = s.replace(" []", "");
    let s = s.replace("()", "");
    let s = s.replace("[]", "");
    s.trim().to_string()
}

/// Default naming template for each media type.
pub fn default_template(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Music => {
            "{Artist Name}/{Album Title} ({Year})/{Track Number:00} - {Track Title}.{Extension}"
        }
        MediaType::Movie => "{Movie Title} ({Year})/{Movie Title} ({Year}) [{Quality}].{Extension}",
        MediaType::Tv => {
            "{Series Title}/Season {Season Number:00}/{Series Title} - S{Season Number:00}E{Episode Number:00} - {Episode Title}.{Extension}"
        }
        MediaType::Audiobook => "{Author Name}/{Series}/{Title}.{Extension}",
        MediaType::Book => "{Author Name}/{Title}.{Extension}",
        MediaType::Comic => "{Series Name}/{Series Name} #{Issue Number:000}.{Extension}",
        MediaType::Podcast => "{Podcast Title}/{Publication Date} - {Episode Title}.{Extension}",
        _ => "{Extension}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn music_meta(
        artist: &str,
        album: &str,
        year: &str,
        track: &str,
        title: &str,
    ) -> HashMap<String, String> {
        [
            ("Artist Name", artist),
            ("Album Title", album),
            ("Year", year),
            ("Track Number", track),
            ("Track Title", title),
            ("Extension", "flac"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    #[test]
    fn music_template_all_tokens() {
        let engine = TemplateEngine::parse(
            "{Artist Name}/{Album Title} ({Year})/{Track Number:00} - {Track Title}.{Extension}",
            MediaType::Music,
        )
        .unwrap();
        let meta = music_meta(
            "Led Zeppelin",
            "Led Zeppelin IV",
            "1971",
            "7",
            "When the Levee Breaks",
        );
        let path = engine.resolve(&meta).unwrap();
        assert_eq!(
            path,
            PathBuf::from("Led Zeppelin/Led Zeppelin IV (1971)/07 - When the Levee Breaks.flac")
        );
    }

    #[test]
    fn template_track_number_zero_padded() {
        let engine =
            TemplateEngine::parse("{Track Number:00}.{Extension}", MediaType::Music).unwrap();
        let meta: HashMap<_, _> = [
            ("Track Number".to_string(), "3".to_string()),
            ("Extension".to_string(), "flac".to_string()),
        ]
        .into();
        let path = engine.resolve(&meta).unwrap();
        assert_eq!(path, PathBuf::from("03.flac"));
    }

    #[test]
    fn template_three_digit_padding() {
        let engine =
            TemplateEngine::parse("{Issue Number:000}.{Extension}", MediaType::Comic).unwrap();
        let meta: HashMap<_, _> = [
            ("Issue Number".to_string(), "5".to_string()),
            ("Extension".to_string(), "cbz".to_string()),
        ]
        .into();
        let path = engine.resolve(&meta).unwrap();
        assert_eq!(path, PathBuf::from("005.cbz"));
    }

    #[test]
    fn template_missing_year_removes_parenthetical() {
        let engine = TemplateEngine::parse("{Artist Name} ({Year})", MediaType::Music).unwrap();
        let meta: HashMap<_, _> = [("Artist Name".to_string(), "Bach".to_string())].into();
        let path = engine.resolve(&meta).unwrap();
        let path_str = path.to_string_lossy();
        assert!(!path_str.contains("()"), "got: {path_str}");
    }

    #[test]
    fn template_unknown_token_error_at_parse_time() {
        let err = TemplateEngine::parse("{Unknown Token}", MediaType::Music);
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("Unknown Token"), "got: {msg}");
    }

    #[test]
    fn tv_template_episode_format() {
        let engine = TemplateEngine::parse(
            "{Series Title}/Season {Season Number:00}/{Series Title} - S{Season Number:00}E{Episode Number:00} - {Episode Title}.{Extension}",
            MediaType::Tv,
        )
        .unwrap();
        let meta: HashMap<_, _> = [
            ("Series Title", "Breaking Bad"),
            ("Season Number", "5"),
            ("Episode Number", "16"),
            ("Episode Title", "Felina"),
            ("Extension", "mkv"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let path = engine.resolve(&meta).unwrap();
        assert_eq!(
            path,
            PathBuf::from("Breaking Bad/Season 05/Breaking Bad - S05E16 - Felina.mkv")
        );
    }

    #[test]
    fn default_template_parseable_for_all_types() {
        for mt in [
            MediaType::Music,
            MediaType::Movie,
            MediaType::Book,
            MediaType::Comic,
            MediaType::Podcast,
            MediaType::Audiobook,
        ] {
            let result = TemplateEngine::parse(default_template(mt), mt);
            assert!(
                result.is_ok(),
                "default template for {mt:?} failed to parse: {:?}",
                result.err()
            );
        }
    }
}
