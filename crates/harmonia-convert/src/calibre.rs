use std::path::Path;

use crate::{ConvertError, ConvertOptions, DeviceProfile, error};

pub async fn convert(
    input: &Path,
    output: &Path,
    opts: &ConvertOptions,
) -> Result<(), ConvertError> {
    let mut args = vec![input.display().to_string(), output.display().to_string()];

    if let Some(profile) = opts.device_profile {
        let profile_str = match profile {
            DeviceProfile::Kindle => "kindle",
            DeviceProfile::Kobo => "kobo",
            DeviceProfile::Generic => "generic_eink",
        };
        args.push(format!("--output-profile={profile_str}"));
    }

    for arg in &opts.extra_args {
        args.push(arg.clone());
    }

    let output_result = crate::run_subprocess("ebook-convert", &args, opts).await?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr).to_string();
        return Err(error::SubprocessFailedSnafu { stderr }.build());
    }

    Ok(())
}
