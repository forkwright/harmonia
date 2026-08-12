//! Cardigann YAML definition schema, loading, and load-time validation.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use tracing::warn;

use crate::client::cardigann::filters;
use crate::client::cardigann::template::ParsedTemplate;
use crate::error::SearchIndexerError;

mod yaml;
pub use yaml::{FilterArgs, OrderedFields, OrderedPairs, ScalarString};

/// One Prowlarr-compatible Cardigann indexer definition, deserialized from a
/// single YAML file.
///
/// Unknown YAML keys are ignored on purpose: real-world definitions carry
/// many blocks this engine does not model, and a permissive schema keeps a
/// definition loadable as long as the parts this engine executes are sound.
///
/// The schema is generic over the template representation `T`
/// (parse-don't-validate, #696): [`ScalarString`] as deserialized,
/// [`ParsedTemplate`] once [`compile_templates`] has parsed every template
/// exactly once at load. Non-template fields stay concrete in both forms.
#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct CardigannDefinition<T = ScalarString> {
    pub id: String, // kanon:ignore RUST/primitive-for-domain-id -- wire DTO mirroring the external Cardigann YAML schema
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default, rename = "type")]
    pub site_type: Option<String>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub legacylinks: Vec<String>,
    pub caps: CapsBlock,
    #[serde(default)]
    pub settings: Vec<SettingsField>,
    #[serde(default)]
    pub login: Option<LoginBlock<T>>,
    pub search: SearchBlock<T>,
    #[serde(default)]
    pub download: Option<DownloadBlock<T>>,
}

/// A definition as deserialized from YAML: every template position holds its
/// raw scalar text. Only definition loading consumes this form.
pub type RawDefinition = CardigannDefinition<ScalarString>;

/// A definition past load: every template position holds a [`ParsedTemplate`]
/// produced exactly once, at load. The render path consumes this form —
/// nothing re-parses per search.
pub type CompiledDefinition = CardigannDefinition<ParsedTemplate>;

