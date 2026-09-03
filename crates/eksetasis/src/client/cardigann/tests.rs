//! CardigannClient + CardigannRegistry behavior tests (mock HTTP).

use super::*;
use crate::cf_bypass::noop::NoProxy;
use crate::test_support::{spawn_one_shot_http, spawn_raw_http, spawn_sequence_http};

const SAMPLE_DEF: &str = r#"---
id: sample-tracker
name: Sample Tracker
type: public
links:
  - https://sample-tracker.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
    - {id: 7, cat: Movies/SD}
    - {id: 12, cat: TV/HD}
  modes:
    search: [q]
    tv-search: [q, season, ep]
    movie-search: [q]
settings:
  - name: sort
    type: select
    default: created
    options:
      created: Created
      size: Size
  - name: freeleech_only
    type: checkbox
    default: "false"
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
"#;

const SAMPLE_HTML: &str = r#"<html><body>
<table id="torrents"><tbody>
<tr>
  <td><a class="title" href="/details/101">Example.Movie.2160p.WEB</a><img class="freeleech" src="/fl.png"></td>
  <td><a class="cat" href="/browse?cat=6">Movies</a></td>
  <td><a class="dl" href="/download/101.torrent">DL</a></td>
  <td class="size">1.5 GiB</td>
  <td class="seeds">42</td>
  <td class="leech">7</td>
  <td class="date">2024-01-15 10:30</td>
  <td class="desc">Great release</td>
</tr>
<tr>
  <td><a class="title" href="/details/102">Example.Show.S01E02.720p</a></td>
  <td><a class="cat" href="/browse?cat=12">TV</a></td>
  <td><a class="dl" href="/download/102.torrent">DL</a></td>
  <td class="size">700 MB</td>
  <td class="seeds">1,024</td>
  <td class="leech">2</td>
  <td class="date">2024-02-01 08:00</td>
  <td class="desc"></td>
</tr>
</tbody></table>
</body></html>"#;

fn definition(yaml: &str) -> Arc<CompiledDefinition> {
    Arc::new(definition::parse_definition(yaml, "test").unwrap())
}

fn client(
    yaml: &str,
    url: String,
    api_key: Option<&str>,
) -> Result<CardigannClient, SearchIndexerError> {
    client_with_settings(yaml, url, api_key, BTreeMap::new())
}

fn client_with_settings(
    yaml: &str,
    url: String,
    api_key: Option<&str>,
    settings: BTreeMap<String, String>,
) -> Result<CardigannClient, SearchIndexerError> {
    client_with_sessions(yaml, url, api_key, settings, Arc::new(SessionStore::new()))
}

fn client_with_sessions(
    yaml: &str,
    url: String,
    api_key: Option<&str>,
    settings: BTreeMap<String, String>,
    sessions: Arc<SessionStore>,
) -> Result<CardigannClient, SearchIndexerError> {
    CardigannClient::new(
        Arc::new(SearchSubsystemConfig::default()),
        reqwest::Client::new(),
        Arc::new(NoProxy),
        Duration::from_secs(5),
        IndexerConfig {
            id: 1,
            name: "Test".to_string(),
            url,
            api_key: api_key.map(str::to_string),
            cf_bypass: false,
            settings,
        },
        definition(yaml),
        sessions,
    )
}

fn movie_query() -> SearchQuery {
    SearchQuery {
        query_text: Some("test query".to_string()),
        category_ids: vec![2000],
        limit: 100,
        ..Default::default()
    }
}

#[tokio::test]
async fn search_extracts_rows_fields_and_maps_categories() {
    let (url, server) = spawn_one_shot_http(200, "OK", &[], SAMPLE_HTML).await;
    let results = client(SAMPLE_DEF, url.clone(), None)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(results.len(), 2);

    let first = &results[0];
    assert_eq!(first.title, "Example.Movie.2160p.WEB");
    assert_eq!(first.download_url, format!("{url}/download/101.torrent"));
    assert_eq!(
        first.guid.as_deref(),
        Some(format!("{url}/details/101").as_str())
    );
    assert_eq!(first.size_bytes, Some(1_610_612_736));
    assert_eq!(first.seeders, Some(42));
    assert_eq!(first.leechers, Some(7));
    assert_eq!(first.category_id, Some(2040));
    assert_eq!(
        first.publication_date.as_deref(),
        Some("2024-01-15T10:30:00Z")
    );
    assert_eq!(first.protocol, ReleaseProtocol::Torrent);
    assert_eq!(first.download_volume_factor, 0.0);
    assert_eq!(first.upload_volume_factor, 1.0);
    assert_eq!(first.indexer_id, 1);
    assert_eq!(
        first.custom_attrs.get("description").map(String::as_str),
        Some("Great release")
    );

    let second = &results[1];
    assert_eq!(second.title, "Example.Show.S01E02.720p");
    assert_eq!(second.category_id, Some(5040));
    assert_eq!(second.seeders, Some(1024));
    assert_eq!(second.size_bytes, Some(700 * 1024 * 1024));
    assert_eq!(second.download_volume_factor, 1.0);
    // WHY: the empty optional description cell must be absent, not "".
    assert!(!second.custom_attrs.contains_key("description"));

    let request_head = server.await.unwrap();
    let request_line = request_head.lines().next().unwrap_or_default().to_string();
    assert!(
        request_line.starts_with("GET /browse?"),
        "got: {request_line}"
    );
    assert!(request_line.contains("q=test.query"), "got: {request_line}");
    assert!(request_line.contains("cats=6%2C7"), "got: {request_line}");
    // WHY: no settings override was supplied — the definition's `default:`
    // value must be what reaches the request.
    assert!(request_line.contains("sort=created"), "got: {request_line}");
}

// ── POST search ─────────────────────────────────────────────────────────

