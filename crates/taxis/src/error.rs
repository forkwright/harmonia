use std::path::PathBuf;

use harmonia_db::DbError;
use snafu::Snafu;

/// Placeholder for when epignosis is implemented.
#[derive(Debug)]
pub struct EpignosisError(pub String);

impl std::fmt::Display for EpignosisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "epignosis: {}", self.0)
    }
}

impl std::error::Error for EpignosisError {}

// EpignosisError is Send + Sync because it only contains a String.
unsafe impl Send for EpignosisError {}
unsafe impl Sync for EpignosisError {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum TaxisError {
    #[snafu(display("scanner init failed: {source}"))]
    ScannerInit {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("watcher init failed for library {library}: {source}"))]
    WatcherInit {
        library: String,
        source: notify::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("watcher event error: {source}"))]
    WatcherEvent {
        source: notify::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("directory walk error at {path:?}: {source}"))]
    ScanWalk {
        path: PathBuf,
        source: walkdir::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("cannot determine media type for {path:?}"))]
    MediaDetect {
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unsupported format: {path:?}"))]
    UnsupportedFormat {
        path: PathBuf,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("tag read failed for {path:?}: {source}"))]
    TagRead {
        path: PathBuf,
        source: lofty::error::LoftyError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("metadata resolution failed for {path:?}: {source}"))]
    MetadataResolutionFailed {
        path: PathBuf,
        source: EpignosisError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("unknown template token '{token}' for {media_type}"))]
    UnknownToken {
        token: String,
        media_type: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("template token '{token}' missing FROM metadata (template: {template})"))]
    TemplateResolution {
        template: String,
        token: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("{operation} failed: {source_path:?} → {target_path:?}: {source}"))]
    FileOperation {
        operation: String,
        source_path: PathBuf,
        target_path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("conflict suffix exhausted at {target_path:?} (max {max})"))]
    ConflictResolution {
        target_path: PathBuf,
        max: usize,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("blocking task failed: {message}"))]
    BlockingTaskFailed {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error: {source}"))]
    Database {
        source: DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
