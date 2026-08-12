//! Template evaluator tests (block constructs, parse-time scope, rendering).

use std::collections::BTreeMap;

use super::*;

/// Every config key the test templates reference, declared as a definition's
/// settings would declare them — parse rejects undeclared keys at load.
const CONFIG_KEYS: &[&str] = &["sort", "freeleech", "multilang", "vip", "missing"];

/// Parses in search/login scope (the position most templates live in).
fn parsed(source: &str) -> ParsedTemplate {
    ParsedTemplate::parse(source, CONFIG_KEYS).unwrap()
}

/// Parses in row scope with the fields the test templates reference declared.
fn parsed_row(source: &str) -> ParsedTemplate {
    let fields = vec!["year".to_string(), "missing".to_string()];
    ParsedTemplate::parse_row_scoped(source, CONFIG_KEYS, &fields).unwrap()
}

fn ctx() -> TemplateContext {
    TemplateContext {
        keywords: "test query".to_string(),
        categories: vec!["6".to_string(), "12".to_string()],
        config: BTreeMap::from([("sort".to_string(), "created".to_string())]),
        query: BTreeMap::from([("Season", "3".to_string())]),
        ..Default::default()
    }
}

#[test]
fn plain_text_passes_through() {
    assert_eq!(
        ctx().render(&parsed("no templates here")).unwrap(),
        "no templates here"
    );
}

#[test]
fn keywords_and_surrounding_text() {
    assert_eq!(
        ctx()
            .render(&parsed("/search?q={{ .Keywords }}&x=1"))
            .unwrap(),
        "/search?q=test query&x=1"
    );
}

#[test]
fn categories_join_comma_by_default() {
    assert_eq!(ctx().render(&parsed("{{ .Categories }}")).unwrap(), "6,12");
}

#[test]
fn join_with_custom_separator() {
    assert_eq!(
        ctx()
            .render(&parsed("{{ join .Categories \";\" }}"))
            .unwrap(),
        "6;12"
    );
}

#[test]
fn config_lookup() {
    assert_eq!(
        ctx().render(&parsed("{{ .Config.sort }}")).unwrap(),
        "created"
    );
}

#[test]
fn config_checkbox_strings_render_literally_in_value_position() {
    // WHY: checkbox settings are stored as the literal strings
    // "true"/"false", and definitions substitute them directly into
    // inputs/URLs — only conditions read them as booleans.
    let c = TemplateContext {
        config: BTreeMap::from([
            ("freeleech".to_string(), "false".to_string()),
            ("vip".to_string(), "true".to_string()),
        ]),
        ..Default::default()
    };
    assert_eq!(
        c.render(&parsed("fl={{ .Config.freeleech }}")).unwrap(),
        "fl=false"
    );
    assert_eq!(
        c.render(&parsed("vip={{ .Config.vip }}")).unwrap(),
        "vip=true"
    );
    assert_eq!(
        c.render(&parsed(
            "{{ if .Config.freeleech }}yes{{ else }}no{{ end }}"
        ))
        .unwrap(),
        "no"
    );
}

#[test]
fn config_declared_but_unset_renders_empty() {
    // WHY: parse rejects undeclared keys at load, so render only ever sees a
    // declared-but-unset key — which is false-valued and renders empty,
    // matching upstream's missing-variable behavior.
    assert_eq!(
        ctx().render(&parsed("[{{ .Config.missing }}]")).unwrap(),
        "[]"
    );
}

#[test]
fn query_field_lookup_and_default_empty() {
    assert_eq!(ctx().render(&parsed("S{{ .Query.Season }}")).unwrap(), "S3");
    assert_eq!(ctx().render(&parsed("[{{ .Query.Ep }}]")).unwrap(), "[]");
}

#[test]
fn result_references_read_the_row_scope() {
    let mut c = ctx();
    c.result.insert("year".to_string(), "2024".to_string());
    assert_eq!(c.render(&parsed_row("{{ .Result.year }}")).unwrap(), "2024");
    assert_eq!(
        c.render(&parsed_row("[{{ .Result.missing }}]")).unwrap(),
        "[]"
    );
    assert_eq!(
        c.render(&parsed_row("{{ or .Result.missing .Result.year }}"))
            .unwrap(),
        "2024"
    );
}