#[tokio::test]
async fn post_search_sends_form_body_and_parses_rows() {
    // WHY: a `method: post` search path sends its inputs as an
    // application/x-www-form-urlencoded body; the endpoint URL carries no
    // input query. spawn_sequence_http captures the request body (spawn_raw
    // stops at the header terminator).
    let (url, server) = spawn_sequence_http(vec![(200, vec![], SAMPLE_HTML.to_string())]).await;
    let post_def = SAMPLE_DEF.replace(
        "    - path: /browse\n",
        "    - path: /browse\n      method: post\n",
    );
    let results = client(&post_def, url.clone(), None)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 2);

    let heads = server.await.unwrap();
    let request = &heads[0];
    let request_line = request.lines().next().unwrap_or_default();
    assert!(
        request_line.starts_with("POST /browse "),
        "got: {request_line}"
    );
    assert!(
        !request_line.contains('?'),
        "POST endpoint must carry no input query: {request_line}"
    );
    assert!(
        request
            .to_lowercase()
            .contains("content-type: application/x-www-form-urlencoded"),
        "missing form content-type: {request}"
    );
    let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
    assert!(body.contains("q=test.query"), "form body: {body}");
    assert!(body.contains("cats=6%2C7"), "form body: {body}");
    assert!(body.contains("sort=created"), "form body: {body}");
}

#[test]
fn cf_bypass_with_post_search_unsupported_at_construction() {
    // WHY: the bypass proxy is GET-only; a POST search body cannot be
    // delivered through it, so construction must fail loudly.
    let post_def = SAMPLE_DEF.replace(
        "    - path: /browse\n",
        "    - path: /browse\n      method: post\n",
    );
    let err = CardigannClient::new(
        Arc::new(SearchSubsystemConfig::default()),
        reqwest::Client::new(),
        Arc::new(NoProxy),
        Duration::from_secs(5),
        IndexerConfig {
            id: 1,
            name: "Test".to_string(),
            url: "https://sample-tracker.example/".to_string(),
            api_key: None,
            cf_bypass: true,
            settings: BTreeMap::new(),
        },
        definition(&post_def),
        Arc::new(SessionStore::new()),
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionUnsupported { ref feature, .. } if feature.contains("POST")),
        "got {err:?}"
    );
}

// ── row filters ─────────────────────────────────────────────────────────

#[tokio::test]
async fn andmatch_rows_filter_drops_titles_missing_a_keyword() {
    // WHY: `rows.filters: andmatch` post-filters rows to those whose title
    // carries every query keyword — the most-used Cardigann rows filter.
    let def = SAMPLE_DEF.replace(
        "  rows:\n    selector: table#torrents > tbody > tr\n",
        "  rows:\n    selector: table#torrents > tbody > tr\n    filters:\n      - name: andmatch\n",
    );
    let (url, server) = spawn_one_shot_http(200, "OK", &[], SAMPLE_HTML).await;
    let query = SearchQuery {
        query_text: Some("Example Movie".to_string()),
        category_ids: vec![2000],
        limit: 100,
        ..Default::default()
    };
    let results = client(&def, url.clone(), None)
        .unwrap()
        .search(&query, CancellationToken::new())
        .await
        .unwrap();

    // only the Movie row carries both keywords; the Show row lacks "Movie"
    assert_eq!(results.len(), 1, "got {results:?}");
    assert_eq!(results[0].title, "Example.Movie.2160p.WEB");
    let _ = server.await;
}

#[tokio::test]
async fn andmatch_is_skipped_for_a_natively_supported_id_search() {
    // WHY: upstream skips andmatch when the tracker itself resolves the id, so
    // the rows it returns are authoritative. This matters for the combined
    // case: an id search WITH season/episode renders " S01E02" into the
    // keywords, which would otherwise AND-match against titles that legitimately
    // do not embed the id.
    let def = SAMPLE_DEF
        .replace(
            "  rows:\n    selector: table#torrents > tbody > tr\n",
            "  rows:\n    selector: table#torrents > tbody > tr\n    filters:\n      - name: andmatch\n",
        )
        .replace("    movie-search: [q]", "    movie-search: [q, imdbid]");
    assert!(def.contains("imdbid"), "caps.modes must advertise imdbid");

    let (url, server) = spawn_one_shot_http(200, "OK", &[], SAMPLE_HTML).await;
    let query = SearchQuery {
        query_text: Some("nothing matches this".to_string()),
        imdb_id: Some("tt1234567".to_string()),
        category_ids: vec![2000],
        limit: 100,
        ..Default::default()
    };
    let results = client(&def, url.clone(), None)
        .unwrap()
        .search(&query, CancellationToken::new())
        .await
        .unwrap();

    // keywords match no title, yet every row survives — the filter was skipped
    assert_eq!(results.len(), 2, "got {results:?}");
    let _ = server.await;
}

// ── JSON responses ──────────────────────────────────────────────────────

const JSON_DEF: &str = r#"---
id: json-tracker
name: JSON Tracker
type: public
links:
  - https://json-tracker.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
    - {id: 7, cat: Movies/SD}
  modes:
    search: [q]
    movie-search: [q]
search:
  paths:
    - path: /api/search
      response:
        type: json
  inputs:
    q: "{{ .Keywords }}"
  rows:
    selector: data.results
  fields:
    title:
      selector: name
    download:
      selector: download
    details:
      selector: url
    size:
      selector: size
    seeders:
      selector: seeders
    leechers:
      selector: leechers
    category:
      selector: category_id
    tags:
      selector: tags
"#;

// WHY: row 0 uses a raw-byte numeric `size` and a whole-number float `seeders`
// to exercise the JSON-API conventions (unitless bytes; float-serialized ints).
const JSON_BODY: &str = r#"{"data": {"results": [
  {"name": "Example.Movie.1080p", "download": "https://t/1.torrent", "url": "https://t/d/1",
   "size": 1610612736, "seeders": 42.0, "leechers": 7, "category_id": 6, "tags": ["x264", "web"]},
  {"name": "Example.Show.720p", "download": "https://t/2.torrent", "url": "https://t/d/2",
   "size": "700 MB", "seeders": 10, "leechers": 1, "category_id": 7, "tags": []}
]}}"#;

#[tokio::test]
async fn json_search_extracts_rows_and_maps_fields() {
    let (url, server) = spawn_one_shot_http(
        200,
        "OK",
        &[("content-type", "application/json")],
        JSON_BODY,
    )
    .await;
    let results = client(JSON_DEF, url.clone(), None)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    let first = &results[0];
    assert_eq!(first.title, "Example.Movie.1080p");
    assert_eq!(first.download_url, "https://t/1.torrent");
    assert_eq!(first.size_bytes, Some(1_610_612_736));
    assert_eq!(first.seeders, Some(42));
    assert_eq!(first.leechers, Some(7));
    // site category 6 -> Movies/HD -> torznab 2040
    assert_eq!(first.category_id, Some(2040));
    // JSON array field -> comma-joined, lands in custom_attrs
    assert_eq!(
        first.custom_attrs.get("tags").map(String::as_str),
        Some("x264,web")
    );

    let request_line = server
        .await
        .unwrap()
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    assert!(
        request_line.starts_with("GET /api/search?"),
        "got: {request_line}"
    );
}

