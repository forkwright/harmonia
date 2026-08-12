//! Evaluator for the Go-template subset Cardigann definitions use.
//!
//! Supported: `{{ .Keywords }}`, `{{ .Categories }}` (comma-joined),
//! `{{ .Config.<key> }}`, `{{ .Query.<field> }}`, `{{ .Result.<field> }}`
//! (row scope only — see [`validate_row_scoped`]),
//! `{{ join .Categories "<sep>" }}`, the block constructs
//! `{{ if <cond> }}…{{ else }}…{{ end }}` and
//! `{{ range .Categories }}…{{ . }}…{{ end }}` (blocks nest), and the
//! operators `and` / `or` (variadic, value-returning), `eq` / `ne` (two
//! operands), `(...)` grouping, quoted string literals, and the boolean
//! constants `.True` / `.False`. Anything else — pipelines, `with`,
//! variables, `not`, `else if` — is rejected with a clear reason at
//! definition load.
//!
//! Truthiness is dynamically typed, matching upstream: an absent or empty
//! value is false. Checkbox settings are stored as the literal strings
//! `"true"` / `"false"`; `.Config` resolution reads those two strings as
//! booleans so `{{ if .Config.freeleech }}` behaves like upstream's typed
//! bool settings. `.Query` / `.Result` values are never bool-normalized.

use std::collections::BTreeMap;

use crate::client::cardigann::definition::{FilterArgs, FilterSpec};

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
    /// Row scope: the values of fields extracted so far for the row being
    /// processed (YAML declaration order), backing `.Result.<field>`. Empty
    /// outside field extraction (search-path/login rendering), where load
    /// validation has already rejected `.Result` references.
    pub result: BTreeMap<String, String>,
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
            .field("result_keys", &self.result.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl TemplateContext {
    /// Renders `template` against this context. Errors carry a
    /// human-readable reason.
    pub fn render(&self, template: &str) -> Result<String, String> {
        self.render_mode(template, Mode::Plain)
    }

    /// Renders `template` for a URL context: literal template text (the
    /// path's own `?`/`&`/`=` structure and branch text) passes through
    /// untouched while every expression expansion is form-urlencoded.
    ///
    /// WHY: expansions are data, not URL structure — a keyword containing
    /// `&` must not split the query and `#` must not start a fragment.
    /// Upstream Cardigann engines encode substituted values the same way.
    pub fn render_url(&self, template: &str) -> Result<String, String> {
        self.render_mode(template, Mode::Url)
    }

    fn render_mode(&self, template: &str, mode: Mode) -> Result<String, String> {
        let nodes = parse(template)?;
        let mut out = String::with_capacity(template.len());
        emit(&nodes, self, None, mode, &mut out)?;
        Ok(out)
    }
}

/// Form-urlencodes one expanded value (`&`→`%26`, `#`→`%23`, space→`+`).
fn encode_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
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

/// Checks every expression in `template` without a runtime context, so
/// unsupported constructs surface at definition load. `.Result` references
/// are rejected here — they are meaningful only in row scope.
pub fn validate(template: &str, config_keys: &[&str]) -> Result<(), String> {
    validate_inner(template, config_keys, None)
}

/// Like [`validate`], but allows `.Result.<field>` references against the
/// definition's declared search fields. Row scope applies to field
/// `text`/`case`/`default` values and field filter args — the positions the
/// extractor renders with the row's accumulated values.
pub fn validate_row_scoped(
    template: &str,
    config_keys: &[&str],
    field_names: &[String],
) -> Result<(), String> {
    validate_inner(template, config_keys, Some(field_names))
}

fn validate_inner(
    template: &str,
    config_keys: &[&str],
    field_names: Option<&[String]>,
) -> Result<(), String> {
    let nodes = parse(template)?;
    validate_nodes(&nodes, config_keys, field_names, false)
}

fn validate_nodes(
    nodes: &[Node],
    config_keys: &[&str],
    field_names: Option<&[String]>,
    in_range: bool,
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Text(_) => {}
            Node::Value(expr) => validate_expr(expr, config_keys, field_names, in_range)?,
            Node::If {
                cond,
                then,
                otherwise,
            } => {
                validate_expr(cond, config_keys, field_names, in_range)?;
                validate_nodes(then, config_keys, field_names, in_range)?;
                validate_nodes(otherwise, config_keys, field_names, in_range)?;
            }
            Node::Range { body } => validate_nodes(body, config_keys, field_names, true)?,
        }
    }
    Ok(())
}

