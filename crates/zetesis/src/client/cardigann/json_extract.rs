//! JSON row/field extraction for Cardigann definitions (`response.type: json`).
//!
//! Mirrors the HTML extractor's shape (rows -> per-field values -> filters) but
//! selects against a parsed `serde_json::Value` using the dotted/bracket path
//! grammar Jackett/Prowlarr hand to Newtonsoft `SelectToken`. This module covers
//! the FLAT-array shape: `rows.selector` resolves directly to the row array, and
//! each field selector is a path relative to a row object.
//!
//! Rejected at load (see `definition::validate`) and tracked in #513: nested
//! rows (`rows.attribute` / `rows.multiple` + the leading-`..` parent switch)
//! and the `:has()/:not()/:contains()` pseudo-filter suffix.

use std::collections::BTreeMap;

use jiff::Zoned;
use serde_json::Value;
use tracing::debug;

use crate::client::cardigann::definition::{CardigannDefinition, FieldBlock, OrderedPairs};
use crate::client::cardigann::{filters, template::TemplateContext};

/// Extracted field values for the rows in one JSON search-results body, in
/// definition field order per row.
pub fn extract_rows_json(
    body: &str,
    def: &CardigannDefinition,
    ctx: &TemplateContext,
    now: &Zoned,
) -> Result<Vec<BTreeMap<String, String>>, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| format!("invalid JSON body: {e}"))?;
    let rows_value = select_path(&root, &def.search.rows.selector)?.ok_or_else(|| {
        format!(
            "rows selector {:?} matched nothing",
            def.search.rows.selector
        )
    })?;
    let Value::Array(rows) = rows_value else {
        // WHY: parity with upstream — a rows selector that resolves to a
        // non-array is a definition/response mismatch, surfaced as an error
        // rather than silently yielding zero rows.
        return Err(format!(
            "rows selector {:?} did not resolve to a JSON array",
            def.search.rows.selector
        ));
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut values = BTreeMap::new();
        for (name, field) in &def.search.fields {
            match extract_field_json(row, field, ctx, now) {
                Ok(Some(value)) => {
                    values.insert(name.clone(), value);
                }
                Ok(None) => {}
                Err(reason) => {
                    // WHY: one broken field must not fail the whole page; the
                    // row-level required checks (title, download) decide whether
                    // the row survives.
                    debug!(
                        definition_id = %def.id,
                        field = %name,
                        reason = %reason,
                        "json field extraction failed; treating as absent"
                    );
                }
            }
        }
        out.push(values);
    }
    Ok(out)
}

fn extract_field_json(
    row: &Value,
    field: &FieldBlock,
    ctx: &TemplateContext,
    now: &Zoned,
) -> Result<Option<String>, String> {
    // WHY: `text` short-circuits selection entirely (incl. `case`), matching
    // upstream `handleJsonSelector`.
    if let Some(text) = &field.text {
        let rendered = ctx.render(&text.0)?;
        return filters::apply(rendered, &field.filters, now).map(Some);
    }

    // Resolve the selector to a raw value. A missing path yields None; a path
    // resolving to JSON `null` coerces to "" (which normalizes to blank below,
    // the same as missing — matching upstream's net result).
    let raw: Option<String> = match &field.selector {
        Some(selector) => select_path(row, selector.trim_start_matches('.'))?.map(coerce_value),
        None => None,
    };

    // `case` for JSON is a value-equality switch (NOT HTML's CSS-match): the
    // first key equal to the resolved value, or `*`, supplies the replacement.
    let raw = match &field.case {
        Some(case) => apply_case_json(raw.as_deref(), case),
        None => raw,
    };

    // Normalize and drop blank, mirroring the HTML extractor: a blank value
    // (missing path, JSON null, empty array/string, no case branch) is absent —
    // an optional field yields None, a required field errors (dropping the row).
    let value = raw.map(|v| normalize_space(&v)).filter(|v| !v.is_empty());
    match value {
        None if field.optional => Ok(None),
        None => Err("no value extracted".to_string()),
        Some(value) => filters::apply(value, &field.filters, now).map(Some),
    }
}

/// Applies a JSON `case` block: the first key equal to `value` (or `*`) yields
/// its replacement value. Returns `None` when no branch matches.
///
/// WHY: the replacement is used verbatim (not template-rendered), matching the
/// HTML extractor's `resolve_case`.
fn apply_case_json(value: Option<&str>, case: &OrderedPairs) -> Option<String> {
    for (key, replacement) in &case.0 {
        if key == "*" || value == Some(key.as_str()) {
            return Some(replacement.0.clone());
        }
    }
    None
}