// ── settings overrides ──────────────────────────────────────────────────

#[tokio::test]
async fn settings_override_reaches_the_request() {
    let (url, server) = spawn_one_shot_http(200, "OK", &[], SAMPLE_HTML).await;
    let settings = BTreeMap::from([("sort".to_string(), "size".to_string())]);
    client_with_settings(SAMPLE_DEF, url, None, settings)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();

    let request_head = server.await.unwrap();
    let request_line = request_head.lines().next().unwrap_or_default().to_string();
    assert!(request_line.contains("sort=size"), "got: {request_line}");
    assert!(
        !request_line.contains("sort=created"),
        "got: {request_line}"
    );
}

#[test]
fn settings_unknown_key_is_rejected() {
    let settings = BTreeMap::from([("nope".to_string(), "x".to_string())]);
    let err = client_with_settings(
        SAMPLE_DEF,
        "http://127.0.0.1:9/".to_string(),
        None,
        settings,
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::SettingsInvalid { ref reason, .. } if reason.contains("unknown setting")),
        "got {err:?}"
    );
}

#[test]
fn settings_select_out_of_options_is_rejected() {
    let settings = BTreeMap::from([("sort".to_string(), "nonexistent".to_string())]);
    let err = client_with_settings(
        SAMPLE_DEF,
        "http://127.0.0.1:9/".to_string(),
        None,
        settings,
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::SettingsInvalid { ref reason, .. } if reason.contains("not one of the declared options")),
        "got {err:?}"
    );
}

#[test]
fn settings_checkbox_non_boolean_is_rejected() {
    let settings = BTreeMap::from([("freeleech_only".to_string(), "yes".to_string())]);
    let err = client_with_settings(
        SAMPLE_DEF,
        "http://127.0.0.1:9/".to_string(),
        None,
        settings,
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::SettingsInvalid { ref reason, .. } if reason.contains("checkbox")),
        "got {err:?}"
    );
}

#[test]
fn settings_checkbox_boolean_string_is_accepted() {
    let settings = BTreeMap::from([("freeleech_only".to_string(), "true".to_string())]);
    client_with_settings(
        SAMPLE_DEF,
        "http://127.0.0.1:9/".to_string(),
        None,
        settings,
    )
    .unwrap();
}

#[test]
fn settings_cookie_key_is_rejected() {
    let settings = BTreeMap::from([("cookie".to_string(), "sneaky=1".to_string())]);
    let err = client_with_settings(
        SAMPLE_DEF,
        "http://127.0.0.1:9/".to_string(),
        None,
        settings,
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::SettingsInvalid { ref reason, .. } if reason.contains("reserved")),
        "got {err:?}"
    );
}

const PATH_TEMPLATE_DEF: &str = r#"---
id: path-template
name: Path Template
links:
  - https://path-template.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
  modes:
    search: [q]
search:
  paths:
    - path: "/browse.php?search={{ .Keywords }}&cat=0"
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    download:
      selector: a.dl
      attribute: href
"#;

#[tokio::test]
async fn keywords_inline_in_path_template_are_percent_encoded() {
    let (url, server) = spawn_one_shot_http(200, "OK", &[], "<html/>").await;
    let query = SearchQuery {
        query_text: Some("AT&T #1".to_string()),
        limit: 100,
        ..Default::default()
    };
    client(PATH_TEMPLATE_DEF, url, None)
        .unwrap()
        .search(&query, CancellationToken::new())
        .await
        .unwrap();

    let request_head = server.await.unwrap();
    let request_line = request_head.lines().next().unwrap_or_default().to_string();
    // WHY: the raw keyword must not smuggle query structure — "&" would
    // split a bogus param and "#" would truncate at a fragment. The value
    // form-urlencodes wholesale; the template's own "?"/"&"/"=" survive.
    assert_eq!(
        request_line,
        "GET /browse.php?search=AT%26T+%231&cat=0 HTTP/1.1"
    );
}

const RAW_INPUT_DEF: &str = r#"---
id: raw-input
name: Raw Input
links:
  - https://raw-input.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
  modes:
    search: [q]
search:
  paths:
    - path: /browse
  inputs:
    $raw: "q={{ .Keywords }}&mode=list"
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    download:
      selector: a.dl
      attribute: href
"#;

#[tokio::test]
async fn raw_input_expansions_are_percent_encoded() {
    // WHY: $raw is spliced verbatim into the query via set_query (no encoding),
    // so a keyword like "a&b=c" must be encoded to a single value — never split
    // into an injected `b=c` parameter. The literal $raw "q="/"&mode=" survive.
    let (url, server) = spawn_one_shot_http(200, "OK", &[], "<html/>").await;
    let query = SearchQuery {
        query_text: Some("a&b=c".to_string()),
        limit: 100,
        ..Default::default()
    };
    client(RAW_INPUT_DEF, url, None)
        .unwrap()
        .search(&query, CancellationToken::new())
        .await
        .unwrap();

    let request_head = server.await.unwrap();
    let request_line = request_head.lines().next().unwrap_or_default().to_string();
    assert_eq!(
        request_line, "GET /browse?q=a%26b%3Dc&mode=list HTTP/1.1",
        "raw expansion must encode; no injected param"
    );
    // WHY: guard against a decoded "&b=c" smuggling a second parameter.
    assert!(
        !request_line.contains("&b=c"),
        "injected param leaked: {request_line}"
    );
}

#[tokio::test]
async fn search_without_inputs_appends_no_bare_question_mark() {
    let yaml = PATH_TEMPLATE_DEF.replace("/browse.php?search={{ .Keywords }}&cat=0", "/browse");
    let (url, server) = spawn_one_shot_http(200, "OK", &[], "<html/>").await;
    client(&yaml, url, None)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();
    let request_head = server.await.unwrap();
    let request_line = request_head.lines().next().unwrap_or_default().to_string();
    assert_eq!(request_line, "GET /browse HTTP/1.1");
}

