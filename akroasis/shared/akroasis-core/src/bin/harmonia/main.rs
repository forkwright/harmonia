use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use akroasis_core::config::{
    CrossfeedConfig, EngineConfig, OutputConfig, ReplayGainConfig, ReplayGainMode, VolumeConfig,
};
use akroasis_core::engine::{AudioSource, Engine, EngineEvent};
use akroasis_core::queue::PlayQueue;
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures::StreamExt;
use serde::Serialize;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// CLI argument types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReplayGainArg {
    Track,
    Album,
    Off,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CrossfeedArg {
    Off,
    Easy,
    Normal,
    Extreme,
}

#[derive(Parser)]
#[command(name = "harmonia", about = "Harmonia media player", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Play audio files or directories.
    Play {
        /// Files or directories to play.
        paths: Vec<PathBuf>,

        /// Output device name (default: system default).
        #[arg(long)]
        device: Option<String>,

        /// Request exclusive device access.
        #[arg(long)]
        exclusive: bool,

        /// Output structured NDJSON instead of the interactive UI.
        #[arg(long)]
        json: bool,

        /// Main volume (0–100).
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
        volume: Option<u8>,

        /// Crossfade duration between tracks in milliseconds.
        #[arg(long)]
        crossfade: Option<u32>,

        /// ReplayGain normalization mode.
        #[arg(long, value_enum)]
        replaygain: Option<ReplayGainArg>,

        /// EQ preset name (not yet implemented in Phase 1).
        #[arg(long)]
        eq_preset: Option<String>,

        /// Crossfeed strength for headphone listening.
        #[arg(long, value_enum)]
        crossfeed: Option<CrossfeedArg>,
    },
}

// ---------------------------------------------------------------------------
// JSON event types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum JsonEvent<'a> {
    PlaybackStarted {
        source: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        signal_path: Option<serde_json::Value>,
    },
    Position {
        current_secs: f64,
        total_secs: f64,
    },
    TrackEnded {
        source: &'a str,
    },
    PlaybackStopped,
    Error {
        message: &'a str,
    },
    Warning {
        message: &'a str,
    },
}

fn emit_json(event: &JsonEvent<'_>) {
    if let Ok(s) = serde_json::to_string(event) {
        println!("{s}");
    }
}

// ---------------------------------------------------------------------------
// Audio file collection
// ---------------------------------------------------------------------------

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "wav", "mp3", "m4a", "ogg", "opus", "aiff", "aif", "wv",
];

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

/// Recursively collects audio files from `path`, sorted by filename within each directory.
pub fn collect_audio_files(path: &Path, out: &mut Vec<PathBuf>, json: bool) {
    if path.is_file() {
        if is_supported(path) {
            out.push(path.to_path_buf());
        } else if json {
            emit_json(&JsonEvent::Warning {
                message: &format!("skipping unsupported file: {}", path.display()),
            });
        } else {
            eprintln!("warning: skipping unsupported file: {}", path.display());
        }
    } else if path.is_dir() {
        match std::fs::read_dir(path) {
            Ok(entries) => {
                let mut children: Vec<PathBuf> =
                    entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
                children.sort();
                for child in children {
                    collect_audio_files(&child, out, json);
                }
            }
            Err(e) => {
                if json {
                    emit_json(&JsonEvent::Warning {
                        message: &format!("cannot read directory {}: {e}", path.display()),
                    });
                } else {
                    eprintln!("warning: cannot read directory {}: {e}", path.display());
                }
            }
        }
    }
}

fn build_queue(paths: &[PathBuf], json: bool) -> PlayQueue {
    let mut files = Vec::new();
    for path in paths {
        collect_audio_files(path, &mut files, json);
    }
    PlayQueue::from_tracks(files)
}

// ---------------------------------------------------------------------------
// Engine configuration from CLI args
// ---------------------------------------------------------------------------