/// Coerces a resolved JSON value to its Cardigann string form: `null` -> "",
/// scalars -> their string form, arrays -> comma-joined element strings.
fn coerce_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n),
        // WHY: upstream `String.Join(",", jarray)` stringifies each element.
        Value::Array(items) => items
            .iter()
            .map(scalar_string)
            .collect::<Vec<_>>()
            .join(","),
        // WHY: a field selector landing on an object is unusual; upstream
        // `.Value<string>()` would throw. Its compact JSON form is the least
        // surprising fallback (the field's filters/required-check decide fate).
        Value::Object(_) => value.to_string(),
    }
}

fn scalar_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => number_to_string(n),
        other => other.to_string(),
    }
}

/// Renders a JSON number in its Cardigann string form, stripping the trailing
/// `.0` a whole-number float carries (so `12.0` -> "12") to match upstream .NET
/// `double.ToString()`. Without this a float-serialized `seeders`/`size` value
/// fails the strict integer parse downstream and is silently lost.
fn number_to_string(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    let rendered = n.to_string();
    rendered
        .strip_suffix(".0")
        .map(str::to_owned)
        .unwrap_or(rendered)
}

/// Collapses runs of ASCII whitespace to single spaces and trims the ends.
///
/// WHY: mirrors the HTML extractor's `normalize_whitespace` so JSON and HTML
/// fields feed the shared filter pipeline identically.
fn normalize_space(value: &str) -> String {
    value
        .split(|c: char| c.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Walks a dotted/bracket path against `root`. `Ok(None)` = path not found;
/// `Err` = malformed path syntax.
///
/// Supports the subset flat Cardigann JSON definitions exercise: dotted keys
/// (`data.movies`), `[n]` array indices, and `['key']`/`["key"]` bracket keys,
/// with an optional leading `$`. The pseudo-filter suffix (`:...(...)`) and the
/// leading-`..` parent switch are rejected at load, so they never reach here.
fn select_path<'a>(root: &'a Value, path: &str) -> Result<Option<&'a Value>, String> {
    let mut current = root;
    for segment in parse_path_segments(path)? {
        let next = match segment {
            Segment::Key(key) => current.get(&key),
            Segment::Index(index) => current.get(index),
        };
        match next {
            Some(value) => current = value,
            None => return Ok(None),
        }
    }
    Ok(Some(current))
}

pub(crate) enum Segment {
    Key(String),
    Index(usize),
}

/// Parses a flat JSON selector into path segments. Used both to extract and,
/// at load, to verify a definition's selectors are well-formed.
pub(crate) fn parse_path_segments(path: &str) -> Result<Vec<Segment>, String> {
    let trimmed = path.trim();
    // WHY: an optional leading `$` roots the path (Newtonsoft convention); a
    // leading `.` is also tolerated.
    let mut rest = trimmed.strip_prefix('$').unwrap_or(trimmed);
    rest = rest.trim_start_matches('.');

    let mut segments = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('[') {
            let end = after
                .find(']')
                .ok_or_else(|| format!("unclosed '[' in path {path:?}"))?;
            let inner = after.get(..end).unwrap_or_default().trim();
            rest = after
                .get(end + 1..)
                .unwrap_or_default()
                .trim_start_matches('.');
            let quoted = (inner.starts_with('\'') && inner.ends_with('\'') && inner.len() >= 2)
                || (inner.starts_with('"') && inner.ends_with('"') && inner.len() >= 2);
            if quoted {
                let unquoted = inner.get(1..inner.len() - 1).unwrap_or_default();
                segments.push(Segment::Key(unquoted.to_string()));
            } else {
                let index = inner
                    .parse::<usize>()
                    .map_err(|_| format!("bad array index {inner:?} in path {path:?}"))?;
                segments.push(Segment::Index(index));
            }
        } else {
            let end = rest.find(['.', '[']).unwrap_or(rest.len());
            let key = rest.get(..end).unwrap_or_default();
            if !key.is_empty() {
                segments.push(Segment::Key(key.to_string()));
            }
            rest = rest.get(end..).unwrap_or_default().trim_start_matches('.');
        }
    }
    Ok(segments)
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

    const FLAT_DEF: &str = r#"
id: j
name: J
links: ["https://j.example/"]
caps:
  categorymappings: [{id: 1, cat: Movies}]
  modes: {search: [q]}
search:
  paths:
    - path: /api
      response:
        type: json
  rows:
    selector: data.results
  fields:
    title: {selector: name}
    download: {selector: dl}
    tags: {selector: tags}
    note: {selector: note, optional: true}
    quality:
      selector: q
      case: {"hd": "high", "*": "other"}
