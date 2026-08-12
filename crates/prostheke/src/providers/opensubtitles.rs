//! OpenSubtitles.com REST API v1 provider.

use std::path::Path;
use std::time::{Duration, Instant};

use aggelmata::{MediaId, MediaType};
use horismos::OpenSubtitlesConfig;
use serde::Deserialize;
use snafu::ResultExt;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

use crate::error::{
    AcquisitionFailedSnafu, DownloadFailedSnafu, InvalidProviderIdSnafu, ProsthekeError,
    ProviderDownSnafu,
};
use crate::providers::SubtitleProvider;
use crate::rate_limit::RateLimiter;
use crate::types::{SubtitleMatch, SubtitleProviderId};

const BASE_URL: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = "Harmonia/1.0";
/// Fallback download cap when the provider is exercised without a config.
const DEFAULT_MAX_DOWNLOAD_BYTES: u64 = 10 * 1024 * 1024;
/// Cap on the buffered `/subtitles` search and `/download-link` JSON bodies.
///
/// WHY: these are small API responses (a handful of KB in practice), never
/// the subtitle file itself — a fixed cap FROM the same 10 MiB family used
/// elsewhere in prostheke/epignosis is generous headroom without wiring a
/// dedicated config field for it.
const MAX_API_RESPONSE_BYTES: u64 = 10 * 1024 * 1024;
// NOTE: OpenSubtitles JWTs are valid for roughly 24 hours; half that is a
// conservative reuse window that keeps a stale token from ever reaching the
// API (same shape as epignosis's TVDB token cache).
const TOKEN_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Computes the OpenSubtitles-specific file hash.
///
/// The hash is the wrapping sum of the file size and the first/last 64 KB of
/// the file, treating each 8-byte chunk as a little-endian u64. For files
/// smaller than 128 KB the two windows overlap; bytes between the windows
/// never affect the hash.
pub fn compute_file_hash(path: &Path) -> std::io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};

    const CHUNK_SIZE: usize = 64 * 1024;
    const WORD_SIZE: usize = 8;

    let mut file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    let mut hash: u64 = file_size;

    let mut tmp = [0u8; WORD_SIZE];

    // Read first 64 KB.
    for _ in 0..(CHUNK_SIZE / WORD_SIZE) {
        match file.read_exact(&mut tmp) {
            Ok(()) => hash = hash.wrapping_add(u64::from_le_bytes(tmp)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
    }

    // Read last 64 KB.
    let tail_offset = file_size.saturating_sub(u64::try_from(CHUNK_SIZE).unwrap_or_default()); // WHY: CHUNK_SIZE is a compile-time constant that fits in u64
    file.seek(SeekFrom::Start(tail_offset))?;

    for _ in 0..(CHUNK_SIZE / WORD_SIZE) {
        match file.read_exact(&mut tmp) {
            Ok(()) => hash = hash.wrapping_add(u64::from_le_bytes(tmp)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
    }

    Ok(format!("{hash:016x}"))
}

/// OpenSubtitles.com REST API v1 client.
///
/// Returns empty results when not configured (no API key). This satisfies the
/// acceptance criterion: "Provider unconfigured → empty results, not error."
pub struct OpenSubtitlesProvider {
    config: Option<OpenSubtitlesConfig>,
    client: reqwest::Client,
    base_url: String,
    /// Bounds requests to `OpenSubtitlesConfig::rate_limit_per_second`.
    /// `None` when unconfigured — `search`/`download` return before ever
    /// reaching a network call in that case, so no limiter is needed.
    rate_limiter: Option<RateLimiter>,
    // NOTE: interior mutability — SubtitleProvider methods take &self.
    // Holds the bearer JWT and the instant it stops being valid; `None`
    // until the first credentialed download logs in.
    token: RwLock<Option<(String, Instant)>>,
}

impl OpenSubtitlesProvider {
    pub fn new(config: Option<OpenSubtitlesConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config
        let rate_limiter = config
            .as_ref()
            .map(|c| RateLimiter::new(c.rate_limit_per_second));
        Self {
            config,
            client,
            base_url: BASE_URL.to_string(),
            rate_limiter,
            token: RwLock::new(None),
        }
    }

    /// Waits for the configured request budget before an OpenSubtitles API call.
    async fn throttle(&self) {
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }
    }

    /// Test-only seam: point the API requests (search, download-link) at a
    /// local mock server instead of the real OpenSubtitles API.
    #[cfg(test)]
    fn with_base_url(config: Option<OpenSubtitlesConfig>, base_url: String) -> Self {
        let mut provider = Self::new(config);
        provider.base_url = base_url;
        provider
    }

    fn api_key(&self) -> Option<&str> {
        self.config.as_ref().map(|c| c.api_key.as_str())
    }

    /// Username/password pair, present only when BOTH are configured and
    /// non-empty. Absent credentials keep the provider anonymous
    /// (quota-limited) — the login flow is never attempted.
    fn credentials(&self) -> Option<(&str, &str)> {
        let config = self.config.as_ref()?;
        match (config.username.as_deref(), config.password.as_deref()) {
            (Some(user), Some(pass)) if !user.is_empty() && !pass.is_empty() => Some((user, pass)),
            _ => None,
        }
    }

    /// Returns the cached bearer JWT, logging in via `POST /login` only on a
    /// cache miss or after the reuse window has elapsed. `None` when no
    /// credentials are configured — the download stays anonymous.
    async fn bearer_token(&self, api_key: &str) -> Result<Option<String>, ProsthekeError> {
        let Some((username, password)) = self.credentials() else {
            return Ok(None);
        };

        if let Some((token, valid_until)) = self.token.read().await.as_ref()
            && Instant::now() < *valid_until
        {
            return Ok(Some(token.clone()));
        }

        let mut slot = self.token.write().await;
        // WHY: double-checked — a concurrent caller may have refreshed the
        // token while this one waited for the write lock; holding the write
        // lock across the login makes the refresh single-flight.
        if let Some((token, valid_until)) = slot.as_ref()
            && Instant::now() < *valid_until
        {
            return Ok(Some(token.clone()));
        }

        let body = serde_json::json!({ "username": username, "password": password });
        self.throttle().await;
        let response = self
            .client
            .post(format!("{}/login", self.base_url))
            .header("Api-Key", api_key)
            .json(&body)
            .send()
            .await
            .context(ProviderDownSnafu)?;

        if !response.status().is_success() {
            let status = response.status();
            return DownloadFailedSnafu {
                detail: format!("HTTP {status} FROM login endpoint"),
            }
            .fail();
        }

        let bytes = read_body_capped(response, MAX_API_RESPONSE_BYTES).await?;
        let parsed: LoginResponse = serde_json::from_slice(&bytes).map_err(|e| {
            DownloadFailedSnafu {
                detail: format!("invalid login response JSON: {e}"),
            }
            .build()
        })?;

        *slot = Some((parsed.token.clone(), Instant::now() + TOKEN_TTL));
        Ok(Some(parsed.token))
    }

    /// Drops the cached bearer JWT.
    ///
    /// WHY: a 401 means the server no longer honors the cached token before
    /// its local TTL elapsed; clearing it makes the next call re-authenticate
    /// instead of failing until the TTL expires.
    async fn invalidate_token(&self) {
        *self.token.write().await = None;
    }

    /// POSTs `/download` for `file_id`, attaching the bearer JWT when one is
    /// available (logged-in downloads get the account's quota).
    async fn request_download_link(
        &self,
        api_key: &str,
        bearer: Option<&str>,
        file_id: u64,
    ) -> Result<reqwest::Response, ProsthekeError> {
        self.throttle().await;
        let mut request = self
            .client
            .post(format!("{}/download", self.base_url))
            .header("Api-Key", api_key)
            .json(&serde_json::json!({ "file_id": file_id }));
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }
        request.send().await.context(ProviderDownSnafu)
    }
}

// ── API response shapes ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<SubtitleData>,
}

