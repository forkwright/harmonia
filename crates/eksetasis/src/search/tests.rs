//! SearchIndexerService behavior tests (in-memory db + mock HTTP).

use super::*;
use crate::types::ReleaseProtocol;

fn make_result(
    title: &str,
    info_hash: Option<&str>,
    guid: Option<&str>,
    indexer_id: i64,
) -> SearchResult {
    SearchResult {
        title: title.to_string(),
        guid: guid.map(str::to_string),
        download_url: format!("https://example.com/{title}"),
        size_bytes: Some(1_000_000),
        seeders: Some(10),
        leechers: Some(2),
        info_hash: info_hash.map(str::to_string),
        category_id: Some(2000),
        publication_date: None,
        indexer_id,
        protocol: ReleaseProtocol::Torrent,
        download_volume_factor: 1.0,
        upload_volume_factor: 1.0,
        custom_attrs: HashMap::new(),
    }
}

#[test]
fn dedup_by_info_hash() {
    let results = vec![
        make_result("Release.A", Some("abc123"), None, 1),
        make_result("Release.A.dupe", Some("abc123"), None, 2),
        make_result("Release.B", Some("def456"), None, 1),
    ];

    let deduped = deduplicate(results);
    assert_eq!(deduped.len(), 2);
    assert_eq!(deduped[0].title, "Release.A");
    assert_eq!(deduped[1].title, "Release.B");
}

#[test]
fn dedup_by_guid() {
    let results = vec![
        make_result("NZB.A", None, Some("guid-1"), 1),
        make_result("NZB.A.dupe", None, Some("guid-1"), 2),
        make_result("NZB.B", None, Some("guid-2"), 1),
    ];

    let deduped = deduplicate(results);
    assert_eq!(deduped.len(), 2);
    assert_eq!(deduped[0].title, "NZB.A");
    assert_eq!(deduped[1].title, "NZB.B");
}

#[test]
fn dedup_registers_guid_of_hash_bearing_result() {
    // WHY: a result carrying both keys must register its guid too — a later
    // copy sharing only the guid must not slip past dedup.
    let results = vec![
        make_result("Release.A", Some("abc123"), Some("guid-1"), 1),
        make_result("Release.A.guid-dupe", None, Some("guid-1"), 2),
    ];

    let deduped = deduplicate(results);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].title, "Release.A");
}

#[test]
fn dedup_same_hash_different_guid_still_dedupes() {
    let results = vec![
        make_result("Release.A", Some("abc123"), Some("guid-1"), 1),
        make_result("Release.A.hash-dupe", Some("abc123"), Some("guid-2"), 2),
    ];

    let deduped = deduplicate(results);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].title, "Release.A");
}

#[test]
fn dedup_case_insensitive_hash() {
    let results = vec![
        make_result("Release.A", Some("ABC123"), None, 1),
        make_result("Release.A.dupe", Some("abc123"), None, 2),
    ];

    let deduped = deduplicate(results);
    assert_eq!(deduped.len(), 1);
}

#[test]
fn dedup_keeps_higher_priority() {
    let results = vec![
        make_result("Release.Priority1", Some("hash1"), None, 1),
        make_result("Release.Priority2", Some("hash1"), None, 2),
    ];

    let deduped = deduplicate(results);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].indexer_id, 1);
}

#[test]
fn dedup_no_hash_no_guid_keeps_all() {
    let results = vec![
        make_result("Release.A", None, None, 1),
        make_result("Release.B", None, None, 2),
    ];

    let deduped = deduplicate(results);
    assert_eq!(deduped.len(), 2);
}

#[test]
fn filter_capability_any_includes_all() {
    let indexers = vec![IndexerRow {
        id: 1,
        name: "Test1".to_string(),
        url: "https://example.com/api".to_string(),
        protocol: "torznab".to_string(),
        api_key: None,
        enabled: true,
        cf_bypass: false,
        status: "active".to_string(),
        last_tested: None,
        caps_json: None,
        priority: 50,
        added_at: "2024-01-01T00:00:00Z".to_string(),
        settings_json: None,
    }];

    let query = SearchQuery {
        media_type: SearchMediaType::Any,
        ..Default::default()
    };

    let eligible = filter_by_capability(&indexers, &query);
    assert_eq!(eligible.len(), 1);
}

