use apotheke::DbError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum SearchIndexerError {
    #[snafu(display("HTTP request to indexer {url} failed"))]
    HttpRequest {
        url: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to parse Torznab/Newznab XML response FROM {url}"))]
    ParseResponse {
        url: String,
        error: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("request to {url} was cancelled"))]
    Cancelled {
        url: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("response FROM {url} exceeds {limit} byte cap (got at least {size} bytes)"))]
    ResponseTooLarge {
        url: String,
        size: u64,
        limit: u64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("refusing to fetch {url}: {reason}"))]
    UnsafeUrl {
        url: String,
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} returned auth failure (bad API key)"))]
    AuthFailed {
        indexer_id: i64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} rate limited — retry after {retry_after_seconds:?}s"))]
    RateLimited {
        indexer_id: i64,
        retry_after_seconds: Option<u64>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("CF bypass proxy not available for {url}"))]
    NoCfBypass {
        url: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("caps negotiation failed for indexer {indexer_id}"))]
    CapsUnavailable {
        indexer_id: i64,
        source: Box<SearchIndexerError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} was not found"))]
    IndexerNotFound {
        indexer_id: i64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Byparr did not respond within {timeout}s for {url}"))]
    CfProxyTimeout {
        url: String,
        timeout: u32,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Byparr returned error for {url}: [{status}] {message}"))]
    CfProxyError {
        url: String,
        status: String,
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("CF cookies expired for indexer {indexer_name}"))]
    CfCookieExpired {
        indexer_name: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("database error: {source}"))]
    Database {
        source: DbError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to load Cardigann definition {path}: {reason}"))]
    DefinitionLoad {
        path: String,
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Cardigann definition {definition_id} is invalid: {reason}"))]
    DefinitionInvalid {
        definition_id: String,
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("Cardigann definition {definition_id} needs unsupported feature: {feature}"))]
    DefinitionUnsupported {
        definition_id: String,
        feature: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display(
        "no loaded Cardigann definition matches indexer {indexer_id} (url {url:?}; \
         set the indexer url to a definition id or a site link)"
    ))]
    DefinitionNotFound {
        indexer_id: i64,
        url: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display(
        "Cardigann definition {definition_id} uses login method {method:?}; supported \
         methods are 'none', 'cookie', 'form', 'post', and 'get' — expose the tracker \
         via a Torznab sidecar instead"
    ))]
    LoginUnsupported {
        definition_id: String,
        method: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    // NOTE: `reason` carries selector diagnostics or the site's own error
    // message — never credentials, request bodies, or full login URLs.
    #[snafu(display(
        "Cardigann login failed for indexer {indexer_id} (definition {definition_id}): \
         {reason}"
    ))]
    LoginFailed {
        definition_id: String,
        indexer_id: i64,
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display(
        "Cardigann definition {definition_id} uses cookie login; set indexer \
         {indexer_id}'s api_key field to the session cookie"
    ))]
    CookieAuthRequired {
        definition_id: String,
        indexer_id: i64,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("indexer {indexer_id} settings_json is not valid JSON: {reason}"))]
    SettingsJsonInvalid {
        indexer_id: i64,
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display(
        "indexer {indexer_id} settings override rejected for Cardigann definition \
         {definition_id}: {reason}"
    ))]
    SettingsInvalid {
        definition_id: String,
        indexer_id: i64,
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