#[derive(Debug, Deserialize)]
struct SubtitleData {
    id: String,
    attributes: SubtitleAttributes,
}

#[derive(Debug, Deserialize)]
struct SubtitleAttributes {
    language: String,
    #[serde(default)]
    hearing_impaired: bool,
    #[serde(default)]
    foreign_parts_only: bool,
    #[serde(default)]
    moviehash_match: bool,
    files: Vec<SubtitleFile>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubtitleFile {
    file_id: u64,
}

#[derive(Debug, Deserialize)]
struct DownloadResponse {
    link: String,
    #[serde(default)]
    file_name: String,
}

// NOTE: the login response also carries `base_url` and user quota fields;
// only the JWT is consumed.
#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

// ── Scoring ───────────────────────────────────────────────────────────────────

/// Score a subtitle result. Hash match is highest quality.
fn score_result(attr: &SubtitleAttributes, requested_lang: &str) -> f64 {
    let base = if attr.moviehash_match { 1.0 } else { 0.75 };

    // Exact language match is full score; otherwise reduce slightly.
    let lang_match = if attr.language == requested_lang {
        1.0
    } else {
        0.9
    };

    base * lang_match
}

// ── Download safety ───────────────────────────────────────────────────────────

/// Validate a provider-supplied download link before fetching it.
///
/// WHY: the link comes FROM the OpenSubtitles API response body — a
/// compromised or spoofed response must not be able to redirect the fetch at
/// internal hosts (SSRF) or downgrade to plaintext.
fn validate_download_url(link: &str) -> Result<url::Url, ProsthekeError> {
    let parsed = url::Url::parse(link).map_err(|e| {
        DownloadFailedSnafu {
            detail: format!("invalid download link: {e}"),
        }
        .build()
    })?;

    if parsed.scheme() != "https" {
        return DownloadFailedSnafu {
            detail: format!(
                "download link scheme must be https, got {}",
                parsed.scheme()
            ),
        }
        .fail();
    }

    let host_ok = parsed
        .host_str()
        .is_some_and(|host| host == "opensubtitles.com" || host.ends_with(".opensubtitles.com"));
    if !host_ok {
        return DownloadFailedSnafu {
            detail: format!(
                "download link host is not an opensubtitles.com domain: {}",
                parsed.host_str().unwrap_or("<none>")
            ),
        }
        .fail();
    }

    Ok(parsed)
}

/// Read a response body up to `max_bytes`, failing once the declared or
/// accumulated size exceeds the cap.
async fn read_body_capped(
    mut response: reqwest::Response,
    max_bytes: u64,
) -> Result<Vec<u8>, ProsthekeError> {
    if let Some(declared) = response.content_length()
        && declared > max_bytes
    {
        return DownloadFailedSnafu {
            detail: format!("subtitle download exceeds the {max_bytes}-byte cap"),
        }
        .fail();
    }

    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.context(ProviderDownSnafu)? {
        let total = (bytes.len() as u64).saturating_add(chunk.len() as u64);
        if total > max_bytes {
            return DownloadFailedSnafu {
                detail: format!("subtitle download exceeds the {max_bytes}-byte cap"),
            }
            .fail();
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

// ── Provider implementation ───────────────────────────────────────────────────

impl SubtitleProvider for OpenSubtitlesProvider {
    fn name(&self) -> &str {
        "opensubtitles"
    }

    #[instrument(skip(self), fields(provider = "opensubtitles", title = %title))]
    async fn search(
        &self,
        _media_id: &MediaId,
        media_type: MediaType,
        title: &str,
        year: Option<u16>,
        season: Option<u32>,
        episode: Option<u32>,
        languages: &[String],
        file_hash: Option<&str>,
    ) -> Result<Vec<SubtitleMatch>, ProsthekeError> {
        let Some(api_key) = self.api_key() else {
            debug!("opensubtitles not configured  -  skipping search");
            return Ok(vec![]);
        };

        if api_key.is_empty() {
            debug!("opensubtitles credential empty  -  skipping search"); // kanon:ignore SECURITY/credential-logging -- literal string, no secret interpolated
            return Ok(vec![]);
        }

        let media_type_str = match media_type {
            MediaType::Movie => "movie",
            MediaType::Tv => "episode",
            _ => "movie",
        };

        let lang_param = languages.join(",");

        let mut params: Vec<(&str, String)> = vec![
            ("query", title.to_string()),
            ("type", media_type_str.to_string()),
            ("languages", lang_param),
        ];

        if let Some(y) = year {
            params.push(("year", y.to_string()));
        }
        if let Some(s) = season {
            params.push(("season_number", s.to_string()));
        }
        if let Some(e) = episode {
            params.push(("episode_number", e.to_string()));
        }
        if let Some(hash) = file_hash {
            params.push(("moviehash", hash.to_string()));
        }

        self.throttle().await;
        let response = self
            .client
            .get(format!("{}/subtitles", self.base_url))
            .header("Api-Key", api_key)
            .query(&params)
            .send()
            .await
            .context(ProviderDownSnafu)?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(status = %status, "opensubtitles search returned non-200");
            return AcquisitionFailedSnafu {
                detail: format!("HTTP {status}"),
            }
            .fail();
        }

        let bytes = read_body_capped(response, MAX_API_RESPONSE_BYTES).await?;
        let body: SearchResponse = serde_json::from_slice(&bytes).map_err(|e| {
            AcquisitionFailedSnafu {
                detail: format!("invalid search response JSON: {e}"),
            }
            .build()
        })?;

        let mut matches = Vec::new();
        for item in body.data {
            let Some(file) = item.attributes.files.first() else {
                continue;
            };

            // Use the best language requested as the match language.
            let matched_lang = languages
                .iter()
                .find(|l| l.as_str() == item.attributes.language)
                .cloned()
                .unwrap_or_else(|| item.attributes.language.clone());

            let score = score_result(&item.attributes, &matched_lang);
            let download_url = item
                .attributes
                .url
                .clone()
                .unwrap_or_else(|| format!("{BASE_URL}/download/{}", file.file_id));

            matches.push(SubtitleMatch {
                provider: self.name().to_string(),
                provider_id: SubtitleProviderId(item.id),
                language: item.attributes.language,
                hearing_impaired: item.attributes.hearing_impaired,
                forced: item.attributes.foreign_parts_only,
                score,
                download_url,
            });
        }

        Ok(matches)
    }

    #[instrument(skip(self, subtitle), fields(provider = "opensubtitles", provider_id = %subtitle.provider_id.0))]
    async fn download(&self, subtitle: &SubtitleMatch) -> Result<Vec<u8>, ProsthekeError> {
        let Some(api_key) = self.api_key() else {
            return DownloadFailedSnafu {
                detail: "opensubtitles not configured".to_string(),
            }
            .fail();
        };

        // First request the download link FROM the API.
        // WHY: a non-numeric provider id must fail the request — a silent
        // file_id=0 fallback would download the wrong subtitle.
        let file_id: u64 = subtitle
            .provider_id
            .0
            .parse()
            .context(InvalidProviderIdSnafu {
                provider_id: subtitle.provider_id.0.clone(),
            })?;

        // WHY: the /download endpoint is quota-degraded (or refused) without a
        // JWT — log in when credentials are configured; stay anonymous when not.
        let bearer = self.bearer_token(api_key).await?;
        let mut dl_resp = self
            .request_download_link(api_key, bearer.as_deref(), file_id)
            .await?;

        // WHY: one re-login on 401 — the server may revoke a JWT before its
        // local TTL elapses; a fresh login either recovers or fails loud.
        if dl_resp.status() == reqwest::StatusCode::UNAUTHORIZED && bearer.is_some() {
            self.invalidate_token().await;
            let bearer = self.bearer_token(api_key).await?;
            dl_resp = self
                .request_download_link(api_key, bearer.as_deref(), file_id)
                .await?;
        }

        if !dl_resp.status().is_success() {
            let status = dl_resp.status();
            return DownloadFailedSnafu {
                detail: format!("HTTP {status} FROM download endpoint"),
            }
            .fail();
        }

        let dl_bytes = read_body_capped(dl_resp, MAX_API_RESPONSE_BYTES).await?;
        let dl_info: DownloadResponse = serde_json::from_slice(&dl_bytes).map_err(|e| {
            DownloadFailedSnafu {
                detail: format!("invalid download-link response JSON: {e}"),
            }
            .build()
        })?;
        debug!(file_name = %dl_info.file_name, "obtained download link");

        // Fetch the actual subtitle file.
        let download_url = validate_download_url(&dl_info.link)?;
        let max_bytes = self
            .config
            .as_ref()
            .map_or(DEFAULT_MAX_DOWNLOAD_BYTES, |c| c.max_download_bytes);
        self.throttle().await;
        let response = self
            .client
            .get(download_url)
            .send()
            .await
            .context(ProviderDownSnafu)?;

        read_body_capped(response, max_bytes).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_returns_empty_results() {
        let provider = OpenSubtitlesProvider::new(None);
        assert!(provider.api_key().is_none());
    }

    #[test]
    fn empty_api_key_treated_as_unconfigured() {
        let config = OpenSubtitlesConfig {
            api_key: String::new(),
            ..OpenSubtitlesConfig::default()
        };
        let provider = OpenSubtitlesProvider::new(Some(config));
        assert_eq!(provider.api_key(), Some(""));
    }

    #[tokio::test(start_paused = true)]
    async fn unconfigured_provider_never_throttles() {
        let provider = OpenSubtitlesProvider::new(None);
        let start = tokio::time::Instant::now();
        provider.throttle().await;
        provider.throttle().await;
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    /// Proves `rate_limit_per_second` is actually wired into the provider's
    /// request path, not just parsed and ignored — mirrors
    /// `rate_limit::tests::rapid_calls_are_throttled_to_the_configured_rate`
    /// but goes through `OpenSubtitlesProvider::new` + `throttle()`.
    #[tokio::test(start_paused = true)]
    async fn configured_provider_throttles_to_the_configured_rate() {
        let provider = OpenSubtitlesProvider::new(Some(OpenSubtitlesConfig {
            api_key: "key".to_string(),
            rate_limit_per_second: 5, // 200ms interval
            ..OpenSubtitlesConfig::default()
        }));
        let start = tokio::time::Instant::now();

        for _ in 0..5 {
            provider.throttle().await;
        }

        assert_eq!(start.elapsed(), Duration::from_millis(800));
    }

    #[test]
    fn score_hash_match_higher_than_title_match() {
        let hash_attr = SubtitleAttributes {
            language: "en".to_string(),
            hearing_impaired: false,
            foreign_parts_only: false,
            moviehash_match: true,
            files: vec![],
            url: None,
        };
        let title_attr = SubtitleAttributes {
            language: "en".to_string(),
            hearing_impaired: false,
            foreign_parts_only: false,
            moviehash_match: false,
            files: vec![],
            url: None,
        };
        assert!(score_result(&hash_attr, "en") > score_result(&title_attr, "en"));
    }

    // NOTE: known-answer vector, hand-computed from the published
    // OpenSubtitles algorithm. 16 bytes = two u64 words (w1 = 8 bytes of
    // 0x01, w2 = 8 bytes of 0x02); head and tail windows both start at
    // offset 0 for a file under 64 KB, so each word is summed twice:
    // 16 + 2*0x0101010101010101 + 2*0x0202020202020202 = 0x0606060606060616.
    #[test]
    fn compute_file_hash_small_file_known_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.bin");
        let mut content = vec![0x01u8; 8];
        content.extend_from_slice(&[0x02u8; 8]);
        std::fs::write(&path, &content).unwrap();

        assert_eq!(compute_file_hash(&path).unwrap(), "0606060606060616");
    }

    // NOTE: known-answer vector, hand-computed. 128 KB file: head window is
    // all zeros (contributes nothing), tail window seeks to offset 65536 and
    // reads 8192 words of 0xFFFFFFFFFFFFFFFF, contributing
    // 8192 * (2^64 - 1) = -8192 (mod 2^64). Hash = 0x20000 - 0x2000 = 0x1e000.
    #[test]
    fn compute_file_hash_large_file_tail_seek_known_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        let mut content = vec![0u8; 128 * 1024];
        content[64 * 1024..].fill(0xFF);
        std::fs::write(&path, &content).unwrap();

        assert_eq!(compute_file_hash(&path).unwrap(), "000000000001e000");
    }

    #[test]
    fn compute_file_hash_ignores_bytes_between_the_windows() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain.bin");
        let mutated = dir.path().join("mutated.bin");

        let content = vec![0u8; 192 * 1024];
        std::fs::write(&plain, &content).unwrap();

        let mut content = content;
        // WHY: only the region outside both 64 KB windows is changed — the
        // hash must not see it.
        content[64 * 1024..128 * 1024].fill(0xAB);
        std::fs::write(&mutated, &content).unwrap();

        let plain_hash = compute_file_hash(&plain).unwrap();
        assert_eq!(plain_hash, compute_file_hash(&mutated).unwrap());
        // NOTE: both windows are zero, so the hash is the file size alone.
        assert_eq!(plain_hash, "0000000000030000");
    }

    #[test]
    fn compute_file_hash_empty_file_is_size_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.bin");
        std::fs::write(&path, b"").unwrap();

        assert_eq!(compute_file_hash(&path).unwrap(), "0000000000000000");
    }

    #[test]
    fn compute_file_hash_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(compute_file_hash(&dir.path().join("absent.bin")).is_err());
    }