#[test]
fn filter_capability_typed_excludes_no_caps() {
    let indexers = vec![IndexerRow {
        id: 1,
        name: "NoCaps".to_string(),
        url: "https://example.com/api".to_string(),
        protocol: "torznab".to_string(),
        api_key: None,
        enabled: true,
        cf_bypass: false,
        status: "active".to_string(),
        last_tested: None,
        caps_json: None,
        priority: 50,
        added_at: "2024-01-01T00:00:00Z".to_string(),
        settings_json: None,
    }];

    let query = SearchQuery {
        media_type: SearchMediaType::Tv,
        ..Default::default()
    };

    let eligible = filter_by_capability(&indexers, &query);
    assert!(eligible.is_empty());
}

#[test]
fn filter_capability_typed_includes_supported() {
    let caps = IndexerCaps {
        server: crate::types::ServerInfo {
            title: None,
            version: None,
        },
        limits: crate::types::SearchLimits::default(),
        search_functions: vec![crate::types::SearchFunction {
            function_type: "tvsearch".to_string(),
            available: true,
        }],
        categories: vec![],
    };

    let indexers = vec![IndexerRow {
        id: 1,
        name: "TVIndexer".to_string(),
        url: "https://example.com/api".to_string(),
        protocol: "torznab".to_string(),
        api_key: None,
        enabled: true,
        cf_bypass: false,
        status: "active".to_string(),
        last_tested: None,
        caps_json: Some(serde_json::to_string(&caps).unwrap()),
        priority: 50,
        added_at: "2024-01-01T00:00:00Z".to_string(),
        settings_json: None,
    }];

    let query = SearchQuery {
        media_type: SearchMediaType::Tv,
        ..Default::default()
    };

    let eligible = filter_by_capability(&indexers, &query);
    assert_eq!(eligible.len(), 1);
}

// ── handle_search_error / refresh_caps (live service over in-memory db) ──

use aggelmata::create_event_bus;
use apotheke::migrate::MIGRATOR;

use crate::cf_bypass::noop::NoProxy;
use crate::repo::InsertIndexerParams;
use crate::test_support::spawn_one_shot_http;

async fn make_service() -> (SearchIndexerService, SqlitePool) {
    make_service_with(SearchSubsystemConfig::default()).await
}

async fn make_service_with(config: SearchSubsystemConfig) -> (SearchIndexerService, SqlitePool) {
    make_service_with_section(horismos::Section::fixed(config)).await
}

async fn make_service_with_section(
    config: horismos::Section<SearchSubsystemConfig>,
) -> (SearchIndexerService, SqlitePool) {
    make_service_with_section_and_proxy(config, Arc::new(NoProxy)).await
}

async fn make_service_with_section_and_proxy(
    config: horismos::Section<SearchSubsystemConfig>,
    cf_proxy: Arc<dyn crate::cf_bypass::CloudflareProxy>,
) -> (SearchIndexerService, SqlitePool) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let (event_tx, _) = create_event_bus(16);
    let service = SearchIndexerService::new(pool.clone(), pool.clone(), cf_proxy, config, event_tx);
    (service, pool)
}

async fn seed_indexer(pool: &SqlitePool, url: &str) -> IndexerRow {
    let id = repo::insert_indexer(
        pool,
        InsertIndexerParams {
            name: "Seeded",
            url,
            protocol: "torznab",
            api_key: Some("key"),
            cf_bypass: false,
            priority: 50,
            settings_json: None,
        },
    )
    .await
    .unwrap();
    repo::get_indexer(pool, id).await.unwrap().unwrap()
}

fn cancelled_error() -> SearchIndexerError {
    SearchIndexerError::Cancelled {
        url: "https://example.com/api".to_string(),
        location: snafu::Location::new(file!(), line!(), column!()),
    }
}

fn rate_limited_error(retry_after_seconds: Option<u64>) -> SearchIndexerError {
    SearchIndexerError::RateLimited {
        indexer_id: 1,
        retry_after_seconds,
        location: snafu::Location::new(file!(), line!(), column!()),
    }
}

