//! Cardigann YAML definition schema, loading, and load-time validation.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use tracing::warn;

use crate::client::cardigann::{filters, template};
use crate::error::SearchIndexerError;

mod yaml;
pub use yaml::{FilterArgs, OrderedFields, OrderedPairs, ScalarString};

/// One Prowlarr-compatible Cardigann indexer definition, deserialized from a
/// single YAML file.
///
/// Unknown YAML keys are ignored on purpose: real-world definitions carry
/// many blocks this engine does not model, and a permissive schema keeps a
/// definition loadable as long as the parts this engine executes are sound.
#[derive(Debug, Clone, Deserialize)]
pub struct CardigannDefinition {
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
    pub login: Option<LoginBlock>,
    pub search: SearchBlock,
    #[serde(default)]
    pub download: Option<DownloadBlock>,
}

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
pub struct LoginBlock {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    /// CSS selector for the login `<form>` (form method; defaults to "form").
    #[serde(default)]
    pub form: Option<String>,
    /// Submit-target override, joined on the site base instead of the
    /// form's `action`.
    #[serde(default)]
    pub submitpath: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, ScalarString>,
    /// Failed-login detectors checked against the post-submit page.
    #[serde(default)]
    pub error: Vec<ErrorBlock>,
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
pub struct ErrorBlock {
    pub selector: String,
    #[serde(default)]
    pub message: Option<FieldBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginTest {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchBlock {
    #[serde(default)]
    pub paths: Vec<SearchPath>,
    /// Legacy single-path form. Folded into `paths` by [`normalize`].
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, ScalarString>,
    #[serde(default)]
    pub keywordsfilters: Vec<FilterSpec>,
    pub rows: RowsBlock,
    #[serde(default)]
    pub fields: OrderedFields,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchPath {
    pub path: String,
    #[serde(default)]
    pub method: Option<String>,
    /// Site category ids this path is limited to (empty = all queries).
    #[serde(default)]
    pub categories: Vec<ScalarString>,
    /// Extra inputs merged over `search.inputs` for this path.
    #[serde(default)]
    pub inputs: BTreeMap<String, ScalarString>,
    #[serde(default)]
    pub response: Option<ResponseBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseBlock {
    #[serde(default, rename = "type")]
    pub response_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RowsBlock {
    pub selector: String,
    #[serde(default)]
    pub filters: Vec<FilterSpec>,
    #[serde(default)]
    pub after: Option<u32>,
    #[serde(default)]
    pub remove: Option<String>,
    #[serde(default)]
    pub dateheaders: Option<FieldBlock>,
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
    pub count: Option<FieldBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldBlock {
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    /// Constant/template value used instead of selecting from the row.
    #[serde(default)]
    pub text: Option<ScalarString>,
    /// Fallback template rendered (row scope, like `text`) when selector
    /// extraction yields nothing (upstream `FieldBlock.Default`).
    #[serde(default)]
    pub default: Option<ScalarString>,
    #[serde(default)]
    pub filters: Vec<FilterSpec>,
    #[serde(default)]
    pub optional: bool,
    /// Selector → value pairs; the first selector matching the selected
    /// element (or one of its descendants) supplies the value.
    ///
    /// WHY: stored as ordered pairs, not a map — first-match-wins semantics
    /// depend on the author's YAML order.
    #[serde(default)]
    pub case: Option<OrderedPairs>,
    /// Selector for descendants to exclude from text extraction.
    #[serde(default)]
    pub remove: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadBlock {
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub filters: Vec<FilterSpec>,
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
pub struct FilterSpec {
    pub name: String,
    #[serde(default)]
    pub args: Option<FilterArgs>,
}

impl FilterSpec {
    pub fn args(&self) -> &[String] {
        self.args.as_ref().map_or(&[], |a| a.0.as_slice())
    }
}

/// Parses, normalizes, and validates one definition file.
pub fn load_definition_file(path: &Path) -> Result<CardigannDefinition, SearchIndexerError> {
    let display_path = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| SearchIndexerError::DefinitionLoad {
        path: display_path.clone(),
        reason: e.to_string(),
        location: std::panic::Location::caller(),
    })?;
    parse_definition(&text, &display_path)
}

/// Parses, normalizes, and validates definition YAML; `origin` labels load
/// errors (usually the file path).
pub fn parse_definition(
    text: &str,
    origin: &str,
) -> Result<CardigannDefinition, SearchIndexerError> {
    let mut definition: CardigannDefinition =
        serde_norway::from_str(text).map_err(|e| SearchIndexerError::DefinitionLoad {
            path: origin.to_string(),
            reason: e.to_string(),
            location: std::panic::Location::caller(),
        })?;
    normalize(&mut definition);
    validate(&definition)?;
    Ok(definition)
}

/// Folds legacy schema forms into their modern equivalents.
fn normalize(def: &mut CardigannDefinition) {
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
                path = %path.path,
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
fn validate(def: &CardigannDefinition) -> Result<(), SearchIndexerError> {
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

    let config_keys: Vec<&str> = def.settings.iter().map(|s| s.name.as_str()).collect();
    let field_names: Vec<String> = def.search.fields.names();
    let check_template = |what: &str, tmpl: &str| {
        template::validate(tmpl, &config_keys).map_err(|e| invalid(format!("{what}: {e}")))
    };
    // Row scope: `.Result.<field>` is meaningful only where the extractor
    // renders with the row's accumulated values — field text/case/default
    // values and field filter args.
    let check_row_template = |what: &str, tmpl: &str| {
        template::validate_row_scoped(tmpl, &config_keys, &field_names)
            .map_err(|e| invalid(format!("{what}: {e}")))
    };
    // WHY: filter args are templates too — validate them so an unsupported
    // construct fails at load instead of reaching the pipeline verbatim.
    let check_filter_args =
        |what: &str, specs: &[FilterSpec], row_scoped: bool| -> Result<(), SearchIndexerError> {
            for spec in specs {
                for (index, arg) in spec.args().iter().enumerate() {
                    let checked = if row_scoped {
                        template::validate_row_scoped(arg, &config_keys, &field_names)
                    } else {
                        template::validate(arg, &config_keys)
                    };
                    checked.map_err(|e| {
                        invalid(format!("{what} filter {:?} arg {index}: {e}", spec.name))
                    })?;
                }
            }
            Ok(())
        };
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
        check_template("search path", &path.path)?;
        for (key, value) in &path.inputs {
            check_template(&format!("search path input {key}"), &value.0)?;
        }
    }
    for (key, value) in &def.search.inputs {
        check_template(&format!("search input {key}"), &value.0)?;
    }
    check_filters("keywordsfilters", &def.search.keywordsfilters)?;
    check_filter_args("keywordsfilters", &def.search.keywordsfilters, false)?;

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
        check_filter_args("rows.filters", &def.search.rows.filters, false)?;
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
        if let Some(text) = &field.text {
            check_row_template(&format!("field {name} text"), &text.0)?;
        }
        if let Some(default) = &field.default {
            check_row_template(&format!("field {name} default"), &default.0)?;
        }
        if let Some(case) = &field.case {
            for (_, case_value) in &case.0 {
                check_row_template(&format!("field {name} case value"), &case_value.0)?;
            }
        }
        check_filters(&format!("field {name}"), &field.filters)?;
        check_filter_args(&format!("field {name}"), &field.filters, true)?;
    }

    if let Some(download) = &def.download {
        if let Some(sel) = &download.selector {
            check_selector("download", sel)?;
        }
        check_filters("download", &download.filters)?;
        check_filter_args("download", &download.filters, false)?;
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
    // Unknown methods fail at client construction with LoginUnsupported, and
    // their inputs routinely use template constructs outside this subset.
    if let Some(login) = &def.login {
        let method = login.method.as_deref().unwrap_or("form");
        let interactive = matches!(method, "form" | "post" | "get");
        if interactive || matches!(method, "none" | "cookie") {
            for (key, value) in &login.inputs {
                check_template(&format!("login input {key}"), &value.0)?;
            }
            if let Some(test) = &login.test
                && let Some(sel) = &test.selector
            {
                check_selector("login test", sel)?;
            }
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
            let Some(path) = &login.path else {
                return Err(invalid(format!(
                    "login method {method:?} requires login.path"
                )));
            };
            check_template("login path", path)?;
            if let Some(form) = &login.form {
                check_selector("login form", form)?;
            }
            if let Some(submitpath) = &login.submitpath {
                check_template("login submitpath", submitpath)?;
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
                        for (case_selector, case_value) in &case.0 {
                            check_selector(&format!("{what} message case"), case_selector)?;
                            check_template(&format!("{what} message case value"), &case_value.0)?;
                        }
                    }
                    if let Some(text) = &message.text {
                        check_template(&format!("{what} message text"), &text.0)?;
                    }
                    check_filters(&format!("{what} message"), &message.filters)?;
                    check_filter_args(&format!("{what} message"), &message.filters, false)?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