#[tokio::test]
async fn search_respects_query_limit() {
    let (url, _server) = spawn_one_shot_http(200, "OK", &[], SAMPLE_HTML).await;
    let query = SearchQuery {
        limit: 1,
        ..movie_query()
    };
    let results = client(SAMPLE_DEF, url, None)
        .unwrap()
        .search(&query, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn search_unmapped_categories_short_circuits_empty() {
    // WHY: the refusing port proves no request is made when the tracker
    // has none of the requested categories.
    let query = SearchQuery {
        category_ids: vec![7000],
        ..movie_query()
    };
    let results = client(SAMPLE_DEF, "http://127.0.0.1:9/".to_string(), None)
        .unwrap()
        .search(&query, CancellationToken::new())
        .await
        .unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn search_401_maps_to_auth_failed() {
    let (url, _server) = spawn_one_shot_http(401, "Unauthorized", &[], "").await;
    let err = client(SAMPLE_DEF, url, None)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SearchIndexerError::AuthFailed { indexer_id: 1, .. }
    ));
}

#[tokio::test]
async fn search_429_maps_to_rate_limited() {
    let (url, _server) =
        spawn_one_shot_http(429, "Too Many Requests", &[("retry-after", "60")], "").await;
    let err = client(SAMPLE_DEF, url, None)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SearchIndexerError::RateLimited {
            retry_after_seconds: Some(60),
            ..
        }
    ));
}

#[tokio::test]
async fn search_cancelled_returns_cancelled() {
    let (url, server) = crate::test_support::spawn_hang_http().await;
    let ct = CancellationToken::new();
    ct.cancel();
    let err = client(SAMPLE_DEF, url, None)
        .unwrap()
        .search(&movie_query(), ct)
        .await
        .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::Cancelled { .. }),
        "got {err:?}"
    );
    server.abort();
}

const COOKIE_DEF_SUFFIX: &str = r#"
login:
  method: cookie
  test:
    path: /profile
"#;

#[tokio::test]
async fn cookie_login_sends_cookie_header() {
    let yaml = format!("{SAMPLE_DEF}{COOKIE_DEF_SUFFIX}");
    let (url, server) = spawn_one_shot_http(200, "OK", &[], SAMPLE_HTML).await;
    client(&yaml, url, Some("uid=1; pass=abc"))
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();
    let request_head = server.await.unwrap().to_lowercase();
    assert!(
        request_head.contains("cookie: uid=1; pass=abc"),
        "got: {request_head}"
    );
}

#[test]
fn cookie_login_without_api_key_errors() {
    let yaml = format!("{SAMPLE_DEF}{COOKIE_DEF_SUFFIX}");
    let err = client(&yaml, "http://127.0.0.1:9/".to_string(), None).unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::CookieAuthRequired { .. }),
        "got {err:?}"
    );
}

#[test]
fn unknown_login_method_is_unsupported_at_construction() {
    let yaml = format!("{SAMPLE_DEF}\nlogin:\n  method: oneurl\n");
    let err = client(&yaml, "http://127.0.0.1:9/".to_string(), None).unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::LoginUnsupported { ref method, .. } if method == "oneurl"),
        "got {err:?}"
    );
}

// ── interactive login (form / post / get) ──────────────────────────────────

const FORM_LOGIN_DEF: &str = r#"---
id: form-tracker
name: Form Tracker
links:
  - https://form-tracker.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
  modes:
    search: [q]
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
    - selector: span.error
  test:
    path: /profile
    selector: a.logout
search:
  paths:
    - path: /browse
  inputs:
    q: "{{ .Keywords }}"
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    download:
      selector: a.dl
      attribute: href
"#;

const POST_LOGIN_DEF: &str = r#"---
id: post-tracker
name: Post Tracker
links:
  - https://post-tracker.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
  modes:
    search: [q]
settings:
  - name: username
    type: text
  - name: password
    type: password
login:
  method: post
  path: /takelogin.php
  inputs:
    username: "{{ .Config.username }}"
    password: "{{ .Config.password }}"
search:
  paths:
    - path: /browse
  inputs:
    q: "{{ .Keywords }}"
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    download:
      selector: a.dl
      attribute: href
"#;

const FORM_LOGIN_PAGE: &str = r#"<html><body>
<form id="login" method="post" action="/login.php">
<input type="hidden" name="csrf_token" value="tok-42">
<input type="text" name="username" value="">
<input type="password" name="password" value="">
</form>
</body></html>"#;

// A definition whose login.path is an absolute off-host URL — the same-host
// gate must refuse it at construction, before any network request.
const FORM_LOGIN_OFFHOST_PATH_DEF: &str = r#"---
id: offhost-tracker
name: Offhost Tracker
links:
  - https://offhost-tracker.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
  modes:
    search: [q]
settings:
  - name: username
    type: text
  - name: password
    type: password
login:
  method: form
  path: http://evil.example/harvest
  inputs:
    username: "{{ .Config.username }}"
    password: "{{ .Config.password }}"
search:
  paths:
    - path: /browse
  inputs:
    q: "{{ .Keywords }}"
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    download:
      selector: a.dl
      attribute: href
"#;

// A form definition with NO login.error and NO login.test — the only success
// signal is the cookie jar, which is exactly the case a pre-credential
// anonymous cookie must not be allowed to satisfy.
const FORM_LOGIN_NO_VERIFY_DEF: &str = r#"---
id: noverify-tracker
name: Noverify Tracker
links:
  - https://noverify-tracker.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
  modes:
    search: [q]
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
search:
  paths:
    - path: /browse
  inputs:
    q: "{{ .Keywords }}"
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    download:
      selector: a.dl
      attribute: href
"#;

const SEARCH_ROWS_HTML: &str = r#"<html><body><table id="torrents"><tbody>
<tr><td><a class="title" href="/d/1">Result.One</a></td>
<td><a class="dl" href="/dl/1.torrent">DL</a></td></tr>
</tbody></table></body></html>"#;

const LOGGED_IN_HTML: &str =
    r#"<html><body><a class="logout" href="/logout">Logout</a></body></html>"#;

fn header(name: &str, value: &str) -> (String, String) {
    (name.to_string(), value.to_string())
}