#[tokio::test]
async fn handle_search_error_cancelled_leaves_status_unchanged() {
    let (service, pool) = make_service().await;
    let indexer = seed_indexer(&pool, "https://example.com/api").await;
    assert_eq!(indexer.status, "active");

    service
        .handle_search_error(&indexer, &cancelled_error())
        .await;

    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert_eq!(row.status, "active");
}

#[tokio::test]
async fn handle_search_error_auth_failed_marks_failed() {
    let (service, pool) = make_service().await;
    let indexer = seed_indexer(&pool, "https://example.com/api").await;

    let error = SearchIndexerError::AuthFailed {
        indexer_id: indexer.id,
        location: snafu::Location::new(file!(), line!(), column!()),
    };
    service.handle_search_error(&indexer, &error).await;

    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert_eq!(row.status, "failed");
}

#[tokio::test]
async fn handle_search_error_parse_response_marks_degraded() {
    let (service, pool) = make_service().await;
    let indexer = seed_indexer(&pool, "https://example.com/api").await;

    let error = SearchIndexerError::ParseResponse {
        url: "https://example.com/api".to_string(),
        error: "bad xml".to_string(),
        location: snafu::Location::new(file!(), line!(), column!()),
    };
    service.handle_search_error(&indexer, &error).await;

    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert_eq!(row.status, "degraded");
}

#[tokio::test]
async fn handle_search_error_http_request_active_marks_degraded() {
    let (service, pool) = make_service().await;
    let indexer = seed_indexer(&pool, "https://example.com/api").await;
    assert_eq!(indexer.status, "active");

    let error = SearchIndexerError::HttpRequest {
        url: "https://example.com/api".to_string(),
        source: reqwest::Client::new()
            .get("http://127.0.0.1:9/")
            .send()
            .await
            .unwrap_err(),
        location: snafu::Location::new(file!(), line!(), column!()),
    };
    service.handle_search_error(&indexer, &error).await;

    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert_eq!(row.status, "degraded");
}

#[tokio::test]
async fn handle_search_error_http_request_degraded_escalates_to_failed() {
    let (service, pool) = make_service().await;
    let mut indexer = seed_indexer(&pool, "https://example.com/api").await;
    repo::update_indexer_status(&pool, indexer.id, "degraded")
        .await
        .unwrap();
    indexer.status = "degraded".to_string();

    let error = SearchIndexerError::HttpRequest {
        url: "https://example.com/api".to_string(),
        source: reqwest::Client::new()
            .get("http://127.0.0.1:9/")
            .send()
            .await
            .unwrap_err(),
        location: snafu::Location::new(file!(), line!(), column!()),
    };
    service.handle_search_error(&indexer, &error).await;

    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert_eq!(row.status, "failed");
}

// WHY: the clock is paused only around the limiter interaction — sqlx's
// sqlite worker runs on a real thread, and a paused clock during pool
// setup auto-advances straight into PoolTimedOut.
#[tokio::test]
async fn handle_search_error_rate_limited_engages_retry_after() {
    let (service, pool) = make_service().await;
    let indexer = seed_indexer(&pool, "https://example.com/api").await;

    tokio::time::pause();
    service
        .handle_search_error(&indexer, &rate_limited_error(Some(120)))
        .await;

    let before = tokio::time::Instant::now();
    assert!(
        service
            .rate_limiter
            .acquire(indexer.id, &CancellationToken::new())
            .await
    );
    let elapsed = before.elapsed();
    tokio::time::resume();
    assert!(
        elapsed >= Duration::from_secs(120),
        "expected >=120s back-off, got {elapsed:?}"
    );

    // 429 is back-pressure, not indexer failure — status must not change.
    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert_eq!(row.status, "active");
}

#[tokio::test]
async fn handle_search_error_rate_limited_without_header_uses_default() {
    let (service, pool) = make_service().await;
    let indexer = seed_indexer(&pool, "https://example.com/api").await;

    tokio::time::pause();
    service
        .handle_search_error(&indexer, &rate_limited_error(None))
        .await;

    let before = tokio::time::Instant::now();
    assert!(
        service
            .rate_limiter
            .acquire(indexer.id, &CancellationToken::new())
            .await
    );
    let elapsed = before.elapsed();
    tokio::time::resume();
    assert!(
        elapsed >= Duration::from_secs(DEFAULT_RETRY_AFTER_SECS),
        "expected >={DEFAULT_RETRY_AFTER_SECS}s default back-off, got {elapsed:?}"
    );
}

