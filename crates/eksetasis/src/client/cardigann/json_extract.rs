//! JSON row/field extraction for Cardigann definitions (`response.type: json`).
//!
//! Mirrors the HTML extractor's shape (rows -> per-field values -> filters) but
//! selects against a parsed `serde_json::Value` using the dotted/bracket path
//! grammar Jackett/Prowlarr hand to Newtonsoft `SelectToken`. Covers the flat
//! shape (`rows.selector` -> row array) and the nested shape
//! (`rows.attribute`/`multiple` drilling into each parent for its sub-rows,
//! with leading-`..` fields reading the outer parent object).
//!
//! Rejected at load (see `definition::validate`) and tracked in #513: the
//! `:has()/:not()/:contains()` pseudo-filter suffix and `$..`/mid-path
//! recursive-descent selectors.

use std::collections::BTreeMap;

use jiff::Zoned;
use serde_json::Value;
use tracing::debug;

use crate::client::cardigann::definition::{CompiledDefinition, FieldBlock, OrderedPairs};
use crate::client::cardigann::template::ParsedTemplate;
use crate::client::cardigann::{filters, template, template::TemplateContext};

/// Extracted field values for the rows in one JSON search-results body.
/// Fields are evaluated in YAML declaration order per row so `.Result`
/// templates see the values extracted before them (upstream semantics).
pub fn extract_rows_json(
    body: &str,
    def: &CompiledDefinition,
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
    let Value::Array(parents) = rows_value else {
        // WHY: parity with upstream — a rows selector that resolves to a
        // non-array is a definition/response mismatch, surfaced as an error
        // rather than silently yielding zero rows.
        return Err(format!(
            "rows selector {:?} did not resolve to a JSON array",
            def.search.rows.selector
        ));
    };

    let mut out = Vec::new();
    // WHY: one scratch context per body — the row's accumulated values ARE
    // the `.Result` scope, and `take` hands each row's map to the output
    // without cloning in the loop.
    let mut row_ctx = ctx.clone();
    for parent in parents {
        // WHY: `rows.attribute` drills into each parent for its sub-row(s) — the
        // nested shape (a movie carrying a `torrents` array). With `multiple`
        // the attribute resolves to an array iterated as multiple sub-rows, each
        // keeping the parent available for `..`-prefixed fields. Without
        // `rows.attribute` the parent IS the row (the flat shape).
        let sub_rows: Vec<(&Value, Option<&Value>)> = match &def.search.rows.attribute {
            None => vec![(parent, None)],
            Some(attr) => match select_path(parent, attr.0.trim_start_matches('.'))? {
                // WHY: a missing attribute key OR one present with JSON `null`
                // both mean the parent has no sub-rows — both honor the skip
                // flag. A common "no sub-rows" convention is `"torrents": null`;
                // without this, one null parent hard-errors the whole search.
                None | Some(Value::Null) => {
                    if def.search.rows.missing_attribute_equals_no_results {
                        continue;
                    }
                    return Err(format!(
                        "rows.attribute {:?} is absent/null on a row (set \
                         missingAttributeEqualsNoResults: true to skip)",
                        attr.0
                    ));
                }
                Some(Value::Array(items)) if def.search.rows.multiple => {
                    items.iter().map(|item| (item, Some(parent))).collect()
                }
                Some(_) if def.search.rows.multiple => {
                    return Err(format!(
                        "rows.attribute {:?} with multiple: true did not resolve to an array",
                        attr.0
                    ));
                }
                Some(value) => vec![(value, Some(parent))],
            },
        };

        for (row, parent_ctx) in sub_rows {
            for (name, field) in def.search.fields.iter() {
                match extract_field_json(row, parent_ctx, field, &row_ctx, now) {
                    Ok(Some(value)) => {
                        row_ctx.result.insert(name.clone(), value);
                    }
                    Ok(None) => {}
                    Err(reason) => {
                        // WHY: one broken field must not fail the whole page; the
                        // row-level required checks (title, download) decide
                        // whether the row survives.
                        debug!(
                            definition_id = %def.id,
                            field = %name,
                            reason = %reason,
                            "json field extraction failed; treating as absent"
                        );
                    }
                }
            }
            out.push(std::mem::take(&mut row_ctx.result));
        }
    }
    Ok(out)
}

