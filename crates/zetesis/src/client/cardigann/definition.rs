//! Cardigann YAML definition schema, loading, and load-time validation.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use tracing::warn;

use crate::client::cardigann::{filters, template};
use crate::error::SearchIndexerError;

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
    #[serde(default)]
    pub inputs: BTreeMap<String, ScalarString>,
    #[serde(default)]
    pub test: Option<LoginTest>,
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
    pub fields: BTreeMap<String, FieldBlock>,
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

/// A YAML scalar (string, number, or bool) normalized to its string form.
///
/// WHY: definition authors write `id: 42` and `id: "42"` interchangeably;
/// downstream code only ever compares/joins string forms.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScalarString(pub String);

impl<'de> Deserialize<'de> for ScalarString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ScalarVisitor;

        impl Visitor<'_> for ScalarVisitor {
            type Value = ScalarString;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a YAML scalar (string, number, or bool)")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }
        }

        deserializer.deserialize_any(ScalarVisitor)
    }
}

/// A YAML mapping with author order preserved.
#[derive(Debug, Clone, Default)]
pub struct OrderedPairs(pub Vec<(String, ScalarString)>);

impl<'de> Deserialize<'de> for OrderedPairs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PairsVisitor;

        impl<'de> Visitor<'de> for PairsVisitor {
            type Value = OrderedPairs;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping")
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, ScalarString>()? {
                    out.push(entry);
                }
                Ok(OrderedPairs(out))
            }
        }

        deserializer.deserialize_map(PairsVisitor)
    }
}

/// Filter arguments: YAML allows a bare scalar or a list of scalars.
#[derive(Debug, Clone)]
pub struct FilterArgs(pub Vec<String>);

impl<'de> Deserialize<'de> for FilterArgs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ArgsVisitor;

        impl<'de> Visitor<'de> for ArgsVisitor {
            type Value = FilterArgs;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a scalar or a list of scalars")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(item) = seq.next_element::<ScalarString>()? {
                    out.push(item.0);
                }
                Ok(FilterArgs(out))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![v.to_string()]))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![v.to_string()]))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![v.to_string()]))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![v.to_string()]))
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![v.to_string()]))
            }
        }

        deserializer.deserialize_any(ArgsVisitor)
    }
}

/// Parses, normalizes, and validates one definition file.
pub fn load_definition_file(path: &Path) -> Result<CardigannDefinition, SearchIndexerError> {
    let display_path = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| SearchIndexerError::DefinitionLoad {
        path: display_path.clone(),
        reason: e.to_string(),
        location: snafu::Location::new(file!(), line!(), column!()),
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
            location: snafu::Location::new(file!(), line!(), column!()),
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
        location: snafu::Location::new(file!(), line!(), column!()),
    };
    let unsupported = |feature: String| SearchIndexerError::DefinitionUnsupported {
        definition_id: def.id.clone(),
        feature,
        location: snafu::Location::new(file!(), line!(), column!()),
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
    let check_template = |what: &str, tmpl: &str| {
        template::validate(tmpl, &config_keys).map_err(|e| invalid(format!("{what}: {e}")))
    };
    let check_selector = |what: &str, sel: &str| {
        scraper::Selector::parse(sel)
            .map(|_| ())
            .map_err(|e| invalid(format!("{what} selector {sel:?}: {e}")))
    };
    let check_filters = |what: &str, specs: &[FilterSpec]| {
        filters::validate(specs).map_err(|e| invalid(format!("{what}: {e}")))
    };

    for path in &def.search.paths {
        match path.method.as_deref() {
            None | Some("get") => {}
            Some(other) => return Err(unsupported(format!("search path method {other:?}"))),
        }
        if let Some(response) = &path.response
            && response.response_type.as_deref() != Some("html")
        {
            return Err(unsupported(format!(
                "search response type {:?}",
                response.response_type
            )));
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

    check_selector("rows", &def.search.rows.selector)?;
    if !def.search.rows.filters.is_empty() {
        warn!(
            definition_id = %def.id,
            "rows.filters are not supported; rows are not post-filtered"
        );
    }
    if def.search.rows.after.is_some() || def.search.rows.dateheaders.is_some() {
        warn!(
            definition_id = %def.id,
            "rows.after / rows.dateheaders are not supported and are ignored"
        );
    }
    if let Some(remove) = &def.search.rows.remove {
        check_selector("rows.remove", remove)?;
    }

    if def.search.fields.is_empty() {
        return Err(invalid("no search fields".to_string()));
    }
    for (name, field) in &def.search.fields {
        if let Some(sel) = &field.selector {
            check_selector(&format!("field {name}"), sel)?;
        }
        if let Some(remove) = &field.remove {
            check_selector(&format!("field {name} remove"), remove)?;
        }
        if let Some(case) = &field.case {
            for (case_selector, _) in &case.0 {
                check_selector(&format!("field {name} case"), case_selector)?;
            }
        }
        if let Some(text) = &field.text {
            check_template(&format!("field {name} text"), &text.0)?;
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

    // NOTE: login templates are only validated for methods this engine runs;
    // unsupported methods already fail at client construction, and their
    // inputs routinely use template constructs outside this engine's subset.
    if let Some(login) = &def.login
        && matches!(login.method.as_deref(), Some("none" | "cookie") | None)
    {
        for (key, value) in &login.inputs {
            check_template(&format!("login input {key}"), &value.0)?;
        }
        if let Some(test) = &login.test
            && let Some(sel) = &test.selector
        {
            check_selector("login test", sel)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
