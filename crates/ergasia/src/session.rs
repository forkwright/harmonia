use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use horismos::ErgasiaConfig;
use librqbit::api::TorrentIdOrHash;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
    SessionPersistenceConfig, TorrentStats,
};
use serde::{Deserialize, Serialize};
use themelion::ids::DownloadId;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::error::{
    AddTorrentSnafu, DeleteActionSnafu, ErgasiaError, PauseActionSnafu, SessionInitSnafu,
    TorrentMapPersistenceSnafu, TorrentNotFoundSnafu,
};
use crate::seeding::SeedingPolicy;

const TORRENT_MAP_FILE: &str = "harmonia-torrent-map.json";

pub struct SeedHandle {
    pub cancel: CancellationToken,
}

// WHY: librqbit persists its own session state but knows nothing about
// harmonia's DownloadId, so the download_id <-> librqbit id mapping is
// persisted in a side-table colocated with the session state and reloaded on
// startup — otherwise every persisted torrent is unmanageable after a restart.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedTorrentMap {
    torrents: Vec<PersistedTorrentEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedTorrentEntry {
    download_id: DownloadId,
    torrent_id: usize,
}

pub struct TorrentSession {
    session: Arc<Session>,
    pub policy: SeedingPolicy,
    pub seed_tracker: Arc<DashMap<DownloadId, SeedHandle>>,
    torrent_map: DashMap<DownloadId, usize>,
    // WHY: librqbit can return AlreadyManaged for a duplicate info-hash, so
    // two DownloadIds may end up mapped to the same torrent_id. This reverse
    // ref count is the source of truth for whether delete_torrent may reach
    // into librqbit, or must only drop its own mapping entry — see
    // acquire_torrent_ref / release_torrent_ref.
    torrent_refcounts: DashMap<usize, usize>,
    map_path: PathBuf,
    persist_lock: tokio::sync::Mutex<()>,
}

impl TorrentSession {
    #[instrument(skip_all, name = "ergasia_session_init")]
    pub async fn new(config: &ErgasiaConfig) -> Result<Self, ErgasiaError> {
        let peer_opts = librqbit::PeerConnectionOptions {
            connect_timeout: Some(Duration::from_secs(config.peer_connect_timeout_seconds)),
            read_write_timeout: Some(Duration::from_secs(10)),
            ..Default::default()
        };

        let persistence = SessionPersistenceConfig::Json {
            folder: Some(PathBuf::from(&config.session_state_path)),
        };

        let opts = SessionOptions {
            disable_dht: false,
            disable_dht_persistence: false,
            // WHY: pin DHT routing-table persistence inside this instance's own
            // session_state_path rather than librqbit's global default
            // (~/.cache/com.rqbit.dht/dht.json). A shared default races and
            // corrupts across concurrent instances — and parallel tests — that
            // initialize the persistent DHT at once; instance-local state keeps
            // each session self-contained.
            dht_config: Some(librqbit::dht::PersistentDhtConfig {
                config_filename: Some(PathBuf::from(&config.session_state_path).join("dht.json")),
                ..Default::default()
            }),
            persistence: Some(persistence),
            listen_port_range: Some(
                config
                    .listen_port_range
                    .first()
                    .copied()
                    .unwrap_or_else(|| unreachable!("listen_port_range is [u16; 2]"))
                    ..config
                        .listen_port_range
                        .get(1)
                        .copied()
                        .unwrap_or_else(|| unreachable!("listen_port_range is [u16; 2]")),
            ),
            enable_upnp_port_forwarding: false,
            peer_opts: Some(peer_opts),
            ..Default::default()
        };

        let session = Session::new_with_opts(config.download_dir.clone(), opts)
            .await
            .map_err(|e| {
                SessionInitSnafu {
                    error: e.to_string(),
                }
                .build()
            })?;

        let policy = SeedingPolicy {
            ratio_threshold: config.seed_ratio_threshold,
            time_threshold: Duration::from_secs(config.seed_time_threshold_hours * 3600),
        };

        let torrent_session = Self {
            session,
            policy,
            seed_tracker: Arc::new(DashMap::new()),
            torrent_map: DashMap::new(),
            torrent_refcounts: DashMap::new(),
            map_path: PathBuf::from(&config.session_state_path).join(TORRENT_MAP_FILE),
            persist_lock: tokio::sync::Mutex::new(()),
        };

        // INVARIANT: the session is not handed out until torrent_map reflects
        // every torrent librqbit restored from persisted state.
        torrent_session.reconcile_persisted_torrents().await?;

        Ok(torrent_session)
    }