fn creds() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("username".to_string(), "alice".to_string()),
        ("password".to_string(), "secret".to_string()),
    ])
}

fn login_query() -> SearchQuery {
    SearchQuery {
        query_text: Some("hello".to_string()),
        limit: 100,
        ..Default::default()
    }
}

fn request_body(head: &str) -> &str {
    head.split("\r\n\r\n").nth(1).unwrap_or_default()
}

#[tokio::test]
async fn form_login_happy_path_harvests_csrf_and_carries_session() {
    let (url, server) = spawn_sequence_http(vec![
        // GET login page: hidden CSRF + a session cookie.
        (
            200,
            vec![header("set-cookie", "sess=abc; Path=/")],
            FORM_LOGIN_PAGE.to_string(),
        ),
        // POST credentials: success page + session cookie.
        (
            200,
            vec![header("set-cookie", "uid=42; Path=/")],
            "<html><body>ok</body></html>".to_string(),
        ),
        // login.test probe.
        (200, vec![], LOGGED_IN_HTML.to_string()),
        // search fetch.
        (200, vec![], SEARCH_ROWS_HTML.to_string()),
    ])
    .await;

    let results = client_with_settings(FORM_LOGIN_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    let heads = server.await.unwrap();
    assert_eq!(heads.len(), 4, "unexpected request count: {heads:?}");
    assert!(heads[0].starts_with("GET /login.php"), "got {}", heads[0]);

    let post = &heads[1];
    assert!(post.starts_with("POST /login.php"), "got {post}");
    let post_lower = post.to_lowercase();
    assert!(
        post_lower.contains("content-type: application/x-www-form-urlencoded"),
        "missing urlencoded content-type: {post}"
    );
    // WHY: the page-GET session cookie must ride the POST.
    assert!(
        post_lower.contains("cookie: sess=abc"),
        "page-GET cookie not sent on POST: {post}"
    );
    let body = request_body(post);
    assert!(
        body.contains("csrf_token=tok-42"),
        "CSRF not harvested: {body}"
    );
    assert!(body.contains("username=alice"), "got body: {body}");
    assert!(body.contains("password=secret"), "got body: {body}");

    assert!(heads[2].starts_with("GET /profile"), "got {}", heads[2]);

    let search = &heads[3];
    assert!(search.starts_with("GET /browse?q=hello"), "got {search}");
    // WHY: both the page cookie and the POST cookie authenticate the search.
    assert!(
        search.to_lowercase().contains("uid=42"),
        "session cookie missing: {search}"
    );
}

#[tokio::test]
async fn set_cookie_captured_on_302_hop() {
    let (url, server) = spawn_sequence_http(vec![
        // POST → 302 that itself carries the session cookie.
        (
            302,
            vec![
                header("location", "/index.php"),
                header("set-cookie", "uid=99; Path=/"),
            ],
            String::new(),
        ),
        // Redirect follow.
        (200, vec![], "<html><body>ok</body></html>".to_string()),
        // Search fetch.
        (200, vec![], SEARCH_ROWS_HTML.to_string()),
    ])
    .await;

    let results = client_with_settings(POST_LOGIN_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    let heads = server.await.unwrap();
    assert_eq!(heads.len(), 3, "{heads:?}");
    assert!(
        heads[0].starts_with("POST /takelogin.php"),
        "got {}",
        heads[0]
    );
    assert!(heads[1].starts_with("GET /index.php"), "got {}", heads[1]);
    assert!(
        heads[2].to_lowercase().contains("uid=99"),
        "cookie set on the 302 hop was lost: {}",
        heads[2]
    );
}

#[tokio::test]
async fn login_error_selector_reports_site_message_never_credentials() {
    let error_page =
        r#"<html><body><span class="error">Invalid username or password</span></body></html>"#;
    let (url, server) = spawn_sequence_http(vec![
        (
            200,
            vec![header("set-cookie", "sess=abc")],
            FORM_LOGIN_PAGE.to_string(),
        ),
        (200, vec![], error_page.to_string()),
    ])
    .await;

    let err = client_with_settings(FORM_LOGIN_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap_err();
    match err {
        SearchIndexerError::LoginFailed { reason, .. } => {
            assert!(
                reason.contains("Invalid username or password"),
                "site message missing: {reason}"
            );
            assert!(!reason.contains("secret"), "credentials leaked: {reason}");
        }
        other => panic!("expected LoginFailed, got {other:?}"),
    }
    let _ = server.await.unwrap();
}

#[tokio::test]
async fn foreign_host_form_action_refuses_and_leaks_no_request() {
    let page = r#"<html><body>
<form id="login" method="post" action="http://evil.example/steal">
<input type="hidden" name="csrf_token" value="x">
<input type="text" name="username" value="">
<input type="password" name="password" value="">
</form></body></html>"#;
    let (url, server) = spawn_sequence_http(vec![(
        200,
        vec![header("set-cookie", "sess=abc")],
        page.to_string(),
    )])
    .await;

    let err = client_with_settings(FORM_LOGIN_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::LoginFailed { .. }),
        "got {err:?}"
    );

    let heads = server.await.unwrap();
    // WHY: only the login-page GET — the credential POST never left for the
    // foreign host, proving the same-host exfiltration gate.
    assert_eq!(heads.len(), 1, "a request leaked off-origin: {heads:?}");
    assert!(heads[0].starts_with("GET /login.php"), "got {}", heads[0]);
    assert!(
        !heads.iter().any(|h| h.contains("evil.example")),
        "off-origin request leaked: {heads:?}"
    );
}

#[tokio::test]
async fn absolute_off_host_login_path_refused_at_construction() {
    // No server: the same-host gate must fire during construction, before any
    // request could leave for the foreign host.
    let err = client_with_settings(
        FORM_LOGIN_OFFHOST_PATH_DEF,
        "https://offhost-tracker.example".to_string(),
        None,
        creds(),
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionInvalid { .. }),
        "an off-host login.path must be refused at construction, got {err:?}"
    );
}

#[tokio::test]
async fn pre_credential_cookie_alone_is_not_login_success() {
    // GET /login.php sets an anonymous session cookie; the credential POST is
    // rejected and re-renders 200 with the SAME cookie and no error/test
    // markup. The pre-credential cookie must not be accepted as proof.
    let (url, server) = spawn_sequence_http(vec![
        (
            200,
            vec![header("set-cookie", "PHPSESSID=anon; Path=/")],
            FORM_LOGIN_PAGE.to_string(),
        ),
        // Rejected login: 200, same anonymous cookie re-set, no new session.
        (
            200,
            vec![header("set-cookie", "PHPSESSID=anon; Path=/")],
            "<html><body>wrong password</body></html>".to_string(),
        ),
    ])
    .await;

    let err = client_with_settings(FORM_LOGIN_NO_VERIFY_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::LoginFailed { .. }),
        "a login proven only by a pre-credential cookie must fail, got {err:?}"
    );
    let _ = server.await.unwrap();
}