#[tokio::test]
async fn handle_search_error_rate_limited_clamps_hostile_retry_after() {
    let (service, pool) = make_service().await;
    let indexer = seed_indexer(&pool, "https://example.com/api").await;

    tokio::time::pause();
    service
        .handle_search_error(&indexer, &rate_limited_error(Some(999_999)))
        .await;

    let before = tokio::time::Instant::now();
    assert!(
        service
            .rate_limiter
            .acquire(indexer.id, &CancellationToken::new())
            .await
    );
    let elapsed = before.elapsed();
    tokio::time::resume();
    assert!(
        elapsed >= Duration::from_secs(MAX_RETRY_AFTER_SECS),
        "expected the cap to engage, got {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(MAX_RETRY_AFTER_SECS + 60),
        "expected clamp at {MAX_RETRY_AFTER_SECS}s, got {elapsed:?}"
    );
}

// ── cardigann dispatch (definitions dir → make_client → search) ──────────

fn cardigann_definition_yaml(link: &str) -> String {
    format!(
        r#"---
id: sample-tracker
name: Sample Tracker
links:
  - {link}/
caps:
  categorymappings:
    - {{id: 6, cat: Movies/HD}}
  modes:
    search: [q]
search:
  paths:
    - path: /browse
  inputs:
    q: "{{{{ .Keywords }}}}"
  rows:
    selector: table#torrents > tbody > tr
  fields:
    title:
      selector: a.title
    download:
      selector: a.dl
      attribute: href
    size:
      selector: td.size
    seeders:
      selector: td.seeds
"#
    )
}

const CARDIGANN_HTML: &str = r#"<html><body><table id="torrents"><tbody>
<tr><td><a class="title" href="/d/1">Cardigann.Result.One</a></td>
<td><a class="dl" href="/dl/1.torrent">DL</a></td>
<td class="size">700 MB</td><td class="seeds">5</td></tr>
<tr><td><a class="title" href="/d/2">Cardigann.Result.Two</a></td>
<td><a class="dl" href="/dl/2.torrent">DL</a></td>
<td class="size">1 GB</td><td class="seeds">9</td></tr>
</tbody></table></body></html>"#;

#[tokio::test]
async fn cardigann_search_end_to_end_through_dispatch() {
    let (url, server) = spawn_one_shot_http(200, "OK", &[], CARDIGANN_HTML).await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("sample-tracker.yml"),
        cardigann_definition_yaml(&url),
    )
    .unwrap();

    let (service, pool) = make_service_with(SearchSubsystemConfig {
        cardigann_definitions_dir: Some(dir.path().to_path_buf()),
        ..SearchSubsystemConfig::default()
    })
    .await;
    let indexer_id = repo::insert_indexer(
        &pool,
        InsertIndexerParams {
            name: "Cardigann Sample",
            // WHY: the url column carries the definition id — base URL
            // resolution must fall back to the definition's links.
            url: "sample-tracker",
            protocol: "cardigann",
            api_key: None,
            cf_bypass: false,
            priority: 50,
            settings_json: None,
        },
    )
    .await
    .unwrap();

    let query = SearchQuery {
        query_text: Some("hello".to_string()),
        limit: 100,
        ..Default::default()
    };
    let results = service
        .search(query, CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].title, "Cardigann.Result.One");
    assert_eq!(results[0].download_url, format!("{url}/dl/1.torrent"));
    assert_eq!(results[0].size_bytes, Some(700 * 1024 * 1024));
    assert_eq!(results[0].seeders, Some(5));
    assert_eq!(results[0].indexer_id, indexer_id);
    assert_eq!(results[1].title, "Cardigann.Result.Two");

    let request_head = server.await.unwrap();
    assert!(
        request_head.starts_with("GET /browse?q=hello"),
        "got: {request_head}"
    );
}

// ── #608: enqueue-by-reference — results cache end-to-end through a live
// credentialed Torznab search ──────────────────────────────────────────