fn build_engine_config(
    device: Option<String>,
    exclusive: bool,
    volume: Option<u8>,
    replaygain: Option<ReplayGainArg>,
    crossfeed: Option<CrossfeedArg>,
) -> EngineConfig {
    let volume_db = volume
        .map(|v| 20.0 * (v as f64 / 100.0).log10())
        .unwrap_or(0.0);

    let rg_config = replaygain.map(|rg| ReplayGainConfig {
        enabled: !matches!(rg, ReplayGainArg::Off),
        mode: match rg {
            ReplayGainArg::Album => ReplayGainMode::Album,
            _ => ReplayGainMode::Track,
        },
        ..Default::default()
    });

    let cf_config = crossfeed.map(|cf| CrossfeedConfig {
        enabled: !matches!(cf, CrossfeedArg::Off),
        strength: match cf {
            CrossfeedArg::Easy => 0.2,
            CrossfeedArg::Normal => 0.4,
            CrossfeedArg::Extreme => 0.7,
            CrossfeedArg::Off => 0.0,
        },
    });

    let dsp = akroasis_core::config::DspConfig {
        volume: VolumeConfig {
            level_db: volume_db,
            dither: true,
        },
        replaygain: rg_config.unwrap_or_default(),
        crossfeed: cf_config.unwrap_or_default(),
        ..Default::default()
    };

    EngineConfig {
        dsp,
        output: OutputConfig {
            device_name: device,
            exclusive_mode: exclusive,
            ..Default::default()
        },
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Playback loop (JSON mode)
// ---------------------------------------------------------------------------

async fn play_json(engine: Arc<Engine>, mut queue: PlayQueue) {
    let mut events = engine.subscribe_events();

    while let Some(path) = queue.current().map(PathBuf::from) {
        let path_str = path.display().to_string();
        emit_json(&JsonEvent::PlaybackStarted {
            source: &path_str,
            signal_path: None,
        });

        let source = AudioSource::File(path.clone());
        if let Err(e) = engine.play(source) {
            emit_json(&JsonEvent::Error {
                message: &e.to_string(),
            });
            break;
        }

        let play_start = Instant::now();
        // Duration is not known without a metadata read; use 0 as sentinel.
        let total_secs = read_duration_secs(&path);

        // Drive the event loop until the track ends. Returns true to continue to next track.
        let advance = 'track: loop {
            // Emit position every second.
            let position_tick = tokio::time::sleep(Duration::from_secs(1));

            tokio::select! {
                _ = position_tick => {
                    let current = play_start.elapsed().as_secs_f64();
                    emit_json(&JsonEvent::Position {
                        current_secs: current,
                        total_secs,
                    });
                }
                evt = events.recv() => {
                    match evt {
                        Ok(EngineEvent::TrackEnded { .. }) => {
                            emit_json(&JsonEvent::TrackEnded { source: &path_str });
                            break 'track true;
                        }
                        Ok(EngineEvent::PlaybackStopped) => {
                            emit_json(&JsonEvent::PlaybackStopped);
                            return;
                        }
                        Ok(EngineEvent::Error { message }) => {
                            emit_json(&JsonEvent::Error { message: &message });
                            break 'track false;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            emit_json(&JsonEvent::Warning {
                                message: &format!("event buffer lagged by {n} messages"),
                            });
                        }
                        _ => {}
                    }
                }
            }
        };

        if !advance {
            break;
        }

        // Advance to next track.
        if queue.next().is_none() {
            break;
        }
    }

    let _ = engine.stop();
    emit_json(&JsonEvent::PlaybackStopped);
}

// ---------------------------------------------------------------------------
// Playback loop (interactive / human mode)
// ---------------------------------------------------------------------------

async fn play_interactive(engine: Arc<Engine>, mut queue: PlayQueue) {
    let mut events = engine.subscribe_events();

    // Print signal path summary header.
    println!("Harmonia Player — press [space] pause  [n] next  [p] prev  [q] quit");
    println!();

    while let Some(path) = queue.current().map(PathBuf::from) {
        let source = AudioSource::File(path.clone());

        // Print now-playing info.
        print_track_info(&path);

        if let Err(e) = engine.play(source) {
            eprintln!("error: {e}");
            break;
        }

        let play_start = Instant::now();
        let total_secs = read_duration_secs(&path);

        // Set up terminal for raw-mode keyboard input.
        if enable_raw_mode().is_err() {
            // Fallback: no keyboard controls (e.g. pipe or CI).
            wait_for_track_end(&mut events).await;
        } else {
            let result =
                interactive_loop(&engine, &mut events, play_start, total_secs, &mut queue).await;
            let _ = disable_raw_mode();
            println!(); // newline after progress bar

            match result {
                LoopAction::Quit => {
                    let _ = engine.stop();
                    return;
                }
                LoopAction::Next => {
                    let _ = engine.stop();
                    if queue.next().is_none() {
                        return;
                    }
                    continue;
                }
                LoopAction::Previous(elapsed) => {
                    let _ = engine.stop();
                    queue.previous(elapsed);
                    continue;
                }
                LoopAction::TrackEnded => {
                    if queue.next().is_none() {
                        return;
                    }
                    continue;
                }
                LoopAction::Stopped => return,
            }
        }

        if queue.next().is_none() {
            break;
        }
    }

    let _ = engine.stop();
}