#[tokio::test]
async fn credential_phase_cookie_confirms_login_without_test_block() {
    // Same no-verify definition, but the POST sets a NEW cookie — a genuine
    // post-authentication session — so the login is accepted and the search
    // proceeds on that cookie.
    let (url, server) = spawn_sequence_http(vec![
        (
            200,
            vec![header("set-cookie", "PHPSESSID=anon; Path=/")],
            FORM_LOGIN_PAGE.to_string(),
        ),
        (
            200,
            vec![header("set-cookie", "auth=live-42; Path=/")],
            "<html><body>welcome</body></html>".to_string(),
        ),
        (200, vec![], SEARCH_ROWS_HTML.to_string()),
    ])
    .await;

    let results = client_with_settings(FORM_LOGIN_NO_VERIFY_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    let heads = server.await.unwrap();
    let search = heads.last().unwrap().to_lowercase();
    assert!(
        search.contains("auth=live-42"),
        "search must carry the post-auth cookie: {search}"
    );
}

#[tokio::test]
async fn login_redirect_hop_cap_fails() {
    let mut responses = vec![(302, vec![header("location", "/r")], String::new())];
    for _ in 0..5 {
        responses.push((302, vec![header("location", "/r")], String::new()));
    }
    let (url, server) = spawn_sequence_http(responses).await;

    let err = client_with_settings(POST_LOGIN_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::LoginFailed { ref reason, .. } if reason.contains("redirect")),
        "got {err:?}"
    );
    let _ = server.await.unwrap();
}

#[tokio::test]
async fn post_login_sends_no_page_get() {
    let (url, server) = spawn_sequence_http(vec![
        (
            200,
            vec![header("set-cookie", "uid=1")],
            "<html>ok</html>".to_string(),
        ),
        (200, vec![], SEARCH_ROWS_HTML.to_string()),
    ])
    .await;

    let results = client_with_settings(POST_LOGIN_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    let heads = server.await.unwrap();
    // WHY: the post method skips the form-page GET entirely.
    assert_eq!(heads.len(), 2, "unexpected extra request: {heads:?}");
    assert!(
        heads[0].starts_with("POST /takelogin.php"),
        "first request must be the credential POST: {}",
        heads[0]
    );
    let body = request_body(&heads[0]);
    assert!(
        body.contains("username=alice") && body.contains("password=secret"),
        "got body: {body}"
    );
    assert!(heads[1].starts_with("GET /browse"), "got {}", heads[1]);
}

#[tokio::test]
async fn session_reused_across_two_searches() {
    let sessions = Arc::new(SessionStore::new());
    let (url, server) = spawn_sequence_http(vec![
        (
            200,
            vec![header("set-cookie", "uid=7")],
            "<html>ok</html>".to_string(),
        ),
        (200, vec![], SEARCH_ROWS_HTML.to_string()),
        (200, vec![], SEARCH_ROWS_HTML.to_string()),
    ])
    .await;

    for _ in 0..2 {
        client_with_sessions(
            POST_LOGIN_DEF,
            url.clone(),
            None,
            creds(),
            Arc::clone(&sessions),
        )
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap();
    }

    let heads = server.await.unwrap();
    assert_eq!(heads.len(), 3, "{heads:?}");
    let logins = heads.iter().filter(|h| h.starts_with("POST")).count();
    assert_eq!(logins, 1, "login ran more than once: {heads:?}");
}

#[tokio::test]
async fn stale_session_invalidates_relogs_and_retries_once() {
    let (url, server) = spawn_sequence_http(vec![
        // Initial login.
        (
            200,
            vec![header("set-cookie", "uid=1")],
            "<html>ok</html>".to_string(),
        ),
        // Search redirects to the login path (expired session).
        (
            302,
            vec![header("location", "/takelogin.php")],
            String::new(),
        ),
        // Auto-followed login page (shared client follows redirects).
        (200, vec![], "<html>login</html>".to_string()),
        // Re-login.
        (
            200,
            vec![header("set-cookie", "uid=2")],
            "<html>ok</html>".to_string(),
        ),
        // Retried search succeeds.
        (200, vec![], SEARCH_ROWS_HTML.to_string()),
    ])
    .await;

    let results = client_with_settings(POST_LOGIN_DEF, url, None, creds())
        .unwrap()
        .search(&login_query(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    let heads = server.await.unwrap();
    assert_eq!(heads.len(), 5, "{heads:?}");
    assert!(
        heads[0].starts_with("POST /takelogin.php"),
        "got {}",
        heads[0]
    );
    assert!(heads[1].starts_with("GET /browse"), "got {}", heads[1]);
    assert!(
        heads[2].starts_with("GET /takelogin.php"),
        "got {}",
        heads[2]
    );
    assert!(
        heads[3].starts_with("POST /takelogin.php"),
        "got {}",
        heads[3]
    );
    assert!(heads[4].starts_with("GET /browse"), "got {}", heads[4]);
    // WHY: the retry rides the fresh session cookie, not the stale one.
    assert!(
        heads[4].to_lowercase().contains("uid=2"),
        "retry did not use the fresh session: {}",
        heads[4]
    );
}

#[tokio::test]
async fn login_test_selector_drives_test_healthy() {
    let (url, server) = spawn_sequence_http(vec![
        (
            200,
            vec![header("set-cookie", "sess=abc")],
            FORM_LOGIN_PAGE.to_string(),
        ),
        (
            200,
            vec![header("set-cookie", "uid=1")],
            "<html>ok</html>".to_string(),
        ),
        // login.test during login().
        (200, vec![], LOGGED_IN_HTML.to_string()),
        // test() content assertion.
        (200, vec![], LOGGED_IN_HTML.to_string()),
    ])
    .await;

    let status = client_with_settings(FORM_LOGIN_DEF, url, None, creds())
        .unwrap()
        .test(CancellationToken::new())
        .await
        .unwrap();
    assert!(status.healthy, "error: {:?}", status.error);
    let _ = server.await.unwrap();
}

#[tokio::test]
async fn login_test_selector_absent_marks_unhealthy() {
    let (url, server) = spawn_sequence_http(vec![
        (
            200,
            vec![header("set-cookie", "sess=abc")],
            FORM_LOGIN_PAGE.to_string(),
        ),
        (
            200,
            vec![header("set-cookie", "uid=1")],
            "<html>ok</html>".to_string(),
        ),
        // login.test probe: no logout link → session not established.
        (
            200,
            vec![],
            "<html><body>no session here</body></html>".to_string(),
        ),
    ])
    .await;

    let status = client_with_settings(FORM_LOGIN_DEF, url, None, creds())
        .unwrap()
        .test(CancellationToken::new())
        .await
        .unwrap();
    assert!(!status.healthy);
    assert!(
        status
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("matched nothing"),
        "got {:?}",
        status.error
    );
    let _ = server.await.unwrap();
}

#[test]
fn cf_bypass_with_form_login_unsupported_at_construction() {
    let err = CardigannClient::new(
        Arc::new(SearchSubsystemConfig::default()),
        reqwest::Client::new(),
        Arc::new(NoProxy),
        Duration::from_secs(5),
        IndexerConfig {
            id: 1,
            name: "Test".to_string(),
            url: "https://form-tracker.example/".to_string(),
            api_key: None,
            cf_bypass: true,
            settings: creds(),
        },
        definition(FORM_LOGIN_DEF),
        Arc::new(SessionStore::new()),
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::DefinitionUnsupported { ref feature, .. } if feature.contains("cf_bypass")),
        "got {err:?}"
    );
}

#[test]
fn form_login_missing_credential_fails_construction() {
    // WHY: password unset → the .Config.password login input cannot resolve.
    let settings = BTreeMap::from([("username".to_string(), "alice".to_string())]);
    let err = client_with_settings(
        FORM_LOGIN_DEF,
        "https://form-tracker.example/".to_string(),
        None,
        settings,
    )
    .unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::SettingsInvalid { ref reason, .. } if reason.contains("password")),
        "got {err:?}"
    );
}