const CREDENTIALED_FEED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Private Tracker</title>
    <item>
      <title>Credentialed.Release.2024</title>
      <guid>cred-guid-1</guid>
      <size>734003200</size>
      <link>https://private-tracker.example/dl/42?apikey=SECRET</link>
      <torznab:attr name="seeders" value="10"/>
      <torznab:attr name="infohash" value="deadbeefcred"/>
    </item>
  </channel>
</rss>"#;

#[tokio::test]
async fn resolve_release_returns_the_exact_credentialed_url_after_search() {
    let (service, pool) = make_service().await;
    let (mock_url, _server) = spawn_one_shot_http(200, "OK", &[], CREDENTIALED_FEED_XML).await;
    seed_indexer(&pool, &mock_url).await;

    let outcome = service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 1);
    let release_id = *outcome.results[0].release_id.as_uuid();

    let resolved = service
        .resolve_release(release_id)
        .expect("a release from a just-completed search must resolve");
    assert_eq!(
        resolved.download_url,
        "https://private-tracker.example/dl/42?apikey=SECRET"
    );

    let cached = service
        .cached_results(outcome.query_id)
        .expect("cached_results must return the same completed search");
    assert_eq!(cached.results.len(), 1);
    assert_eq!(
        cached.results[0].release_id.as_uuid(),
        outcome.results[0].release_id.as_uuid()
    );
}

#[tokio::test]
async fn zero_ttl_config_makes_resolve_release_an_immediate_miss() {
    let (service, pool) = make_service_with(SearchSubsystemConfig {
        result_cache_ttl_seconds: 0,
        ..SearchSubsystemConfig::default()
    })
    .await;
    let (mock_url, _server) = spawn_one_shot_http(200, "OK", &[], CREDENTIALED_FEED_XML).await;
    seed_indexer(&pool, &mock_url).await;

    let outcome = service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();
    let release_id = *outcome.results[0].release_id.as_uuid();

    assert!(service.cached_results(outcome.query_id).is_none());
    assert!(service.resolve_release(release_id).is_none());
}

#[tokio::test]
async fn cardigann_row_without_definition_marks_failed() {
    let (service, pool) = make_service().await;
    let indexer_id = repo::insert_indexer(
        &pool,
        InsertIndexerParams {
            name: "Ghost",
            url: "no-such-definition",
            protocol: "cardigann",
            api_key: None,
            cf_bypass: false,
            priority: 50,
            settings_json: None,
        },
    )
    .await
    .unwrap();

    let results = service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();
    assert!(results.is_empty());

    let row = repo::get_indexer(&pool, indexer_id).await.unwrap().unwrap();
    assert_eq!(row.status, "failed");
}

#[tokio::test]
async fn corrupt_settings_json_is_a_hard_error_not_silent_defaults() {
    // WHY: a corrupt settings_json blob must fail loud at make_client — it
    // must never silently fall back to the definition's plain defaults.
    let (service, pool) = make_service().await;
    let indexer_id = repo::insert_indexer(
        &pool,
        InsertIndexerParams {
            name: "Corrupt Settings",
            url: "https://example.com/api",
            protocol: "torznab",
            api_key: None,
            cf_bypass: false,
            priority: 50,
            settings_json: Some("not json"),
        },
    )
    .await
    .unwrap();

    let results = service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();
    assert!(results.is_empty());

    let row = repo::get_indexer(&pool, indexer_id).await.unwrap().unwrap();
    assert_eq!(row.status, "failed");
}

// ── #575: per-indexer result cap (`max_results_per_indexer`) ──────────────

const TWO_ITEM_FEED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Test Indexer</title>
    <item>
      <title>Release.One.2024</title>
      <guid>guid-one</guid>
      <size>1000</size>
      <link>https://example.com/download/one</link>
      <torznab:attr name="infohash" value="aaa111"/>
    </item>
    <item>
      <title>Release.Two.2024</title>
      <guid>guid-two</guid>
      <size>2000</size>
      <link>https://example.com/download/two</link>
      <torznab:attr name="infohash" value="bbb222"/>
    </item>
  </channel>
</rss>"#;

