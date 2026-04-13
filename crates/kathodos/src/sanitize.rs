use std::path::PathBuf;

use unicode_normalization::UnicodeNormalization as _;

/// Characters that are illegal in filenames on Windows, macOS, or Linux.
const UNSAFE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// Maximum byte length for a single path component (POSIX NAME_MAX).
const MAX_COMPONENT_BYTES: usize = 255;

/// Sanitize a single path component for cross-platform filesystem safety.
///
/// Applies, in order:
/// 1. Unicode NFC normalization
/// 2. Replace `/ \ : * ? " < > |` with `-`
/// 3. Collapse any run of whitespace to a single ASCII space
/// 4. Trim leading/trailing whitespace and dots
/// 5. Remove a leading dot (hidden-file prevention)
/// 6. Truncate to 255 bytes, preserving UTF-8 character boundaries
/// 7. Return `"unnamed"` for empty or all-dots inputs
pub fn sanitize_component(input: &str) -> String {
    // Step 1: NFC normalization.
    let nfc: String = input.nfc().collect();

    // Step 2: replace unsafe chars with `-`.
    let replaced: String = nfc
        .chars()
        .map(|c| if UNSAFE_CHARS.contains(&c) { '-' } else { c })
        .collect();

    // Step 3: collapse whitespace runs to a single space.
    let collapsed = collapse_whitespace(&replaced);

    // Step 4: trim leading/trailing whitespace and dots.
    let trimmed = collapsed.trim_matches(|c: char| c.is_whitespace() || c == '.');

    // Step 5: remove a leading dot that survives after trimming (shouldn't happen after
    // trim_matches above, but guard against inputs like "." or ".hidden" where a single
    // leading dot remains after the previous trim of dots on both ends fails to apply
    // because the character is interior).  The trim already handles pure-dot strings;
    // this handles ".hidden" where only the left side has a dot.
    let no_leading_dot = trimmed.strip_prefix('.').unwrap_or(trimmed);

    // Step 6: truncate to 255 bytes at a UTF-8 character boundary.
    let truncated = truncate_to_bytes(no_leading_dot, MAX_COMPONENT_BYTES);

    // Step 7: fallback for empty / all-whitespace / all-dots inputs.
    if truncated.is_empty() {
        "unnamed".to_string()
    } else {
        truncated.to_string()
    }
}

