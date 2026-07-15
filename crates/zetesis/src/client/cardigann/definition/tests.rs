//! Cardigann definition schema parse + validation tests.

use super::*;

const SAMPLE: &str = r#"---
id: sample-tracker
name: Sample Tracker
description: A sample tracker for tests
language: en-US
type: public
encoding: UTF-8
links:
  - https://sample-tracker.example/

caps:
  categorymappings:
    - {id: 6, cat: Movies/HD, desc: "Movies HD"}
    - {id: 7, cat: Movies/SD}
    - {id: 12, cat: TV/HD}
  modes:
    search: [q]
    tv-search: [q, season, ep]
    movie-search: [q]

settings:
  - name: sort
    type: select
    label: Sort by
    default: created
    options:
      created: Created
      seeders: Seeders

login:
  method: none

search:
  paths:
    - path: /browse
  inputs:
    q: "{{ .Keywords }}"
    cats: "{{ .Categories }}"
    sort: "{{ .Config.sort }}"
  keywordsfilters:
    - name: re_replace
      args: ["\\s+", "."]
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    details:
      selector: a.title
      attribute: href
    category:
      selector: a.cat
      attribute: href
      filters:
        - name: querystring
          args: cat
    download:
      selector: a.dl
      attribute: href
    size:
      selector: td.size
    seeders:
      selector: td.seeds
    leechers:
      selector: td.leech
    date:
      selector: td.date
      filters:
        - name: dateparse
          args: "2006-01-02 15:04"
    downloadvolumefactor:
      case:
        img.freeleech: 0
        "*": 1
    uploadvolumefactor:
      text: 1
    description:
      selector: td.desc
      optional: true

download:
  selector: a#dl
  attribute: href
"#;

#[test]
fn parses_representative_definition() {
    let def = parse_definition(SAMPLE, "test").unwrap();
    assert_eq!(def.id, "sample-tracker");
    assert_eq!(def.name, "Sample Tracker");
    assert_eq!(def.site_type.as_deref(), Some("public"));
    assert_eq!(def.links, vec!["https://sample-tracker.example/"]);

    assert_eq!(def.caps.categorymappings.len(), 3);
    assert_eq!(def.caps.categorymappings[0].id.0, "6");
    assert_eq!(def.caps.categorymappings[0].cat, "Movies/HD");
    assert_eq!(
        def.caps.categorymappings[0].desc.as_deref(),
        Some("Movies HD")
    );
    assert_eq!(
        def.caps.modes.get("tv-search"),
        Some(&vec![
            "q".to_string(),
            "season".to_string(),
            "ep".to_string()
        ])
    );

    assert_eq!(def.settings.len(), 1);
    assert_eq!(def.settings[0].name, "sort");
    assert_eq!(def.settings[0].default.as_ref().unwrap().0, "created");

    assert_eq!(def.login.as_ref().unwrap().method.as_deref(), Some("none"));

    assert_eq!(def.search.paths.len(), 1);
    assert_eq!(def.search.paths[0].path, "/browse");
    assert_eq!(def.search.inputs.get("q").unwrap().0, "{{ .Keywords }}");
    assert_eq!(def.search.keywordsfilters.len(), 1);
    assert_eq!(def.search.keywordsfilters[0].name, "re_replace");
    assert_eq!(def.search.keywordsfilters[0].args(), ["\\s+", "."]);
    assert_eq!(def.search.rows.selector, "table#torrents > tbody > tr");
    assert_eq!(def.search.fields.len(), 11);

    let category = def.search.fields.get("category").unwrap();
    assert_eq!(category.selector.as_deref(), Some("a.cat"));
    assert_eq!(category.attribute.as_deref(), Some("href"));
    assert_eq!(category.filters[0].name, "querystring");
    assert_eq!(category.filters[0].args(), ["cat"]);

    let dvf = def.search.fields.get("downloadvolumefactor").unwrap();
    let case = dvf.case.as_ref().unwrap();
    assert_eq!(
        case.0[0],
        ("img.freeleech".to_string(), ScalarString("0".to_string()))
    );
    assert_eq!(case.0[1].0, "*");

    assert!(def.search.fields.get("description").unwrap().optional);

    let download = def.download.as_ref().unwrap();
    assert_eq!(download.selector.as_deref(), Some("a#dl"));
    assert_eq!(download.attribute.as_deref(), Some("href"));
}

#[test]
fn legacy_single_path_and_categories_normalize() {
    let def = parse_definition(
        r#"
id: legacy
name: Legacy
links: ["https://legacy.example/"]
caps:
  categories:
    "1": Movies
    "2": TV
  modes:
    search: [q]
search:
  path: /torrents
  rows:
    selector: tr.torrent
  fields:
    title:
      selector: a
    download:
      selector: a
      attribute: href
"#,
        "test",
    )
    .unwrap();
    assert_eq!(def.search.paths.len(), 1);
    assert_eq!(def.search.paths[0].path, "/torrents");
    assert_eq!(def.caps.categorymappings.len(), 2);
    assert!(def.caps.categories.is_empty());
    assert!(
        def.caps
            .categorymappings
            .iter()
            .any(|m| m.id.0 == "1" && m.cat == "Movies")
    );
}