fn validate_expr(
    expr: &Expr,
    config_keys: &[&str],
    field_names: Option<&[String]>,
    in_range: bool,
) -> Result<(), String> {
    match expr {
        Expr::Atom(atom) => validate_atom(atom, config_keys, field_names, in_range),
        Expr::Join(_) => Ok(()),
        Expr::And(args) | Expr::Or(args) => {
            for arg in args {
                validate_expr(arg, config_keys, field_names, in_range)?;
            }
            Ok(())
        }
        Expr::Eq(a, b) | Expr::Ne(a, b) => {
            validate_expr(a, config_keys, field_names, in_range)?;
            validate_expr(b, config_keys, field_names, in_range)
        }
    }
}

fn validate_atom(
    atom: &Atom,
    config_keys: &[&str],
    field_names: Option<&[String]>,
    in_range: bool,
) -> Result<(), String> {
    match atom {
        Atom::Keywords | Atom::Categories | Atom::Bool(_) | Atom::Str(_) => Ok(()),
        Atom::Dot if in_range => Ok(()),
        Atom::Dot => Err("{{ . }} is only valid inside a range block".to_string()),
        // WHY: "cookie" is injected at client construction from the indexer
        // row, so cookie-login definitions may reference it without a
        // matching settings entry.
        Atom::Config(key) if key == "cookie" || config_keys.contains(&key.as_str()) => Ok(()),
        Atom::Config(key) => Err(format!("unknown config key {key:?}")),
        // WHY: membership in QUERY_FIELDS is enforced when the atom is
        // classified during parsing.
        Atom::Query(_) => Ok(()),
        Atom::Result(field) => {
            let Some(names) = field_names else {
                return Err(format!(
                    ".Result references are only supported in field text/case/default values \
                     and field filter args, got .Result.{field}"
                ));
            };
            if names.iter().any(|name| name == field) {
                Ok(())
            } else {
                Err(format!(".Result references undeclared field {field:?}"))
            }
        }
    }
}

/// `.Config.<key>` names referenced by `template` in VALUE positions.
///
/// Condition atoms (`{{ if .Config.x }}`) are excluded on purpose: a missing
/// or empty optional setting simply makes the branch false, so callers using
/// this list to demand non-empty values (interactive-login construction) only
/// see keys whose rendered value the definition actually substitutes.
///
/// NOTE: unparseable templates yield an empty list — load-time [`validate`]
/// has already rejected them for every template this is called on.
pub fn config_keys(template: &str) -> Vec<String> {
    let mut keys = Vec::new();
    if let Ok(nodes) = parse(template) {
        collect_value_config_keys(&nodes, &mut keys);
    }
    keys
}

fn collect_value_config_keys(nodes: &[Node], keys: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Text(_) => {}
            Node::Value(expr) => collect_expr_config_keys(expr, keys),
            Node::If {
                then, otherwise, ..
            } => {
                collect_value_config_keys(then, keys);
                collect_value_config_keys(otherwise, keys);
            }
            Node::Range { body } => collect_value_config_keys(body, keys),
        }
    }
}

fn collect_expr_config_keys(expr: &Expr, keys: &mut Vec<String>) {
    match expr {
        Expr::Atom(Atom::Config(key)) => keys.push(key.clone()),
        Expr::Atom(_) | Expr::Join(_) => {}
        Expr::And(args) | Expr::Or(args) => {
            for arg in args {
                collect_expr_config_keys(arg, keys);
            }
        }
        Expr::Eq(a, b) | Expr::Ne(a, b) => {
            collect_expr_config_keys(a, keys);
            collect_expr_config_keys(b, keys);
        }
    }
}

