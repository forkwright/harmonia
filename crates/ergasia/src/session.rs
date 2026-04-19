use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
    SessionPersistenceConfig, TorrentStats, api::TorrentIdOrHash,
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use horismos::ErgasiaConfig;
use themelion::ids::DownloadId;

use crate::error::{
    AddTorrentSnafu, ErgasiaError, PauseActionSnafu, SessionInitSnafu, TorrentNotFoundSnafu,
};
use crate::seeding::SeedingPolicy;

pub struct SeedHandle {
    pub cancel: CancellationToken,
}

pub struct ErgasiaSession {
    session: Arc<Session>,
    pub policy: SeedingPolicy,
    pub seed_tracker: Arc<DashMap<DownloadId, SeedHandle>>,
    torrent_map: DashMap<DownloadId, usize>,
}

impl ErgasiaSession {
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
            persistence: Some(persistence),
            listen_port_range: Some(config.listen_port_range[0]..config.listen_port_range[1]),
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

        Ok(Self {
            session,
            policy,
            seed_tracker: Arc::new(DashMap::new()),
            torrent_map: DashMap::new(),
        })
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
        let torrent_id = self
            .torrent_map
            .get(&download_id)
            .map(|v| *v)
            .ok_or_else(|| TorrentNotFoundSnafu { download_id }.build())?;

        self.session
            .delete(TorrentIdOrHash::Id(torrent_id), false)
            .await
            .map_err(|e| {
                PauseActionSnafu {
                    download_id,
                    error: e.to_string(),
                }
                .build()
            })?;

        self.torrent_map.remove(&download_id);
        Ok(())
    }

    pub fn reconcile_persisted_torrents(&self) {
        let count = self.session.with_torrents(|torrents| torrents.count());
        tracing::info!(count, "reconciled persisted torrents");
    }
}
