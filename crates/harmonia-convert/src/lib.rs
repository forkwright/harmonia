pub mod calibre;
pub mod error;
pub mod kepubify;
pub mod pandoc;

use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

pub use error::ConvertError;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::instrument;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ConvertOptions {
    pub device_profile: Option<DeviceProfile>,
    pub timeout: Duration,
    pub extra_args: Vec<String>,
}

impl ConvertOptions {
    pub fn new() -> Self {
        Self {
            device_profile: None,
            timeout: Duration::from_secs(300),
            extra_args: Vec::new(),
        }
    }
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceProfile {
    Kindle,
    Kobo,
    Generic,
}

#[derive(Debug)]
pub struct ConvertOutcome {
    pub output_path: std::path::PathBuf,
}

#[instrument(skip(opts))]
pub async fn convert_ebook(
    input: &Path,
    output: &Path,
    opts: ConvertOptions,
) -> Result<ConvertOutcome, ConvertError> {
    if !input.exists() {
        return Err(error::InputMissingSnafu {
            path: input.to_path_buf(),
        }
        .build());
    }

    if output.file_name().is_none() {
        return Err(error::OutputPathInvalidSnafu {
            path: output.to_path_buf(),
        }
        .build());
    }

    let input_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let output_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    if input_ext.as_deref() == Some("docx") || input_ext.as_deref() == Some("odt") {
        if output_ext.as_deref() == Some("epub") {
            pandoc::convert(input, output, &opts).await?;
        } else {
            calibre::convert(input, output, &opts).await?;
        }
    } else if input_ext.as_deref() == Some("epub")
        && output
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".kepub.epub"))
    {
        kepubify::convert(input, output, &opts).await?;
    } else {
        calibre::convert(input, output, &opts).await?;
    }

    Ok(ConvertOutcome {
        output_path: output.to_path_buf(),
    })
}

pub(crate) async fn run_subprocess(
    binary: &'static str,
    args: &[String],
    opts: &ConvertOptions,
) -> Result<std::process::Output, ConvertError> {
    let path_env =
        std::env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/local/bin:/usr/bin:/bin"));

    let mut cmd = Command::new(binary);
    cmd.args(args)
        .env_clear()
        .env("PATH", &path_env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let result = timeout(opts.timeout, cmd.output()).await;

    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e))
            if e.kind() == std::io::ErrorKind::NotFound
                || e.kind() == std::io::ErrorKind::NotADirectory =>
        {
            Err(error::BinaryNotFoundSnafu { binary }.build())
        }
        Ok(Err(e)) => Err(error::SubprocessFailedSnafu {
            stderr: e.to_string(),
        }
        .build()),
        Err(_) => Err(error::SubprocessTimeoutSnafu.build()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn error_variants_display() {
        let e1 = error::InputMissingSnafu {
            path: PathBuf::from("/foo"),
        }
        .build();
        assert!(e1.to_string().contains("input file not found"));

        let e2 = error::OutputPathInvalidSnafu {
            path: PathBuf::from("/foo"),
        }
        .build();
        assert!(e2.to_string().contains("output path invalid"));

        let e3 = error::SubprocessFailedSnafu {
            stderr: "broken".to_string(),
        }
        .build();
        assert!(e3.to_string().contains("subprocess failed"));

        let e4 = error::SubprocessTimeoutSnafu.build();
        assert!(e4.to_string().contains("subprocess timed out"));

        let e5 = error::UnsupportedFormatSnafu {
            format: "xyz".to_string(),
        }
        .build();
        assert!(e5.to_string().contains("unsupported format"));

        let e6 = error::BinaryNotFoundSnafu { binary: "foo" }.build();
        assert!(e6.to_string().contains("binary not found"));
    }

    #[tokio::test]
    async fn input_missing_error() {
        let opts = ConvertOptions::new();
        let result = convert_ebook(
            Path::new("/nonexistent/path/to/book.epub"),
            Path::new("/tmp/out.epub"),
            opts,
        )
        .await;
        assert!(
            matches!(result, Err(ConvertError::InputMissing { .. })),
            "expected InputMissing, got {result:?}"
        );
    }

    #[tokio::test]
    async fn output_path_invalid_error() {
        let temp_dir = std::env::temp_dir();
        let input = temp_dir.join("harmonia_convert_test_input.txt");
        std::fs::write(&input, "test").unwrap();
        let opts = ConvertOptions::new();
        let result = convert_ebook(&input, Path::new("/"), opts).await;
        let _ = std::fs::remove_file(&input);
        assert!(
            matches!(result, Err(ConvertError::OutputPathInvalid { .. })),
            "expected OutputPathInvalid, got {result:?}"
        );
    }

    #[tokio::test]
    async fn binary_not_found_with_empty_path() {
        let original = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", "/dev/null") };

        let temp_dir = std::env::temp_dir();
        let input = temp_dir.join("harmonia_convert_test_input.txt");
        std::fs::write(&input, "test").unwrap();
        let output = temp_dir.join("harmonia_convert_test_output.epub");
        let opts = ConvertOptions::new();

        let result = convert_ebook(&input, &output, opts).await;

        if let Some(orig) = original {
            unsafe { std::env::set_var("PATH", orig) };
        } else {
            unsafe { std::env::remove_var("PATH") };
        }

        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&output);

        assert!(
            matches!(
                result,
                Err(ConvertError::BinaryNotFound {
                    binary: "ebook-convert",
                    ..
                })
            ),
            "expected BinaryNotFound for ebook-convert, got {result:?}"
        );
    }

    #[tokio::test]
    #[ignore = "requires ebook-convert on PATH — see #214"]
    async fn real_calibre_conversion() {
        let temp_dir = std::env::temp_dir();
        let input = temp_dir.join("harmonia_convert_real_test.txt");
        std::fs::write(
            &input,
            "Hello, this is a test book.\n\nChapter 1\n\nIt was a dark and stormy night.\n",
        )
        .unwrap();
        let output = temp_dir.join("harmonia_convert_real_test.epub");
        let opts = ConvertOptions::new();

        let result = convert_ebook(&input, &output, opts).await;

        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&output);

        assert!(result.is_ok(), "calibre conversion failed: {result:?}");
    }
}
