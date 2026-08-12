//! Plex collection management — Kometa replacement.
//!
//! Maps Harmonia media tags to Plex collections, keeping library metadata
//! consistent without a separate Kometa/PMM process.

use snafu::ResultExt;
use themelion::MediaType;

use super::BoxFuture;
use crate::error::{PlexApiCallSnafu, SyndesmodError};
use crate::plex::PlexClient;

/// Plex library item kind a collection groups, keyed by its Plex API type code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlexCollectionKind {
    /// Plex type 1 — movie libraries.
    Movie,
    /// Plex type 2 — TV show libraries.
    TvShow,
    /// Plex type 8 — music libraries (collections of artists).
    Artist,
}

impl PlexCollectionKind {
    fn type_code(self) -> u8 {
        match self {
            Self::Movie => 1,
            Self::TvShow => 2,
            Self::Artist => 8,
        }
    }

    /// The Plex collection kind a Harmonia media type maps to, or `None` when
    /// Plex has no native collection for it — the caller degrades to a logged
    /// no-op, matching the unconfigured-section path.
    pub(crate) fn for_media_type(media_type: MediaType) -> Option<Self> {
        match media_type {
            MediaType::Movie => Some(Self::Movie),
            MediaType::Tv => Some(Self::TvShow),
            MediaType::Music => Some(Self::Artist),
            _ => None,
        }
    }
}

/// Manages Plex collections derived from Harmonia metadata.
pub(crate) trait CollectionManager: Send + Sync {
    /// Creates a Plex collection named `name` in library section `section_id`,
    /// containing the items named by their Plex rating keys.
    fn sync_collection<'a>(
        &'a self,
        name: &'a str,
        section_id: u32,
        kind: PlexCollectionKind,
        rating_keys: &'a [String],
    ) -> BoxFuture<'a, Result<(), SyndesmodError>>;
}

impl CollectionManager for PlexClient {
    fn sync_collection<'a>(
        &'a self,
        name: &'a str,
        section_id: u32,
        kind: PlexCollectionKind,
        rating_keys: &'a [String],
    ) -> BoxFuture<'a, Result<(), SyndesmodError>> {
        Box::pin(async move {
            let machine_id = self.machine_identifier().await?;
            let url = format!(
                "{}/library/collections",
                self.config.url.trim_end_matches('/')
            );
            self.http
                .post(&url)
                .header("X-Plex-Token", &self.config.token)
                .query(&[
                    ("type", kind.type_code().to_string()),
                    ("title", name.to_string()),
                    ("smart", "0".to_string()),
                    ("sectionId", section_id.to_string()),
                    ("uri", collection_uri(&machine_id, rating_keys)),
                ])
                .send()
                .await
                .context(PlexApiCallSnafu)?
                .error_for_status()
                .context(PlexApiCallSnafu)?;
            Ok(())
        })
    }
}

impl PlexClient {
    /// The server's `machineIdentifier`, required to build `library://` item
    /// URIs for collection membership.
    async fn machine_identifier(&self) -> Result<String, SyndesmodError> {
        let url = self.config.url.trim_end_matches('/').to_string();
        let identity: ServerIdentity = self
            .http
            .get(&url)
            .header("X-Plex-Token", &self.config.token)
            .header("Accept", "application/json")
            .send()
            .await
            .context(PlexApiCallSnafu)?
            .error_for_status()
            .context(PlexApiCallSnafu)?
            .json()
            .await
            .context(PlexApiCallSnafu)?;
        Ok(identity.media_container.machine_identifier)
    }
}

/// Builds the `library://<machine>/directory/<item paths>` membership URI Plex
/// expects on collection create. The paths segment is percent-encoded once
/// here and again by the query serializer — the double encoding is the wire
/// shape Plex clients produce.
fn collection_uri(machine_id: &str, rating_keys: &[String]) -> String {
    let paths = rating_keys
        .iter()
        .map(|key| format!("/library/metadata/{key}"))
        .collect::<Vec<_>>()
        .join(",");
    let encoded: String = url::form_urlencoded::byte_serialize(paths.as_bytes()).collect();
    format!("library://{machine_id}/directory/{encoded}")
}

// WHY: wire DTO — Plex server identity response (`GET /` as JSON).
#[derive(serde::Deserialize)]
struct ServerIdentity {
    #[serde(rename = "MediaContainer")]
    media_container: ServerIdentityContainer,
}

#[derive(serde::Deserialize)]
struct ServerIdentityContainer {
    #[serde(rename = "machineIdentifier")]
    machine_identifier: String,
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use horismos::PlexConfig;

    use super::*;

    pub(crate) struct RecordedCollection {
        pub(crate) name: String,
        pub(crate) section_id: u32,
        pub(crate) kind: PlexCollectionKind,
        pub(crate) rating_keys: Vec<String>,
    }

    #[derive(Default)]
    pub(crate) struct MockCollectionManager {
        pub(crate) calls: Mutex<Vec<RecordedCollection>>,
    }

