//! Chromaprint fingerprinting via the external fpcalc binary.

use std::path::Path;
use std::process::Stdio;

use serde::Deserialize;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;

use crate::error::{
    EpignosisError, FingerprintFailedSnafu, FingerprintOutputParseSnafu, FingerprintProcessSnafu,
};

/// Name of the chromaprint fingerprinting binary, resolved from `PATH`.
pub(crate) const FPCALC_BINARY: &str = "fpcalc";

/// Raw fingerprint as reported by `fpcalc -json`.
// WHY: pure data — chromaprint fingerprint plus track duration in seconds.
#[derive(Debug, Deserialize)]
pub(crate) struct RawFingerprint {
    pub fingerprint: String,
    pub duration: f64,
}

/// Runs `binary -json <file_path>` and parses the fingerprint output.
///
/// The binary is probed at runtime: a missing executable yields a
/// `FingerprintFailed` error naming the dependency instead of a panic or a
/// compile-time stub.
pub(crate) async fn compute(
    binary: &str,
    file_path: &Path,
    ct: &CancellationToken,
) -> Result<RawFingerprint, EpignosisError> {
    let mut command = tokio::process::Command::new(binary);
    command
        .arg("-json")
        .arg(file_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // WHY: a cancelled or dropped future must not leave an fpcalc
        // process running against a possibly large media file.
        .kill_on_drop(true);

    let child = match command.spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return FingerprintFailedSnafu {
                path: file_path.to_path_buf(),
                message: format!(
                    "{binary} binary not found on PATH; install chromaprint to enable audio fingerprinting"
                ),
            }
            .fail();
        }
        Err(e) => {
            return Err(e).context(FingerprintProcessSnafu {
                path: file_path.to_path_buf(),
            });
        }
    };

    let output = tokio::select! {
        output = child.wait_with_output() => output.context(FingerprintProcessSnafu {
            path: file_path.to_path_buf(),
        })?,
        _ = ct.cancelled() => {
            return FingerprintFailedSnafu {
                path: file_path.to_path_buf(),
                message: "fingerprinting cancelled".to_string(),
            }
            .fail();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return FingerprintFailedSnafu {
            path: file_path.to_path_buf(),
            message: format!("{binary} exited with {}: {}", output.status, stderr.trim()),
        }
        .fail();
    }

    parse_output(file_path, &output.stdout)
}

/// Parses `fpcalc -json` stdout into a raw fingerprint.
pub(crate) fn parse_output(
    file_path: &Path,
    stdout: &[u8],
) -> Result<RawFingerprint, EpignosisError> {
    serde_json::from_slice(stdout).context(FingerprintOutputParseSnafu {
        path: file_path.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_output_reads_fingerprint_and_duration() {
        let stdout = br#"{"duration": 123.46, "fingerprint": "AQAAA1example"}"#;
        let raw = parse_output(Path::new("/audio/track.flac"), stdout).unwrap();
        assert_eq!(raw.fingerprint, "AQAAA1example");
        assert!((raw.duration - 123.46).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_output_rejects_malformed_json() {
        let err = parse_output(Path::new("/audio/track.flac"), b"ERROR: boom").unwrap_err();
        assert!(matches!(err, EpignosisError::FingerprintOutputParse { .. }));
    }

    #[tokio::test]
    async fn missing_binary_is_a_clean_runtime_error() {
        let ct = CancellationToken::new();
        let err = compute(
            "fpcalc-test-binary-that-does-not-exist",
            Path::new("/audio/track.flac"),
            &ct,
        )
        .await
        .unwrap_err();

        match err {
            EpignosisError::FingerprintFailed { message, .. } => {
                assert!(
                    message.contains("not found on PATH"),
                    "error must name the missing dependency: {message}"
                );
            }
            other => panic!("expected FingerprintFailed, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stub_binary_output_is_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let stub = crate::test_support::write_stub_script(
            dir.path(),
            "fpcalc-stub",
            "#!/bin/sh\nprintf '{\"duration\": 42.5, \"fingerprint\": \"AQFAKE\"}'\n",
        );
        let ct = CancellationToken::new();

        let raw = compute(stub.to_str().unwrap(), Path::new("/audio/track.flac"), &ct)
            .await
            .unwrap();

        assert_eq!(raw.fingerprint, "AQFAKE");
        assert!((raw.duration - 42.5).abs() < f64::EPSILON);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nonzero_exit_is_a_clean_runtime_error() {
        let dir = tempfile::tempdir().unwrap();
        let stub = crate::test_support::write_stub_script(
            dir.path(),
            "fpcalc-fail",
            "#!/bin/sh\necho 'ERROR: could not decode' >&2\nexit 3\n",
        );
        let ct = CancellationToken::new();

        let err = compute(stub.to_str().unwrap(), Path::new("/audio/track.flac"), &ct)
            .await
            .unwrap_err();

        match err {
            EpignosisError::FingerprintFailed { message, .. } => {
                assert!(message.contains("exited with"), "{message}");
                assert!(message.contains("could not decode"), "{message}");
            }
            other => panic!("expected FingerprintFailed, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_interrupts_a_running_fpcalc() {
        let dir = tempfile::tempdir().unwrap();
        let stub = crate::test_support::write_stub_script(
            dir.path(),
            "fpcalc-hang",
            "#!/bin/sh\nexec sleep 30\n",
        );
        let ct = CancellationToken::new();
        ct.cancel();

        let err = compute(stub.to_str().unwrap(), Path::new("/audio/track.flac"), &ct)
            .await
            .unwrap_err();

        match err {
            EpignosisError::FingerprintFailed { message, .. } => {
                assert!(message.contains("cancelled"), "{message}");
            }
            other => panic!("expected FingerprintFailed, got {other:?}"),
        }
    }
}
