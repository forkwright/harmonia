use harmonia_db::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum KomideError {
    #[snafu(display("feed parse failed: {source}"))]
    FeedParse {
        source: feed_rs::parser::ParseFeedError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("feed fetch failed for {url}: {source}"))]
    FeedFetch {
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("episode download failed FROM {url}: {source}"))]
    EpisodeDownload {
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("episode file I/O failed for {path}: {source}"))]
    EpisodeIo {
        path: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("invalid feed URL: {url}"))]
    InvalidUrl {
        url: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("feed not found: {feed_id}"))]
    FeedNotFound {
        feed_id: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("retention error: {message}"))]
    RetentionViolation {
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