#[test]
fn unsupported_constructs_rejected_at_parse() {
    for tmpl in [
        "{{ .Keywords | tolower }}",
        "{{ with .x }}{{ end }}",
        "{{ not .Keywords }}",
        "{{ .Result.date | jsomething }}",
    ] {
        assert!(
            ParsedTemplate::parse(tmpl, &[]).is_err(),
            "should reject {tmpl}"
        );
    }
}

#[test]
fn unclosed_braces_rejected_at_parse() {
    assert!(ParsedTemplate::parse("{{ .Keywords", &[]).is_err());
}

#[test]
fn multiple_expressions() {
    assert_eq!(
        ctx()
            .render(&parsed("{{ .Keywords }}-{{ .Config.sort }}"))
            .unwrap(),
        "test query-created"
    );
}

#[test]
fn render_url_encodes_expansions_but_not_structure() {
    let c = TemplateContext {
        keywords: "AT&T #1".to_string(),
        ..ctx()
    };
    assert_eq!(
        c.render_url(&parsed("/browse.php?search={{ .Keywords }}&cat=0"))
            .unwrap(),
        "/browse.php?search=AT%26T+%231&cat=0"
    );
    // WHY: join output is data too — an "&" separator must not
    // masquerade as a query delimiter.
    assert_eq!(
        c.render_url(&parsed("{{ join .Categories \"&\" }}"))
            .unwrap(),
        "6%2612"
    );
}

#[test]
fn parse_accepts_known_and_rejects_unknown_config_keys() {
    assert!(ParsedTemplate::parse("{{ .Keywords }} {{ .Config.sort }}", &["sort"]).is_ok());
    assert!(ParsedTemplate::parse("{{ .Config.cookie }}", &[]).is_ok());
    assert!(ParsedTemplate::parse("{{ .Config.nope }}", &["sort"]).is_err());
    assert!(ParsedTemplate::parse("{{ if .x }}{{ end }}", &[]).is_err());
}

// ── block constructs (#513) ──────────────────────────────────────────

fn block_ctx() -> TemplateContext {
    TemplateContext {
        keywords: "test query".to_string(),
        categories: vec!["6".to_string(), "12".to_string()],
        config: BTreeMap::from([
            ("freeleech".to_string(), "true".to_string()),
            ("multilang".to_string(), "false".to_string()),
            ("sort".to_string(), "created".to_string()),
        ]),
        query: BTreeMap::from([("Q", "test query".to_string())]),
        ..Default::default()
    }
}

#[test]
fn if_else_end_selects_branch_on_checkbox_truthiness() {
    // WHY: checkbox settings are stored as the literal strings
    // "true"/"false"; a condition must read them as booleans (upstream
    // settings are typed bools), or `if .Config.freeleech` would treat a
    // "false" string as truthy.
    let c = block_ctx();
    assert_eq!(
        c.render(&parsed(
            "{{ if .Config.freeleech }}fl=1{{ else }}fl=0{{ end }}"
        ))
        .unwrap(),
        "fl=1"
    );
    assert_eq!(
        c.render(&parsed(
            "{{ if .Config.multilang }}ml=1{{ else }}ml=0{{ end }}"
        ))
        .unwrap(),
        "ml=0"
    );
}

#[test]
fn if_supports_and_or_eq_ne_and_parens() {
    let c = block_ctx();
    assert_eq!(
        c.render(&parsed(
            "{{ if and .Keywords .Config.freeleech }}yes{{ else }}no{{ end }}"
        ))
        .unwrap(),
        "yes"
    );
    assert_eq!(
        c.render(&parsed(
            "{{ if or .Config.multilang .Config.freeleech }}yes{{ else }}no{{ end }}"
        ))
        .unwrap(),
        "yes"
    );
    assert_eq!(
        c.render(&parsed(
            "{{ if eq .Config.sort \"created\" }}new{{ else }}old{{ end }}"
        ))
        .unwrap(),
        "new"
    );
    assert_eq!(
        c.render(&parsed("{{ if ne .Config.sort \"score\" }}ok{{ end }}"))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        c.render(&parsed(
            "{{ if and (eq .Config.sort \"created\") .Keywords }}both{{ end }}"
        ))
        .unwrap(),
        "both"
    );
}