    #[instrument(skip(self, magnet_uri), fields(download_id = %download_id))]
    pub async fn add_torrent_from_magnet(
        &self,
        download_id: DownloadId,
        magnet_uri: &str,
    ) -> Result<(usize, Arc<ManagedTorrent>), ErgasiaError> {
        let source = AddTorrent::Url(Cow::Borrowed(magnet_uri));
        self.add_torrent_inner(download_id, source, None).await
    }

    #[instrument(skip(self, torrent_bytes), fields(download_id = %download_id))]
    pub async fn add_torrent_from_bytes(
        &self,
        download_id: DownloadId,
        torrent_bytes: bytes::Bytes,
    ) -> Result<(usize, Arc<ManagedTorrent>), ErgasiaError> {
        let source = AddTorrent::TorrentFileBytes(torrent_bytes);
        self.add_torrent_inner(download_id, source, None).await
    }

    async fn add_torrent_inner(
        &self,
        download_id: DownloadId,
        source: AddTorrent<'_>,
        output_folder: Option<String>,
    ) -> Result<(usize, Arc<ManagedTorrent>), ErgasiaError> {
        let opts = Some(AddTorrentOptions {
            output_folder,
            ..Default::default()
        });

        let response = self.session.add_torrent(source, opts).await.map_err(|e| {
            AddTorrentSnafu {
                reason: "add_torrent call failed".to_string(),
                error: e.to_string(),
            }
            .build()
        })?;

        match response {
            AddTorrentResponse::Added(id, handle)
            | AddTorrentResponse::AlreadyManaged(id, handle) => {
                self.torrent_map.insert(download_id, id);
                self.acquire_torrent_ref(id);
                if let Err(persist_err) = self.persist_torrent_map().await {
                    // WHY: fail loudly but non-destructively — the mapping is
                    // rolled back so the caller sees a consistent failure, while
                    // the torrent stays in librqbit (an AlreadyManaged torrent
                    // may belong to another download, so deleting it here could
                    // destroy live data).
                    self.torrent_map.remove(&download_id);
                    self.release_torrent_ref(id);
                    return Err(persist_err);
                }
                Ok((id, handle))
            }
            AddTorrentResponse::ListOnly(_) => Err(AddTorrentSnafu {
                reason: "unexpected ListOnly response".to_string(),
                error: String::new(),
            }
            .build()),
        }
    }

    pub fn get_torrent(
        &self,
        download_id: DownloadId,
    ) -> Result<Arc<ManagedTorrent>, ErgasiaError> {
        let torrent_id = self
            .torrent_map
            .get(&download_id)
            .map(|v| *v)
            .ok_or_else(|| TorrentNotFoundSnafu { download_id }.build())?;

        self.session
            .get(TorrentIdOrHash::Id(torrent_id))
            .ok_or_else(|| TorrentNotFoundSnafu { download_id }.build())
    }

    pub fn get_stats(&self, download_id: DownloadId) -> Result<TorrentStats, ErgasiaError> {
        let handle = self.get_torrent(download_id)?;
        Ok(handle.stats())
    }

    pub async fn pause_torrent(&self, download_id: DownloadId) -> Result<(), ErgasiaError> {
        let handle = self.get_torrent(download_id)?;
        self.session.pause(&handle).await.map_err(|e| {
            PauseActionSnafu {
                download_id,
                error: e.to_string(),
            }
            .build()
        })
    }