#[test]
fn debug_client_and_store_hide_credential_values() {
    let sessions = Arc::new(SessionStore::new());
    sessions.store(
        1,
        BTreeMap::from([("uid".to_string(), "supersecretcookievalue".to_string())]),
    );
    let client = client_with_sessions(
        FORM_LOGIN_DEF,
        "https://form-tracker.example/".to_string(),
        None,
        creds(),
        Arc::clone(&sessions),
    )
    .unwrap();

    let client_debug = format!("{client:?}");
    assert!(
        !client_debug.contains("secret"),
        "client Debug leaked credentials: {client_debug}"
    );

    let store_debug = format!("{sessions:?}");
    assert!(
        !store_debug.contains("supersecretcookievalue"),
        "store Debug leaked a cookie value: {store_debug}"
    );
    // WHY: cookie NAMES are safe to show and aid debugging.
    assert!(
        store_debug.contains("uid"),
        "cookie name not shown: {store_debug}"
    );
}

#[tokio::test]
async fn caps_derive_from_definition() {
    let caps = client(SAMPLE_DEF, "http://127.0.0.1:9/".to_string(), None)
        .unwrap()
        .caps(CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(caps.server.title.as_deref(), Some("Sample Tracker"));
    let available: Vec<&str> = caps
        .search_functions
        .iter()
        .filter(|f| f.available)
        .map(|f| f.function_type.as_str())
        .collect();
    assert_eq!(available, vec!["search", "tvsearch", "movie"]);
    let ids: Vec<u32> = caps.categories.iter().map(|c| c.id).collect();
    assert_eq!(ids, vec![2030, 2040, 5040]);
}

#[tokio::test]
async fn test_reports_healthy_on_success() {
    let yaml = format!("{SAMPLE_DEF}{COOKIE_DEF_SUFFIX}");
    let (url, server) = spawn_one_shot_http(200, "OK", &[], "<html/>").await;
    let status = client(&yaml, url, Some("uid=1"))
        .unwrap()
        .test(CancellationToken::new())
        .await
        .unwrap();
    assert!(status.healthy);
    assert!(status.caps.is_some());
    assert!(status.error.is_none());
    // WHY: the login test path must drive the probe, not the site root.
    let request_head = server.await.unwrap();
    assert!(
        request_head.starts_with("GET /profile"),
        "got: {request_head}"
    );
}

#[tokio::test]
async fn test_reports_unhealthy_on_server_error() {
    let (url, _server) = spawn_one_shot_http(500, "Internal Server Error", &[], "").await;
    let status = client(SAMPLE_DEF, url, None)
        .unwrap()
        .test(CancellationToken::new())
        .await
        .unwrap();
    assert!(!status.healthy);
    assert!(status.error.is_some());
}

#[tokio::test]
async fn download_magnet_short_circuits_without_network() {
    let magnet = "magnet:?xt=urn:btih:abc123";
    let response = client(SAMPLE_DEF, "http://127.0.0.1:9/".to_string(), None)
        .unwrap()
        .download(magnet, CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(response, DownloadResponse::MagnetUri(uri) if uri == magnet));
}

#[tokio::test]
async fn download_rejects_ssrf_targets() {
    for target in [
        "http://127.0.0.1:8080/steal",
        "http://169.254.169.254/latest/meta-data/",
        "file:///etc/passwd",
    ] {
        let err = client(SAMPLE_DEF, "http://127.0.0.1:9/".to_string(), None)
            .unwrap()
            .download(target, CancellationToken::new())
            .await
            .unwrap_err();
        assert!(
            matches!(err, SearchIndexerError::UnsafeUrl { .. }),
            "expected UnsafeUrl for {target}, got {err:?}"
        );
    }
}

#[tokio::test]
async fn fetch_bytes_returns_binary_body() {
    // WHY: torrent files are bencoded binary — the byte path must not
    // round-trip through UTF-8.
    let body: Vec<u8> = vec![0x64, 0x38, 0xFF, 0x80, 0x00, 0x65];
    let mut raw = format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    raw.extend_from_slice(&body);
    let (url, _server) = spawn_raw_http(raw).await;
    let bytes = client(SAMPLE_DEF, url.clone(), None)
        .unwrap()
        .fetch_bytes(&url, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(bytes.as_ref(), body.as_slice());
}

#[test]
fn base_url_prefers_http_indexer_url_and_falls_back_to_links() {
    let from_row = client(SAMPLE_DEF, "http://row.example/sub".to_string(), None).unwrap();
    assert_eq!(from_row.base_url.as_str(), "http://row.example/sub/");

    let from_links = client(SAMPLE_DEF, "sample-tracker".to_string(), None).unwrap();
    assert_eq!(
        from_links.base_url.as_str(),
        "https://sample-tracker.example/"
    );
}

// ── registry ──────────────────────────────────────────────────────────────

fn registry_with(files: &[(&str, &str)]) -> (CardigannRegistry, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    let config = SearchSubsystemConfig {
        cardigann_definitions_dir: Some(dir.path().to_path_buf()),
        ..SearchSubsystemConfig::default()
    };
    (CardigannRegistry::load(Arc::new(config)), dir)
}

#[test]
fn registry_loads_valid_and_skips_broken_definitions() {
    let (registry, _dir) = registry_with(&[
        ("sample.yml", SAMPLE_DEF),
        ("broken.yml", ": not yaml :"),
        ("ignored.txt", "not a definition"),
    ]);
    assert_eq!(registry.len(), 1);
    assert!(registry.resolve("sample-tracker").is_some());
}

#[test]
fn registry_resolves_by_id_and_by_link() {
    let (registry, _dir) = registry_with(&[("sample.yml", SAMPLE_DEF)]);
    assert!(registry.resolve("sample-tracker").is_some());
    assert!(
        registry
            .resolve("https://sample-tracker.example/")
            .is_some()
    );
    assert!(registry.resolve("https://sample-tracker.example").is_some());
    assert!(registry.resolve("https://other.example/").is_none());
    assert!(registry.resolve("unknown-id").is_none());
}

#[test]
fn registry_client_for_unknown_url_errors() {
    let (registry, _dir) = registry_with(&[("sample.yml", SAMPLE_DEF)]);
    let err = registry
        .client_for(
            IndexerConfig {
                id: 7,
                name: "Nope".to_string(),
                url: "no-such-definition".to_string(),
                api_key: None,
                cf_bypass: false,
                settings: BTreeMap::new(),
            },
            reqwest::Client::new(),
            Arc::new(NoProxy),
            Duration::from_secs(5),
            Arc::new(SessionStore::new()),
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            SearchIndexerError::DefinitionNotFound { indexer_id: 7, .. }
        ),
        "got {err:?}"
    );
}

#[test]
fn registry_without_configured_dir_is_empty() {
    let registry = CardigannRegistry::load(Arc::new(SearchSubsystemConfig::default()));
    assert!(registry.is_empty());
}

// ── template block constructs end to end (#513) ─────────────────────────

const BLOCK_DEF: &str = r#"---
id: block-tracker
name: Block Tracker
type: private
links:
  - https://block-tracker.example/
caps:
  categorymappings:
    - {id: 6, cat: Movies/HD}
    - {id: 7, cat: Movies/SD}
  modes:
    search: [q]
settings:
  - name: freeleech
    type: checkbox
    default: "false"
search:
  paths:
    - path: "{{ if .Keywords }}/search{{ else }}/browse{{ end }}"
  inputs:
    q: "{{ .Keywords }}"
    fl: "{{ if .Config.freeleech }}1{{ else }}0{{ end }}"
    $raw: "{{ range .Categories }}&cat[]={{ . }}{{ end }}"
  rows:
    selector: tr
  fields:
    name:
      selector: td.name
    year:
      selector: td.year
      optional: true
    title:
      text: "{{ if .Result.year }}{{ .Result.name }} ({{ .Result.year }}){{ else }}{{ .Result.name }}{{ end }}"
    download:
      selector: a
      attribute: href
"#;

const BLOCK_HTML: &str = r#"<html><body><table><tbody>
<tr><td class="name">Movie</td><td class="year">2024</td><td><a href="/dl/1.torrent">d</a></td></tr>
<tr><td class="name">Plain</td><td class="year"></td><td><a href="/dl/2.torrent">d</a></td></tr>
</tbody></table></body></html>"#;

#[tokio::test]
async fn search_supports_conditional_templates_range_and_result_fields() {
    let (url, server) = spawn_one_shot_http(200, "OK", &[], BLOCK_HTML).await;
    let results = client(BLOCK_DEF, url.clone(), None)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(results.len(), 2, "got {results:?}");
    assert_eq!(results[0].title, "Movie (2024)");
    assert_eq!(results[1].title, "Plain");

    let request_head = server.await.unwrap();
    let request_line = request_head.lines().next().unwrap_or_default().to_string();
    // `if .Keywords` chose the /search branch
    assert!(
        request_line.starts_with("GET /search?"),
        "got: {request_line}"
    );
    // checkbox default "false" reads as false -> the else branch renders 0
    assert!(request_line.contains("fl=0"), "got: {request_line}");
    // `range .Categories` repeats the body per site category id
    assert!(
        request_line.contains("cat[]=6&cat[]=7"),
        "got: {request_line}"
    );
}

#[tokio::test]
async fn settings_override_flips_conditional_input() {
    // WHY: the checkbox override flows settings -> config seed -> template
    // truthiness; an overridden "true" must take the if-branch.
    let (url, server) = spawn_one_shot_http(200, "OK", &[], BLOCK_HTML).await;
    let settings = BTreeMap::from([("freeleech".to_string(), "true".to_string())]);
    let results = client_with_settings(BLOCK_DEF, url.clone(), None, settings)
        .unwrap()
        .search(&movie_query(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(results.len(), 2);

    let request_head = server.await.unwrap();
    let request_line = request_head.lines().next().unwrap_or_default().to_string();
    assert!(request_line.contains("fl=1"), "got: {request_line}");
}
