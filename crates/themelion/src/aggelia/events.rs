use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::{DownloadId, EpisodeId, FeedId, MediaId, QueryId, SessionId, UserId};
use crate::media::{MediaType, QualityProfile};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HarmoniaEvent {
    // Acquisition pipeline
    /// Taxis successfully imported a media item into the library.
    /// Subscribers: Syndesmos (Plex notify), Kritike (quality assessment),
    ///              Prostheke (subtitle acquisition), web UI (library update)
    ImportCompleted {
        media_id: MediaId,
        media_type: MediaType,
        path: PathBuf,
    },

    /// Kritike determined a library item does not meet its quality profile.
    /// Subscribers: Episkope (re-trigger acquisition for the item)
    QualityUpgradeTriggered {
        media_id: MediaId,
        current_quality: QualityProfile,
    },

    /// Ergasia is reporting download progress during an active transfer.
    /// Subscribers: web UI / API layer (real-time progress display)
    DownloadProgress {
        download_id: DownloadId,
        percent: u8,
        bytes_downloaded: u64,
        bytes_total: u64,
    },

    /// Ergasia completed a download successfully.
    /// Subscribers: Syntaxis (trigger post-processing pipeline)
    DownloadCompleted {
        download_id: DownloadId,
        path: PathBuf,
    },

    /// Ergasia failed a download — all retries exhausted.
    /// Subscribers: Syntaxis (handle failure escalation, update queue state)
    DownloadFailed {
        download_id: DownloadId,
        reason: String,
    },

    /// Ergasia's seeding monitor determined ratio/time policy is satisfied.
    /// Subscribers: web UI (display seeding completion status)
    SeedPolicySatisfied {
        download_id: DownloadId,
        uploaded_bytes: u64,
        downloaded_bytes: u64,
    },

    /// Zetesis completed a search against configured indexers.
    /// Subscribers: Episkope (evaluate candidates for acquisition)
    SearchCompleted {
        query_id: QueryId,
        result_count: usize,
    },

    // Integration events
    /// Taxis imported new media — Plex library needs a scan notification.
    /// Subscribers: Syndesmos (call Plex refresh endpoint)
    PlexNotifyRequired { media_id: MediaId },

    /// Paroche detected playback of a track — scrobbling is now warranted.
    /// Subscribers: Syndesmos (submit scrobble to Last.fm)
    ScrobbleRequired { track_id: MediaId, user_id: UserId },

    /// Syndesmos completed a Tidal want-list sync.
    /// Subscribers: Episkope (add new want-list items to monitored set)
    TidalWantListSynced { added: Vec<MediaId> },

    // Library events
    /// Epignosis completed metadata enrichment for a library item.
    /// Subscribers: web UI / API layer (update displayed metadata),
    ///              library indexer (update search index)
    MetadataEnriched {
        media_id: MediaId,
        media_type: MediaType,
    },

    /// A full library scan completed.
    /// Subscribers: web UI / API layer (refresh library view),
    ///              Kritike (run health assessment on newly scanned items)
    LibraryScanCompleted {
        items_scanned: usize,
        items_added: usize,
        items_removed: usize,
    },

    /// Prostheke acquired subtitle tracks for a media item.
    /// Subscribers: Paroche (update available subtitle tracks for active streams)
    SubtitleAcquired {
        media_id: MediaId,
        languages: Vec<String>,
    },

    // Playback events
    /// Paroche started streaming a track — now-playing display should update.
    /// Subscribers: Syndesmos (Last.fm updateNowPlaying), web UI
    NowPlayingStarted {
        track_id: MediaId,
        user_id: UserId,
        media_type: MediaType,
    },

    /// Playback session completed (track finished, user skipped, or stream ended).
    /// Subscribers: play history repo (persist session + update stats)
    PlaybackSessionEnded {
        session_id: SessionId,
        media_id: MediaId,
        user_id: UserId,
        duration_ms: u64,
        total_ms: u64,
        completed: bool,
    },

    // Feed events
    /// Komide discovered a new podcast episode from a subscribed feed.
    /// Subscribers: web UI (new episode badge), auto-download pipeline
    EpisodeAvailable {
        subscription_id: FeedId,
        episode_id: EpisodeId,
        title: String,
    },

    /// Komide completed a feed refresh cycle.
    /// Subscribers: web UI (feed last-updated display)
    FeedRefreshed {
        feed_id: FeedId,
        new_items: usize,
        media_type: MediaType,
    },
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::aggelia::create_event_bus;

    #[tokio::test]
    async fn event_bus_send_receive() {
        let (tx, mut rx) = create_event_bus(16);
        let media_id = MediaId::new();
        tx.send(HarmoniaEvent::ImportCompleted {
            media_id,
            media_type: MediaType::Music,
            path: PathBuf::from("/music/track.flac"),
        })
        .unwrap();
        match rx.recv().await.unwrap() {
            HarmoniaEvent::ImportCompleted {
                media_id: received_id,
                ..
            } => assert_eq!(received_id, media_id),
            _ => panic!("unexpected event variant"),
        }
    }

    #[test]
    fn import_completed_serde_roundtrip() {
        let event = HarmoniaEvent::ImportCompleted {
            media_id: MediaId::new(),
            media_type: MediaType::Music,
            path: PathBuf::from("/library/track.flac"),
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn download_progress_serde_roundtrip() {
        let event = HarmoniaEvent::DownloadProgress {
            download_id: DownloadId::new(),
            percent: 42,
            bytes_downloaded: 1_000_000,
            bytes_total: 10_000_000,
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn download_failed_serde_roundtrip() {
        let event = HarmoniaEvent::DownloadFailed {
            download_id: DownloadId::new(),
            reason: "connection timeout".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn search_completed_serde_roundtrip() {
        let event = HarmoniaEvent::SearchCompleted {
            query_id: QueryId::new(),
            result_count: 5,
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn quality_upgrade_triggered_serde_roundtrip() {
        let event = HarmoniaEvent::QualityUpgradeTriggered {
            media_id: MediaId::new(),
            current_quality: QualityProfile::new(128),
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn scrobble_required_serde_roundtrip() {
        let event = HarmoniaEvent::ScrobbleRequired {
            track_id: MediaId::new(),
            user_id: UserId::new(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn tidal_want_list_synced_serde_roundtrip() {
        let event = HarmoniaEvent::TidalWantListSynced {
            added: vec![MediaId::new(), MediaId::new()],
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn library_scan_completed_serde_roundtrip() {
        let event = HarmoniaEvent::LibraryScanCompleted {
            items_scanned: 1000,
            items_added: 50,
            items_removed: 3,
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn subtitle_acquired_serde_roundtrip() {
        let event = HarmoniaEvent::SubtitleAcquired {
            media_id: MediaId::new(),
            languages: vec!["en".to_string(), "fr".to_string()],
        };
        let json = serde_json::to_string(&event).unwrap();
        let _recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn now_playing_started_serde_roundtrip() {
        let event = HarmoniaEvent::NowPlayingStarted {
            track_id: MediaId::new(),
            user_id: UserId::new(),
            media_type: MediaType::Music,
        };
        let json = serde_json::to_string(&event).unwrap();
        let recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
        match recovered {
            HarmoniaEvent::NowPlayingStarted { media_type, .. } => {
                assert_eq!(media_type, MediaType::Music);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn playback_session_ended_serde_roundtrip() {
        let session_id = SessionId::new();
        let event = HarmoniaEvent::PlaybackSessionEnded {
            session_id,
            media_id: MediaId::new(),
            user_id: UserId::new(),
            duration_ms: 180_000,
            total_ms: 210_000,
            completed: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        let recovered: HarmoniaEvent = serde_json::from_str(&json).unwrap();
        match recovered {
            HarmoniaEvent::PlaybackSessionEnded {
                session_id: recovered_id,
                duration_ms,
                completed,
                ..
            } => {
                assert_eq!(recovered_id, session_id);
                assert_eq!(duration_ms, 180_000);
                assert!(completed);
            }
            _ => panic!("unexpected variant"),
        }
    }
}