fn extract_field_json(
    row: &Value,
    parent: Option<&Value>,
    field: &FieldBlock<ParsedTemplate>,
    ctx: &TemplateContext,
    now: &Zoned,
) -> Result<Option<String>, String> {
    // WHY: `text` short-circuits selection entirely (incl. `case`), matching
    // upstream `handleJsonSelector`.
    if let Some(text) = &field.text {
        let rendered = ctx.render(text)?;
        let specs = template::render_specs(&field.filters, ctx)?;
        return filters::apply(rendered, &specs, now).map(Some);
    }

    // Resolve the selector to a raw value. A missing path yields None; a path
    // resolving to JSON `null` coerces to "" (which normalizes to blank below,
    // the same as missing — matching upstream's net result).
    let raw: Option<String> = match &field.selector {
        Some(selector) => {
            // WHY: a leading `..` selects against the parent (outer) object in a
            // nested-rows definition (e.g. `..year` reads the movie's year while
            // the row is a torrent); with no parent it falls back to the row
            // itself, a no-op matching upstream where parentObj == Row.
            let target = if selector.trim_start().starts_with("..") {
                parent.unwrap_or(row)
            } else {
                row
            };
            select_path(target, selector.trim_start_matches('.'))?.map(coerce_value)
        }
        None => None,
    };

    // `case` for JSON is a value-equality switch (NOT HTML's CSS-match): the
    // first key equal to the resolved value, or `*`, supplies the replacement.
    let raw = match &field.case {
        Some(case) => apply_case_json(raw.as_deref(), case, ctx)?,
        None => raw,
    };

    // Normalize and drop blank, mirroring the HTML extractor: a blank value
    // (missing path, JSON null, empty array/string, no case branch) is absent —
    // an optional field yields None, a required field errors (dropping the row).
    let value = raw.map(|v| normalize_space(&v)).filter(|v| !v.is_empty());
    // WHY: `default` is a fallback value source (upstream FieldBlock.Default),
    // rendered in row scope and consulted only when extraction is blank.
    let value = match (value, &field.default) {
        (None, Some(default)) => {
            let rendered = normalize_space(&ctx.render(default)?);
            (!rendered.is_empty()).then_some(rendered)
        }
        (value, _) => value,
    };
    match value {
        None if field.optional => Ok(None),
        None => Err("no value extracted".to_string()),
        Some(value) => {
            let specs = template::render_specs(&field.filters, ctx)?;
            filters::apply(value, &specs, now).map(Some)
        }
    }
}

