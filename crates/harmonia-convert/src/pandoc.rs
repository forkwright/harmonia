use std::path::Path;

use crate::{ConvertError, ConvertOptions, error};

pub async fn convert(
    input: &Path,
    output: &Path,
    opts: &ConvertOptions,
) -> Result<(), ConvertError> {
    let mut args = vec![
        input.display().to_string(),
        "-o".to_string(),
        output.display().to_string(),
    ];

    for arg in &opts.extra_args {
        args.push(arg.clone());
    }

    let output_result = crate::run_subprocess("pandoc", &args, opts).await?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr).to_string();
        return Err(error::SubprocessFailedSnafu { stderr }.build());
    }

    Ok(())
}