    impl MockCollectionManager {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn recorded(&self) -> Vec<(String, u32, PlexCollectionKind, Vec<String>)> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .map(|c| (c.name.clone(), c.section_id, c.kind, c.rating_keys.clone()))
                .collect()
        }
    }

    impl CollectionManager for MockCollectionManager {
        fn sync_collection<'a>(
            &'a self,
            name: &'a str,
            section_id: u32,
            kind: PlexCollectionKind,
            rating_keys: &'a [String],
        ) -> BoxFuture<'a, Result<(), SyndesmodError>> {
            self.calls.lock().unwrap().push(RecordedCollection {
                name: name.to_string(),
                section_id,
                kind,
                rating_keys: rating_keys.to_vec(),
            });
            Box::pin(async { Ok(()) })
        }
    }

    fn test_config(url: String) -> PlexConfig {
        PlexConfig {
            url,
            token: "plextok".to_string(),
            library_sections: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn media_type_maps_to_expected_collection_kind() {
        assert_eq!(
            PlexCollectionKind::for_media_type(MediaType::Movie),
            Some(PlexCollectionKind::Movie)
        );
        assert_eq!(
            PlexCollectionKind::for_media_type(MediaType::Tv),
            Some(PlexCollectionKind::TvShow)
        );
        assert_eq!(
            PlexCollectionKind::for_media_type(MediaType::Music),
            Some(PlexCollectionKind::Artist)
        );
    }

    #[test]
    fn unmapped_media_types_have_no_collection_kind() {
        for media_type in [
            MediaType::Audiobook,
            MediaType::Book,
            MediaType::Comic,
            MediaType::Podcast,
            MediaType::News,
        ] {
            assert_eq!(PlexCollectionKind::for_media_type(media_type), None);
        }
    }

    #[test]
    fn collection_uri_double_encodes_item_paths() {
        let uri = collection_uri("machine1", &["101".to_string(), "102".to_string()]);
        assert_eq!(
            uri,
            "library://machine1/directory/%2Flibrary%2Fmetadata%2F101%2C%2Flibrary%2Fmetadata%2F102"
        );
    }

    #[tokio::test]
    async fn sync_collection_fetches_identity_then_creates_collection() {
        let identity = r#"{"MediaContainer":{"machineIdentifier":"machine1"}}"#;
        let (base_url, _auth_url, server) = crate::test_support::spawn_sequential_http(vec![
            (200, identity.to_string()),
            (200, String::new()),
        ])
        .await;
        let client = PlexClient::new(test_config(base_url));

        client
            .sync_collection(
                "Jazz",
                7,
                PlexCollectionKind::Artist,
                &["101".to_string(), "102".to_string()],
            )
            .await
            .unwrap();

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[0].starts_with("GET / HTTP/1.1"),
            "identity probe must hit the server root: {}",
            requests[0].lines().next().unwrap()
        );
        let create_line = requests[1].lines().next().unwrap();
        assert!(
            create_line.starts_with("POST /library/collections?"),
            "expected a collection create: {create_line}"
        );
        for expected in [
            "type=8",
            "title=Jazz",
            "smart=0",
            "sectionId=7",
            "uri=library%3A%2F%2Fmachine1%2Fdirectory%2F%252Flibrary%252Fmetadata%252F101%252C%252Flibrary%252Fmetadata%252F102",
        ] {
            assert!(
                create_line.contains(expected),
                "create request missing {expected}: {create_line}"
            );
        }
    }

    #[tokio::test]
    async fn sync_collection_errors_when_identity_probe_fails() {
        let (base_url, _auth_url, server) =
            crate::test_support::spawn_sequential_http(vec![(401, String::new())]).await;
        let client = PlexClient::new(test_config(base_url));

        let result = client
            .sync_collection("Jazz", 7, PlexCollectionKind::Artist, &["101".to_string()])
            .await;

        assert!(matches!(result, Err(SyndesmodError::PlexApiCall { .. })));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn sync_collection_errors_when_create_fails() {
        let identity = r#"{"MediaContainer":{"machineIdentifier":"machine1"}}"#;
        let (base_url, _auth_url, server) = crate::test_support::spawn_sequential_http(vec![
            (200, identity.to_string()),
            (500, String::new()),
        ])
        .await;
        let client = PlexClient::new(test_config(base_url));

        let result = client
            .sync_collection("Jazz", 7, PlexCollectionKind::Artist, &["101".to_string()])
            .await;

        assert!(matches!(result, Err(SyndesmodError::PlexApiCall { .. })));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn sync_collection_survives_circuit_wrapper() {
        // WHY: the production caller routes through `with_retry` — prove the
        // trait future composes with it instead of only firing standalone.
        use crate::retry::{CircuitBreaker, with_retry};

        let mock = MockCollectionManager::new();
        let circuit = CircuitBreaker::new("plex", 5, std::time::Duration::from_secs(300));
        let keys = vec!["101".to_string()];

        with_retry(
            || mock.sync_collection("Jazz", 7, PlexCollectionKind::Artist, &keys),
            &circuit,
        )
        .await
        .unwrap();

        assert_eq!(mock.recorded().len(), 1);
    }
}
