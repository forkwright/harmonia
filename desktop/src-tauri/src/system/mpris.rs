//! MPRIS v2 D-Bus service for media key and desktop environment integration.
use std::sync::Arc;

use mpris_server::zbus::{self, fdo};
use mpris_server::{
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property, RootInterface,
    Server, Time, TrackId, Volume,
};
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex, broadcast};
use tracing::warn;

use crate::playback::{PlaybackEngine, PlaybackStateEvent, PlaybackStatus as EngineStatus};

struct MprisState {
    playback_status: PlaybackStatus,
    metadata: Metadata,
    volume: f64,
    position_us: i64,
}

impl Default for MprisState {
    fn default() -> Self {
        Self {
            playback_status: PlaybackStatus::Stopped,
            metadata: Metadata::new(),
            volume: 1.0,
            position_us: 0,
        }
    }
}

struct HarmoniaPlayer {
    app: tauri::AppHandle,
    state: Arc<Mutex<MprisState>>,
}

impl RootInterface for HarmoniaPlayer {
    async fn raise(&self) -> fdo::Result<()> {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        self.app.exit(0);
        Ok(())
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok("Harmonia".to_string())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("harmonia".to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["file".to_string()])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![
            "audio/flac".to_string(),
            "audio/mpeg".to_string(),
            "audio/mp4".to_string(),
            "audio/ogg".to_string(),
            "audio/opus".to_string(),
            "audio/wav".to_string(),
            "audio/aac".to_string(),
        ])
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> zbus::Result<()> {
        Ok(())
    }
}

impl PlayerInterface for HarmoniaPlayer {
    async fn play(&self) -> fdo::Result<()> {
        let engine = self.app.state::<PlaybackEngine>();
        let _ = engine.resume(&self.app).await;
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        let engine = self.app.state::<PlaybackEngine>();
        let _ = engine.pause(&self.app).await;
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        let engine = self.app.state::<PlaybackEngine>();
        let state = engine.playback_state().await;
        match state.status {
            EngineStatus::Playing => {
                let _ = engine.pause(&self.app).await;
            }
            EngineStatus::Paused => {
                let _ = engine.resume(&self.app).await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        let engine = self.app.state::<PlaybackEngine>();
        engine.stop(&self.app).await;
        Ok(())
    }

    async fn next(&self) -> fdo::Result<()> {
        let _ = self.app.emit("tray-next", ());
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        let _ = self.app.emit("tray-prev", ());
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let current_us = self.state.lock().await.position_us;
        let new_us = current_us.saturating_add(offset.as_micros());
        let new_ms = (new_us / 1000).max(0) as u64;
        let engine = self.app.state::<PlaybackEngine>();
        let _ = engine.seek(new_ms).await;
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        let pos_ms = (position.as_micros() / 1000).max(0) as u64;
        let engine = self.app.state::<PlaybackEngine>();
        let _ = engine.seek(pos_ms).await;
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Ok(())
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(self.state.lock().await.playback_status)
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::None)
    }

    async fn set_loop_status(&self, _loop_status: LoopStatus) -> zbus::Result<()> {
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        let engine = self.app.state::<PlaybackEngine>();
        Ok(engine.playback_state().await.shuffle)
    }

    async fn set_shuffle(&self, shuffle: bool) -> zbus::Result<()> {
        let engine = self.app.state::<PlaybackEngine>();
        engine.set_shuffle(shuffle, &self.app).await;
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(self.state.lock().await.metadata.clone())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(self.state.lock().await.volume)
    }

    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        let clamped = volume.clamp(0.0, 1.0);
        let engine = self.app.state::<PlaybackEngine>();
        engine.set_volume(clamped).await;
        self.state.lock().await.volume = clamped;
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_micros(self.state.lock().await.position_us))
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}

/// Starts the MPRIS server and bridges playback state changes to D-Bus.
///
/// Runs until the broadcast channel closes (i.e. the playback engine shuts
/// down).  Callers should `tokio::spawn` this function.
pub(crate) async fn start(
    app: tauri::AppHandle,
    mut state_rx: broadcast::Receiver<PlaybackStateEvent>,
) -> Result<(), mpris_server::zbus::Error> {
    let shared_state = Arc::new(Mutex::new(MprisState::default()));
    let player = HarmoniaPlayer {
        app,
        state: Arc::clone(&shared_state),
    };

    let server = Server::new("harmonia", player).await?;

    // Bridge: PlaybackEngine state changes → MPRIS PropertiesChanged signals.
    loop {
        match state_rx.recv().await {
            Ok(event) => {
                let new_status = match event.status {
                    EngineStatus::Playing => PlaybackStatus::Playing,
                    EngineStatus::Paused => PlaybackStatus::Paused,
                    _ => PlaybackStatus::Stopped,
                };

                let new_metadata = build_metadata(event.track.as_ref());

                {
                    let mut state = shared_state.lock().await;
                    state.playback_status = new_status;
                    state.metadata = new_metadata.clone();
                }

                if let Err(e) = server
                    .properties_changed([
                        Property::PlaybackStatus(new_status),
                        Property::Metadata(new_metadata),
                    ])
                    .await
                {
                    warn!(error = %e, "MPRIS properties_changed failed");
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    Ok(())
}

fn build_metadata(track: Option<&crate::playback::TrackInfo>) -> Metadata {
    let mut m = Metadata::new();
    let Some(t) = track else {
        m.set_trackid(Some(TrackId::NO_TRACK));
        return m;
    };

    let track_id_str = format!("/org/mpris/MediaPlayer2/Track/{}", sanitise_id(&t.track_id));
    if let Ok(id) = TrackId::try_from(track_id_str.as_str()) {
        m.set_trackid(Some(id));
    } else {
        m.set_trackid(Some(TrackId::NO_TRACK));
    }

    m.set_title(Some(t.title.clone()));

    if let Some(artist) = &t.artist {
        m.set_artist(Some(vec![artist.clone()]));
    }

    if let Some(album) = &t.album {
        m.set_album(Some(album.clone()));
    }

    if let Some(duration_ms) = t.duration_ms {
        m.set_length(Some(Time::from_millis(duration_ms as i64)));
    }

    m
}

/// Strips characters that are invalid in D-Bus object path components.
fn sanitise_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