#[tokio::test]
async fn search_truncates_each_indexer_to_max_results_per_indexer() {
    let (service, pool) = make_service_with(SearchSubsystemConfig {
        max_results_per_indexer: 1,
        ..SearchSubsystemConfig::default()
    })
    .await;
    let (url, _server) = spawn_one_shot_http(200, "OK", &[], TWO_ITEM_FEED_XML).await;
    seed_indexer(&pool, &url).await;

    let results = service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(
        results.len(),
        1,
        "an indexer returning 2 results over a cap of 1 must contribute exactly 1"
    );
    assert_eq!(results[0].title, "Release.One.2024");
}

const CAPS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<caps>
  <server title="Test Indexer" version="1.0"/>
  <limits default="100" max="500"/>
  <searching>
    <search available="yes"/>
  </searching>
  <categories>
    <category id="2000" name="Movies">
      <subcat id="2010" name="Movies/Foreign"/>
    </category>
  </categories>
</caps>"#;

#[tokio::test]
async fn refresh_caps_commits_caps_categories_and_status_together() {
    let (service, pool) = make_service().await;
    let (url, _server) = spawn_one_shot_http(200, "OK", &[], CAPS_XML).await;
    let indexer = seed_indexer(&pool, &url).await;
    repo::update_indexer_status(&pool, indexer.id, "degraded")
        .await
        .unwrap();

    let caps = service
        .refresh_caps(indexer.id, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(caps.categories.len(), 1);

    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert_eq!(row.status, "active");
    assert!(row.caps_json.is_some());
    assert!(row.last_tested.is_some());

    let categories = sqlx::query_as::<_, (i64, String)>(
        "SELECT category_id, name FROM indexer_categories
             WHERE indexer_id = ? ORDER BY category_id",
    )
    .bind(indexer.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        categories,
        vec![
            (2000, "Movies".to_string()),
            (2010, "Movies/Foreign".to_string())
        ]
    );
}

// ── #575: scheduled caps refresh (`caps_refresh_hours`) ───────────────────

#[test]
fn caps_stale_when_never_tested() {
    assert!(caps_stale(
        None,
        Duration::from_secs(24 * 3600),
        jiff::Timestamp::now()
    ));
}

#[test]
fn caps_stale_when_older_than_max_age() {
    assert!(caps_stale(
        Some("2020-01-01T00:00:00Z"),
        Duration::from_secs(24 * 3600),
        jiff::Timestamp::now()
    ));
}

#[test]
fn caps_fresh_when_within_max_age() {
    let recent = jiff::Timestamp::now().to_string();
    assert!(!caps_stale(
        Some(&recent),
        Duration::from_secs(24 * 3600),
        jiff::Timestamp::now()
    ));
}

#[test]
fn caps_stale_when_timestamp_unparseable() {
    assert!(caps_stale(
        Some("not-a-timestamp"),
        Duration::from_secs(24 * 3600),
        jiff::Timestamp::now()
    ));
}

#[test]
fn caps_fresh_when_timestamp_in_future() {
    // WHY: clock skew — a future observation must not wrap into "stale".
    let future = (jiff::Timestamp::now() + jiff::SignedDuration::from_secs(600)).to_string();
    assert!(!caps_stale(
        Some(&future),
        Duration::from_secs(24 * 3600),
        jiff::Timestamp::now()
    ));
}

#[tokio::test]
async fn refresh_stale_caps_refreshes_stale_and_skips_fresh() {
    let (service, pool) = make_service().await;
    // WHY: BOTH indexers get a live caps server — if the fresh one were
    // wrongly swept, its refresh would succeed and the assertions below
    // would catch it (rather than masking the bug behind a fetch failure).
    let (stale_url, _stale_server) = spawn_one_shot_http(200, "OK", &[], CAPS_XML).await;
    let (fresh_url, _fresh_server) = spawn_one_shot_http(200, "OK", &[], CAPS_XML).await;
    let stale = seed_indexer(&pool, &stale_url).await;
    let fresh = seed_indexer(&pool, &fresh_url).await;
    repo::update_indexer_caps(&pool, stale.id, "{}", "2020-01-01T00:00:00Z")
        .await
        .unwrap();
    let fresh_stamp = jiff::Timestamp::now().to_string();
    repo::update_indexer_caps(&pool, fresh.id, "{}", &fresh_stamp)
        .await
        .unwrap();

    let refreshed = service
        .refresh_stale_caps(Duration::from_secs(24 * 3600), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(refreshed, vec![stale.id]);
    let stale_row = repo::get_indexer(&pool, stale.id).await.unwrap().unwrap();
    assert_ne!(
        stale_row.last_tested.as_deref(),
        Some("2020-01-01T00:00:00Z"),
        "the stale indexer's caps observation must advance"
    );
    let fresh_row = repo::get_indexer(&pool, fresh.id).await.unwrap().unwrap();
    assert_eq!(
        fresh_row.last_tested.as_deref(),
        Some(fresh_stamp.as_str()),
        "the fresh indexer must be skipped untouched"
    );
}

#[tokio::test]
async fn refresh_stale_caps_treats_never_tested_as_stale() {
    let (service, pool) = make_service().await;
    let (url, _server) = spawn_one_shot_http(200, "OK", &[], CAPS_XML).await;
    let indexer = seed_indexer(&pool, &url).await;
    assert!(indexer.last_tested.is_none());

    let refreshed = service
        .refresh_stale_caps(Duration::from_secs(24 * 3600), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(refreshed, vec![indexer.id]);
    let row = repo::get_indexer(&pool, indexer.id).await.unwrap().unwrap();
    assert!(row.last_tested.is_some());
    assert!(row.caps_json.is_some());
}

#[tokio::test]
async fn refresh_stale_caps_continues_past_a_failing_indexer() {
    let (service, pool) = make_service().await;
    // WHY: priority order puts the unreachable indexer FIRST — the sweep
    // must log-and-continue, not abort before reaching the healthy one.
    let broken = repo::insert_indexer(
        &pool,
        InsertIndexerParams {
            name: "Broken",
            url: "http://127.0.0.1:9/",
            protocol: "torznab",
            api_key: None,
            cf_bypass: false,
            priority: 10,
            settings_json: None,
        },
    )
    .await
    .unwrap();
    let (url, _server) = spawn_one_shot_http(200, "OK", &[], CAPS_XML).await;
    let healthy = repo::insert_indexer(
        &pool,
        InsertIndexerParams {
            name: "Healthy",
            url: &url,
            protocol: "torznab",
            api_key: None,
            cf_bypass: false,
            priority: 90,
            settings_json: None,
        },
    )
    .await
    .unwrap();

    let refreshed = service
        .refresh_stale_caps(Duration::from_secs(24 * 3600), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(refreshed, vec![healthy]);
    let broken_row = repo::get_indexer(&pool, broken).await.unwrap().unwrap();
    assert!(
        broken_row.last_tested.is_none(),
        "a failed refresh must leave the indexer stale for the next tick"
    );
}

// ── #529 step 7: live `Section`, cf-proxy swap ────────────────────────────

use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::cf_bypass::{CloudflareProxy, ProxyResponse};

fn valid_horismos_config() -> horismos::Config {
    let mut config = horismos::Config::default();
    // WHY: validate_config rejects a short/placeholder jwt_secret; nothing
    // else under test touches exousia.
    config.exousia.jwt_secret = "test-secret-that-is-long-enough-for-hs256".to_string();
    config
}

/// A TCP server that records the number of connections concurrently
/// in-flight (peak), holds each connection briefly to create overlap
/// opportunity, then answers with a minimal valid torznab feed.
async fn spawn_concurrency_probe(
    concurrent: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let now = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
        peak.fetch_max(now, Ordering::SeqCst);

        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = stream.read(&mut chunk).await.unwrap();
            buf.extend_from_slice(&chunk[..n]);
            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        // WHY: hold the connection open briefly so overlapping fan-out
        // requests genuinely race — without this, sequential dispatch and
        // parallel dispatch are indistinguishable at this timescale.
        tokio::time::sleep(Duration::from_millis(80)).await;
        concurrent.fetch_sub(1, Ordering::SeqCst);

        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel><title>Probe</title></channel>
</rss>"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.ok();
        stream.flush().await.ok();
        stream.shutdown().await.ok();
    });
    (url, handle)
}

#[tokio::test]
async fn replace_lowering_max_concurrent_searches_bounds_the_next_fan_out() {
    let concurrent = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    let mut initial = valid_horismos_config();
    initial.zetesis.max_concurrent_searches = 4;
    let (manager, handle) = horismos::ConfigManager::new(
        initial,
        std::path::PathBuf::from("unused.toml"),
        horismos::ConfigOverrides::default(),
    );
    let (service, pool) = make_service_with_section(handle.section(|c| &c.zetesis)).await;

    let mut urls = Vec::new();
    let mut probe_handles = Vec::new();
    for _ in 0..3 {
        let (url, h) = spawn_concurrency_probe(Arc::clone(&concurrent), Arc::clone(&peak)).await;
        urls.push(url);
        probe_handles.push(h);
    }
    for url in &urls {
        seed_indexer(&pool, url).await;
    }

    // Lower the bound to 1 BEFORE the search fan-out runs.
    let mut lowered = valid_horismos_config();
    lowered.zetesis.max_concurrent_searches = 1;
    manager.replace(lowered).unwrap();

    service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();

    for h in probe_handles {
        h.await.unwrap();
    }

    assert_eq!(
        peak.load(Ordering::SeqCst),
        1,
        "max_concurrent_searches = 1 must serialize the fan-out across all 3 indexers"
    );
}

struct StubCfProxy {
    body: String,
    calls: AtomicUsize,
}

impl CloudflareProxy for StubCfProxy {
    fn get(
        &self,
        _url: &str,
        _ct: CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<ProxyResponse, SearchIndexerError>> + Send + '_,
        >,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body = self.body.clone();
        Box::pin(async move {
            Ok(ProxyResponse {
                status: 200,
                body,
                cookies: Vec::new(),
                user_agent: "stub-agent".to_string(),
            })
        })
    }
}

const CF_FEED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Test Indexer</title>
    <item>
      <title>Test.Release.2024.FLAC</title>
      <guid>abc123</guid>
      <size>734003200</size>
      <link>https://example.com/download/abc123</link>
      <torznab:attr name="seeders" value="42"/>
      <torznab:attr name="infohash" value="deadbeef"/>
    </item>
  </channel>
</rss>"#;

#[tokio::test]
async fn cf_proxy_swap_turns_a_no_proxy_service_into_a_proxied_one() {
    let (service, pool) = make_service_with_section_and_proxy(
        horismos::Section::fixed(SearchSubsystemConfig {
            cloudflare_bypass_enabled: true,
            cf_proxy_url: Some("https://cf-proxy.example".to_string()),
            ..SearchSubsystemConfig::default()
        }),
        Arc::new(NoProxy),
    )
    .await;

    let indexer_id = repo::insert_indexer(
        &pool,
        InsertIndexerParams {
            name: "CF Bypass",
            url: "https://cf-protected.example/api",
            protocol: "torznab",
            api_key: None,
            cf_bypass: true,
            priority: 50,
            settings_json: None,
        },
    )
    .await
    .unwrap();

    // Before the swap: NoProxy rejects every cf_bypass request — the search
    // must complete with zero results and the indexer marked degraded.
    let before = service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();
    assert!(before.is_empty(), "NoProxy must produce zero results");
    let row = repo::get_indexer(&pool, indexer_id).await.unwrap().unwrap();
    assert_eq!(row.status, "degraded");

    // Reset status so the post-swap search's outcome is unambiguous.
    repo::update_indexer_status(&pool, indexer_id, "active")
        .await
        .unwrap();

    let stub = Arc::new(StubCfProxy {
        body: CF_FEED_XML.to_string(),
        calls: AtomicUsize::new(0),
    });
    let proxy = Arc::clone(&stub) as Arc<dyn CloudflareProxy>;
    service.set_cf_proxy(proxy);

    let after = service
        .search(SearchQuery::new(), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(
        stub.calls.load(Ordering::SeqCst),
        1,
        "the stub proxy must be reached after the swap"
    );
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].title, "Test.Release.2024.FLAC");
    let row = repo::get_indexer(&pool, indexer_id).await.unwrap().unwrap();
    assert_eq!(row.status, "active");
}
