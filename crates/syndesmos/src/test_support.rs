//! Shared test fixtures — in-memory catalog seeding and a one-shot HTTP server.
#![cfg(test)]

use aggelmata::MediaId;
use sqlx::{Row, SqlitePool};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Creates an in-memory SQLite pool with all apotheke migrations applied.
pub(crate) async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    apotheke::run_migrations(&pool).await.unwrap();
    pool
}

/// Seeds a full track chain (release group, release, medium, track) with a
/// primary track artist, returning the track's `MediaId`.
pub(crate) async fn seed_scrobble_track(
    pool: &SqlitePool,
    artist: &str,
    title: &str,
    album: &str,
) -> MediaId {
    let track_id = seed_track_without_artist(pool, title, album).await;

    let artist_id = uuid::Uuid::now_v7().as_bytes().to_vec();
    apotheke::repo::registry::insert_registry_entry(
        pool,
        &apotheke::repo::registry::RegistryEntry {
            id: artist_id.clone(),
            entity_type: "person".to_string(),
            display_name: artist.to_string(),
            sort_name: None,
            created_at: now(),
            updated_at: now(),
        },
    )
    .await
    .unwrap();
    apotheke::repo::music::insert_track_artist(pool, track_id.as_bytes(), &artist_id, "primary")
        .await
        .unwrap();

    track_id
}

/// Seeds a track chain with no artist link, returning the track's `MediaId`.
pub(crate) async fn seed_track_without_artist(
    pool: &SqlitePool,
    title: &str,
    album: &str,
) -> MediaId {
    let group_id = uuid::Uuid::now_v7().as_bytes().to_vec();
    apotheke::repo::music::insert_release_group(
        pool,
        &apotheke::repo::music::MusicReleaseGroup {
            id: group_id.clone(),
            registry_id: None,
            title: album.to_string(),
            rg_type: "album".to_string(),
            mb_release_group_id: None,
            year: None,
            quality_profile_id: None,
            added_at: now(),
        },
    )
    .await
    .unwrap();

    let release_id = uuid::Uuid::now_v7().as_bytes().to_vec();
    apotheke::repo::music::insert_release(
        pool,
        &apotheke::repo::music::MusicRelease {
            id: release_id.clone(),
            release_group_id: group_id,
            title: album.to_string(),
            release_date: None,
            country: None,
            label: None,
            catalog_number: None,
            mb_release_id: None,
            added_at: now(),
        },
    )
    .await
    .unwrap();

    let medium_id = uuid::Uuid::now_v7().as_bytes().to_vec();
    apotheke::repo::music::insert_medium(
        pool,
        &apotheke::repo::music::MusicMedium {
            id: medium_id.clone(),
            release_id,
            position: 1,
            format: "Digital".to_string(),
            title: None,
        },
    )
    .await
    .unwrap();

    let track_id = MediaId::new();
    apotheke::repo::music::insert_track(
        pool,
        &apotheke::repo::music::MusicTrack {
            id: track_id.as_bytes().to_vec(),
            medium_id,
            position: 1,
            title: title.to_string(),
            duration_ms: None,
            mb_recording_id: None,
            acoustid_fingerprint: None,
            acoustid_id: None,
            file_path: None,
            file_size_bytes: None,
            bit_depth: None,
            sample_rate: None,
            codec: None,
            quality_score: None,
            replay_gain_track_db: None,
            replay_gain_album_db: None,
            source_type: "local".to_string(),
            added_at: now(),
        },
    )
    .await
    .unwrap();

    track_id
}

/// Seeds a want row persisted by a previous Tidal sync.
pub(crate) async fn seed_tidal_want(pool: &SqlitePool, source_ref: &str) {
    let profile_id: i64 =
        sqlx::query("SELECT id FROM quality_profiles WHERE media_type = 'music' LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap()
            .try_get("id")
            .unwrap();

    apotheke::repo::want::insert_want(
        pool,
        &apotheke::repo::want::Want {
            id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            media_type: "music_album".to_string(),
            title: format!("Tidal favorite {source_ref}"),
            registry_id: None,
            quality_profile_id: profile_id,
            status: "searching".to_string(),
            source: Some(crate::tidal::wantlist::TIDAL_WANT_SOURCE.to_string()),
            source_ref: Some(source_ref.to_string()),
            added_at: now(),
            fulfilled_at: None,
        },
    )
    .await
    .unwrap();
}

fn now() -> String {
    "2026-01-01T00:00:00Z".to_string()
}

/// Installs the process-wide rustls crypto provider for tests.
///
/// WHY: reqwest builds with `rustls-no-provider` (fleet convention: install
/// explicitly, never let a library link one implicitly — see main.rs), so
/// `reqwest::Client::new()`/`::builder().build()` panics ("No rustls crypto
/// provider is configured") in any process that never called
/// `install_default()` — and a nextest test binary never runs `main()`.
/// Safe to call repeatedly: install_default() on an already-installed
/// process just returns Err, discarded here.
pub(crate) fn install_test_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Spawns a TCP server that answers exactly one HTTP request with the given
/// status and body, then resolves to the raw request bytes it received.
pub(crate) async fn spawn_one_shot_http(
    status: u16,
    reason: &'static str,
    body: &'static str,
) -> (String, JoinHandle<String>) {
    install_test_crypto_provider();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];

        let header_end = loop {
            let n = stream.read(&mut chunk).await.unwrap();
            assert!(n > 0, "client closed before sending full headers");
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                break pos + 4;
            }
        };

        let content_length = parse_content_length(&buf[..header_end]);
        while buf.len() < header_end + content_length {
            let n = stream.read(&mut chunk).await.unwrap();
            assert!(n > 0, "client closed before sending full body");
            buf.extend_from_slice(&chunk[..n]);
        }

        let response = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len(),
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();

        String::from_utf8_lossy(&buf).into_owned()
    });

    (base_url, handle)
}

/// Spawns a TCP server that answers `responses.len()` sequential HTTP
/// requests (one connection each) with the given status and body, then
/// resolves to the raw request bytes it received, in order. The same URL is
/// returned twice — API base and auth base — so a client under test can
/// point both at this one server (epignosis's provider-test pattern).
pub(crate) async fn spawn_sequential_http(
    responses: Vec<(u16, String)>,
) -> (String, String, JoinHandle<Vec<String>>) {
    install_test_crypto_provider();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];

            let header_end = loop {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0, "client closed before sending full headers");
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                    break pos + 4;
                }
            };

            let content_length = parse_content_length(&buf[..header_end]);
            while buf.len() < header_end + content_length {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0, "client closed before sending full body");
                buf.extend_from_slice(&chunk[..n]);
            }

            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();

            requests.push(String::from_utf8_lossy(&buf).into_owned());
        }
        requests
    });

    (base_url.clone(), base_url, handle)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_content_length(headers: &[u8]) -> usize {
    String::from_utf8_lossy(headers)
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}
