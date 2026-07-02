//! Minimal evaluator for the Go-template subset Cardigann definitions use.
//!
//! Supported: `{{ .Keywords }}`, `{{ .Categories }}` (comma-joined),
//! `{{ .Config.<key> }}`, `{{ .Query.<field> }}`, and
//! `{{ join .Categories "<sep>" }}`. Anything else — `if`, `range`, `with`,
//! pipelines — is rejected with a clear reason at definition load time.

use std::collections::BTreeMap;

/// Values a template can reference during one search.
#[derive(Clone, Default)]
pub struct TemplateContext {
    // NOTE: search keywords, not a credential — the name trips the
    // key-shaped-field heuristic.
    pub keywords: String, // kanon:ignore RUST/plain-string-secret -- search keywords, not a secret
    /// Site-native category ids mapped from the query's Torznab categories.
    pub categories: Vec<String>,
    /// Setting name → value (definition defaults, plus the session cookie).
    pub config: BTreeMap<String, String>,
    /// `.Query.<field>` values; absent query parts render as empty strings.
    pub query: BTreeMap<&'static str, String>,
}

// WHY: manual Debug — `config` can carry the session cookie, so values are
// redacted and only the referenceable key names are shown.
impl std::fmt::Debug for TemplateContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateContext")
            .field("keywords", &self.keywords)
            .field("categories", &self.categories)
            .field("config_keys", &self.config.keys().collect::<Vec<_>>())
            .field("query", &self.query)
            .finish()
    }
}

impl TemplateContext {
    /// Renders `template` against this context. Errors carry a
    /// human-readable reason.
    pub fn render(&self, template: &str) -> Result<String, String> {
        walk(template, &mut |expr| eval_expr(expr, self))
    }
}

/// `.Query.` fields this evaluator resolves.
pub const QUERY_FIELDS: &[&str] = &[
    "Q",
    "Season",
    "Ep",
    "IMDBID",
    "IMDBIDShort",
    "TVDBID",
    "TMDBID",
    "Artist",
    "Album",
    "Author",
    "Limit",
    "Offset",
];

/// Checks every `{{ ... }}` expression in `template` without a runtime
/// context, so unsupported constructs surface at definition load.
pub fn validate(template: &str, config_keys: &[&str]) -> Result<(), String> {
    walk(template, &mut |expr| {
        validate_expr(expr, config_keys).map(|()| String::new())
    })
    .map(|_| ())
}

fn walk(
    template: &str,
    on_expr: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<String, String> {
    let unclosed = || format!("unclosed {{{{ in template {template:?}");
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(rest.get(..start).unwrap_or_default());
        let after = rest.get(start + 2..).ok_or_else(unclosed)?;
        let end = after.find("}}").ok_or_else(unclosed)?;
        let expr = after.get(..end).ok_or_else(unclosed)?;
        out.push_str(&on_expr(expr.trim())?);
        rest = after.get(end + 2..).unwrap_or_default();
    }
    out.push_str(rest);
    Ok(out)
}

fn eval_expr(expr: &str, ctx: &TemplateContext) -> Result<String, String> {
    match classify(expr)? {
        Expr::Keywords => Ok(ctx.keywords.clone()),
        Expr::Categories => Ok(ctx.categories.join(",")),
        Expr::Join(sep) => Ok(ctx.categories.join(&sep)),
        Expr::Config(key) => ctx
            .config
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("unknown config key {key:?}")),
        Expr::Query(field) => Ok(ctx.query.get(field).cloned().unwrap_or_default()),
    }
}

fn validate_expr(expr: &str, config_keys: &[&str]) -> Result<(), String> {
    match classify(expr)? {
        Expr::Keywords | Expr::Categories | Expr::Join(_) | Expr::Query(_) => Ok(()),
        // WHY: "cookie" is injected at client construction from the indexer
        // row, so cookie-login definitions may reference it without a
        // matching settings entry.
        Expr::Config(key) if key == "cookie" || config_keys.contains(&key.as_str()) => Ok(()),
        Expr::Config(key) => Err(format!("unknown config key {key:?}")),
    }
}

enum Expr {
    Keywords,
    Categories,
    Config(String),
    Query(&'static str),
    Join(String),
}

fn classify(expr: &str) -> Result<Expr, String> {
    if expr == ".Keywords" {
        return Ok(Expr::Keywords);
    }
    if expr == ".Categories" {
        return Ok(Expr::Categories);
    }
    if let Some(key) = expr.strip_prefix(".Config.") {
        if key.is_empty() || key.contains(char::is_whitespace) {
            return Err(format!("malformed config reference {expr:?}"));
        }
        return Ok(Expr::Config(key.to_string()));
    }
    if let Some(field) = expr.strip_prefix(".Query.") {
        return QUERY_FIELDS
            .iter()
            .find(|known| **known == field)
            .map(|known| Expr::Query(known))
            .ok_or_else(|| format!("unsupported query field {field:?}"));
    }
    if let Some(rest) = expr.strip_prefix("join ") {
        let rest = rest.trim();
        let args = rest
            .strip_prefix(".Categories")
            .ok_or_else(|| format!("join supports only .Categories, got {rest:?}"))?
            .trim();
        let sep = args
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .ok_or_else(|| format!("join separator must be a quoted string, got {args:?}"))?;
        return Ok(Expr::Join(sep.to_string()));
    }
    Err(format!(
        "unsupported template construct {expr:?} (supported: .Keywords, .Categories, \
         .Config.<key>, .Query.<field>, join .Categories \"<sep>\")"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> TemplateContext {
        TemplateContext {
            keywords: "test query".to_string(),
            categories: vec!["6".to_string(), "12".to_string()],
            config: BTreeMap::from([("sort".to_string(), "created".to_string())]),
            query: BTreeMap::from([("Season", "3".to_string())]),
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
    fn config_unknown_key_errors() {
        let err = ctx().render("{{ .Config.missing }}").unwrap_err();
        assert!(err.contains("missing"), "got: {err}");
    }

    #[test]
    fn query_field_lookup_and_default_empty() {
        assert_eq!(ctx().render("S{{ .Query.Season }}").unwrap(), "S3");
        assert_eq!(ctx().render("[{{ .Query.Ep }}]").unwrap(), "[]");
    }

    #[test]
    fn unsupported_constructs_error() {
        for tmpl in [
            "{{ if .Keywords }}x{{ end }}",
            "{{ range .Categories }}{{.}}{{ end }}",
            "{{ .Keywords | tolower }}",
            "{{ .Result.date }}",
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
    fn validate_accepts_known_and_rejects_unknown() {
        assert!(validate("{{ .Keywords }} {{ .Config.sort }}", &["sort"]).is_ok());
        assert!(validate("{{ .Config.cookie }}", &[]).is_ok());
        assert!(validate("{{ .Config.nope }}", &["sort"]).is_err());
        assert!(validate("{{ if .x }}{{ end }}", &[]).is_err());
    }
}