/// Renders every filter argument through `ctx` (plain mode).
///
/// WHY: filter args are templates (`args: " ({{ .Result.year }})"`); the
/// pipeline must never see a raw `{{ ... }}` — an unrendered arg would
/// silently corrupt the value it transforms.
pub fn render_specs(
    specs: &[FilterSpec],
    ctx: &TemplateContext,
) -> Result<Vec<FilterSpec>, String> {
    specs
        .iter()
        .map(|spec| {
            let args = spec
                .args
                .as_ref()
                .map(|args| {
                    args.0
                        .iter()
                        .map(|arg| ctx.render(arg))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;
            Ok(FilterSpec {
                name: spec.name.clone(),
                args: args.map(FilterArgs),
            })
        })
        .collect()
}

// ── parsing ─────────────────────────────────────────────────────────────

enum Node {
    Text(String),
    Value(Expr),
    If {
        cond: Expr,
        then: Vec<Node>,
        otherwise: Vec<Node>,
    },
    /// `range .Categories` — the only iterable this subset models.
    Range {
        body: Vec<Node>,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Atom(Atom),
    Join(String),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
enum Atom {
    Keywords,
    Categories,
    /// The range-loop item; valid only inside a range body.
    Dot,
    Config(String),
    Query(&'static str),
    Result(String),
    Bool(bool),
    Str(String),
}

enum Token {
    Text(String),
    Expr(String),
}

/// What terminated a nested [`parse_nodes`] call.
enum Stop {
    /// Consumed `{{ end }}`; index is past it.
    End(usize),
    /// Stopped before `{{ else }}` (not consumed).
    Else(usize),
    /// Ran out of tokens.
    Eof,
}

/// Splits `template` into literal text and `{{ ... }}` tag tokens.
fn scan(template: &str) -> Result<Vec<Token>, String> {
    let unclosed = || format!("unclosed {{{{ in template {template:?}");
    let mut tokens = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        if start > 0 {
            tokens.push(Token::Text(
                rest.get(..start).unwrap_or_default().to_string(),
            ));
        }
        let after = rest.get(start + 2..).ok_or_else(unclosed)?;
        let end = after.find("}}").ok_or_else(unclosed)?;
        tokens.push(Token::Expr(
            after.get(..end).ok_or_else(unclosed)?.to_string(),
        ));
        rest = after.get(end + 2..).unwrap_or_default();
    }
    if !rest.is_empty() {
        tokens.push(Token::Text(rest.to_string()));
    }
    Ok(tokens)
}

fn parse(template: &str) -> Result<Vec<Node>, String> {
    let tokens = scan(template)?;
    let (nodes, stop) = parse_nodes(&tokens, 0)?;
    match stop {
        Stop::Eof => Ok(nodes),
        Stop::End(_) => Err(format!(
            "{{{{ end }}}} without a matching block in {template:?}"
        )),
        Stop::Else(_) => Err(format!(
            "{{{{ else }}}} without a matching if in {template:?}"
        )),
    }
}

fn parse_nodes(tokens: &[Token], mut i: usize) -> Result<(Vec<Node>, Stop), String> {
    let mut nodes = Vec::new();
    while let Some(token) = tokens.get(i) {
        match token {
            Token::Text(text) => {
                nodes.push(Node::Text(text.clone()));
                i += 1;
            }
            Token::Expr(raw) => {
                let tag = raw.trim();
                let (head, rest) = match tag.split_once(char::is_whitespace) {
                    Some((head, rest)) => (head, rest.trim()),
                    None => (tag, ""),
                };
                match head {
                    "end" if rest.is_empty() => return Ok((nodes, Stop::End(i + 1))),
                    "else" if rest.is_empty() => return Ok((nodes, Stop::Else(i))),
                    "else" => {
                        return Err(
                            "{{ else }} takes no condition (\"else if\" is not supported)"
                                .to_string(),
                        );
                    }
                    "if" if rest.is_empty() => {
                        return Err("{{ if }} requires a condition".to_string());
                    }
                    "if" => {
                        let cond = parse_expr(rest)?;
                        let (then, stop) = parse_nodes(tokens, i + 1)?;
                        let (otherwise, next) = match stop {
                            Stop::Else(j) => match parse_nodes(tokens, j + 1)? {
                                (else_nodes, Stop::End(k)) => (else_nodes, k),
                                (_, Stop::Else(_)) => {
                                    return Err(
                                        "an if block has at most one {{ else }}".to_string()
                                    );
                                }
                                (_, Stop::Eof) => {
                                    return Err("unclosed if block".to_string());
                                }
                            },
                            Stop::End(j) => (Vec::new(), j),
                            Stop::Eof => return Err("unclosed if block".to_string()),
                        };
                        nodes.push(Node::If {
                            cond,
                            then,
                            otherwise,
                        });
                        i = next;
                    }
                    "range" => {
                        if rest != ".Categories" {
                            return Err(format!("range supports only .Categories, got {rest:?}"));
                        }
                        let (body, stop) = parse_nodes(tokens, i + 1)?;
                        match stop {
                            Stop::End(j) => {
                                nodes.push(Node::Range { body });
                                i = j;
                            }
                            Stop::Else(_) => {
                                return Err("{{ range }} … {{ else }} is not supported".to_string());
                            }
                            Stop::Eof => return Err("unclosed range block".to_string()),
                        }
                    }
                    _ => {
                        nodes.push(Node::Value(parse_expr(tag)?));
                        i += 1;
                    }
                }
            }
        }
    }
    Ok((nodes, Stop::Eof))
}

// ── expression parsing ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    LParen,
    RParen,
    Word(String),
    Str(String),
}

fn lex(expr: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let mut rest = expr;
    loop {
        rest = rest.trim_start_matches(char::is_whitespace);
        let Some(ch) = rest.chars().next() else {
            return Ok(toks);
        };
        match ch {
            '(' => {
                toks.push(Tok::LParen);
                rest = rest.get(1..).unwrap_or_default();
            }
            ')' => {
                toks.push(Tok::RParen);
                rest = rest.get(1..).unwrap_or_default();
            }
            '"' => {
                let after_open = rest.get(1..).unwrap_or_default();
                let close = after_open
                    .find('"')
                    .ok_or_else(|| format!("unclosed quoted string in expression {expr:?}"))?;
                toks.push(Tok::Str(
                    after_open.get(..close).unwrap_or_default().to_string(),
                ));
                rest = after_open.get(close + 1..).unwrap_or_default();
            }
            _ => {
                let end = rest
                    .find(|c: char| c.is_whitespace() || c == '(' || c == ')')
                    .unwrap_or(rest.len());
                toks.push(Tok::Word(rest.get(..end).unwrap_or_default().to_string()));
                rest = rest.get(end..).unwrap_or_default();
            }
        }
    }
}

fn parse_expr(src: &str) -> Result<Expr, String> {
    let toks = lex(src)?;
    let mut parser = ExprParser { toks, pos: 0 };
    let expr = parser.term()?;
    if parser.pos != parser.toks.len() {
        return Err(format!(
            "unsupported template construct {src:?} (supported: .Keywords, .Categories, \
             .Config.<key>, .Query.<field>, .Result.<field>, join .Categories \"<sep>\", \
             if/else/end, range .Categories, and/or/eq/ne)"
        ));
    }
    Ok(expr)
}

struct ExprParser {
    toks: Vec<Tok>,
    pos: usize,
}

impl ExprParser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    /// Parses one term: an atom, a quoted literal, a parenthesized
    /// sub-expression, or a prefix operator call.
    fn term(&mut self) -> Result<Expr, String> {
        match self.toks.get(self.pos).cloned() {
            Some(Tok::LParen) => {
                self.pos += 1;
                let inner = self.term()?;
                match self.toks.get(self.pos) {
                    Some(Tok::RParen) => {
                        self.pos += 1;
                        Ok(inner)
                    }
                    _ => Err("unclosed parenthesis in expression".to_string()),
                }
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Expr::Atom(Atom::Str(s)))
            }
            Some(Tok::Word(word)) => {
                self.pos += 1;
                match word.as_str() {
                    "and" | "or" => {
                        let args = self.args()?;
                        if word == "and" {
                            Ok(Expr::And(args))
                        } else {
                            Ok(Expr::Or(args))
                        }
                    }
                    "eq" | "ne" => {
                        let mut args = self.args()?;
                        if args.len() != 2 {
                            return Err(format!(
                                "{word} takes exactly 2 operands, got {}",
                                args.len()
                            ));
                        }
                        let b = args.remove(1);
                        let a = args.remove(0);
                        if word == "eq" {
                            Ok(Expr::Eq(Box::new(a), Box::new(b)))
                        } else {
                            Ok(Expr::Ne(Box::new(a), Box::new(b)))
                        }
                    }
                    "join" => self.join(),
                    other if other.starts_with('.') => Ok(Expr::Atom(classify_atom(other)?)),
                    other => Err(format!(
                        "unsupported template construct {other:?} (supported: .Keywords, \
                         .Categories, .Config.<key>, .Query.<field>, .Result.<field>, \
                         join .Categories \"<sep>\", if/else/end, range .Categories, \
                         and/or/eq/ne)"
                    )),
                }
            }
            Some(Tok::RParen) => Err("unexpected ) in expression".to_string()),
            None => Err("empty expression".to_string()),
        }
    }

    /// Parses operator operands: atoms, quoted literals, and parenthesized
    /// sub-expressions. Nested operator calls need parentheses (Go template
    /// semantics), which keeps operand counts unambiguous.
    fn args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        loop {
            match self.peek() {
                Some(Tok::LParen) | Some(Tok::Str(_)) => args.push(self.term()?),
                Some(Tok::Word(word)) if word.starts_with('.') => {
                    args.push(Expr::Atom(classify_atom(word)?));
                    self.pos += 1;
                }
                _ => break,
            }
        }
        if args.is_empty() {
            return Err("operator requires at least one operand".to_string());
        }
        Ok(args)
    }

    fn join(&mut self) -> Result<Expr, String> {
        match self.toks.get(self.pos).cloned() {
            Some(Tok::Word(word)) if word == ".Categories" => {
                self.pos += 1;
            }
            other => {
                return Err(format!("join supports only .Categories, got {other:?}"));
            }
        }
        match self.toks.get(self.pos).cloned() {
            Some(Tok::Str(sep)) => {
                self.pos += 1;
                Ok(Expr::Join(sep))
            }
            other => Err(format!(
                "join separator must be a quoted string, got {other:?}"
            )),
        }
    }
}

