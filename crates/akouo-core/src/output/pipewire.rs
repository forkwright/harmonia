use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::error::OutputError;

const PIPEWIRE_SETTINGS: &str = "settings";
const FORCE_RATE_KEY: &str = "clock.force-rate";
const VERIFY_TIMEOUT: Duration = Duration::from_millis(1_500);
const VERIFY_INTERVAL: Duration = Duration::from_millis(50);

/// Force PipeWire's graph clock rate and verify it took effect.
///
/// This intentionally does not set `clock.force-quantum`; forcing quantum can
/// desynchronize cpal/rodio mixer threads and cause high-rate underruns.
pub fn force_pipewire_rate(rate: u32) -> Result<(), OutputError> {
    set_pipewire_rate(rate)?;
    verify_pipewire_rate(rate)
}

/// Forces PipeWire rate only when `pw-metadata` is available on this host.
///
/// Missing `pw-metadata` means either PipeWire is absent or the runtime package is
/// incomplete. In both cases playback should continue through cpal's normal path.
pub fn force_pipewire_rate_if_available(rate: u32) -> Result<bool, OutputError> {
    if !command_available("pw-metadata") {
        return Ok(false);
    }
    force_pipewire_rate(rate)?;
    Ok(true)
}

/// Reset PipeWire's forced clock rate so other applications are not stranded.
pub fn reset_pipewire_rate() -> Result<(), OutputError> {
    set_pipewire_rate(0)
}

/// Enumerates discrete hardware sample rates from ALSA USB stream descriptors.
pub fn alsa_hardware_sample_rates() -> Result<Vec<u32>, OutputError> {
    let root = Path::new("/proc/asound");
    let Ok(cards) = fs::read_dir(root) else {
        return Ok(Vec::new());
    };

    let mut rates = BTreeSet::new();
    for card in cards {
        let card = card.map_err(|e| OutputError::StreamError {
            message: format!("failed to read /proc/asound entry: {e}"),
        })?;
        let card_name = card.file_name();
        if !card_name.to_string_lossy().starts_with("card") {
            continue;
        }

        let stream = card.path().join("stream0");
        match fs::read_to_string(&stream) {
            Ok(contents) => rates.extend(parse_alsa_stream_sample_rates(&contents)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(OutputError::StreamError {
                    message: format!("failed to read {}: {e}", stream.display()),
                });
            }
        }
    }

    Ok(rates.into_iter().collect())
}

fn set_pipewire_rate(rate: u32) -> Result<(), OutputError> {
    let rate = rate.to_string();
    let output = Command::new("pw-metadata")
        .args([OsStr::new("-n"), OsStr::new(PIPEWIRE_SETTINGS)])
        .args([
            OsStr::new("0"),
            OsStr::new(FORCE_RATE_KEY),
            OsStr::new(rate.as_str()),
        ])
        .output()
        .map_err(|e| OutputError::StreamError {
            message: format!("failed to execute pw-metadata: {e}"),
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(OutputError::StreamError {
        message: format!(
            "pw-metadata failed to set {FORCE_RATE_KEY}: {}",
            command_output_text(&output)
        ),
    })
}

fn verify_pipewire_rate(expected: u32) -> Result<(), OutputError> {
    let deadline = Instant::now() + VERIFY_TIMEOUT;
    loop {
        match read_pipewire_rate()? {
            Some(actual) if actual == expected => return Ok(()),
            _ if Instant::now() >= deadline => {
                return Err(OutputError::StreamError {
                    message: format!(
                        "PipeWire did not apply {FORCE_RATE_KEY}={expected} within {VERIFY_TIMEOUT:?}"
                    ),
                });
            }
            _ => std::thread::sleep(VERIFY_INTERVAL),
        }
    }
}

fn read_pipewire_rate() -> Result<Option<u32>, OutputError> {
    let output = Command::new("pw-metadata")
        .args([OsStr::new("-n"), OsStr::new(PIPEWIRE_SETTINGS)])
        .args([OsStr::new("0"), OsStr::new(FORCE_RATE_KEY)])
        .output()
        .map_err(|e| OutputError::StreamError {
            message: format!("failed to execute pw-metadata: {e}"),
        })?;

    if !output.status.success() {
        return Err(OutputError::StreamError {
            message: format!(
                "pw-metadata failed to read {FORCE_RATE_KEY}: {}",
                command_output_text(&output)
            ),
        });
    }

    let text = command_output_text(&output);
    Ok(parse_pipewire_rate(&text))
}

fn command_available(command: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|path| path.join(command).is_file()))
}

fn command_output_text(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}").trim().to_owned()
}

fn parse_pipewire_rate(text: &str) -> Option<u32> {
    for line in text.lines().filter(|line| line.contains(FORCE_RATE_KEY)) {
        if let Some(rate) = last_u32_in(line) {
            return Some(rate);
        }
    }

    text.trim().parse().ok()
}

fn parse_alsa_stream_sample_rates(contents: &str) -> Vec<u32> {
    let mut rates = BTreeSet::new();
    for line in contents.lines() {
        let Some(rate_list) = line.trim_start().strip_prefix("Rates:") else {
            continue;
        };
        if rate_list.contains("continuous") {
            continue;
        }
        for rate in u32_values(rate_list) {
            rates.insert(rate);
        }
    }
    rates.into_iter().collect()
}

fn last_u32_in(text: &str) -> Option<u32> {
    u32_values(text).last()
}

fn u32_values(text: &str) -> impl Iterator<Item = u32> + '_ {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pipewire_metadata_value_line() {
        let text = "update: id:0 key:'clock.force-rate' type:'Spa:String:JSON' value:'96000'";
        assert_eq!(parse_pipewire_rate(text), Some(96000));
    }

    #[test]
    fn parses_plain_pipewire_metadata_value() {
        assert_eq!(parse_pipewire_rate("48000\n"), Some(48000));
    }

    #[test]
    fn parses_discrete_alsa_stream_rates() {
        let stream = r#"
Playback:
  Altset 1
    Format: S24_3LE
    Channels: 2
    Rates: 44100, 48000, 88200, 96000, 176400, 192000
  Altset 2
    Rates: 352800, 384000
"#;
        assert_eq!(
            parse_alsa_stream_sample_rates(stream),
            vec![44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000]
        );
    }

    #[test]
    fn skips_continuous_alsa_rate_ranges() {
        let stream = "Rates: 44100 - 192000 (continuous)\n";
        assert!(parse_alsa_stream_sample_rates(stream).is_empty());
    }

    #[test]
    #[ignore = "requires PipeWire pw-metadata and ALSA audio hardware"]
    fn force_pipewire_rate_roundtrip() {
        let rate = alsa_hardware_sample_rates()
            .unwrap()
            .into_iter()
            .find(|&rate| rate != 0)
            .unwrap_or(48_000);

        force_pipewire_rate(rate).unwrap();
        assert_eq!(read_pipewire_rate().unwrap(), Some(rate));
        reset_pipewire_rate().unwrap();
    }
}