#[derive(Debug)]
enum LoopAction {
    Quit,
    Next,
    Previous(f64),
    TrackEnded,
    Stopped,
}

async fn interactive_loop(
    engine: &Arc<Engine>,
    events: &mut broadcast::Receiver<EngineEvent>,
    play_start: Instant,
    total_secs: f64,
    _queue: &mut PlayQueue,
) -> LoopAction {
    let mut key_stream = EventStream::new();
    let mut volume_db: f64 = 0.0;

    loop {
        let elapsed = play_start.elapsed().as_secs_f64();
        print_progress(elapsed, total_secs);

        let tick = tokio::time::sleep(Duration::from_millis(200));

        tokio::select! {
            _ = tick => {
                // Progress update handled at top of loop.
            }

            maybe_key = key_stream.next() => {
                if let Some(Ok(Event::Key(KeyEvent { code, .. }))) = maybe_key {
                    match code {
                        KeyCode::Char(' ') => {
                            // Toggle pause/resume.
                            if engine.pause().is_err() {
                                let _ = engine.resume();
                            }
                        }
                        KeyCode::Char('n') => return LoopAction::Next,
                        KeyCode::Char('p') => return LoopAction::Previous(elapsed),
                        KeyCode::Char('s') | KeyCode::Char('q') => return LoopAction::Quit,
                        KeyCode::Right => {
                            let _ = engine.seek(Duration::from_secs_f64((elapsed + 10.0).max(0.0)));
                        }
                        KeyCode::Left => {
                            let _ = engine.seek(Duration::from_secs_f64((elapsed - 10.0).max(0.0)));
                        }
                        KeyCode::Up => {
                            volume_db = (volume_db + 1.0).min(0.0);
                            let mut dsp = akroasis_core::config::DspConfig::default();
                            dsp.volume.level_db = volume_db;
                            engine.configure_dsp(dsp);
                        }
                        KeyCode::Down => {
                            volume_db -= 1.0;
                            let mut dsp = akroasis_core::config::DspConfig::default();
                            dsp.volume.level_db = volume_db;
                            engine.configure_dsp(dsp);
                        }
                        _ => {}
                    }
                }
            }

            evt = events.recv() => {
                match evt {
                    Ok(EngineEvent::TrackEnded { .. }) => return LoopAction::TrackEnded,
                    Ok(EngineEvent::PlaybackStopped) => return LoopAction::Stopped,
                    Err(broadcast::error::RecvError::Closed) => return LoopAction::Stopped,
                    _ => {}
                }
            }
        }
    }
}