fn classify_atom(word: &str) -> Result<Atom, String> {
    match word {
        ".Keywords" => return Ok(Atom::Keywords),
        ".Categories" => return Ok(Atom::Categories),
        "." => return Ok(Atom::Dot),
        ".True" => return Ok(Atom::Bool(true)),
        ".False" => return Ok(Atom::Bool(false)),
        _ => {}
    }
    if let Some(key) = word.strip_prefix(".Config.") {
        if key.is_empty() || key.contains(char::is_whitespace) {
            return Err(format!("malformed config reference {word:?}"));
        }
        return Ok(Atom::Config(key.to_string()));
    }
    if let Some(field) = word.strip_prefix(".Query.") {
        return QUERY_FIELDS
            .iter()
            .find(|known| **known == field)
            .map(|known| Atom::Query(known))
            .ok_or_else(|| format!("unsupported query field {field:?}"));
    }
    if let Some(field) = word.strip_prefix(".Result.") {
        if field.is_empty() || field.contains(char::is_whitespace) {
            return Err(format!("malformed result reference {word:?}"));
        }
        return Ok(Atom::Result(field.to_string()));
    }
    Err(format!(
        "unsupported template construct {word:?} (supported: .Keywords, .Categories, \
         .Config.<key>, .Query.<field>, .Result.<field>, join .Categories \"<sep>\", \
         if/else/end, range .Categories, and/or/eq/ne)"
    ))
}

