use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[expect(dead_code, reason = "variants used as subsystems gain route handlers")]
pub enum HostError {
    #[snafu(display("configuration error: {source}"))]
    Config {
        source: horismos::HorismosError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error: {source}"))]
    Database {
        source: harmonia_db::DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("scanner error: {source}"))]
    Scanner {
        #[snafu(source(from(taxis::error::TaxisError, Box::new)))]
        source: Box<taxis::error::TaxisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("feed scheduler error: {source}"))]
    FeedScheduler {
        #[snafu(source(from(komide::KomideError, Box::new)))]
        source: Box<komide::KomideError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("server error: {source}"))]
    Server {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("auth error: {source}"))]
    Auth {
        #[snafu(source(from(exousia::ExousiaError, Box::new)))]
        source: Box<exousia::ExousiaError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("tracing init error: {message}"))]
    Tracing {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer error: {source}"))]
    Indexer {
        #[snafu(source(from(zetesis::ZetesisError, Box::new)))]
        source: Box<zetesis::ZetesisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("download engine error: {source}"))]
    DownloadEngine {
        #[snafu(source(from(ergasia::ErgasiaError, Box::new)))]
        source: Box<ergasia::ErgasiaError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("download queue error: {source}"))]
    DownloadQueue {
        #[snafu(source(from(syntaxis::SyntaxisError, Box::new)))]
        source: Box<syntaxis::SyntaxisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("request service error: {source}"))]
    RequestService {
        #[snafu(source(from(aitesis::AitesisError, Box::new)))]
        source: Box<aitesis::AitesisError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("external integration error: {source}"))]
    ExternalIntegration {
        #[snafu(source(from(syndesmos::SyndesmodError, Box::new)))]
        source: Box<syndesmos::SyndesmodError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("subtitle service error: {source}"))]
    SubtitleService {
        #[snafu(source(from(prostheke::ProsthekeError, Box::new)))]
        source: Box<prostheke::ProsthekeError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("audio engine error: {source}"))]
    AudioEngine {
        #[snafu(source(from(akouo_core::EngineError, Box::new)))]
        source: Box<akouo_core::EngineError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("render error: {message}"))]
    Render {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