#[derive(Debug, Clone, Deserialize)]
pub struct CapsBlock {
    #[serde(default)]
    pub categorymappings: Vec<CategoryMapping>,
    /// Legacy short form: site category id → Torznab category name. Folded
    /// into `categorymappings` by [`normalize`].
    #[serde(default)]
    pub categories: BTreeMap<ScalarString, String>,
    /// Search mode → supported query fields, e.g. `tv-search: [q, season, ep]`.
    #[serde(default)]
    pub modes: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryMapping {
    /// Site-native category id (number or string in YAML).
    pub id: ScalarString,
    /// Torznab category name, e.g. `Movies/HD`.
    pub cat: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub default: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SettingsField {
    pub name: String,
    #[serde(default, rename = "type")]
    pub field_type: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub default: Option<ScalarString>,
    #[serde(default)]
    pub options: Option<BTreeMap<ScalarString, ScalarString>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct LoginBlock<T = ScalarString> {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<T>,
    /// CSS selector for the login `<form>` (form method; defaults to "form").
    #[serde(default)]
    pub form: Option<String>,
    /// Submit-target override, joined on the site base instead of the
    /// form's `action`.
    #[serde(default)]
    pub submitpath: Option<T>,
    #[serde(default)]
    pub inputs: BTreeMap<String, T>,
    /// Failed-login detectors checked against the post-submit page.
    #[serde(default)]
    pub error: Vec<ErrorBlock<T>>,
    #[serde(default)]
    pub test: Option<LoginTest>,
    /// Unmodeled selector-driven inputs; presence is rejected at load for
    /// interactive login methods.
    #[serde(default)]
    pub selectorinputs: Option<serde_norway::Value>,
    /// Unmodeled captcha block; presence is rejected at load for
    /// interactive login methods.
    #[serde(default)]
    pub captcha: Option<serde_norway::Value>,
}

/// One failed-login detector: `selector` marks the post-submit page as a
/// login error; `message` (FieldBlock semantics) refines the reported text.
#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct ErrorBlock<T = ScalarString> {
    pub selector: String,
    #[serde(default)]
    pub message: Option<FieldBlock<T>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginTest {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct SearchBlock<T = ScalarString> {
    #[serde(default)]
    pub paths: Vec<SearchPath<T>>,
    /// Legacy single-path form. Folded into `paths` by [`normalize`].
    #[serde(default)]
    pub path: Option<T>,
    #[serde(default)]
    pub inputs: BTreeMap<String, T>,
    #[serde(default)]
    pub keywordsfilters: Vec<FilterSpec<T>>,
    pub rows: RowsBlock<T>,
    #[serde(default)]
    pub fields: OrderedFields<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct SearchPath<T = ScalarString> {
    pub path: T,
    #[serde(default)]
    pub method: Option<String>,
    /// Site category ids this path is limited to (empty = all queries).
    #[serde(default)]
    pub categories: Vec<ScalarString>,
    /// Extra inputs merged over `search.inputs` for this path.
    #[serde(default)]
    pub inputs: BTreeMap<String, T>,
    #[serde(default)]
    pub response: Option<ResponseBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseBlock {
    #[serde(default, rename = "type")]
    pub response_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct RowsBlock<T = ScalarString> {
    pub selector: String,
    #[serde(default)]
    pub filters: Vec<FilterSpec<T>>,
    #[serde(default)]
    pub after: Option<u32>,
    #[serde(default)]
    pub remove: Option<String>,
    #[serde(default)]
    pub dateheaders: Option<FieldBlock<T>>,
    /// JSON nested-row drill-down: a path into each parent row yielding the
    /// sub-row(s). Combined with `multiple`, the attribute resolves to an array.
    #[serde(default)]
    pub attribute: Option<ScalarString>,
    /// JSON nested-row expansion: when true, `attribute` resolves to an array
    /// iterated as multiple sub-rows; otherwise a single sub-row object.
    #[serde(default)]
    pub multiple: bool,
    /// JSON nested-row: skip a parent whose `attribute` path is missing instead
    /// of erroring (upstream `missingAttributeEqualsNoResults`).
    #[serde(default, rename = "missingAttributeEqualsNoResults")]
    pub missing_attribute_equals_no_results: bool,
    /// JSON advisory pre-count selector; parsed but not executed.
    #[serde(default)]
    pub count: Option<FieldBlock<T>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct FieldBlock<T = ScalarString> {
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    /// Constant/template value used instead of selecting from the row.
    #[serde(default)]
    pub text: Option<T>,
    /// Fallback template rendered (row scope, like `text`) when selector
    /// extraction yields nothing (upstream `FieldBlock.Default`).
    #[serde(default)]
    pub default: Option<T>,
    #[serde(default)]
    pub filters: Vec<FilterSpec<T>>,
    #[serde(default)]
    pub optional: bool,
    /// Selector → value pairs; the first selector matching the selected
    /// element (or one of its descendants) supplies the value.
    ///
    /// WHY: stored as ordered pairs, not a map — first-match-wins semantics
    /// depend on the author's YAML order.
    #[serde(default)]
    pub case: Option<OrderedPairs<T>>,
    /// Selector for descendants to exclude from text extraction.
    #[serde(default)]
    pub remove: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct DownloadBlock<T = ScalarString> {
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub filters: Vec<FilterSpec<T>>,
    #[serde(default)]
    pub method: Option<String>,
    /// Unmodeled pre-request block; presence is detected and deferred.
    #[serde(default)]
    pub before: Option<serde_norway::Value>,
    /// Unmodeled infohash fallback block; presence is detected and deferred.
    #[serde(default)]
    pub infohash: Option<serde_norway::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de> + From<String>"))]
pub struct FilterSpec<T = ScalarString> {
    pub name: String,
    #[serde(default)]
    pub args: Option<FilterArgs<T>>,
}

impl<T> FilterSpec<T> {
    pub fn args(&self) -> &[T] {
        self.args.as_ref().map_or(&[], |a| a.0.as_slice())
    }
}

/// Parses, normalizes, and validates one definition file.
pub fn load_definition_file(path: &Path) -> Result<CompiledDefinition, SearchIndexerError> {
    let display_path = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| SearchIndexerError::DefinitionLoad {
        path: display_path.clone(),
        reason: e.to_string(),
        location: std::panic::Location::caller(),
    })?;
    parse_definition(&text, &display_path)
}

/// Parses, normalizes, and validates definition YAML; `origin` labels load
/// errors (usually the file path). The returned definition is compiled:
/// every template parsed exactly once (see [`compile_templates`]).
pub fn parse_definition(
    text: &str,
    origin: &str,
) -> Result<CompiledDefinition, SearchIndexerError> {
    let mut definition: RawDefinition =
        serde_norway::from_str(text).map_err(|e| SearchIndexerError::DefinitionLoad {
            path: origin.to_string(),
            reason: e.to_string(),
            location: std::panic::Location::caller(),
        })?;
    normalize(&mut definition);
    validate(&definition)?;
    compile_templates(definition)
}

/// Folds legacy schema forms into their modern equivalents.
fn normalize<T: AsRef<str>>(def: &mut CardigannDefinition<T>) {
    if def.search.paths.is_empty()
        && let Some(path) = def.search.path.take()
    {
        def.search.paths.push(SearchPath {
            path,
            method: None,
            categories: Vec::new(),
            inputs: BTreeMap::new(),
            response: None,
        });
    }

    let legacy: Vec<CategoryMapping> = def
        .caps
        .categories
        .iter()
        .map(|(id, cat)| CategoryMapping {
            id: id.clone(),
            cat: cat.clone(),
            desc: None,
            default: false,
        })
        .collect();
    def.caps.categorymappings.extend(legacy);
    def.caps.categories.clear();

    // WHY: a leading "!" negates a path-category constraint — a feature this
    // engine defers. Treating the path as unconstrained fetches a superset,
    // which is correct-but-wider; dropping the path would silently lose rows.
    for path in &mut def.search.paths {
        if path.categories.iter().any(|c| c.0.starts_with('!')) {
            warn!(
                definition_id = %def.id,
                path = %path.path.as_ref(),
                "negated path categories are not supported; treating path as unconstrained"
            );
            path.categories.clear();
        }
    }
}

/// Rejects definitions whose executable parts this engine cannot honor.
///
/// WHY: a definition that would fail on every search must fail at load with
/// one clear reason, not degrade into per-query noise. Blocks that only add
/// information (rows post-filters, date headers) are warned and ignored
/// instead — skipping them yields a superset of rows, never wrong values.
///
/// Template syntax and scope are NOT checked here: [`compile_templates`]
/// runs next and parses every template position once, which is where those
/// errors surface (parse-don't-validate, #696).
fn validate(def: &RawDefinition) -> Result<(), SearchIndexerError> {
    let invalid = |reason: String| SearchIndexerError::DefinitionInvalid {
        definition_id: def.id.clone(),
        reason,
        location: std::panic::Location::caller(),
    };
    let unsupported = |feature: String| SearchIndexerError::DefinitionUnsupported {
        definition_id: def.id.clone(),
        feature,
        location: std::panic::Location::caller(),
    };

    if def.id.trim().is_empty() {
        return Err(invalid("empty id".to_string()));
    }
    if def.links.is_empty() {
        return Err(invalid("no links".to_string()));
    }
    for link in def.links.iter().chain(&def.legacylinks) {
        if link.contains("{{") {
            return Err(unsupported(format!("templated link {link}")));
        }
        url::Url::parse(link).map_err(|e| invalid(format!("link {link}: {e}")))?;
    }
    if def.search.paths.is_empty() {
        return Err(invalid("no search paths".to_string()));
    }

    let check_selector = |what: &str, sel: &str| {
        scraper::Selector::parse(sel)
            .map(|_| ())
            .map_err(|e| invalid(format!("{what} selector {sel:?}: {e}")))
    };
    let check_filters = |what: &str, specs: &[FilterSpec]| {
        filters::validate(specs).map_err(|e| invalid(format!("{what}: {e}")))
    };
    let check_json_selector =
        |what: &str, sel: &str, allow_parent_switch: bool| -> Result<(), SearchIndexerError> {
            let trimmed = sel.trim();
            // WHY: a leading `..` is the nested-rows parent switch — meaningful
            // only for FIELD selectors (which have a parent sub-row context); for
            // rows.selector / rows.attribute there is no parent, so it is not
            // allowed. Any OTHER `..` — after `$`, or mid-path — is Newtonsoft
            // recursive descent, which this walker does not implement.
            let is_parent_switch = allow_parent_switch
                && trimmed.starts_with("..")
                && !trimmed.trim_start_matches('.').contains("..");
            if trimmed.contains("..") && !is_parent_switch {
                return Err(unsupported(format!(
                    "{what}: recursive-descent '..' selectors are not supported for json responses"
                )));
            }
            if trimmed.contains(':') {
                return Err(unsupported(format!(
                    "{what}: ':' pseudo-filter selectors are not yet supported for json responses"
                )));
            }
            // WHY: a selector that is only dots/`$` (no path) resolves to the
            // object itself — a definition mistake, not a real field.
            if trimmed
                .trim_start_matches('$')
                .trim_start_matches('.')
                .is_empty()
            {
                return Err(invalid(format!("{what}: empty json selector {sel:?}")));
            }
            crate::client::cardigann::json_extract::parse_path_segments(trimmed)
                .map(|_| ())
                .map_err(|e| invalid(format!("{what} selector {sel:?}: {e}")))
        };

    // WHY: response type is declared per path, but rows/fields are shared, so a
    // definition is coherently single-type. All paths must agree; xml and other
    // types are deferred.
    fn path_kind(path: &SearchPath) -> Option<&str> {
        path.response
            .as_ref()
            .and_then(|r| r.response_type.as_deref())
    }
    let first_kind = def.search.paths.first().and_then(path_kind);
    if def.search.paths.iter().any(|p| path_kind(p) != first_kind) {
        return Err(unsupported(
            "mixed response types across search paths".to_string(),
        ));
    }
    let is_json = match first_kind {
        None | Some("html") => false,
        Some("json") => true,
        Some(other) => return Err(unsupported(format!("search response type {other:?}"))),
    };

    for path in &def.search.paths {
        let is_post = match path.method.as_deref() {
            None | Some("get") => false,
            Some("post") => true,
            Some(other) => return Err(unsupported(format!("search path method {other:?}"))),
        };
        if is_post && (def.search.inputs.contains_key("$raw") || path.inputs.contains_key("$raw")) {
            // WHY: $raw splices verbatim into a query string; a POST search
            // sends its inputs as a form body, where that splice has no
            // meaning. Reject rather than silently drop it (fail-loud).
            return Err(unsupported("$raw input with POST search".to_string()));
        }
    }
    check_filters("keywordsfilters", &def.search.keywordsfilters)?;

    if is_json {
        if def.search.rows.remove.is_some() {
            return Err(unsupported(
                "rows.remove is not supported for json responses".to_string(),
            ));
        }
        // WHY: rows.multiple / missingAttributeEqualsNoResults only apply to a
        // nested drill-down; without rows.attribute they silently no-op (and a
        // typo'd `attribute` key is dropped by serde) — reject at load rather
        // than treat the parents as flat rows forever.
        if (def.search.rows.multiple || def.search.rows.missing_attribute_equals_no_results)
            && def.search.rows.attribute.is_none()
        {
            return Err(invalid(
                "rows.multiple / missingAttributeEqualsNoResults require rows.attribute"
                    .to_string(),
            ));
        }
        check_json_selector("rows", &def.search.rows.selector, false)?;
        if let Some(attr) = &def.search.rows.attribute {
            check_json_selector("rows.attribute", &attr.0, false)?;
        }
        if def.search.rows.count.is_some() {
            warn!(
                definition_id = %def.id,
                "rows.count is advisory and not evaluated; results are unaffected"
            );
        }
    } else {
        check_selector("rows", &def.search.rows.selector)?;
        if let Some(remove) = &def.search.rows.remove {
            check_selector("rows.remove", remove)?;
        }
    }
    if !def.search.rows.filters.is_empty() {
        filters::parse_row_filters(&def.search.rows.filters)
            .map_err(|e| invalid(format!("rows.filters: {e}")))?;
    }
    if def.search.rows.after.is_some() || def.search.rows.dateheaders.is_some() {
        warn!(
            definition_id = %def.id,
            "rows.after / rows.dateheaders are not supported and are ignored"
        );
    }

    if def.search.fields.is_empty() {
        return Err(invalid("no search fields".to_string()));
    }
    // WHY: result mapping drops every row lacking these fields — a
    // definition without them (e.g. a typo'd field name) would load fine
    // and then return zero results forever with no signal.
    if !def.search.fields.contains_key("title") {
        return Err(invalid(
            "search fields are missing required field \"title\"".to_string(),
        ));
    }
    if !def.search.fields.contains_key("download") && !def.search.fields.contains_key("magnet") {
        return Err(invalid(
            "search fields are missing a download source (\"download\" or \"magnet\")".to_string(),
        ));
    }
    for (name, field) in def.search.fields.iter() {
        if let Some(sel) = &field.selector {
            if is_json {
                check_json_selector(&format!("field {name}"), sel, true)?;
            } else {
                check_selector(&format!("field {name}"), sel)?;
            }
        }
        if is_json {
            // WHY: attribute (HTML attr read) and remove (HTML subtree exclude)
            // have no JSON meaning; silent-ignore would mask a definition
            // mistake, so reject. JSON `case` keys are literal values, not CSS
            // selectors, so they are not selector-validated.
            if field.attribute.is_some() {
                return Err(unsupported(format!(
                    "field {name}: attribute is not supported for json responses"
                )));
            }
            if field.remove.is_some() {
                return Err(unsupported(format!(
                    "field {name}: remove is not supported for json responses"
                )));
            }
        } else {
            if let Some(remove) = &field.remove {
                check_selector(&format!("field {name} remove"), remove)?;
            }
            if let Some(case) = &field.case {
                for (case_selector, _) in &case.0 {
                    check_selector(&format!("field {name} case"), case_selector)?;
                }
            }
        }
        check_filters(&format!("field {name}"), &field.filters)?;
    }

    if let Some(download) = &def.download {
        if let Some(sel) = &download.selector {
            check_selector("download", sel)?;
        }
        check_filters("download", &download.filters)?;
        match download.method.as_deref() {
            None | Some("get") => {}
            Some(other) => return Err(unsupported(format!("download method {other:?}"))),
        }
        if download.before.is_some() {
            warn!(
                definition_id = %def.id,
                "download.before pre-requests are not supported and are ignored"
            );
        }
        if download.infohash.is_some() {
            warn!(
                definition_id = %def.id,
                "download.infohash fallback is not supported and is ignored"
            );
        }
    }

    // NOTE: login blocks are only validated for methods this engine runs
    // (none, cookie, form, post, get; an omitted method defaults to "form").
    // Unknown methods fail at client construction with LoginUnsupported.
    if let Some(login) = &def.login {
        let method = login.method.as_deref().unwrap_or("form");
        let interactive = matches!(method, "form" | "post" | "get");
        let known_method = interactive || matches!(method, "none" | "cookie");
        if known_method
            && let Some(test) = &login.test
            && let Some(sel) = &test.selector
        {
            check_selector("login test", sel)?;
        }
        if interactive {
            // WHY: these blocks change what a login submits — silently
            // skipping them would send an incomplete login (and captcha
            // pages hard-require operator interaction this engine lacks).
            if login.selectorinputs.is_some() {
                return Err(unsupported("login.selectorinputs".to_string()));
            }
            if login.captcha.is_some() {
                return Err(unsupported("login.captcha".to_string()));
            }
            if login.path.is_none() {
                return Err(invalid(format!(
                    "login method {method:?} requires login.path"
                )));
            }
            if let Some(form) = &login.form {
                check_selector("login form", form)?;
            }
            for (index, block) in login.error.iter().enumerate() {
                let what = format!("login error {index}");
                check_selector(&what, &block.selector)?;
                if let Some(message) = &block.message {
                    if let Some(sel) = &message.selector {
                        check_selector(&format!("{what} message"), sel)?;
                    }
                    if let Some(remove) = &message.remove {
                        check_selector(&format!("{what} message remove"), remove)?;
                    }
                    if let Some(case) = &message.case {
                        for (case_selector, _) in &case.0 {
                            check_selector(&format!("{what} message case"), case_selector)?;
                        }
                    }
                    check_filters(&format!("{what} message"), &message.filters)?;
                }
            }
        }
    }

    Ok(())
}

/// Parses every template in a raw definition exactly once, producing the
/// compiled form the render path consumes (parse-don't-validate, #696).
///
/// The parse IS the load-time template check the pre-#696 `validate` /
/// `validate_row_scoped` pair performed and discarded: unsupported
/// constructs, unknown config keys, and out-of-scope `.Result` references
/// fail here with the same messages, and the parsed AST is what the
/// definition stores — an invalid template is unrepresentable past load.
///
/// SCOPE NOTE: login template positions are parsed whenever present,
/// regardless of the declared login method. The pre-#696 validators skipped
/// them for methods this engine does not run; those definitions fail at
/// client construction regardless, so the check surfacing at load instead
/// can only affect a definition that was never usable.
fn compile_templates(def: RawDefinition) -> Result<CompiledDefinition, SearchIndexerError> {
    fn invalid(definition_id: &str, reason: String) -> SearchIndexerError {
        SearchIndexerError::DefinitionInvalid {
            definition_id: definition_id.to_string(),
            reason,
            location: std::panic::Location::caller(),
        }
    }

    let id = def.id.clone();
    // WHY owned: `config_keys` borrows must not pin `def` — the rebuild below
    // moves every field out of it.
    let config_keys_owned: Vec<String> = def.settings.iter().map(|s| s.name.clone()).collect();
    let config_keys: Vec<&str> = config_keys_owned.iter().map(String::as_str).collect();
    let field_names: Vec<String> = def.search.fields.names();

    let parse = |what: &str, raw: &str| {
        ParsedTemplate::parse(raw, &config_keys).map_err(|e| invalid(&id, format!("{what}: {e}")))
    };
    // Row scope: `.Result.<field>` is meaningful only where the extractor
    // renders with the row's accumulated values — field text/case/default
    // values and field filter args.
    let parse_row = |what: &str, raw: &str| {
        ParsedTemplate::parse_row_scoped(raw, &config_keys, &field_names)
            .map_err(|e| invalid(&id, format!("{what}: {e}")))
    };
    // WHY: filter args are templates too — parse them so an unsupported
    // construct fails at load instead of reaching the pipeline verbatim.
    let compile_specs = |what: &str,
                         specs: Vec<FilterSpec>,
                         row_scoped: bool|
     -> Result<Vec<FilterSpec<ParsedTemplate>>, SearchIndexerError> {
        let parse_arg: &dyn Fn(&str, &str) -> Result<ParsedTemplate, SearchIndexerError> =
            if row_scoped { &parse_row } else { &parse };
        let mut out = Vec::with_capacity(specs.len());
        for spec in specs {
            let args = spec
                .args
                .map(|args| {
                    args.0
                        .into_iter()
                        .enumerate()
                        .map(|(index, arg)| {
                            parse_arg(
                                &format!("{what} filter {:?} arg {index}", spec.name),
                                arg.as_ref(),
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;
            out.push(FilterSpec {
                name: spec.name,
                args: args.map(FilterArgs),
            });
        }
        Ok(out)
    };
    let compile_field = |what: &str,
                         field: FieldBlock,
                         row_scoped: bool|
     -> Result<FieldBlock<ParsedTemplate>, SearchIndexerError> {
        let parse_value: &dyn Fn(&str, &str) -> Result<ParsedTemplate, SearchIndexerError> =
            if row_scoped { &parse_row } else { &parse };
        let FieldBlock {
            selector,
            attribute,
            text,
            default,
            filters,
            optional,
            case,
            remove,
        } = field;
        let text = text
            .map(|t| parse_value(&format!("{what} text"), t.as_ref()))
            .transpose()?;
        let default = default
            .map(|d| parse_value(&format!("{what} default"), d.as_ref()))
            .transpose()?;
        let case = case
            .map(|case| {
                case.0
                    .into_iter()
                    .map(|(sel, value)| {
                        Ok((
                            sel,
                            parse_value(&format!("{what} case value"), value.as_ref())?,
                        ))
                    })
                    .collect::<Result<Vec<_>, SearchIndexerError>>()
            })
            .transpose()?
            .map(OrderedPairs);
        let filters = compile_specs(what, filters, row_scoped)?;
        Ok(FieldBlock {
            selector,
            attribute,
            text,
            default,
            filters,
            optional,
            case,
            remove,
        })
    };
    let compile_inputs = |what: &str,
                          inputs: BTreeMap<String, ScalarString>|
     -> Result<BTreeMap<String, ParsedTemplate>, SearchIndexerError> {
        inputs
            .into_iter()
            .map(|(key, value)| {
                Ok((
                    key.clone(),
                    parse(&format!("{what} {key}"), value.as_ref())?,
                ))
            })
            .collect()
    };

    let CardigannDefinition {
        id,
        name,
        description,
        language,
        site_type,
        encoding,
        links,
        legacylinks,
        caps,
        settings,
        login,
        search,
        download,
    } = def;

    let SearchBlock {
        paths,
        path: _, // legacy single-path form — already folded into `paths` by normalize
        inputs,
        keywordsfilters,
        rows,
        fields,
    } = search;

    let mut compiled_paths = Vec::with_capacity(paths.len());
    for search_path in paths {
        let SearchPath {
            path,
            method,
            categories,
            inputs: path_inputs,
            response,
        } = search_path;
        compiled_paths.push(SearchPath {
            path: parse("search path", path.as_ref())?,
            method,
            categories,
            inputs: compile_inputs("search path input", path_inputs)?,
            response,
        });
    }

    let RowsBlock {
        selector,
        filters: rows_filters,
        after,
        remove,
        dateheaders,
        attribute,
        multiple,
        missing_attribute_equals_no_results,
        count,
    } = rows;

    let mut compiled_fields: Vec<(String, FieldBlock<ParsedTemplate>)> =
        Vec::with_capacity(fields.0.len());
    for (name, field) in fields.0 {
        compiled_fields.push((
            name.clone(),
            compile_field(&format!("field {name}"), field, true)?,
        ));
    }

    let login = login
        .map(|login| {
            let LoginBlock {
                method,
                path,
                form,
                submitpath,
                inputs,
                error,
                test,
                selectorinputs,
                captcha,
            } = login;
            let path = path.map(|p| parse("login path", p.as_ref())).transpose()?;
            let submitpath = submitpath
                .map(|p| parse("login submitpath", p.as_ref()))
                .transpose()?;
            let mut compiled_error = Vec::with_capacity(error.len());
            for (index, block) in error.into_iter().enumerate() {
                let what = format!("login error {index}");
                let message = block
                    .message
                    .map(|message| compile_field(&format!("{what} message"), message, false))
                    .transpose()?;
                compiled_error.push(ErrorBlock {
                    selector: block.selector,
                    message,
                });
            }
            Ok(LoginBlock {
                method,
                path,
                form,
                submitpath,
                inputs: compile_inputs("login input", inputs)?,
                error: compiled_error,
                test,
                selectorinputs,
                captcha,
            })
        })
        .transpose()?;

    let download = download
        .map(|download| {
            let DownloadBlock {
                selector,
                attribute,
                filters,
                method,
                before,
                infohash,
            } = download;
            Ok(DownloadBlock {
                selector,
                attribute,
                filters: compile_specs("download", filters, false)?,
                method,
                before,
                infohash,
            })
        })
        .transpose()?;

    Ok(CardigannDefinition {
        id,
        name,
        description,
        language,
        site_type,
        encoding,
        links,
        legacylinks,
        caps,
        settings,
        login,
        search: SearchBlock {
            paths: compiled_paths,
            path: None,
            inputs: compile_inputs("search input", inputs)?,
            keywordsfilters: compile_specs("keywordsfilters", keywordsfilters, false)?,
            rows: RowsBlock {
                selector,
                filters: compile_specs("rows.filters", rows_filters, false)?,
                after,
                remove,
                dateheaders: dateheaders
                    .map(|d| compile_field("rows.dateheaders", d, true))
                    .transpose()?,
                attribute,
                multiple,
                missing_attribute_equals_no_results,
                count: count
                    .map(|c| compile_field("rows.count", c, true))
                    .transpose()?,
            },
            fields: OrderedFields(compiled_fields),
        },
        download,
    })
}

#[cfg(test)]
mod tests;