// ── evaluation ──────────────────────────────────────────────────────────

/// A dynamically-typed template value, mirroring upstream's variables: a
/// string, a boolean, or the category list.
#[derive(Debug, Clone)]
enum Val {
    Str(String),
    Bool(bool),
    List(Vec<String>),
}

fn truthy(value: &Val) -> bool {
    match value {
        Val::Str(s) => !s.is_empty(),
        Val::Bool(b) => *b,
        Val::List(items) => !items.is_empty(),
    }
}

/// The string form an expression contributes to the output: `false` renders
/// empty (an absent optional value), `true` as the literal `true`, a list
/// comma-joined (matching `{{ .Categories }}`).
fn val_string(value: Val) -> String {
    match value {
        Val::Str(s) => s,
        Val::Bool(true) => "true".to_string(),
        Val::Bool(false) => String::new(),
        Val::List(items) => items.join(","),
    }
}

fn eval_expr(expr: &Expr, ctx: &TemplateContext, dot: Option<&str>) -> Result<Val, String> {
    match expr {
        Expr::Atom(atom) => eval_atom(atom, ctx, dot),
        Expr::Join(sep) => Ok(Val::Str(ctx.categories.join(sep))),
        // WHY: Go value semantics — `and` yields the first falsy operand or
        // the last one; `or` the first truthy or the last. Conditions read
        // the truthiness of the result; `{{ or .Result.a .Result.b }}` in
        // value position coalesces to the first present field.
        Expr::And(args) => {
            let mut last = Val::Bool(true);
            for arg in args {
                last = eval_expr(arg, ctx, dot)?;
                if !truthy(&last) {
                    return Ok(last);
                }
            }
            Ok(last)
        }
        Expr::Or(args) => {
            let mut last = Val::Bool(false);
            for arg in args {
                last = eval_expr(arg, ctx, dot)?;
                if truthy(&last) {
                    return Ok(last);
                }
            }
            Ok(last)
        }
        Expr::Eq(a, b) => Ok(Val::Bool(vals_equal(
            &eval_expr(a, ctx, dot)?,
            &eval_expr(b, ctx, dot)?,
        ))),
        Expr::Ne(a, b) => Ok(Val::Bool(!vals_equal(
            &eval_expr(a, ctx, dot)?,
            &eval_expr(b, ctx, dot)?,
        ))),
    }
}

