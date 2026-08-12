//! Template evaluator tests (block constructs, validation, rendering).

use std::collections::BTreeMap;

use super::*;

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
        ctx().render("no templates here").unwrap(),
        "no templates here"
    );
}

#[test]
fn keywords_and_surrounding_text() {
    assert_eq!(
        ctx().render("/search?q={{ .Keywords }}&x=1").unwrap(),
        "/search?q=test query&x=1"
    );
}

#[test]
fn categories_join_comma_by_default() {
    assert_eq!(ctx().render("{{ .Categories }}").unwrap(), "6,12");
}

#[test]
fn join_with_custom_separator() {
    assert_eq!(
        ctx().render("{{ join .Categories \";\" }}").unwrap(),
        "6;12"
    );
}

#[test]
fn config_lookup() {
    assert_eq!(ctx().render("{{ .Config.sort }}").unwrap(), "created");
}

#[test]
fn config_missing_renders_empty_after_load_validation() {
    // WHY: load-time validate() rejects undeclared keys, so render only
    // ever sees a declared-but-unset key — which is false-valued and
    // renders empty, matching upstream's missing-variable behavior.
    assert_eq!(ctx().render("[{{ .Config.missing }}]").unwrap(), "[]");
}

#[test]
fn query_field_lookup_and_default_empty() {
    assert_eq!(ctx().render("S{{ .Query.Season }}").unwrap(), "S3");
    assert_eq!(ctx().render("[{{ .Query.Ep }}]").unwrap(), "[]");
}

#[test]
fn result_references_read_the_row_scope() {
    let mut c = ctx();
    c.result.insert("year".to_string(), "2024".to_string());
    assert_eq!(c.render("{{ .Result.year }}").unwrap(), "2024");
    assert_eq!(c.render("[{{ .Result.missing }}]").unwrap(), "[]");
    assert_eq!(
        c.render("{{ or .Result.missing .Result.year }}").unwrap(),
        "2024"
    );
}

#[test]
fn unsupported_constructs_error() {
    for tmpl in [
        "{{ .Keywords | tolower }}",
        "{{ with .x }}{{ end }}",
        "{{ not .Keywords }}",
        "{{ .Result.date | jsomething }}",
    ] {
        assert!(ctx().render(tmpl).is_err(), "should reject {tmpl}");
    }
}

#[test]
fn unclosed_braces_error() {
    assert!(ctx().render("{{ .Keywords").is_err());
}

#[test]
fn multiple_expressions() {
    assert_eq!(
        ctx().render("{{ .Keywords }}-{{ .Config.sort }}").unwrap(),
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
        c.render_url("/browse.php?search={{ .Keywords }}&cat=0")
            .unwrap(),
        "/browse.php?search=AT%26T+%231&cat=0"
    );
    // WHY: join output is data too — an "&" separator must not
    // masquerade as a query delimiter.
    assert_eq!(
        c.render_url("{{ join .Categories \"&\" }}").unwrap(),
        "6%2612"
    );
}

#[test]
fn validate_accepts_known_and_rejects_unknown() {
    assert!(validate("{{ .Keywords }} {{ .Config.sort }}", &["sort"]).is_ok());
    assert!(validate("{{ .Config.cookie }}", &[]).is_ok());
    assert!(validate("{{ .Config.nope }}", &["sort"]).is_err());
    assert!(validate("{{ if .x }}{{ end }}", &[]).is_err());
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
        c.render("{{ if .Config.freeleech }}fl=1{{ else }}fl=0{{ end }}")
            .unwrap(),
        "fl=1"
    );
    assert_eq!(
        c.render("{{ if .Config.multilang }}ml=1{{ else }}ml=0{{ end }}")
            .unwrap(),
        "ml=0"
    );
}