    #[test]
    fn validate_download_url_accepts_opensubtitles_hosts() {
        assert!(validate_download_url("https://www.opensubtitles.com/download/abc.srt").is_ok());
        assert!(validate_download_url("https://opensubtitles.com/download/abc.srt").is_ok());
    }

    #[test]
    fn validate_download_url_rejects_foreign_and_internal_hosts() {
        for link in [
            "http://169.254.169.254/latest/meta-data",
            "http://localhost:1234/x.srt",
            "https://evil.com/x.srt",
            "https://evilopensubtitles.com/x.srt",
            "https://opensubtitles.com.evil.com/x.srt",
            "not a url",
        ] {
            let err = validate_download_url(link).unwrap_err();
            assert!(
                matches!(err, ProsthekeError::DownloadFailed { .. }),
                "{link} must be rejected before any request is issued"
            );
        }
    }

    #[test]
    fn validate_download_url_rejects_plain_http_even_on_allowed_host() {
        assert!(validate_download_url("https://www.opensubtitles.com/x.srt").is_ok());
        assert!(validate_download_url("http://www.opensubtitles.com/x.srt").is_err());
    }

    #[tokio::test]
    async fn download_rejects_non_numeric_provider_id_before_any_request() {
        let provider = OpenSubtitlesProvider::new(Some(OpenSubtitlesConfig {
            api_key: "key".to_string(),
            ..OpenSubtitlesConfig::default()
        }));
        let subtitle = SubtitleMatch {
            provider: "opensubtitles".to_string(),
            provider_id: SubtitleProviderId("abc".to_string()),
            language: "en".to_string(),
            hearing_impaired: false,
            forced: false,
            score: 0.9,
            download_url: "https://www.opensubtitles.com/x.srt".to_string(),
        };

        let err = provider.download(&subtitle).await.unwrap_err();
        assert!(
            matches!(err, ProsthekeError::InvalidProviderId { .. }),
            "non-numeric provider id must error, not silently become file_id=0: {err:?}"
        );
    }