    pub async fn delete_torrent(&self, download_id: DownloadId) -> Result<(), ErgasiaError> {
        // WHY: remove() is an atomic claim — of two concurrent deletes for the
        // same id, exactly one proceeds into librqbit; the other sees the entry
        // already gone and gets TorrentNotFound instead of a confusing
        // wrapped librqbit failure.
        let (_, torrent_id) = self
            .torrent_map
            .remove(&download_id)
            .ok_or_else(|| TorrentNotFoundSnafu { download_id }.build())?;

        // WHY: librqbit dedups a duplicate info-hash onto one torrent_id shared
        // by two DownloadIds (AlreadyManaged) — only the last DownloadId still
        // referencing torrent_id may delete it from librqbit; an earlier
        // caller just drops its own mapping entry and leaves the torrent live.
        let last_ref = self.release_torrent_ref(torrent_id);

        if last_ref
            && let Err(e) = self
                .session
                .delete(TorrentIdOrHash::Id(torrent_id), false)
                .await
        {
            // WHY: the torrent still exists in librqbit — restore the
            // mapping and its ref so the caller can retry instead of
            // orphaning it.
            self.torrent_map.insert(download_id, torrent_id);
            self.acquire_torrent_ref(torrent_id);
            return Err(DeleteActionSnafu {
                download_id,
                error: e.to_string(),
            }
            .build());
        }

        self.persist_torrent_map().await?;
        Ok(())
    }

    fn acquire_torrent_ref(&self, torrent_id: usize) {
        *self.torrent_refcounts.entry(torrent_id).or_insert(0) += 1;
    }

    // Decrements the ref count for `torrent_id` and reports whether this was
    // the last reference — i.e. whether the caller may now delete it from
    // librqbit.
    // INVARIANT: entry() holds the shard lock for `torrent_id` across the
    // whole read-modify-write, so two concurrent releases for the same
    // torrent_id can never both observe "last reference".
    fn release_torrent_ref(&self, torrent_id: usize) -> bool {
        match self.torrent_refcounts.entry(torrent_id) {
            Entry::Occupied(mut e) => {
                *e.get_mut() -= 1;
                if *e.get() == 0 {
                    e.remove();
                    true
                } else {
                    false
                }
            }
            // WHY: an untracked torrent_id has no evidence of another owner
            // (e.g. a mapping inserted directly without acquire_torrent_ref)
            // — fail toward attempting the real delete rather than silently
            // orphaning it.
            Entry::Vacant(_) => true,
        }
    }

    async fn reconcile_persisted_torrents(&self) -> Result<(), ErgasiaError> {
        let persisted = self.load_torrent_map().await?;

        let mut restored = 0usize;
        let mut dropped = 0usize;
        for entry in persisted {
            if self
                .session
                .get(TorrentIdOrHash::Id(entry.torrent_id))
                .is_some()
            {
                self.torrent_map.insert(entry.download_id, entry.torrent_id);
                self.acquire_torrent_ref(entry.torrent_id);
                restored += 1;
            } else {
                tracing::warn!(
                    download_id = %entry.download_id,
                    torrent_id = entry.torrent_id,
                    "dropping torrent map entry no longer present in the session"
                );
                dropped += 1;
            }
        }

        if dropped > 0 {
            self.persist_torrent_map().await?;
        }

        let live = self.session.with_torrents(|torrents| torrents.count());
        tracing::info!(restored, dropped, live, "reconciled persisted torrents");
        Ok(())
    }

