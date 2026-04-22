use std::path::PathBuf;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum ConvertError {
    #[snafu(display("input file not found: {}", path.display()))]
    InputMissing {
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("output path invalid: {}", path.display()))]
    OutputPathInvalid {
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subprocess failed: {stderr}"))]
    SubprocessFailed {
        stderr: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subprocess timed out"))]
    SubprocessTimeout {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unsupported format: {format}"))]
    UnsupportedFormat {
        format: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("binary not found: {binary}"))]
    BinaryNotFound {
        binary: &'static str,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
