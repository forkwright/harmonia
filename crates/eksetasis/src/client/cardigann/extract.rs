//! HTML row/field extraction for Cardigann definitions (CSS selectors via
//! `scraper`).
//!
//! Everything here is synchronous on purpose: `scraper::Html` is `!Send`, so
//! parsing must start and finish between awaits. Callers fetch the body,
//! then hand the owned string in and get owned rows back.

use std::collections::{BTreeMap, HashSet};

use jiff::Zoned;
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::client::cardigann::definition::{CardigannDefinition, FieldBlock};
use crate::client::cardigann::{filters, template::TemplateContext};

/// Extracted field values for the rows in one search-results page, in
/// definition field order per row.
pub fn extract_rows(
    html: &str,
    def: &CardigannDefinition,
    ctx: &TemplateContext,
    now: &Zoned,
) -> Result<Vec<BTreeMap<String, String>>, String> {
    let document = Html::parse_document(html);
    let row_selector = parse_selector(&def.search.rows.selector)?;
    let mut rows = Vec::new();
    for row in document.select(&row_selector) {
        let mut values = BTreeMap::new();
        for (name, field) in &def.search.fields {
            match extract_field(row, field, ctx, now) {
                Ok(Some(value)) => {
                    values.insert(name.clone(), value);
                }
                Ok(None) => {}
                Err(reason) => {
                    // WHY: one broken cell must not fail the whole page; the
                    // row-level required checks (title, download) decide
                    // whether the row survives.
                    debug!(
                        definition_id = %def.id,
                        field = %name,
                        reason = %reason,
                        "field extraction failed; treating as absent"
                    );
                }
            }
        }
        rows.push(values);
    }
    Ok(rows)
}

fn extract_field(
    row: ElementRef,
    field: &FieldBlock,
    ctx: &TemplateContext,
    now: &Zoned,
) -> Result<Option<String>, String> {
    let raw: Option<String> = if let Some(text) = &field.text {
        Some(ctx.render(&text.0)?)
    } else {
        let element = match &field.selector {
            Some(selector) => row.select(&parse_selector(selector)?).next(),
            None => Some(row),
        };
        match element {
            None => None,
            Some(element) => {
                if let Some(case) = &field.case {
                    resolve_case(element, case)?
                } else if let Some(attribute) = &field.attribute {
                    element.value().attr(attribute).map(str::to_string)
                } else {
                    Some(element_text(element, field.remove.as_deref())?)
                }
            }
        }
    };

    let raw = raw.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    match raw {
        None if field.optional => Ok(None),
        None => Err("no value extracted".to_string()),
        Some(value) => filters::apply(value, &field.filters, now).map(Some),
    }
}

/// Resolves a `case:` block: the first selector matching the element itself
/// or one of its descendants supplies the value.
fn resolve_case(
    element: ElementRef,
    case: &crate::client::cardigann::definition::OrderedPairs,
) -> Result<Option<String>, String> {
    for (selector_str, value) in &case.0 {
        let selector = parse_selector(selector_str)?;
        if selector.matches(&element) || element.select(&selector).next().is_some() {
            return Ok(Some(value.0.clone()));
        }
    }
    Ok(None)
}

/// Whitespace-normalized text of `element`, excluding subtrees matched by
/// `remove`.
///
/// WHY: Cardigann definitions are authored against jsoup's `Element.text()`,
/// which collapses internal whitespace — source-formatting newlines/indent
/// must not leak into extracted values. Attribute extraction is never
/// normalized (jsoup leaves attributes verbatim).
fn element_text(element: ElementRef, remove: Option<&str>) -> Result<String, String> {
    let raw = match remove {
        None => element.text().collect::<String>(),
        Some(remove) => {
            let selector = parse_selector(remove)?;
            let removed: HashSet<_> = element.select(&selector).map(|e| e.id()).collect();
            let mut out = String::new();
            collect_text(*element, &removed, &mut out);
            out
        }
    };
    Ok(normalize_whitespace(&raw))
}