#[test]
fn if_supports_and_or_eq_ne_and_parens() {
    let c = block_ctx();
    assert_eq!(
        c.render("{{ if and .Keywords .Config.freeleech }}yes{{ else }}no{{ end }}")
            .unwrap(),
        "yes"
    );
    assert_eq!(
        c.render("{{ if or .Config.multilang .Config.freeleech }}yes{{ else }}no{{ end }}")
            .unwrap(),
        "yes"
    );
    assert_eq!(
        c.render("{{ if eq .Config.sort \"created\" }}new{{ else }}old{{ end }}")
            .unwrap(),
        "new"
    );
    assert_eq!(
        c.render("{{ if ne .Config.sort \"score\" }}ok{{ end }}")
            .unwrap(),
        "ok"
    );
    assert_eq!(
        c.render("{{ if and (eq .Config.sort \"created\") .Keywords }}both{{ end }}")
            .unwrap(),
        "both"
    );
}

#[test]
fn if_missing_query_field_is_false() {
    let c = block_ctx();
    assert_eq!(
        c.render("{{ if .Query.IMDBID }}imdb{{ else }}keywords{{ end }}")
            .unwrap(),
        "keywords"
    );
    assert_eq!(
        c.render("{{ if eq .Query.IMDBID .False }}no-id{{ end }}")
            .unwrap(),
        "no-id"
    );
}

#[test]
fn range_categories_repeats_body_with_dot() {
    let c = block_ctx();
    assert_eq!(
        c.render("{{ range .Categories }}&cat[]={{ . }}{{ end }}")
            .unwrap(),
        "&cat[]=6&cat[]=12"
    );
}

#[test]
fn nested_if_blocks_evaluate() {
    let c = block_ctx();
    let tmpl = "{{ if .Keywords }}k{{ if .Config.freeleech }}+fl{{ end }}{{ end }}";
    assert_eq!(c.render(tmpl).unwrap(), "k+fl");
}

#[test]
fn or_as_value_returns_first_truthy() {
    let c = block_ctx();
    assert_eq!(
        c.render("{{ or .Query.IMDBID .Keywords }}").unwrap(),
        "test query"
    );
    assert_eq!(
        c.render("{{ or .Config.multilang .Config.sort }}").unwrap(),
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
        c.render_url("{{ if .Keywords }}/search?q={{ .Keywords }}{{ else }}/browse{{ end }}")
            .unwrap(),
        "/search?q=a%26b"
    );
}

#[test]
fn unbalanced_blocks_error() {
    let c = block_ctx();
    assert!(c.render("{{ if .Keywords }}x").is_err());
    assert!(c.render("{{ end }}").is_err());
    assert!(c.render("{{ else }}").is_err());
    assert!(c.render("{{ range .Categories }}{{ . }}").is_err());
}

#[test]
fn dot_and_range_misuse_error() {
    let c = block_ctx();
    assert!(c.render("{{ . }}").is_err());
    assert!(c.render("{{ range .Keywords }}x{{ end }}").is_err());
    assert!(
        c.render("{{ if .Keywords }}x{{ else }}y{{ else }}z{{ end }}")
            .is_err()
    );
}

#[test]
fn validate_checks_condition_atoms() {
    assert!(validate("{{ if .Config.sort }}a{{ end }}", &["sort"]).is_ok());
    assert!(validate("{{ if .Config.nope }}a{{ end }}", &["sort"]).is_err());
    assert!(validate("{{ range .Categories }}{{ . }}{{ end }}", &[]).is_ok());
    assert!(validate("{{ range .Keywords }}x{{ end }}", &[]).is_err());
    assert!(validate("{{ . }}", &[]).is_err());
    assert!(validate("{{ if .Keywords }}x", &[]).is_err());
}

#[test]
fn validate_row_scoped_gates_result_references() {
    let fields = vec!["title".to_string(), "year".to_string()];
    assert!(validate("{{ .Result.year }}", &[]).is_err());
    assert!(validate_row_scoped("{{ .Result.year }}", &[], &fields).is_ok());
    assert!(validate_row_scoped("{{ .Result.nope }}", &[], &fields).is_err());
    assert!(validate_row_scoped("{{ if .Result.year }}y{{ end }}", &[], &fields).is_ok());
}