/// Applies a JSON `case` block: the first key equal to `value` (or `*`) yields
/// its replacement value, rendered as a template (row scope, like the HTML
/// extractor's `resolve_case`). Returns `None` when no branch matches.
fn apply_case_json(
    value: Option<&str>,
    case: &OrderedPairs<ParsedTemplate>,
    ctx: &TemplateContext,
) -> Result<Option<String>, String> {
    for (key, replacement) in &case.0 {
        if key == "*" || value == Some(key.as_str()) {
            return ctx.render(replacement).map(Some);
        }
    }
    Ok(None)
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

    const NESTED_DEF: &str = r#"
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
    selector: data.movies
    attribute: torrents
    multiple: true
  fields:
    title: {selector: ..title}
    year: {selector: ..year}
    download: {selector: url}
    quality: {selector: quality}
"#;

    #[test]
    fn nested_rows_drill_down_with_parent_switch() {
        let def = parse_definition(NESTED_DEF, "test").unwrap();
        let body = r#"{"data": {"movies": [
            {"year": 2024, "title": "Movie A", "torrents": [
                {"url": "http://x/a1", "quality": "1080p"},
                {"url": "http://x/a2", "quality": "720p"}
            ]},
            {"year": 2023, "title": "Movie B", "torrents": [
                {"url": "http://x/b1", "quality": "2160p"}
            ]}
        ]}}"#;
        let rows = extract_rows_json(body, &def, &TemplateContext::default(), &utc_now()).unwrap();
        // 2 movies -> 3 torrents (multiple: true iterates the sub-row array)
        assert_eq!(rows.len(), 3);
        // sub-row field (url) from the torrent; parent fields (..year/..title) from the movie
        assert_eq!(
            rows[0].get("download").map(String::as_str),
            Some("http://x/a1")
        );
        assert_eq!(rows[0].get("quality").map(String::as_str), Some("1080p"));
        assert_eq!(rows[0].get("year").map(String::as_str), Some("2024"));
        assert_eq!(rows[0].get("title").map(String::as_str), Some("Movie A"));
        // second sub-row of the SAME parent keeps the parent's year
        assert_eq!(
            rows[1].get("download").map(String::as_str),
            Some("http://x/a2")
        );
        assert_eq!(rows[1].get("year").map(String::as_str), Some("2024"));
        assert_eq!(
            rows[2].get("download").map(String::as_str),
            Some("http://x/b1")
        );
        assert_eq!(rows[2].get("title").map(String::as_str), Some("Movie B"));
    }

    #[test]
    fn nested_rows_missing_attribute_errors_then_skips_with_flag() {
        let body = r#"{"data": {"movies": [{"year": 2024, "title": "A"}]}}"#;
        // default (flag absent): a parent missing the attribute is an error
        let def = parse_definition(NESTED_DEF, "test").unwrap();
        let err =
            extract_rows_json(body, &def, &TemplateContext::default(), &utc_now()).unwrap_err();
        assert!(err.contains("missing"), "got {err}");
        // missingAttributeEqualsNoResults: true -> the parent is skipped
        let skip_yaml = NESTED_DEF.replace(
            "    multiple: true\n",
            "    multiple: true\n    missingAttributeEqualsNoResults: true\n",
        );
        let skip_def = parse_definition(&skip_yaml, "test").unwrap();
        let rows =
            extract_rows_json(body, &skip_def, &TemplateContext::default(), &utc_now()).unwrap();
        assert!(
            rows.is_empty(),
            "missing-attr parent should be skipped: {rows:?}"
        );
    }

    #[test]
    fn nested_rows_null_attribute_is_no_sub_rows_not_a_hard_error() {
        // A present-but-null attribute (a common "no sub-rows" convention) is
        // treated like a missing one — the skip flag applies, and without it the
        // failure is fail-loud rather than a silent whole-search drop.
        let body = r#"{"data": {"movies": [
            {"year": 2024, "title": "A", "torrents": null},
            {"year": 2023, "title": "B", "torrents": [{"url": "http://x/b1"}]}
        ]}}"#;
        let def = parse_definition(NESTED_DEF, "test").unwrap();
        assert!(
            extract_rows_json(body, &def, &TemplateContext::default(), &utc_now()).is_err(),
            "a null attribute without the skip flag must fail loud"
        );
        let skip_yaml = NESTED_DEF.replace(
            "    multiple: true\n",
            "    multiple: true\n    missingAttributeEqualsNoResults: true\n",
        );
        let skip_def = parse_definition(&skip_yaml, "test").unwrap();
        let rows =
            extract_rows_json(body, &skip_def, &TemplateContext::default(), &utc_now()).unwrap();
        assert_eq!(rows.len(), 1, "only the parent with sub-rows survives");
        assert_eq!(
            rows[0].get("download").map(String::as_str),
            Some("http://x/b1")
        );
    }

    #[test]
    fn json_result_references_compose_fields_per_row() {
        // WHY: `.Result` composition is response-format-agnostic — JSON field
        // text/case/args render against the fields extracted so far, exactly
        // like the HTML path.
        let def = parse_definition(
            r#"
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
    name: {selector: name}
    year: {selector: year, optional: true}
    title:
      text: "{{ if .Result.year }}{{ .Result.name }} ({{ .Result.year }}){{ else }}{{ .Result.name }}{{ end }}"
    download: {selector: dl}
"#,
            "test",
        )
        .unwrap();
        let body = r#"{"data": {"results": [
            {"name": "Movie", "year": 2024, "dl": "http://x/a"},
            {"name": "Plain", "dl": "http://x/b"}
        ]}}"#;
        let rows = extract_rows_json(body, &def, &TemplateContext::default(), &utc_now()).unwrap();
        assert_eq!(
            rows[0].get("title").map(String::as_str),
            Some("Movie (2024)")
        );
        assert_eq!(rows[1].get("title").map(String::as_str), Some("Plain"));
    }
}