/// Collapses runs of ASCII whitespace to single spaces and trims the ends.
///
/// WHY: ASCII-only on purpose — jsoup collapses only ASCII whitespace and
/// preserves U+00A0 (`&nbsp;`), which definition filters may split on.
fn normalize_whitespace(value: &str) -> String {
    value
        .split(|c: char| c.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_text(
    node: ego_tree::NodeRef<scraper::Node>,
    removed: &HashSet<ego_tree::NodeId>,
    out: &mut String,
) {
    for child in node.children() {
        if removed.contains(&child.id()) {
            continue;
        }
        if let Some(text) = child.value().as_text() {
            out.push_str(text);
        }
        collect_text(child, removed, out);
    }
}

/// Evaluates one `login.error` block against a post-login page: `selector`
/// decides whether the page reports a failed login; `message` (FieldBlock
/// semantics, evaluated from the document root like upstream Cardigann)
/// refines the reported text, falling back to the matched element's text.
///
/// Returns `Ok(None)` when the selector does not match (no error).
pub fn extract_error_message(
    html: &str,
    selector: &str,
    message: Option<&FieldBlock>,
    ctx: &TemplateContext,
    now: &Zoned,
) -> Result<Option<String>, String> {
    let document = Html::parse_document(html);
    let Some(element) = document.select(&parse_selector(selector)?).next() else {
        return Ok(None);
    };
    if let Some(field) = message
        && let Ok(Some(value)) = extract_field(document.root_element(), field, ctx, now)
    {
        return Ok(Some(value));
    }
    let text = element_text(element, None)?;
    Ok(Some(if text.is_empty() {
        "site reported a login error".to_string()
    } else {
        text
    }))
}

/// True when `selector` matches anywhere in `html` (`login.test` assertion).
pub fn selector_matches(html: &str, selector: &str) -> Result<bool, String> {
    let document = Html::parse_document(html);
    Ok(document.select(&parse_selector(selector)?).next().is_some())
}

/// Pulls the follow-on download link out of a details/interstitial page
/// (the `download:` block's selector + attribute).
pub fn extract_download_link(
    html: &str,
    selector: &str,
    attribute: Option<&str>,
) -> Result<String, String> {
    let document = Html::parse_document(html);
    let selector = parse_selector(selector)?;
    let element = document
        .select(&selector)
        .next()
        .ok_or_else(|| "download selector matched nothing".to_string())?;
    let attribute = attribute.unwrap_or("href");
    element
        .value()
        .attr(attribute)
        .map(str::to_string)
        .ok_or_else(|| format!("download element has no {attribute:?} attribute"))
}

fn parse_selector(selector: &str) -> Result<Selector, String> {
    Selector::parse(selector).map_err(|e| format!("selector {selector:?}: {e}"))
}

/// Parses a human-readable size ("1.5 GB", "700MB", "1,024 KiB") to bytes.
///
/// NOTE: decimal suffixes are treated as binary multiples (KB = 1024), which
/// is what Cardigann-compatible indexers conventionally emit.
pub fn parse_size(value: &str) -> Option<u64> {
    let cleaned = value.trim().replace(',', "");
    // WHY: a unitless numeric size is a raw byte count (the JSON-API
    // convention) — upstream `ParseUtil.GetBytes` falls through to the value
    // itself when no unit matches. A human-formatted size carries a unit.
    let (number_str, exponent): (&str, i32) = match cleaned.find(|c: char| c.is_ascii_alphabetic())
    {
        Some(unit_start) => {
            let exponent = match cleaned
                .get(unit_start..)?
                .trim()
                .to_ascii_uppercase()
                .as_str()
            {
                "B" => 0,
                "K" | "KB" | "KIB" => 1,
                "M" | "MB" | "MIB" => 2,
                "G" | "GB" | "GIB" => 3,
                "T" | "TB" | "TIB" => 4,
                "P" | "PB" | "PIB" => 5,
                _ => return None,
            };
            (cleaned.get(..unit_start)?, exponent)
        }
        None => (cleaned.as_str(), 0),
    };
    let number: f64 = number_str.trim().parse().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    let bytes = number * 1024_f64.powi(exponent);
    // NOTE: 2^63 bounds the round-trip: every finite f64 below it fits u64.
    if bytes >= 9_223_372_036_854_775_808.0 {
        return None;
    }
    // kanon:ignore RUST/as-cast -- f64→u64 has no TryFrom; range-guarded above
    Some(bytes.round() as u64)
}

/// Parses an integer that may carry thousands separators or whitespace.
pub fn parse_u32_loose(value: &str) -> Option<u32> {
    value.trim().replace([',', ' '], "").parse().ok()
}

pub fn parse_f64_loose(value: &str) -> Option<f64> {
    value.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::cardigann::definition::parse_definition;

    fn utc_now() -> Zoned {
        "2026-06-15T12:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .to_zoned(jiff::tz::TimeZone::UTC)
    }

    #[test]
    fn remove_selector_excludes_subtree_text() {
        let def = parse_definition(
            r#"
id: r
name: R
links: ["https://r.example/"]
caps:
  categorymappings:
    - {id: 1, cat: Movies}
  modes:
    search: [q]
search:
  paths:
    - path: /
  rows:
    selector: div.row
  fields:
    title:
      selector: span.name
      remove: span.tag
    download:
      selector: a
      attribute: href
"#,
            "test",
        )
        .unwrap();
        let html = r#"<div class="row">
            <span class="name">Good Title<span class="tag"> [FL]</span></span>
        </div>"#;
        let rows = extract_rows(html, &def, &TemplateContext::default(), &utc_now()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("title").map(String::as_str), Some("Good Title"));
    }

    #[test]
    fn field_without_selector_reads_row_text() {
        let def = parse_definition(
            r#"
id: r
name: R
links: ["https://r.example/"]
caps:
  categorymappings:
    - {id: 1, cat: Movies}
  modes:
    search: [q]
search:
  paths:
    - path: /
  rows:
    selector: li
  fields:
    title: {}
    download:
      selector: a
      attribute: href
"#,
            "test",
        )
        .unwrap();
        let rows = extract_rows(
            "<ul><li> Whole Row </li></ul>",
            &def,
            &TemplateContext::default(),
            &utc_now(),
        )
        .unwrap();
        assert_eq!(rows[0].get("title").map(String::as_str), Some("Whole Row"));
    }

    #[test]
    fn text_extraction_collapses_whitespace_but_attributes_keep_it() {
        let def = parse_definition(
            r#"
id: r
name: R
links: ["https://r.example/"]
caps:
  categorymappings:
    - {id: 1, cat: Movies}
  modes:
    search: [q]
search:
  paths:
    - path: /
  rows:
    selector: div.row
  fields:
    title:
      selector: span
    download:
      selector: a
      attribute: href
"#,
            "test",
        )
        .unwrap();
        let html = "<div class=\"row\"><span>Good\n  <b>Title</b>\n  Extra</span>\
                    <a href=\"x  y.torrent\">dl</a></div>";
        let rows = extract_rows(html, &def, &TemplateContext::default(), &utc_now()).unwrap();
        assert_eq!(
            rows[0].get("title").map(String::as_str),
            Some("Good Title Extra")
        );
        // WHY: attribute values must stay verbatim — jsoup only
        // normalizes text.
        assert_eq!(
            rows[0].get("download").map(String::as_str),
            Some("x  y.torrent")
        );
    }

    #[test]
    fn extract_download_link_reads_attribute() {
        let html = r#"<html><body><a id="dl" href="/file.torrent">get</a></body></html>"#;
        assert_eq!(
            extract_download_link(html, "a#dl", None).unwrap(),
            "/file.torrent"
        );
        assert_eq!(
            extract_download_link(html, "a#dl", Some("id")).unwrap(),
            "dl"
        );
        assert!(extract_download_link(html, "a.missing", None).is_err());
        assert!(extract_download_link(html, "a#dl", Some("nope")).is_err());
    }

    #[test]
    fn parse_size_units() {
        assert_eq!(parse_size("700 MB"), Some(700 * 1024 * 1024));
        assert_eq!(parse_size("700MB"), Some(700 * 1024 * 1024));
        assert_eq!(parse_size("1.5 GiB"), Some(1_610_612_736));
        assert_eq!(parse_size("1,024 KB"), Some(1024 * 1024));
        assert_eq!(parse_size("2 TB"), Some(2 * 1024_u64.pow(4)));
        assert_eq!(parse_size("512 B"), Some(512));
    }

    #[test]
    fn parse_size_rejects_garbage() {
        assert_eq!(parse_size("unknown"), None);
        assert_eq!(parse_size("12 XB"), None);
        assert_eq!(parse_size("-3 GB"), None);
        assert_eq!(parse_size(""), None);
    }

    #[test]
    fn parse_size_unitless_is_raw_bytes() {
        // WHY: JSON APIs report size as a raw byte count with no unit.
        assert_eq!(parse_size("734003216"), Some(734_003_216));
        assert_eq!(parse_size("0"), Some(0));
        assert_eq!(parse_size("1,610,612,736"), Some(1_610_612_736));
    }

    #[test]
    fn parse_u32_loose_strips_separators() {
        assert_eq!(parse_u32_loose(" 1,234 "), Some(1234));
        assert_eq!(parse_u32_loose("42"), Some(42));
        assert_eq!(parse_u32_loose("n/a"), None);
    }
}