/// Sanitize a full relative path by sanitizing each component individually.
///
/// Splits on `/` and `\`, sanitizes each non-empty component, filters components
/// that collapsed to `"unnamed"` (empty or all-dots segments between separators),
/// then reassembles into a `PathBuf`.  The result is always a relative path.
///
/// A single-component input that sanitizes to `"unnamed"` is preserved as-is
/// (the caller asked to sanitize something, and "unnamed" is the safe fallback).
pub fn sanitize_path(input: &str) -> PathBuf {
    let components: Vec<String> = input
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(sanitize_component)
        .collect();

    match components.as_slice() {
        [] => PathBuf::from("unnamed"),
        [single] => PathBuf::from(single),
        _ => components.into_iter().filter(|c| c != "unnamed").collect(),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

/// Truncate `s` to at most `max_bytes` bytes without splitting a multi-byte char.
fn truncate_to_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Walk back from max_bytes to find a valid char boundary.
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── sanitize_component ────────────────────────────────────────────────────

    #[test]
    fn ascii_special_chars_replaced_with_dash() {
        for ch in UNSAFE_CHARS {
            let input = format!("before{ch}after");
            let result = sanitize_component(&input);
            assert!(
                !result.contains(*ch),
                "char {ch:?} should be replaced, got: {result:?}"
            );
            assert!(
                result.contains('-'),
                "char {ch:?} replacement should be '-', got: {result:?}"
            );
        }
    }

    #[test]
    fn whitespace_collapsed_and_trimmed() {
        assert_eq!(sanitize_component("  hello   world  "), "hello world");
    }

    #[test]
    fn tabs_and_newlines_collapsed() {
        assert_eq!(sanitize_component("a\t\tb\nc"), "a b c");
    }

    #[test]
    fn leading_dot_removed() {
        assert_eq!(sanitize_component(".hidden"), "hidden");
    }

    #[test]
    fn leading_dots_trimmed() {
        assert_eq!(sanitize_component("...hidden"), "hidden");
    }

    #[test]
    fn trailing_dots_trimmed() {
        assert_eq!(sanitize_component("file..."), "file");
    }

    #[test]
    fn empty_string_returns_unnamed() {
        assert_eq!(sanitize_component(""), "unnamed");
    }

    #[test]
    fn all_whitespace_returns_unnamed() {
        assert_eq!(sanitize_component("   "), "unnamed");
    }

    #[test]
    fn all_dots_returns_unnamed() {
        assert_eq!(sanitize_component("..."), "unnamed");
    }

    #[test]
    fn all_dots_and_spaces_returns_unnamed() {
        assert_eq!(sanitize_component(". . ."), "unnamed");
    }

    #[test]
    fn normal_ascii_unchanged() {
        assert_eq!(sanitize_component("AC-DC"), "AC-DC");
        assert_eq!(
            sanitize_component("The Dark Side of the Moon"),
            "The Dark Side of the Moon"
        );
    }

    #[test]
    fn colon_and_slash_replaced() {
        let result = sanitize_component("AC/DC: Rock & Roll");
        assert!(!result.contains('/'));
        assert!(!result.contains(':'));
    }

    #[test]
    fn unicode_preserved_nfc() {
        // "café" in NFD (decomposed) should round-trip as NFC
        let nfd = "cafe\u{0301}"; // e + combining acute
        let result = sanitize_component(nfd);
        assert_eq!(result, "caf\u{00e9}"); // NFC: é as single codepoint
    }

    #[test]
    fn unicode_cjk_preserved() {
        assert_eq!(sanitize_component("山田太郎"), "山田太郎");
    }

    #[test]
    fn truncation_at_255_bytes() {
        let long = "a".repeat(300);
        let result = sanitize_component(&long);
        assert!(
            result.len() <= 255,
            "result too long: {} bytes",
            result.len()
        );
        assert_eq!(result.len(), 255);
    }

    #[test]
    fn truncation_is_utf8_safe() {
        // Each '山' is 3 bytes in UTF-8. 255 / 3 = 85 chars exactly → 255 bytes.
        // 256 chars → 768 bytes. After truncation we should have exactly 85 '山'.
        let long: String = "山".repeat(256);
        let result = sanitize_component(&long);
        assert!(
            result.len() <= 255,
            "result too long: {} bytes",
            result.len()
        );
        assert!(
            std::str::from_utf8(result.as_bytes()).is_ok(),
            "result is not valid UTF-8"
        );
        // 85 × 3 = 255
        assert_eq!(result, "山".repeat(85));
    }

    #[test]
    fn truncation_boundary_two_byte_char() {
        // 'é' is 2 bytes. Fill 254 bytes with 'a' then add 'é' (2 bytes) → 256 bytes.
        // After truncation at 255 bytes we must not split 'é', so the last byte is 'a'.
        let mut s = "a".repeat(254);
        s.push('é');
        let result = sanitize_component(&s);
        assert!(result.len() <= 255);
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
        assert_eq!(result.len(), 254); // 'é' is dropped, keeping 254 'a's
    }

    // ── sanitize_path ─────────────────────────────────────────────────────────

    #[test]
    fn sanitize_path_splits_components() {
        let result = sanitize_path("Artist Name/Album Title/track.flac");
        assert_eq!(result, PathBuf::from("Artist Name/Album Title/track.flac"));
    }

    #[test]
    fn sanitize_path_sanitizes_each_component() {
        let result = sanitize_path("AC/DC: Back in Black/01 - Hells Bells.flac");
        // The first component "AC" and "DC: Back in Black" arise from splitting on /
        // "AC" → "AC", "DC: Back in Black" → "DC- Back in Black", "01 - Hells Bells.flac" unchanged
        let s = result.to_string_lossy();
        assert!(!s.contains(':'), "colon should be replaced, got: {s}");
    }

    #[test]
    fn sanitize_path_backslash_treated_as_separator() {
        let result = sanitize_path("Artist\\Album\\track.flac");
        assert_eq!(result, PathBuf::from("Artist/Album/track.flac"));
    }
}
