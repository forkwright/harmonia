//! Cardigann filter pipeline — string transforms applied to extracted values.
//!
//! Implemented: `regexp`, `re_replace`, `replace`, `split`, `trim`, `prepend`,
//! `append`, `tolower`, `toupper`, `querystring`, `dateparse` (alias
//! `timeparse`), `timeago` (alias `reltime`), `urldecode`, `urlencode`,
//! `validfilename`, `validate`, `diacritics`.
//!
//! Unknown filters are rejected at definition load by [`validate`] — including
//! the row-level `andmatch` and the `fuzzytime` / `htmldecode` tail, tracked in
//! #513.

use jiff::Zoned;
use jiff::civil::DateTime;
use jiff::fmt::strtime;
use jiff::tz::TimeZone;
use regex::Regex;
use tracing::debug;

use crate::client::cardigann::definition::FilterSpec;

/// Runs `value` through `specs` in order.
///
/// `now` anchors the relative-time filters; injecting it keeps them
/// deterministic under test.
pub fn apply(value: String, specs: &[FilterSpec], now: &Zoned) -> Result<String, String> {
    let mut value = value;
    for spec in specs {
        value = apply_one(value, spec, now)?;
    }
    Ok(value)
}

/// Checks filter names, arity, and static arguments (regex syntax, split
/// index) so an unusable definition fails at load with one clear reason.
pub fn validate(specs: &[FilterSpec]) -> Result<(), String> {
    for spec in specs {
        let args = spec.args();
        let arity = |min: usize, max: usize| {
            if args.len() < min || args.len() > max {
                Err(format!(
                    "filter {:?} takes {min}..={max} args, got {}",
                    spec.name,
                    args.len()
                ))
            } else {
                Ok(())
            }
        };
        match spec.name.as_str() {
            "regexp" | "re_replace" => {
                arity(1, 2)?;
                if spec.name == "re_replace" && args.len() != 2 {
                    return Err("filter \"re_replace\" takes exactly 2 args".to_string());
                }
                let pattern = args.first().map(String::as_str).unwrap_or_default();
                Regex::new(pattern).map_err(|e| format!("filter {:?} pattern: {e}", spec.name))?;
            }
            "replace" => arity(2, 2)?,
            "split" => {
                arity(2, 2)?;
                let index = args.get(1).map(String::as_str).unwrap_or_default();
                index
                    .parse::<i64>()
                    .map_err(|_| format!("split index {index:?} is not an integer"))?;
            }
            "trim" => arity(0, 1)?,
            // WHY: `timeparse`/`reltime` are upstream aliases of `dateparse`/`timeago`.
            "prepend" | "append" | "querystring" | "dateparse" | "timeparse" => arity(1, 1)?,
            "tolower" | "toupper" | "timeago" | "reltime" => arity(0, 1)?,
            "urldecode" | "urlencode" | "validfilename" => arity(0, 0)?,
            "validate" => arity(1, 1)?,
            "diacritics" => {
                arity(1, 1)?;
                // WHY: upstream accepts only "replace" and throws otherwise —
                // reject at load rather than silently no-op on a typo.
                let mode = args.first().map(String::as_str).unwrap_or_default();
                if mode != "replace" {
                    return Err(format!(
                        "filter \"diacritics\" takes only \"replace\", got {mode:?}"
                    ));
                }
            }
            other => {
                return Err(format!(
                    "unsupported filter {other:?} (supported: regexp, re_replace, replace, \
                     split, trim, prepend, append, tolower, toupper, querystring, dateparse, \
                     timeparse, timeago, reltime, urldecode, urlencode, validfilename, \
                     validate, diacritics)"
                ));
            }
        }
    }
    Ok(())
}

