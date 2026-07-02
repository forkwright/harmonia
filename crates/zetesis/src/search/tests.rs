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
    }];

    let query = SearchQuery {
        media_type: SearchMediaType::Tv,
        ..Default::default()
    };

    let eligible = filter_by_capability(&indexers, &query);
    assert_eq!(eligible.len(), 1);
}

// ── handle_search_error / refresh_caps (live service over in-memory db) ──

use apotheke::migrate::MIGRATOR;
use themelion::create_event_bus;

use crate::cf_bypass::noop::NoProxy;
use crate::repo::InsertIndexerParams;
use crate::test_support::spawn_one_shot_http;

async fn make_service() -> (SearchIndexerService, SqlitePool) {
    make_service_with(SearchSubsystemConfig::default()).await
}

async fn make_service_with(config: SearchSubsystemConfig) -> (SearchIndexerService, SqlitePool) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let (event_tx, _) = create_event_bus(16);
    let service = SearchIndexerService::new(
        pool.clone(),
        pool.clone(),
        Arc::new(NoProxy),
        config,
        event_tx,
    );
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