    async fn load_torrent_map(&self) -> Result<Vec<PersistedTorrentEntry>, ErgasiaError> {
        let bytes = match tokio::fs::read(&self.map_path).await {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build());
            }
        };

        match serde_json::from_slice::<PersistedTorrentMap>(&bytes) {
            Ok(map) => Ok(map.torrents),
            Err(e) => {
                // WHY: a corrupt side-table must not brick startup — librqbit's
                // own session state is intact. Quarantine the file for forensics
                // and continue with an empty map (same managability as before
                // the side-table existed).
                let quarantine = self.map_path.with_extension("json.corrupt");
                tokio::fs::rename(&self.map_path, &quarantine).await.ok();
                tracing::warn!(
                    path = %self.map_path.display(),
                    quarantine = %quarantine.display(),
                    error = %e,
                    "torrent map file is corrupt; quarantined and starting with an empty map"
                );
                Ok(Vec::new())
            }
        }
    }

    async fn persist_torrent_map(&self) -> Result<(), ErgasiaError> {
        let _guard = self.persist_lock.lock().await;

        let torrents: Vec<PersistedTorrentEntry> = self
            .torrent_map
            .iter()
            .map(|kv| PersistedTorrentEntry {
                download_id: *kv.key(),
                torrent_id: *kv.value(),
            })
            .collect();

        let payload =
            serde_json::to_vec_pretty(&PersistedTorrentMap { torrents }).map_err(|e| {
                TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build()
            })?;

        if let Some(parent) = self.map_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build()
            })?;
        }

        // WHY: write-then-rename keeps the side-table atomic — a crash mid-write
        // leaves the previous map intact instead of a truncated file.
        let tmp_path = self.map_path.with_extension("json.tmp");
        tokio::fs::write(&tmp_path, &payload).await.map_err(|e| {
            TorrentMapPersistenceSnafu {
                path: tmp_path.clone(),
                error: e.to_string(),
            }
            .build()
        })?;
        tokio::fs::rename(&tmp_path, &self.map_path)
            .await
            .map_err(|e| {
                TorrentMapPersistenceSnafu {
                    path: self.map_path.clone(),
                    error: e.to_string(),
                }
                .build()
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::LazyLock;

    use super::*;

    // WHY: each session binds listen ports and starts a DHT; serializing the
    // session tests avoids port/DHT-persistence races inside one test binary.
    static SESSION_TEST_LOCK: LazyLock<tokio::sync::Mutex<()>> =
        LazyLock::new(|| tokio::sync::Mutex::new(()));

    fn test_config(root: &Path, port_base: u16) -> ErgasiaConfig {
        ErgasiaConfig {
            download_dir: root.join("downloads"),
            session_state_path: root.join("state"),
            listen_port_range: [port_base, port_base + 8],
            ..ErgasiaConfig::default()
        }
    }

    fn minimal_torrent_bytes(name: &str) -> bytes::Bytes {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"d4:infod6:lengthi11e4:name");
        buf.extend_from_slice(format!("{}:{}", name.len(), name).as_bytes());
        buf.extend_from_slice(b"12:piece lengthi16384e6:pieces20:");
        buf.extend_from_slice(&[0xAA; 20]);
        buf.extend_from_slice(b"ee");
        bytes::Bytes::from(buf)
    }

    async fn wait_for_session_state(state_dir: &Path) {
        for _ in 0..100 {
            let has_state = std::fs::read_dir(state_dir)
                .map(|entries| {
                    entries.flatten().any(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "json")
                            .unwrap_or(false)
                            && e.metadata().map(|m| m.len() > 2).unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            if has_state {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("librqbit session state was never persisted to {state_dir:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn torrent_map_rebuilt_after_restart() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24101);
        let download_id = DownloadId::new();

        {
            let session = TorrentSession::new(&config).await.unwrap();
            session
                .add_torrent_from_bytes(download_id, minimal_torrent_bytes("restart.bin"))
                .await
                .unwrap();
            assert!(session.get_stats(download_id).is_ok());
            wait_for_session_state(&config.session_state_path).await;
            session.session.stop().await;
        }

        let session = TorrentSession::new(&config).await.unwrap();
        assert!(
            session.get_stats(download_id).is_ok(),
            "download must be manageable after restart"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconcile_drops_stale_entries() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24201);
        let stale_id = DownloadId::new();

        std::fs::create_dir_all(&config.session_state_path).unwrap();
        let map_path = config.session_state_path.join(TORRENT_MAP_FILE);
        let stale = PersistedTorrentMap {
            torrents: vec![PersistedTorrentEntry {
                download_id: stale_id,
                torrent_id: 4242,
            }],
        };
        std::fs::write(&map_path, serde_json::to_vec(&stale).unwrap()).unwrap();

        let session = TorrentSession::new(&config).await.unwrap();
        assert!(
            matches!(
                session.get_stats(stale_id),
                Err(ErgasiaError::TorrentNotFound { .. })
            ),
            "stale entry must be dropped, not resurrected"
        );

        let rewritten: PersistedTorrentMap =
            serde_json::from_slice(&std::fs::read(&map_path).unwrap()).unwrap();
        assert!(
            rewritten.torrents.is_empty(),
            "stale entry must be pruned from the persisted map"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_download_id_errors_torrent_not_found() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24401);
        let session = TorrentSession::new(&config).await.unwrap();
        let unknown = DownloadId::new();

        assert!(matches!(
            session.get_torrent(unknown),
            Err(ErgasiaError::TorrentNotFound { .. })
        ));
        assert!(matches!(
            session.get_stats(unknown),
            Err(ErgasiaError::TorrentNotFound { .. })
        ));
        assert!(matches!(
            session.delete_torrent(unknown).await,
            Err(ErgasiaError::TorrentNotFound { .. })
        ));
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_deletes_yield_one_ok_one_not_found() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24501);
        let download_id = DownloadId::new();

        let session = TorrentSession::new(&config).await.unwrap();
        session
            .add_torrent_from_bytes(download_id, minimal_torrent_bytes("race.bin"))
            .await
            .unwrap();

        let (a, b) = tokio::join!(
            session.delete_torrent(download_id),
            session.delete_torrent(download_id)
        );

        let ok_count = [&a, &b].iter().filter(|r| r.is_ok()).count();
        assert_eq!(ok_count, 1, "exactly one delete must win: {a:?} / {b:?}");
        let loser = if a.is_err() { a } else { b };
        assert!(
            matches!(loser, Err(ErgasiaError::TorrentNotFound { .. })),
            "loser must see TorrentNotFound, got {loser:?}"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shared_torrent_id_survives_first_owners_delete() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24551);
        let download_a = DownloadId::new();
        let download_b = DownloadId::new();
        let torrent_bytes = minimal_torrent_bytes("shared.bin");

        let session = TorrentSession::new(&config).await.unwrap();
        let (id_a, _) = session
            .add_torrent_from_bytes(download_a, torrent_bytes.clone())
            .await
            .unwrap();
        let (id_b, _) = session
            .add_torrent_from_bytes(download_b, torrent_bytes)
            .await
            .unwrap();
        assert_eq!(
            id_a, id_b,
            "duplicate info-hash must dedup onto one torrent_id (AlreadyManaged)"
        );

        session.delete_torrent(download_a).await.unwrap();

        assert!(
            session.get_stats(download_b).is_ok(),
            "download_b must still resolve after download_a's delete"
        );
        assert!(
            session.session.get(TorrentIdOrHash::Id(id_b)).is_some(),
            "the shared torrent must not be deleted from librqbit while \
             download_b still references it"
        );

        session.delete_torrent(download_b).await.unwrap();

        assert!(
            matches!(
                session.get_stats(download_b),
                Err(ErgasiaError::TorrentNotFound { .. })
            ),
            "download_b must be gone after its own delete"
        );
        assert!(
            session.session.get(TorrentIdOrHash::Id(id_b)).is_none(),
            "the shared torrent must be deleted once the last owner deletes it"
        );

        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_failure_reports_delete_action_and_restores_mapping() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24601);
        let download_id = DownloadId::new();

        let session = TorrentSession::new(&config).await.unwrap();
        // WHY: a mapping to a torrent id librqbit does not manage forces the
        // session.delete failure path deterministically.
        session.torrent_map.insert(download_id, 999_999);

        let err = session.delete_torrent(download_id).await.unwrap_err();
        assert!(
            matches!(err, ErgasiaError::DeleteAction { .. }),
            "expected DeleteAction (not PauseAction), got {err:?}"
        );
        assert!(
            session.torrent_map.contains_key(&download_id),
            "mapping must be restored after a failed delete"
        );
        session.session.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn corrupt_torrent_map_is_quarantined() {
        let _guard = SESSION_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), 24301);

        std::fs::create_dir_all(&config.session_state_path).unwrap();
        let map_path = config.session_state_path.join(TORRENT_MAP_FILE);
        std::fs::write(&map_path, b"{ not json").unwrap();

        let session = TorrentSession::new(&config)
            .await
            .expect("corrupt side-table must not brick startup");
        assert!(
            map_path.with_extension("json.corrupt").exists(),
            "corrupt map must be quarantined for forensics"
        );
        session.session.stop().await;
    }
}