async fn wait_for_track_end(events: &mut broadcast::Receiver<EngineEvent>) {
    loop {
        match events.recv().await {
            Ok(EngineEvent::TrackEnded { .. } | EngineEvent::PlaybackStopped) => break,
            Err(_) => break,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

fn print_track_info(path: &Path) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unknown>");
    println!("Playing: {name}");
}

fn print_progress(current: f64, total: f64) {
    let width = 24usize;
    let fraction = if total > 0.0 { current / total } else { 0.0 };
    let filled = ((fraction * width as f64) as usize).min(width);
    let bar: String = std::iter::repeat_n('█', filled)
        .chain(std::iter::repeat_n('░', width - filled))
        .collect();

    let fmt_time = |secs: f64| {
        let s = secs as u64;
        format!("{}:{:02}", s / 60, s % 60)
    };

    let line = format!(
        "\r  [▶ {} / {}] {} ",
        fmt_time(current),
        fmt_time(total),
        bar
    );
    let _ = io::stdout().write_all(line.as_bytes());
    let _ = io::stdout().flush();
}

// ---------------------------------------------------------------------------
// Metadata helpers
// ---------------------------------------------------------------------------

fn read_duration_secs(path: &Path) -> f64 {
    use akroasis_core::decode::metadata::read_track_metadata;
    read_track_metadata(path)
        .ok()
        .and_then(|m| m.duration)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("akroasis_core=warn".parse().unwrap_or_else(|_| unreachable!("static tracing directive is valid"))),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Play {
            paths,
            device,
            exclusive,
            json,
            volume,
            crossfade: _crossfade,
            replaygain,
            eq_preset: _eq_preset,
            crossfeed,
        } => {
            if paths.is_empty() {
                eprintln!("error: no paths provided");
                std::process::exit(1);
            }

            let queue = build_queue(&paths, json);
            if queue.is_empty() {
                if json {
                    emit_json(&JsonEvent::Error {
                        message: "no supported audio files found",
                    });
                } else {
                    eprintln!("error: no supported audio files found");
                }
                std::process::exit(1);
            }

            let config = build_engine_config(device, exclusive, volume, replaygain, crossfeed);
            let engine = Arc::new(Engine::new(config).map_err(|e| e.to_string())?);

            if json {
                play_json(engine, queue).await;
            } else {
                play_interactive(engine, queue).await;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn make_wav_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let sample_rate: u32 = 44100;
        let channels: u16 = 2;
        let n_samples: u32 = 100;
        let data_len = n_samples * 4; // 16-bit stereo
        let byte_rate = sample_rate * channels as u32 * 2;
        let block_align = channels * 2;

        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&block_align.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.extend(std::iter::repeat(0u8).take(data_len as usize));

        std::fs::write(&path, v).unwrap();
        path
    }

    #[test]
    fn collect_audio_files_finds_supported_extensions() {
        let dir = TempDir::new().unwrap();
        make_wav_file(dir.path(), "track01.wav");
        make_wav_file(dir.path(), "track02.flac"); // extension only — not real FLAC
        std::fs::write(dir.path().join("cover.jpg"), b"fake").unwrap();

        let mut files = Vec::new();
        collect_audio_files(dir.path(), &mut files, false);

        assert_eq!(files.len(), 2, "should find 2 audio files");
        assert!(
            files.iter().all(|f| {
                let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
                SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
            }),
            "all collected files have supported extensions"
        );
    }

    #[test]
    fn collect_audio_files_skips_unsupported() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"hello").unwrap();
        std::fs::write(dir.path().join("image.png"), b"png").unwrap();

        let mut files = Vec::new();
        collect_audio_files(dir.path(), &mut files, false);
        assert!(files.is_empty(), "no audio files should be found");
    }

    #[test]
    fn collect_audio_files_recurses_subdirectory() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("album");
        std::fs::create_dir(&sub).unwrap();
        make_wav_file(&sub, "t01.wav");
        make_wav_file(&sub, "t02.wav");
        make_wav_file(dir.path(), "t00.wav");

        let mut files = Vec::new();
        collect_audio_files(dir.path(), &mut files, false);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn build_queue_from_files_has_correct_length() {
        let dir = TempDir::new().unwrap();
        make_wav_file(dir.path(), "a.wav");
        make_wav_file(dir.path(), "b.wav");

        let queue = build_queue(&[dir.path().to_path_buf()], false);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn json_event_playback_started_is_valid_json() {
        let event = JsonEvent::PlaybackStarted {
            source: "/tmp/track.flac",
            signal_path: None,
        };
        let s = serde_json::to_string(&event).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["event"], "playback_started");
        assert_eq!(v["source"], "/tmp/track.flac");
    }

    #[test]
    fn json_event_position_is_valid_json() {
        let event = JsonEvent::Position {
            current_secs: 42.5,
            total_secs: 240.0,
        };
        let s = serde_json::to_string(&event).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["event"], "position");
        assert!((v["current_secs"].as_f64().unwrap() - 42.5).abs() < 1e-6);
    }

    #[test]
    fn json_event_track_ended_is_valid_json() {
        let event = JsonEvent::TrackEnded {
            source: "/tmp/done.flac",
        };
        let s = serde_json::to_string(&event).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["event"], "track_ended");
    }

    #[test]
    fn json_events_are_ndjson_compatible() {
        // Each event serializes to a single-line JSON object.
        let events: Vec<String> = vec![
            serde_json::to_string(&JsonEvent::PlaybackStarted {
                source: "/a.flac",
                signal_path: None,
            })
            .unwrap(),
            serde_json::to_string(&JsonEvent::Position {
                current_secs: 1.0,
                total_secs: 5.0,
            })
            .unwrap(),
            serde_json::to_string(&JsonEvent::PlaybackStopped).unwrap(),
        ];

        for line in &events {
            assert!(
                !line.contains('\n'),
                "NDJSON line must not contain newlines"
            );
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object(), "each NDJSON line must be a JSON object");
            assert!(
                parsed["event"].is_string(),
                "each object must have an 'event' field"
            );
        }
    }
}
