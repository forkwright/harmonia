//! CardigannClient + CardigannRegistry behavior tests (mock HTTP).

use super::*;
use crate::cf_bypass::noop::NoProxy;
use crate::test_support::{spawn_one_shot_http, spawn_raw_http};

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

fn definition(yaml: &str) -> Arc<CardigannDefinition> {
    Arc::new(definition::parse_definition(yaml, "test").unwrap())
}

fn client(
    yaml: &str,
    url: String,
    api_key: Option<&str>,
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
        },
        definition(yaml),
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
    assert!(request_line.contains("sort=created"), "got: {request_line}");
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
fn form_login_is_unsupported_at_construction() {
    let yaml = format!("{SAMPLE_DEF}\nlogin:\n  method: form\n");
    let err = client(&yaml, "http://127.0.0.1:9/".to_string(), None).unwrap_err();
    assert!(
        matches!(err, SearchIndexerError::LoginUnsupported { ref method, .. } if method == "form"),
        "got {err:?}"
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
            },
            reqwest::Client::new(),
            Arc::new(NoProxy),
            Duration::from_secs(5),
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