fn apply_one(value: String, spec: &FilterSpec, now: &Zoned) -> Result<String, String> {
    let args = spec.args();
    // WHY: arity is checked at definition load, but this accessor keeps the
    // pipeline panic-free if a spec ever arrives unvalidated.
    let arg = |i: usize| {
        args.get(i)
            .map(String::as_str)
            .ok_or_else(|| format!("filter {:?} is missing argument {i}", spec.name))
    };
    match spec.name.as_str() {
        "regexp" => {
            let pattern = arg(0)?;
            let re = Regex::new(pattern).map_err(|e| format!("regexp: {e}"))?;
            let caps = re
                .captures(&value)
                .ok_or_else(|| format!("regexp {pattern:?} did not match {value:?}"))?;
            // WHY: Cardigann yields the first capture group; a group-less
            // pattern falls back to the whole match. `Captures::len()` counts
            // group 0 plus all capture groups, so `> 1` means the pattern
            // statically has a group 1 — in that case a non-participating
            // group (e.g. the other arm of an alternation) yields the empty
            // string, not the whole match, matching upstream Cardigann/Go
            // semantics. Only a pattern with no group 1 at all falls back to
            // the whole match.
            let m = if caps.len() > 1 {
                caps.get(1)
                    .map_or_else(String::new, |m| m.as_str().to_string())
            } else {
                caps.get(0)
                    .map_or_else(String::new, |m| m.as_str().to_string())
            };
            Ok(m)
        }
        "re_replace" => {
            let re = Regex::new(arg(0)?).map_err(|e| format!("re_replace: {e}"))?;
            Ok(re.replace_all(&value, arg(1)?).into_owned())
        }
        "replace" => Ok(value.replace(arg(0)?, arg(1)?)),
        "split" => {
            let separator = arg(0)?;
            let index_arg = arg(1)?;
            let parts: Vec<&str> = value.split(separator).collect();
            let index: i64 = index_arg
                .parse()
                .map_err(|_| "split: bad index".to_string())?;
            let index = if index < 0 {
                i64::try_from(parts.len()).unwrap_or(i64::MAX) + index
            } else {
                index
            };
            usize::try_from(index)
                .ok()
                .and_then(|i| parts.get(i))
                .map(|s| (*s).to_string())
                .ok_or_else(|| {
                    format!(
                        "split index {index_arg} out of range for {} parts",
                        parts.len()
                    )
                })
        }
        "trim" => match args.first().map(String::as_str) {
            None | Some("") => Ok(value.trim().to_string()),
            Some(chars) => Ok(value.trim_matches(|c| chars.contains(c)).to_string()),
        },
        "prepend" => Ok(format!("{}{value}", arg(0)?)),
        "append" => Ok(format!("{value}{}", arg(0)?)),
        "tolower" => Ok(value.to_lowercase()),
        "toupper" => Ok(value.to_uppercase()),
        "querystring" => {
            let param = arg(0)?;
            // WHY: extracted hrefs are usually relative; a fixed placeholder
            // base makes them parseable without affecting query extraction.
            let url = url::Url::parse("https://cardigann.invalid/")
                .and_then(|base| base.join(&value))
                .map_err(|e| format!("querystring: {value:?}: {e}"))?;
            url.query_pairs()
                .find(|(k, _)| k.as_ref() == param)
                .map(|(_, v)| v.into_owned())
                .ok_or_else(|| format!("querystring: no {param:?} parameter in {value:?}"))
        }
        "dateparse" | "timeparse" => Ok(dateparse(&value, arg(0)?, now)),
        "timeago" | "reltime" => Ok(timeago(&value, now)),
        "urldecode" => Ok(url_decode(&value)),
        "urlencode" => Ok(url_encode(&value)),
        "validfilename" => Ok(valid_filename(&value)),
        "validate" => Ok(validate_against(&value, arg(0)?)),
        "diacritics" => strip_diacritics(&value),
        other => Err(format!("unsupported filter {other:?}")),
    }
}