fn eval_atom(atom: &Atom, ctx: &TemplateContext, dot: Option<&str>) -> Result<Val, String> {
    Ok(match atom {
        Atom::Keywords => Val::Str(ctx.keywords.clone()),
        Atom::Categories => Val::List(ctx.categories.clone()),
        Atom::Dot => Val::Str(dot.ok_or("{{ . }} outside a range block")?.to_string()),
        Atom::Config(key) => match ctx.config.get(key).map(String::as_str) {
            // WHY: checkbox settings are stored as the literal strings
            // "true"/"false"; conditions must read them as the booleans they
            // represent (upstream settings are typed bools). A declared-but-
            // unset setting (no default) is false.
            Some("true") => Val::Bool(true),
            Some("false") | Some("") | None => Val::Bool(false),
            Some(value) => Val::Str(value.to_string()),
        },
        Atom::Query(field) => match ctx.query.get(field) {
            Some(value) if !value.is_empty() => Val::Str(value.clone()),
            _ => Val::Bool(false),
        },
        Atom::Result(field) => match ctx.result.get(field) {
            Some(value) if !value.is_empty() => Val::Str(value.clone()),
            _ => Val::Bool(false),
        },
        Atom::Bool(b) => Val::Bool(*b),
        Atom::Str(s) => Val::Str(s.clone()),
    })
}

/// Equality over dynamic values: two strings compare textually; anything
/// involving a boolean or list compares truthiness — the corpus pattern is
/// `eq .Query.IMDBID .False` (an absent id versus the false constant).
fn vals_equal(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Str(x), Val::Str(y)) => x == y,
        (Val::List(x), Val::List(y)) => x == y,
        _ => truthy(a) == truthy(b),
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Plain,
    Url,
}

fn emit(
    nodes: &[Node],
    ctx: &TemplateContext,
    dot: Option<&str>,
    mode: Mode,
    out: &mut String,
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Value(expr) => {
                let rendered = val_string(eval_expr(expr, ctx, dot)?);
                match mode {
                    Mode::Plain => out.push_str(&rendered),
                    Mode::Url => out.push_str(&encode_value(&rendered)),
                }
            }
            Node::If {
                cond,
                then,
                otherwise,
            } => {
                let branch = if truthy(&eval_expr(cond, ctx, dot)?) {
                    then
                } else {
                    otherwise
                };
                emit(branch, ctx, dot, mode, out)?;
            }
            Node::Range { body } => {
                for item in &ctx.categories {
                    emit(body, ctx, Some(item), mode, out)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