"#;

    fn extract(body: &str) -> Vec<BTreeMap<String, String>> {
        let def = parse_definition(FLAT_DEF, "test").unwrap();
        extract_rows_json(body, &def, &TemplateContext::default(), &utc_now()).unwrap()
    }

    #[test]
    fn flat_rows_fields_arrays_null_and_case() {
        let body = r#"{"data": {"results": [
            {"name": "A", "dl": "http://x/a", "tags": ["x264","web"], "note": null, "q": "hd"},
            {"name": "B", "dl": "http://x/b", "tags": [], "q": "sd"}
        ]}}"#;
        let rows = extract(body);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("title").map(String::as_str), Some("A"));
        assert_eq!(
            rows[0].get("download").map(String::as_str),
            Some("http://x/a")
        );
        // array -> comma-join
        assert_eq!(rows[0].get("tags").map(String::as_str), Some("x264,web"));
        // JSON null on an OPTIONAL field -> blank -> omitted (upstream parity)
        assert!(!rows[0].contains_key("note"));
        // case value-equality
        assert_eq!(rows[0].get("quality").map(String::as_str), Some("high"));
        assert_eq!(rows[1].get("quality").map(String::as_str), Some("other"));
        // empty array on a REQUIRED field -> blank -> dropped (row-required decides)
        assert!(!rows[1].contains_key("tags"));
        // missing optional field -> absent
        assert!(!rows[1].contains_key("note"));
    }

    #[test]
    fn missing_required_field_is_absent_not_error() {
        // WHY: a required field missing on a row is dropped to absent here; the
        // row-required decision happens later in rows_to_results, not extraction.
        let rows = extract(r#"{"data": {"results": [{"name": "A", "tags": [], "q": "hd"}]}}"#);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].contains_key("download"));
    }

    #[test]
    fn rows_selector_missing_errors() {
        let def = parse_definition(FLAT_DEF, "test").unwrap();
        let err = extract_rows_json(
            r#"{"data": {}}"#,
            &def,
            &TemplateContext::default(),
            &utc_now(),
        )
        .unwrap_err();
        assert!(err.contains("matched nothing"), "got {err}");
    }

    #[test]
    fn rows_selector_non_array_errors() {
        let def = parse_definition(FLAT_DEF, "test").unwrap();
        let err = extract_rows_json(
            r#"{"data": {"results": {"x": 1}}}"#,
            &def,
            &TemplateContext::default(),
            &utc_now(),
        )
        .unwrap_err();
        assert!(err.contains("did not resolve to a JSON array"), "got {err}");
    }

    #[test]
    fn invalid_json_body_errors() {
        let def = parse_definition(FLAT_DEF, "test").unwrap();
        let err = extract_rows_json("not json", &def, &TemplateContext::default(), &utc_now())
            .unwrap_err();
        assert!(err.contains("invalid JSON body"), "got {err}");
    }

    #[test]
    fn path_walker_dotted_index_and_bracket() {
        let v: Value = serde_json::from_str(r#"{"a": [{"b": 1}, {"b": 2}], "c.d": "x"}"#).unwrap();
        assert_eq!(select_path(&v, "a[1].b").unwrap().unwrap(), &Value::from(2));
        assert_eq!(select_path(&v, "a[0].b").unwrap().unwrap(), &Value::from(1));
        assert_eq!(
            select_path(&v, "['c.d']").unwrap().unwrap(),
            &Value::from("x")
        );
        assert_eq!(
            select_path(&v, "$.a[0].b").unwrap().unwrap(),
            &Value::from(1)
        );
        assert!(select_path(&v, "a[9].b").unwrap().is_none());
        assert!(select_path(&v, "missing").unwrap().is_none());
    }

    #[test]
    fn coerce_scalars_and_arrays() {
        assert_eq!(coerce_value(&Value::Null), "");
        assert_eq!(coerce_value(&Value::from(42)), "42");
        assert_eq!(coerce_value(&Value::from(true)), "true");
        assert_eq!(coerce_value(&Value::from("s")), "s");
        assert_eq!(coerce_value(&serde_json::json!(["a", 1, null])), "a,1,");
        // whole-number float -> integer form (no ".0"); fractional preserved
        assert_eq!(coerce_value(&serde_json::json!(12.0)), "12");
        assert_eq!(coerce_value(&serde_json::json!(12.5)), "12.5");
        assert_eq!(coerce_value(&serde_json::json!([1.0, 2.0])), "1,2");
    }
}