/// Percent-decodes `value`, `+` as space, leniently — a malformed `%` sequence
/// passes through unchanged.
///
/// WHY: mirrors .NET `WebUtility.UrlDecode`, which real definitions are written
/// against. Decoding is byte-wise then UTF-8; a definition declaring a non-UTF-8
/// `encoding` is not honored here (no definition in the upstream corpus pairs a
/// legacy charset with this filter).
fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(byte) = bytes.get(i) {
        match byte {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                let decoded = bytes
                    .get(i + 1..i + 3)
                    .and_then(|pair| std::str::from_utf8(pair).ok())
                    .and_then(|pair| u8::from_str_radix(pair, 16).ok());
                match decoded {
                    Some(b) => {
                        out.push(b);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            other => {
                out.push(*other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encodes `value` with .NET `WebUtility.UrlEncode`'s safe set
/// (`A-Za-z0-9` plus `-_.!*()`), space as `+`, uppercase hex, byte-wise.
fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'*'
            | b'('
            | b')' => out.push(char::from(*byte)),
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Replaces filename-invalid characters with `_`.
///
/// WHY: upstream calls .NET `Path.GetInvalidFileNameChars()`, which is
/// platform-dependent; on Linux (where real deployments run) that set is only
/// `/` and NUL. Matching the measured behavior rather than the stricter Windows
/// set the filter's name suggests. An all-invalid value collapses to `_`.
fn valid_filename(value: &str) -> String {
    let replaced: String = value
        .chars()
        .map(|c| if c == '/' || c == '\0' { '_' } else { c })
        .collect();
    if replaced.is_empty() {
        "_".to_string()
    } else {
        replaced
    }
}

/// Keeps the allowlist tokens that also appear in `value`, in allowlist order.
///
/// WHY: upstream lowercases and splits both sides on a fixed delimiter set, then
/// takes a LINQ `Intersect` — which yields the FIRST sequence's (the
/// allowlist's) order and de-duplicates. Used to normalize genre/tag text
/// against a fixed vocabulary.
fn validate_against(value: &str, allowlist: &str) -> String {
    const DELIMITERS: &[char] = &[',', ' ', '/', ')', '(', '.', ';', '[', ']', '"', '|', ':'];
    let present: std::collections::HashSet<String> = value
        .to_lowercase()
        .split(|c| DELIMITERS.contains(&c))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect();
    let mut seen = std::collections::HashSet::new();
    allowlist
        .to_lowercase()
        .split(|c| DELIMITERS.contains(&c))
        .filter(|token| !token.is_empty())
        .filter(|token| present.contains(*token))
        .filter(|token| seen.insert((*token).to_string()))
        .collect::<Vec<_>>()
        .join(",")
}

/// Strips non-spacing marks (NFD-decompose, drop `Mn`, NFC-recompose).
///
/// WHY: upstream is a combining-mark strip, NOT transliteration — precomposed
/// letters without a decomposition (`Đ`) and non-Latin scripts pass through
/// unchanged. A transliterating crate would corrupt those.
///
/// WHY `\p{Mn}` and not `unicode_normalization::char::is_combining_mark`: that
/// predicate is the whole `Mark` category (Mn+Mc+Me), while upstream drops only
/// `NonSpacingMark`. The superset deletes SPACING combining marks — Devanagari,
/// Bengali and Tamil vowel signs — which changes what the text says (`काम`
/// "work" would become `कम`) where upstream leaves it untouched.
fn strip_diacritics(value: &str) -> Result<String, String> {
    use unicode_normalization::UnicodeNormalization;

    let non_spacing = Regex::new(r"\p{Mn}").map_err(|e| format!("diacritics: {e}"))?;
    let decomposed: String = value.nfd().collect();
    Ok(non_spacing.replace_all(&decomposed, "").nfc().collect())
}

/// Best-effort Go-layout date parse; the raw value passes through unchanged
/// when the layout or the value cannot be handled.
///
/// WHY: `publication_date` is an advisory string downstream — a raw site
/// date is more useful than a hard row failure over a locale quirk.
fn dateparse(value: &str, go_layout: &str, now: &Zoned) -> String {
    let Some(fmt) = go_layout_to_strptime(go_layout) else {
        debug!(layout = %go_layout, "dateparse layout has unsupported tokens; passing raw value");
        return value.to_string();
    };
    let Ok(mut parsed) = strtime::parse(&fmt, value.trim()) else {
        debug!(layout = %go_layout, value = %value, "dateparse failed; passing raw value");
        return value.to_string();
    };
    if parsed.year().is_none() && parsed.set_year(Some(now.year())).is_err() {
        return value.to_string();
    }
    if let Ok(ts) = parsed.to_timestamp() {
        return ts.to_string();
    }
    render_civil(parsed.to_datetime(), value)
}

fn render_civil(datetime: Result<DateTime, jiff::Error>, raw: &str) -> String {
    datetime
        .and_then(|dt| dt.to_zoned(TimeZone::UTC))
        .map(|z| z.timestamp().to_string())
        .unwrap_or_else(|_| raw.to_string())
}

/// Converts a Go reference-time layout (`2006-01-02 15:04:05`) into a
/// strptime format string. Returns `None` when the layout uses tokens with
/// no strptime equivalent (e.g. `MST`, `Z07:00`).
fn go_layout_to_strptime(layout: &str) -> Option<String> {
    const TOKENS: &[(&str, Option<&str>)] = &[
        ("January", Some("%B")),
        ("Monday", Some("%A")),
        ("Jan", Some("%b")),
        ("Mon", Some("%a")),
        ("Z07:00", None),
        ("-07:00", Some("%:z")),
        ("-0700", Some("%z")),
        ("MST", None),
        ("2006", Some("%Y")),
        ("_2", Some("%e")),
        ("15", Some("%H")),
        ("06", Some("%y")),
        ("05", Some("%S")),
        ("04", Some("%M")),
        ("03", Some("%I")),
        ("02", Some("%d")),
        ("01", Some("%m")),
        ("PM", Some("%p")),
        ("pm", Some("%P")),
        // NOTE: single-digit Go tokens; jiff parses %-padded specifiers
        // leniently, so unpadded values still parse.
        ("5", Some("%S")),
        ("4", Some("%M")),
        ("3", Some("%I")),
        ("2", Some("%d")),
        ("1", Some("%m")),
    ];

    let mut out = String::with_capacity(layout.len() + 8);
    let mut rest = layout;
    'outer: while !rest.is_empty() {
        for (token, replacement) in TOKENS {
            if let Some(tail) = rest.strip_prefix(token) {
                out.push_str((*replacement)?);
                rest = tail;
                continue 'outer;
            }
        }
        let ch = rest.chars().next()?;
        if ch == '%' {
            out.push_str("%%");
        } else {
            out.push(ch);
        }
        rest = rest.get(ch.len_utf8()..).unwrap_or("");
    }
    Some(out)
}

/// Best-effort relative-time parse ("2 hours ago"); the raw value passes
/// through unchanged when no duration can be read.
fn timeago(value: &str, now: &Zoned) -> String {
    let cleaned = value.to_lowercase().replace(',', " ");
    let mut total_seconds: i64 = 0;
    let mut pending_number: Option<i64> = None;
    for token in cleaned.split_whitespace() {
        if token == "ago" {
            continue;
        }
        if let Ok(n) = token.parse::<i64>() {
            pending_number = Some(n);
            continue;
        }
        if token == "a" || token == "an" {
            pending_number = Some(1);
            continue;
        }
        let Some(n) = pending_number.take() else {
            continue;
        };
        let unit_seconds = match token {
            t if t.starts_with("mo") => 30 * 86_400,
            t if t.starts_with('y') => 365 * 86_400,
            t if t.starts_with('w') => 7 * 86_400,
            t if t.starts_with('d') => 86_400,
            t if t.starts_with('h') => 3_600,
            t if t.starts_with('m') => 60,
            t if t.starts_with('s') => 1,
            _ => continue,
        };
        total_seconds = total_seconds.saturating_add(n.saturating_mul(unit_seconds));
    }
    if total_seconds == 0 {
        debug!(value = %value, "timeago could not read a duration; passing raw value");
        return value.to_string();
    }
    jiff::Timestamp::from_second(now.timestamp().as_second().saturating_sub(total_seconds))
        .map(|ts| ts.to_string())
        .unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(name: &str, args: &[&str]) -> FilterSpec {
        FilterSpec {
            name: name.to_string(),
            args: if args.is_empty() {
                None
            } else {
                Some(crate::client::cardigann::definition::FilterArgs(
                    args.iter().map(|s| (*s).to_string()).collect(),
                ))
            },
        }
    }

    fn now() -> Zoned {
        "2026-06-15T12:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
    }

    fn run(value: &str, specs: &[FilterSpec]) -> Result<String, String> {
        apply(value.to_string(), specs, &now())
    }

    #[test]
    fn regexp_returns_first_capture_group() {
        assert_eq!(
            run("Size: 1.4 GB", &[spec("regexp", &[r"Size: ([\d.]+ \w+)"])]).unwrap(),
            "1.4 GB"
        );
    }

    #[test]
    fn regexp_without_groups_returns_whole_match() {
        assert_eq!(
            run("abc123def", &[spec("regexp", &[r"\d+"])]).unwrap(),
            "123"
        );
    }

    #[test]
    fn regexp_non_participating_group_returns_empty() {
        // WHY: group 1 exists in the pattern but the "abc" arm of the
        // alternation matched instead — upstream Cardigann/Go semantics
        // yield the empty string here, not the whole match.
        assert_eq!(
            run("abc", &[spec("regexp", &[r"(?:(\d+)|abc)"])]).unwrap(),
            ""
        );
    }

    #[test]
    fn regexp_participating_group_returns_group_text() {
        assert_eq!(
            run("123", &[spec("regexp", &[r"(?:(\d+)|abc)"])]).unwrap(),
            "123"
        );
    }

    #[test]
    fn regexp_no_match_errors() {
        assert!(run("nope", &[spec("regexp", &[r"\d+"])]).is_err());
    }

    #[test]
    fn re_replace_supports_group_references() {
        // NOTE: `${1}` (braced) — both Go's regexp and the regex crate parse
        // a bare `$1x` as the group named "1x", so definition semantics match.
        assert_eq!(
            run(
                "S01E02",
                &[spec("re_replace", &[r"S(\d+)E(\d+)", "${1}x$2"])]
            )
            .unwrap(),
            "01x02"
        );
        assert_eq!(
            run("a  b", &[spec("re_replace", &[r"\s+", "."])]).unwrap(),
            "a.b"
        );
    }

    #[test]
    fn replace_literal() {
        assert_eq!(
            run("a.b.c", &[spec("replace", &[".", " "])]).unwrap(),
            "a b c"
        );
    }

    #[test]
    fn split_positive_and_negative_index() {
        assert_eq!(run("a/b/c", &[spec("split", &["/", "1"])]).unwrap(), "b");
        assert_eq!(run("a/b/c", &[spec("split", &["/", "-1"])]).unwrap(), "c");
        assert!(run("a/b", &[spec("split", &["/", "5"])]).is_err());
    }

    #[test]
    fn trim_whitespace_and_chars() {
        assert_eq!(run("  x  ", &[spec("trim", &[])]).unwrap(), "x");
        assert_eq!(run("--x--", &[spec("trim", &["-"])]).unwrap(), "x");
    }

    #[test]
    fn prepend_append() {
        assert_eq!(run("path", &[spec("prepend", &["/"])]).unwrap(), "/path");
        assert_eq!(
            run("file", &[spec("append", &[".torrent"])]).unwrap(),
            "file.torrent"
        );
    }

    #[test]
    fn case_folding() {
        assert_eq!(run("MiXeD", &[spec("tolower", &[])]).unwrap(), "mixed");
        assert_eq!(run("MiXeD", &[spec("toupper", &[])]).unwrap(), "MIXED");
    }

    #[test]
    fn querystring_extracts_parameter() {
        assert_eq!(
            run("/browse.php?cat=6&page=2", &[spec("querystring", &["cat"])]).unwrap(),
            "6"
        );
        assert_eq!(
            run("https://x.example/t?id=42", &[spec("querystring", &["id"])]).unwrap(),
            "42"
        );
        assert!(run("/browse.php?cat=6", &[spec("querystring", &["id"])]).is_err());
    }

    #[test]
    fn dateparse_iso_layout() {
        assert_eq!(
            run(
                "2024-01-15 10:30:00",
                &[spec("dateparse", &["2006-01-02 15:04:05"])]
            )
            .unwrap(),
            "2024-01-15T10:30:00Z"
        );
    }

    #[test]
    fn dateparse_month_name_layout() {
        assert_eq!(
            run("Jan 15, 2024", &[spec("dateparse", &["Jan 2, 2006"])]).unwrap(),
            "2024-01-15T00:00:00Z"
        );
    }

    #[test]
    fn dateparse_missing_year_uses_now() {
        assert_eq!(
            run("06-15 08:00", &[spec("dateparse", &["01-02 15:04"])]).unwrap(),
            "2026-06-15T08:00:00Z"
        );
    }

    #[test]
    fn dateparse_failure_passes_raw_value() {
        assert_eq!(
            run("yesterday", &[spec("dateparse", &["2006-01-02"])]).unwrap(),
            "yesterday"
        );
        assert_eq!(
            run(
                "2024-01-15 10:30:00 MST",
                &[spec("dateparse", &["2006-01-02 15:04:05 MST"])]
            )
            .unwrap(),
            "2024-01-15 10:30:00 MST"
        );
    }

    #[test]
    fn timeago_hours() {
        assert_eq!(
            run("2 hours ago", &[spec("timeago", &[])]).unwrap(),
            "2026-06-15T10:00:00Z"
        );
    }

    #[test]
    fn timeago_compound_and_articles() {
        assert_eq!(
            run("1 day, 2 hours ago", &[spec("timeago", &[])]).unwrap(),
            "2026-06-14T10:00:00Z"
        );
        assert_eq!(
            run("an hour ago", &[spec("timeago", &[])]).unwrap(),
            "2026-06-15T11:00:00Z"
        );
    }

    #[test]
    fn timeago_month_vs_minute_disambiguation() {
        assert_eq!(
            run("1 month ago", &[spec("timeago", &[])]).unwrap(),
            "2026-05-16T12:00:00Z"
        );
        assert_eq!(
            run("5 min ago", &[spec("timeago", &[])]).unwrap(),
            "2026-06-15T11:55:00Z"
        );
    }

    #[test]
    fn timeago_unreadable_passes_raw_value() {
        assert_eq!(
            run("just now", &[spec("timeago", &[])]).unwrap(),
            "just now"
        );
    }

    #[test]
    fn chain_applies_in_order() {
        assert_eq!(
            run(
                " Size: 700 MB ",
                &[
                    spec("trim", &[]),
                    spec("regexp", &[r"Size: (.+)"]),
                    spec("append", &["!"]),
                ],
            )
            .unwrap(),
            "700 MB!"
        );
    }

    #[test]
    fn validate_rejects_unknown_filter_and_bad_args() {
        assert!(validate(&[spec("regexp", &[r"\d+"])]).is_ok());
        assert!(validate(&[spec("andmatch", &[])]).is_err());
        assert!(validate(&[spec("regexp", &["("])]).is_err());
        assert!(validate(&[spec("split", &["/", "x"])]).is_err());
        assert!(validate(&[spec("replace", &["only-one"])]).is_err());
    }

    #[test]
    fn urldecode_handles_plus_percent_and_malformed() {
        assert_eq!(run("a%2Bb+c", &[spec("urldecode", &[])]).unwrap(), "a+b c");
        assert_eq!(run("caf%C3%A9", &[spec("urldecode", &[])]).unwrap(), "café");
        // a malformed `%` passes through, matching .NET's lenient decode
        assert_eq!(
            run("100%25 %zz", &[spec("urldecode", &[])]).unwrap(),
            "100% %zz"
        );
    }

    #[test]
    fn urlencode_uses_dotnet_safe_set_and_plus_for_space() {
        assert_eq!(
            run("Foo Bar & Baz", &[spec("urlencode", &[])]).unwrap(),
            "Foo+Bar+%26+Baz"
        );
        // .NET's safe set keeps -_.!*() unescaped
        assert_eq!(
            run("a-_.!*()z", &[spec("urlencode", &[])]).unwrap(),
            "a-_.!*()z"
        );
        assert_eq!(run("café", &[spec("urlencode", &[])]).unwrap(), "caf%C3%A9");
    }

    #[test]
    fn validfilename_replaces_only_linux_invalid_chars() {
        // `:` is legal on Linux and survives — the platform behavior upstream has
        assert_eq!(
            run("Show/Name: S01E02", &[spec("validfilename", &[])]).unwrap(),
            "Show_Name: S01E02"
        );
        assert_eq!(run("/", &[spec("validfilename", &[])]).unwrap(), "_");
    }

    #[test]
    fn validate_keeps_allowlist_order_and_dedups() {
        assert_eq!(
            run(
                "Genres: Horror/Comedy (2024)",
                &[spec("validate", &["Action, Comedy, Horror"])]
            )
            .unwrap(),
            "comedy,horror"
        );
        assert_eq!(
            run("Drama", &[spec("validate", &["Action, Comedy"])]).unwrap(),
            ""
        );
    }

    #[test]
    fn diacritics_strips_marks_but_not_precomposed_or_other_scripts() {
        assert_eq!(
            run("café", &[spec("diacritics", &["replace"])]).unwrap(),
            "cafe"
        );
        // Đ/đ have no NFD decomposition — upstream leaves them
        assert_eq!(
            run("Đorđe", &[spec("diacritics", &["replace"])]).unwrap(),
            "Đorđe"
        );
        // no transliteration: non-Latin scripts pass through
        assert_eq!(
            run("Москва", &[spec("diacritics", &["replace"])]).unwrap(),
            "Москва"
        );
    }

    #[test]
    fn diacritics_keeps_spacing_combining_marks() {
        // WHY: upstream drops ONLY Mn. Devanagari/Bengali vowel signs are Mc
        // (SPACING combining marks) — dropping them changes what the word says,
        // so they must survive: "काम" ("work") must not become "कम".
        assert_eq!(
            run("काम", &[spec("diacritics", &["replace"])]).unwrap(),
            "काम"
        );
        assert_eq!(run("মা", &[spec("diacritics", &["replace"])]).unwrap(), "মা");
    }

    #[test]
    fn diacritics_rejects_non_replace_argument_at_load() {
        assert!(validate(&[spec("diacritics", &["replace"])]).is_ok());
        assert!(validate(&[spec("diacritics", &["strip"])]).is_err());
        assert!(validate(&[spec("diacritics", &[])]).is_err());
    }

    #[test]
    fn timeparse_and_reltime_alias_dateparse_and_timeago() {
        // timeparse resolves exactly as dateparse does
        assert_eq!(
            run(
                "2024-01-15 10:30:00",
                &[spec("timeparse", &["2006-01-02 15:04:05"])]
            )
            .unwrap(),
            "2024-01-15T10:30:00Z"
        );
        // reltime resolves exactly as timeago does (now = 2026-06-15T12:00:00Z)
        assert_eq!(
            run("2 hours ago", &[spec("reltime", &[])]).unwrap(),
            "2026-06-15T10:00:00Z"
        );
        assert!(validate(&[spec("timeparse", &["2006-01-02"])]).is_ok());
        assert!(validate(&[spec("reltime", &[])]).is_ok());
    }
}