#[test]
fn if_missing_query_field_is_false() {
    let c = block_ctx();
    assert_eq!(
        c.render(&parsed(
            "{{ if .Query.IMDBID }}imdb{{ else }}keywords{{ end }}"
        ))
        .unwrap(),
        "keywords"
    );
    assert_eq!(
        c.render(&parsed("{{ if eq .Query.IMDBID .False }}no-id{{ end }}"))
            .unwrap(),
        "no-id"
    );
}

#[test]
fn range_categories_repeats_body_with_dot() {
    let c = block_ctx();
    assert_eq!(
        c.render(&parsed("{{ range .Categories }}&cat[]={{ . }}{{ end }}"))
            .unwrap(),
        "&cat[]=6&cat[]=12"
    );
}

#[test]
fn nested_if_blocks_evaluate() {
    let c = block_ctx();
    let tmpl = parsed("{{ if .Keywords }}k{{ if .Config.freeleech }}+fl{{ end }}{{ end }}");
    assert_eq!(c.render(&tmpl).unwrap(), "k+fl");
}

#[test]
fn or_as_value_returns_first_truthy() {
    let c = block_ctx();
    assert_eq!(
        c.render(&parsed("{{ or .Query.IMDBID .Keywords }}"))
            .unwrap(),
        "test query"
    );
    assert_eq!(
        c.render(&parsed("{{ or .Config.multilang .Config.sort }}"))
            .unwrap(),
        "created"
    );
}

#[test]
fn render_url_encodes_expansions_inside_taken_branch() {
    let c = TemplateContext {
        keywords: "a&b".to_string(),
        ..block_ctx()
    };
    assert_eq!(
        c.render_url(&parsed(
            "{{ if .Keywords }}/search?q={{ .Keywords }}{{ else }}/browse{{ end }}"
        ))
        .unwrap(),
        "/search?q=a%26b"
    );
}

#[test]
fn unbalanced_blocks_rejected_at_parse() {
    assert!(ParsedTemplate::parse("{{ if .Keywords }}x", CONFIG_KEYS).is_err());
    assert!(ParsedTemplate::parse("{{ end }}", CONFIG_KEYS).is_err());
    assert!(ParsedTemplate::parse("{{ else }}", CONFIG_KEYS).is_err());
    assert!(ParsedTemplate::parse("{{ range .Categories }}{{ . }}", CONFIG_KEYS).is_err());
}

#[test]
fn dot_and_range_misuse_rejected_at_parse() {
    assert!(ParsedTemplate::parse("{{ . }}", CONFIG_KEYS).is_err());
    assert!(ParsedTemplate::parse("{{ range .Keywords }}x{{ end }}", CONFIG_KEYS).is_err());
    assert!(
        ParsedTemplate::parse(
            "{{ if .Keywords }}x{{ else }}y{{ else }}z{{ end }}",
            CONFIG_KEYS
        )
        .is_err()
    );
}

#[test]
fn parse_checks_condition_atoms() {
    assert!(ParsedTemplate::parse("{{ if .Config.sort }}a{{ end }}", &["sort"]).is_ok());
    assert!(ParsedTemplate::parse("{{ if .Config.nope }}a{{ end }}", &["sort"]).is_err());
    assert!(ParsedTemplate::parse("{{ range .Categories }}{{ . }}{{ end }}", &[]).is_ok());
    assert!(ParsedTemplate::parse("{{ range .Keywords }}x{{ end }}", &[]).is_err());
    assert!(ParsedTemplate::parse("{{ . }}", &[]).is_err());
    assert!(ParsedTemplate::parse("{{ if .Keywords }}x", &[]).is_err());
}

#[test]
fn parse_row_scoped_gates_result_references() {
    let fields = vec!["title".to_string(), "year".to_string()];
    assert!(ParsedTemplate::parse("{{ .Result.year }}", &[]).is_err());
    assert!(ParsedTemplate::parse_row_scoped("{{ .Result.year }}", &[], &fields).is_ok());
    assert!(ParsedTemplate::parse_row_scoped("{{ .Result.nope }}", &[], &fields).is_err());
    assert!(
        ParsedTemplate::parse_row_scoped("{{ if .Result.year }}y{{ end }}", &[], &fields).is_ok()
    );
}