    #[tokio::test]
    async fn read_body_capped_rejects_oversized_declared_body() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            // WHY: the request content is irrelevant — one read drains
            // enough of it to keep the client happy before responding.
            let _bytes_read = stream.read(&mut buf).await.unwrap();
            let body = "x".repeat(2048);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            stream.write_all(response.as_bytes()).await.ok();
        });

        let response = reqwest::get(format!("http://{addr}")).await.unwrap();
        let err = read_body_capped(response, 1024).await.unwrap_err();

        assert!(
            matches!(err, ProsthekeError::DownloadFailed { .. }),
            "oversized body must abort before buffering: {err:?}"
        );
        server.await.ok();
    }

    #[tokio::test]
    async fn read_body_capped_accepts_body_within_cap() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            // WHY: the request content is irrelevant — one read drains
            // enough of it to keep the client happy before responding.
            let _bytes_read = stream.read(&mut buf).await.unwrap();
            let body = "subtitle content";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let response = reqwest::get(format!("http://{addr}")).await.unwrap();
        let bytes = read_body_capped(response, 1024).await.unwrap();

        assert_eq!(bytes, b"subtitle content");
        server.await.unwrap();
    }

    /// Spins up a raw-TCP HTTP/1.1 server that answers the one request it
    /// receives with `body`, declared via `content-length`.
    ///
    /// WHY raw TCP: prostheke has no shared test_support module (unlike
    /// epignosis/komide) — this mirrors the `read_body_capped_*` tests
    /// already in this file.
    async fn spawn_body_server(body: String) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            // WHY: the request content is irrelevant — one read drains
            // enough of it to keep the client happy before responding.
            let _bytes_read = stream.read(&mut buf).await.unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            stream.write_all(response.as_bytes()).await.ok();
        });
        (format!("http://{addr}"), handle)
    }

    /// Raw-TCP HTTP/1.1 server answering `responses` (status, body) in order,
    /// one connection per request; records each raw request (start-line +
    /// headers + body, lowercased) for assertion.
    async fn spawn_scripted_server(
        responses: Vec<(u16, String)>,
    ) -> (
        String,
        std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        tokio::task::JoinHandle<()>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let seen: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();
        let record = std::sync::Arc::clone(&seen);
        let handle = tokio::spawn(async move {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = stream.read(&mut chunk).await.unwrap();
                    buf.extend_from_slice(&chunk[..n]);
                    let head_end = buf.windows(4).position(|w| w == b"\r\n\r\n");
                    if let Some(head_end) = head_end {
                        let head = String::from_utf8_lossy(&buf[..head_end]).to_lowercase();
                        let declared = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if buf.len() - (head_end + 4) >= declared {
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }
                record
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&buf).to_lowercase());
                let reason = if status == 401 { "Unauthorized" } else { "OK" };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len(),
                );
                stream.write_all(response.as_bytes()).await.ok();
            }
        });
        (format!("http://{addr}"), seen, handle)
    }

    fn creds_config() -> OpenSubtitlesConfig {
        OpenSubtitlesConfig {
            api_key: "key".to_string(),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            // WHY: effectively unthrottled — these tests assert request
            // sequencing, not politeness pacing.
            rate_limit_per_second: 1000,
            ..OpenSubtitlesConfig::default()
        }
    }

    fn test_subtitle() -> SubtitleMatch {
        SubtitleMatch {
            provider: "opensubtitles".to_string(),
            provider_id: SubtitleProviderId("123".to_string()),
            language: "en".to_string(),
            hearing_impaired: false,
            forced: false,
            score: 0.9,
            download_url: "https://www.opensubtitles.com/x.srt".to_string(),
        }
    }

    /// A `/download-link` body whose link fails local validation (non-https),
    /// stopping the flow after the point under test without any network fetch.
    fn dead_end_link_body() -> String {
        serde_json::json!({ "link": "http://blocked.invalid/x.srt", "file_name": "x.srt" })
            .to_string()
    }

    #[test]
    fn empty_or_partial_credentials_are_treated_as_absent() {
        for (username, password) in [
            (None, None),
            (Some("user".to_string()), None),
            (None, Some("pass".to_string())),
            (Some(String::new()), Some("pass".to_string())),
            (Some("user".to_string()), Some(String::new())),
        ] {
            let provider = OpenSubtitlesProvider::new(Some(OpenSubtitlesConfig {
                api_key: "key".to_string(),
                username,
                password,
                ..OpenSubtitlesConfig::default()
            }));
            assert!(provider.credentials().is_none());
        }
    }

    #[tokio::test]
    async fn download_with_credentials_logs_in_once_and_attaches_bearer() {
        let login_body = serde_json::json!({ "token": "jwt-abc" }).to_string();
        let (base_url, seen, handle) = spawn_scripted_server(vec![
            (200, login_body),
            (200, dead_end_link_body()),
            (200, dead_end_link_body()),
        ])
        .await;
        let provider = OpenSubtitlesProvider::with_base_url(Some(creds_config()), base_url);

        let _ = provider.download(&test_subtitle()).await;
        let _ = provider.download(&test_subtitle()).await;
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("both downloads must reach the server")
            .unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert!(seen[0].starts_with("post /login"));
        assert!(seen[0].contains("api-key: key"));
        assert!(seen[0].contains("\"username\":\"user\""));
        assert!(seen[0].contains("\"password\":\"pass\""));
        assert!(seen[1].starts_with("post /download"));
        assert!(
            seen[1].contains("authorization: bearer jwt-abc"),
            "the download-link request must carry the login JWT: {}",
            seen[1]
        );
        assert!(
            seen[2].starts_with("post /download"),
            "the second download must reuse the cached JWT, not re-login"
        );
        assert!(seen[2].contains("authorization: bearer jwt-abc"));
    }

    #[tokio::test]
    async fn download_401_triggers_one_relogin_and_retry() {
        let (base_url, seen, handle) = spawn_scripted_server(vec![
            (200, serde_json::json!({ "token": "jwt-old" }).to_string()),
            (401, "{}".to_string()),
            (200, serde_json::json!({ "token": "jwt-new" }).to_string()),
            (200, dead_end_link_body()),
        ])
        .await;
        let provider = OpenSubtitlesProvider::with_base_url(Some(creds_config()), base_url);

        let _ = provider.download(&test_subtitle()).await;
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("the 401 must trigger exactly one re-login and one retry")
            .unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 4);
        assert!(seen[0].starts_with("post /login"));
        assert!(seen[1].starts_with("post /download"));
        assert!(seen[1].contains("authorization: bearer jwt-old"));
        assert!(seen[2].starts_with("post /login"));
        assert!(seen[3].starts_with("post /download"));
        assert!(
            seen[3].contains("authorization: bearer jwt-new"),
            "the retry must carry the fresh JWT: {}",
            seen[3]
        );
    }

    #[tokio::test]
    async fn download_without_credentials_never_logs_in() {
        let (base_url, seen, handle) =
            spawn_scripted_server(vec![(200, dead_end_link_body())]).await;
        let provider = OpenSubtitlesProvider::with_base_url(
            Some(OpenSubtitlesConfig {
                api_key: "key".to_string(),
                rate_limit_per_second: 1000,
                ..OpenSubtitlesConfig::default()
            }),
            base_url,
        );

        let _ = provider.download(&test_subtitle()).await;
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("the anonymous download must reach the server")
            .unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1, "no login request may be issued: {seen:?}");
        assert!(seen[0].starts_with("post /download"));
        assert!(
            !seen[0].contains("authorization:"),
            "anonymous downloads must not carry a bearer: {}",
            seen[0]
        );
    }

    #[tokio::test]
    async fn search_oversized_json_response_is_capped_not_buffered() {
        let oversized_body = "x".repeat((MAX_API_RESPONSE_BYTES as usize) + 1);
        let (base_url, handle) = spawn_body_server(oversized_body).await;

        let provider = OpenSubtitlesProvider::with_base_url(
            Some(OpenSubtitlesConfig {
                api_key: "key".to_string(),
                ..OpenSubtitlesConfig::default()
            }),
            base_url,
        );
        let media_id = aggelmata::MediaId::new();

        let result = provider
            .search(
                &media_id,
                MediaType::Movie,
                "Inception",
                None,
                None,
                None,
                &["en".to_string()],
                None,
            )
            .await;

        // WHY: SubtitleMatch (the Ok payload) does not derive Debug, so
        // unwrap_err() is unavailable — match directly instead.
        let Err(err) = result else {
            panic!("an oversized /subtitles response must error, not succeed");
        };
        assert!(
            matches!(err, ProsthekeError::DownloadFailed { .. }),
            "an oversized /subtitles response must be capped, not buffered whole into memory: {err:?}"
        );
        handle.await.ok();
    }

    #[tokio::test]
    async fn search_parses_response_within_cap() {
        let body = serde_json::json!({
            "data": [{
                "id": "abc123",
                "attributes": {
                    "language": "en",
                    "moviehash_match": true,
                    "files": [{"file_id": 42}],
                    "url": "https://www.opensubtitles.com/download/abc.srt",
                }
            }]
        })
        .to_string();
        let (base_url, handle) = spawn_body_server(body).await;

        let provider = OpenSubtitlesProvider::with_base_url(
            Some(OpenSubtitlesConfig {
                api_key: "key".to_string(),
                ..OpenSubtitlesConfig::default()
            }),
            base_url,
        );
        let media_id = aggelmata::MediaId::new();

        let matches = provider
            .search(
                &media_id,
                MediaType::Movie,
                "Inception",
                None,
                None,
                None,
                &["en".to_string()],
                None,
            )
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].provider_id.0, "abc123");
        assert_eq!(
            matches[0].download_url,
            "https://www.opensubtitles.com/download/abc.srt"
        );
        handle.await.ok();
    }

    #[tokio::test]
    async fn download_oversized_download_link_response_is_capped_not_buffered() {
        let oversized_body = "x".repeat((MAX_API_RESPONSE_BYTES as usize) + 1);
        let (base_url, handle) = spawn_body_server(oversized_body).await;

        let provider = OpenSubtitlesProvider::with_base_url(
            Some(OpenSubtitlesConfig {
                api_key: "key".to_string(),
                ..OpenSubtitlesConfig::default()
            }),
            base_url,
        );
        let subtitle = SubtitleMatch {
            provider: "opensubtitles".to_string(),
            provider_id: SubtitleProviderId("123".to_string()),
            language: "en".to_string(),
            hearing_impaired: false,
            forced: false,
            score: 0.9,
            download_url: "https://www.opensubtitles.com/x.srt".to_string(),
        };

        let err = provider.download(&subtitle).await.unwrap_err();

        assert!(
            matches!(err, ProsthekeError::DownloadFailed { .. }),
            "an oversized /download-link response must be capped, not buffered whole into memory: {err:?}"
        );
        handle.await.ok();
    }

    #[tokio::test]
    async fn unconfigured_search_returns_empty_not_error() {
        let provider = OpenSubtitlesProvider::new(None);
        let media_id = aggelmata::MediaId::new();
        let result = provider
            .search(
                &media_id,
                MediaType::Movie,
                "Inception",
                Some(2010),
                None,
                None,
                &["en".to_string()],
                None,
            )
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
