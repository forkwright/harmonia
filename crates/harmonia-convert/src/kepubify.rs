use std::path::Path;

use crate::{ConvertError, ConvertOptions, error};

pub async fn convert(
    input: &Path,
    output: &Path,
    opts: &ConvertOptions,
) -> Result<(), ConvertError> {
    if !input.exists() {
        return Err(error::InputMissingSnafu {
            path: input.to_path_buf(),
        }
        .build());
    }

    let mut args = vec![
        input.display().to_string(),
        "-o".to_string(),
        output.display().to_string(),
    ];

    for arg in &opts.extra_args {
        args.push(arg.clone());
    }

    let output_result = crate::run_subprocess("kepubify", &args, opts).await?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr).to_string();
        return Err(error::SubprocessFailedSnafu { stderr }.build());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::convert;
    use crate::{ConvertError, ConvertOptions};

    #[tokio::test]
    async fn convert_returns_input_missing_when_input_absent() {
        let opts = ConvertOptions::new();
        let result = convert(
            Path::new("/nonexistent/path/to/book.epub"),
            Path::new("/tmp/out.kepub.epub"),
            &opts,
        )
        .await;
        assert!(
            matches!(result, Err(ConvertError::InputMissing { .. })),
            "expected InputMissing, got {result:?}"
        );
    }

    #[tokio::test]
    async fn convert_returns_binary_not_found_when_binary_absent() {
        let original = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", "/dev/null") };

        let temp_dir = std::env::temp_dir();
        let input = temp_dir.join("harmonia_convert_kepubify_test_input.txt");
        std::fs::write(&input, "test").unwrap();
        let output = temp_dir.join("harmonia_convert_kepubify_test_output.kepub.epub");
        let opts = ConvertOptions::new();

        let result = convert(&input, &output, &opts).await;

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
                    binary: "kepubify",
                    ..
                })
            ),
            "expected BinaryNotFound for kepubify, got {result:?}"
        );
    }
}