#[test]
fn negated_path_categories_are_cleared() {
    let def = parse_definition(
        r#"
id: negated
name: Negated
links: ["https://negated.example/"]
caps:
  categorymappings:
    - {id: 1, cat: Movies}
  modes:
    search: [q]
search:
  paths:
    - path: /a
      categories: ["!", "2"]
  rows:
    selector: tr
  fields:
    title:
      selector: a
    download:
      selector: a
      attribute: href
"#,
        "test",
    )
    .unwrap();
    assert!(def.search.paths[0].categories.is_empty());
}

fn minimal_with(search_extra: &str) -> String {
    format!(
        r#"
id: x
name: X
links: ["https://x.example/"]
caps:
  categorymappings:
    - {{id: 1, cat: Movies}}
  modes:
    search: [q]
search:
{search_extra}
  rows:
    selector: tr
  fields:
    title:
      selector: a
    download:
      selector: a
      attribute: href
"#
    )
}

#[test]
fn unsupported_filter_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a
  keywordsfilters:
    - name: diacritics
      args: replace"#,
    );
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionInvalid { .. }),
        "got {err:?}"
    );
    assert!(err.to_string().contains("diacritics"), "got {err}");
}

#[test]
fn post_search_path_accepted_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a
      method: post"#,
    );
    parse_definition(&yaml, "test").expect("POST search paths are supported");
}

#[test]
fn raw_input_with_post_search_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a
      method: post
  inputs:
    $raw: "q={{ .Keywords }}""#,
    );
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionUnsupported { .. }),
        "got {err:?}"
    );
    assert!(err.to_string().contains("$raw"), "got {err}");
}

#[test]
fn json_response_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a
      response:
        type: json"#,
    );
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionUnsupported { .. }),
        "got {err:?}"
    );
}

#[test]
fn unsupported_template_construct_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a
  inputs:
    q: "{{ if .Keywords }}{{ .Keywords }}{{ end }}""#,
    );
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionInvalid { .. }),
        "got {err:?}"
    );
}

#[test]
fn unknown_config_key_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a
  inputs:
    q: "{{ .Config.nonexistent }}""#,
    );
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(err.to_string().contains("nonexistent"), "got {err}");
}

#[test]
fn bad_selector_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    )
    .replace("selector: tr", "selector: \"tr[\"");
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionInvalid { .. }),
        "got {err:?}"
    );
}

#[test]
fn templated_link_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    )
    .replace("https://x.example/", "{{ .Config.sitelink }}");
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionUnsupported { .. }),
        "got {err:?}"
    );
}

#[test]
fn missing_title_field_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    )
    .replace("title:", "titel:");
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionInvalid { .. }),
        "got {err:?}"
    );
    assert!(err.to_string().contains("\"title\""), "got {err}");
}

#[test]
fn missing_download_source_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    )
    .replace("download:", "downlod:");
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionInvalid { .. }),
        "got {err:?}"
    );
    assert!(err.to_string().contains("download"), "got {err}");
}

#[test]
fn magnet_field_satisfies_download_source() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    )
    .replace("download:", "magnet:");
    parse_definition(&yaml, "test").unwrap();
}

#[test]
fn form_login_definition_loads_with_declared_credentials() {
    // WHY: a form login is executable — it loads when its credential inputs
    // reference declared settings and a login.path is present.
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    ) + r#"
settings:
  - name: username
    type: text
  - name: password
    type: password
login:
  method: form
  path: /login.php
  form: form#login
  inputs:
    username: "{{ .Config.username }}"
    password: "{{ .Config.password }}"
  error:
    - selector: div.error
  test:
    path: /profile
    selector: a.logout
"#;
    let def = parse_definition(&yaml, "test").unwrap();
    let login = def.login.unwrap();
    assert_eq!(login.method.as_deref(), Some("form"));
    assert_eq!(login.form.as_deref(), Some("form#login"));
    assert_eq!(login.error.len(), 1);
    assert_eq!(login.error[0].selector, "div.error");
}

#[test]
fn interactive_login_without_path_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    ) + r#"
login:
  method: post
  inputs:
    q: "{{ .Keywords }}"
"#;
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionInvalid { .. }),
        "got {err:?}"
    );
    assert!(err.to_string().contains("login.path"), "got {err}");
}

#[test]
fn login_captcha_block_rejected_at_load() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a"#,
    ) + r#"
login:
  method: form
  path: /login.php
  captcha:
    type: image
    selector: img.captcha
"#;
    let err = parse_definition(&yaml, "test").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionUnsupported { ref feature, .. } if feature.contains("captcha")),
        "got {err:?}"
    );
}

#[test]
fn scalar_and_list_filter_args_both_parse() {
    let yaml = minimal_with(
        r#"  paths:
    - path: /a
  keywordsfilters:
    - name: append
      args: " extra"
    - name: re_replace
      args: ["\\s+", "+"]
    - name: trim"#,
    );
    let def = parse_definition(&yaml, "test").unwrap();
    assert_eq!(def.search.keywordsfilters[0].args(), [" extra"]);
    assert_eq!(def.search.keywordsfilters[1].args(), ["\\s+", "+"]);
    assert!(def.search.keywordsfilters[2].args().is_empty());
}

#[test]
fn malformed_yaml_is_a_load_error() {
    let err = parse_definition(": not yaml :", "somefile.yml").unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionLoad { .. }),
        "got {err:?}"
    );
    assert!(err.to_string().contains("somefile.yml"), "got {err}");
}
