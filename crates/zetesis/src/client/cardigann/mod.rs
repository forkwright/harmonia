//! Cardigann definition-driven indexer client.
//!
//! Executes Prowlarr-compatible YAML definitions (loaded at startup from
//! `cardigann_definitions_dir`) against HTML trackers that lack a native
//! Torznab/Newznab API: templated search URLs, CSS row/field selectors, a
//! filter pipeline, and category mapping. Login support covers `none` and
//! `cookie`; form/multi-step login defers to a Torznab sidecar.

pub mod categories;
pub mod definition;
mod extract;
mod filters;
mod template;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use horismos::SearchSubsystemConfig;
use jiff::Zoned;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument, warn};
use url::Url;

use crate::cf_bypass::CloudflareProxy;
use crate::client::{
    IndexerClient, IndexerConfig, read_body_bounded, read_body_bytes_bounded, redact_api_key,
    validate_fetch_url,
};
use crate::error::{self, SearchIndexerError};
use crate::types::{
    DownloadResponse, IndexerCaps, IndexerStatus, ReleaseProtocol, SearchFunction, SearchLimits,
    SearchQuery, SearchResult, ServerInfo,
};
use definition::{CardigannDefinition, SearchPath};
use template::TemplateContext;

/// Cardigann definitions loaded from `cardigann_definitions_dir`, keyed by
/// definition id.
pub struct CardigannRegistry {
    config: Arc<SearchSubsystemConfig>,
    definitions: HashMap<String, Arc<CardigannDefinition>>,
}

impl CardigannRegistry {
    /// Reads every `.yml`/`.yaml` file under the configured definitions
    /// directory. Files that fail to parse or validate are skipped with a
    /// warning; a broken definition must not take down startup.
    pub fn load(config: Arc<SearchSubsystemConfig>) -> Self {
        let mut definitions = HashMap::new();
        if let Some(dir) = &config.cardigann_definitions_dir {
            match std::fs::read_dir(dir) {
                Ok(entries) => {
                    let mut paths: Vec<_> = entries
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| {
                            matches!(
                                path.extension().and_then(|e| e.to_str()),
                                Some("yml" | "yaml")
                            )
                        })
                        .collect();
                    // WHY: deterministic load order makes duplicate-id
                    // resolution (first file wins) reproducible.
                    paths.sort();
                    for path in paths {
                        match definition::load_definition_file(&path) {
                            Ok(def) => {
                                if definitions.contains_key(&def.id) {
                                    warn!(
                                        definition_id = %def.id,
                                        path = %path.display(),
                                        "duplicate Cardigann definition id; keeping the first"
                                    );
                                    continue;
                                }
                                info!(
                                    definition_id = %def.id,
                                    name = %def.name,
                                    "loaded Cardigann definition"
                                );
                                definitions.insert(def.id.clone(), Arc::new(def));
                            }
                            Err(e) => {
                                warn!(
                                    path = %path.display(),
                                    error = %e,
                                    "skipping Cardigann definition"
                                );
                            }
                        }
                    }
                    info!(count = definitions.len(), "Cardigann definitions loaded");
                }
                Err(e) => {
                    warn!(
                        dir = %dir.display(),
                        error = %e,
                        "cannot read cardigann_definitions_dir"
                    );
                }
            }
        }
        Self {
            config,
            definitions,
        }
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Resolves an indexer row's `url` column to a loaded definition: an
    /// exact definition-id match first, then a match against the
    /// definition's declared site links (so a row may carry either form).
    pub fn resolve(&self, indexer_url: &str) -> Option<Arc<CardigannDefinition>> {
        if let Some(def) = self.definitions.get(indexer_url) {
            return Some(Arc::clone(def));
        }
        let normalized = indexer_url.trim_end_matches('/');
        self.definitions
            .values()
            .find(|def| {
                def.links
                    .iter()
                    .chain(&def.legacylinks)
                    .any(|link| link.trim_end_matches('/').eq_ignore_ascii_case(normalized))
            })
            .cloned()
    }

    pub fn client_for(
        &self,
        indexer: IndexerConfig,
        http_client: reqwest::Client,
        cf_proxy: Arc<dyn CloudflareProxy>,
        timeout: Duration,
    ) -> Result<CardigannClient, SearchIndexerError> {
        let definition =
            self.resolve(&indexer.url)
                .ok_or_else(|| SearchIndexerError::DefinitionNotFound {
                    indexer_id: indexer.id,
                    url: indexer.url.clone(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?;
        CardigannClient::new(
            Arc::clone(&self.config),
            http_client,
            cf_proxy,
            timeout,
            indexer,
            definition,
        )
    }
}

pub struct CardigannClient {
    config: Arc<SearchSubsystemConfig>,
    http_client: reqwest::Client,
    cf_proxy: Arc<dyn CloudflareProxy>,
    timeout: Duration,
    indexer: IndexerConfig,
    definition: Arc<CardigannDefinition>,
    base_url: Url,
    /// Cookie header for `login.method: cookie`, taken from the indexer
    /// row's `api_key` column (the per-instance secret slot).
    cookie: Option<String>,
}

impl std::fmt::Debug for CardigannClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardigannClient")
            .field("indexer_id", &self.indexer.id)
            .field("definition_id", &self.definition.id)
            .field("base_url", &self.base_url.as_str())
            .field("cookie", &self.cookie.as_ref().map(|_| "[redacted]"))
            .finish_non_exhaustive()
    }
}

impl CardigannClient {
    pub fn new(
        config: Arc<SearchSubsystemConfig>,
        http_client: reqwest::Client,
        cf_proxy: Arc<dyn CloudflareProxy>,
        timeout: Duration,
        indexer: IndexerConfig,
        definition: Arc<CardigannDefinition>,
    ) -> Result<Self, SearchIndexerError> {
        let base_url = resolve_base_url(&indexer, &definition)?;
        let cookie = resolve_login(&indexer, &definition)?;
        if cookie.is_some() && indexer.cf_bypass {
            // WHY: the bypass proxy carries no request headers, so a session
            // cookie silently vanishes — fail loudly instead.
            return Err(SearchIndexerError::DefinitionUnsupported {
                definition_id: definition.id.clone(),
                feature: "cf_bypass combined with cookie login".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
        Ok(Self {
            config,
            http_client,
            cf_proxy,
            timeout,
            indexer,
            definition,
            base_url,
            cookie,
        })
    }

    fn invalid(&self, reason: String) -> SearchIndexerError {
        SearchIndexerError::DefinitionInvalid {
            definition_id: self.definition.id.clone(),
            reason,
            location: snafu::Location::new(file!(), line!(), column!()),
        }
    }

    fn template_context(
        &self,
        query: &SearchQuery,
        site_categories: Vec<String>,
        now: &Zoned,
    ) -> Result<TemplateContext, SearchIndexerError> {
        let keywords = filters::apply(
            build_keywords(query),
            &self.definition.search.keywordsfilters,
            now,
        )
        .map_err(|e| self.invalid(format!("keywordsfilters: {e}")))?;

        let mut config = BTreeMap::new();
        for setting in &self.definition.settings {
            config.insert(
                setting.name.clone(),
                setting
                    .default
                    .as_ref()
                    .map(|d| d.0.clone())
                    .unwrap_or_default(),
            );
        }
        if let Some(cookie) = &self.cookie {
            config.insert("cookie".to_string(), cookie.clone());
        }

        Ok(TemplateContext {
            keywords,
            categories: site_categories,
            config,
            query: query_vars(query),
        })
    }

    fn build_search_url(
        &self,
        path: &SearchPath,
        ctx: &TemplateContext,
    ) -> Result<Url, SearchIndexerError> {
        let rendered = ctx
            .render(&path.path)
            .map_err(|e| self.invalid(format!("search path: {e}")))?;
        // WHY: Cardigann paths resolve under the site link even when they
        // start with "/" — stripping the slash keeps a sub-path base
        // (https://host/sub/) intact under Url::join. A rare absolute
        // http(s) path replaces the base wholesale.
        let mut url = match parse_absolute_http(&rendered) {
            Some(absolute) => absolute,
            None => self
                .base_url
                .join(rendered.trim_start_matches('/'))
                .map_err(|e| self.invalid(format!("search path {rendered:?}: {e}")))?,
        };

        let mut inputs: BTreeMap<&str, &str> = BTreeMap::new();
        for (key, value) in &self.definition.search.inputs {
            inputs.insert(key, &value.0);
        }
        for (key, value) in &path.inputs {
            inputs.insert(key, &value.0);
        }

        let mut raw_parts: Vec<String> = Vec::new();
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in inputs {
                let rendered = ctx
                    .render(value)
                    .map_err(|e| self.invalid(format!("search input {key}: {e}")))?;
                if key == "$raw" {
                    raw_parts.push(rendered);
                } else {
                    pairs.append_pair(key, &rendered);
                }
            }
        }
        if !raw_parts.is_empty() {
            let extra = raw_parts.join("");
            let extra = extra.trim_matches('&');
            let combined = match url.query() {
                Some(q) if !q.is_empty() => format!("{q}&{extra}"),
                _ => extra.to_string(),
            };
            url.set_query(Some(&combined));
        }
        Ok(url)
    }

    async fn send(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<reqwest::Response, SearchIndexerError> {
        let mut request = self.http_client.get(url).timeout(self.timeout);
        if let Some(cookie) = &self.cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        let fut = request.send();
        let response = tokio::select! {
            result = fut => result.context(error::HttpRequestSnafu { url: redact_api_key(url) })?,
            () = ct.cancelled() => {
                return Err(SearchIndexerError::Cancelled {
                    url: redact_api_key(url),
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(SearchIndexerError::AuthFailed {
                indexer_id: self.indexer.id,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            return Err(SearchIndexerError::RateLimited {
                indexer_id: self.indexer.id,
                retry_after_seconds: retry_after,
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
        Ok(response)
    }

    async fn fetch_text(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<String, SearchIndexerError> {
        if self.indexer.cf_bypass {
            let response = self.cf_proxy.get(url, ct).await?;
            return Ok(response.body);
        }
        let response = self.send(url, ct).await?;
        read_body_bounded(response, url, self.config.max_response_body_bytes).await
    }

    async fn fetch_bytes(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<Bytes, SearchIndexerError> {
        if self.indexer.cf_bypass {
            let response = self.cf_proxy.get(url, ct).await?;
            return Ok(Bytes::from(response.body));
        }
        let response = self.send(url, ct).await?;
        read_body_bytes_bounded(response, url, self.config.max_response_body_bytes)
            .await
            .map(Bytes::from)
    }

    /// Joins a row-extracted link against the site base; magnet and absolute
    /// links pass through unchanged.
    fn absolutize(&self, link: &str) -> Result<String, url::ParseError> {
        if link.starts_with("magnet:") {
            return Ok(link.to_string());
        }
        self.base_url.join(link).map(String::from)
    }

    fn rows_to_results(&self, rows: Vec<BTreeMap<String, String>>) -> Vec<SearchResult> {
        let mappings = &self.definition.caps.categorymappings;
        rows.into_iter()
            .filter_map(|mut row| {
                let Some(title) = row.remove("title") else {
                    warn!(
                        indexer_id = self.indexer.id,
                        "skipping Cardigann row without a title"
                    );
                    return None;
                };
                let Some(raw_link) = row.remove("magnet").or_else(|| row.remove("download")) else {
                    warn!(
                        indexer_id = self.indexer.id,
                        title = %title,
                        "skipping Cardigann row without a download/magnet link"
                    );
                    return None;
                };
                let download_url = match self.absolutize(&raw_link) {
                    Ok(link) => link,
                    Err(e) => {
                        warn!(
                            indexer_id = self.indexer.id,
                            title = %title,
                            error = %e,
                            "skipping Cardigann row with an unjoinable link"
                        );
                        return None;
                    }
                };
                let guid = row
                    .remove("details")
                    .and_then(|details| self.absolutize(&details).ok());
                let size_bytes = row.remove("size").and_then(|s| extract::parse_size(&s));
                let seeders = row
                    .remove("seeders")
                    .and_then(|s| extract::parse_u32_loose(&s));
                let leechers = row
                    .remove("leechers")
                    .and_then(|s| extract::parse_u32_loose(&s));
                let category_id = row
                    .remove("category")
                    .and_then(|c| categories::torznab_id_for_site(mappings, &c));
                let publication_date = row.remove("date");
                let info_hash = row.remove("infohash");
                let download_volume_factor = row
                    .remove("downloadvolumefactor")
                    .and_then(|v| extract::parse_f64_loose(&v))
                    .unwrap_or(1.0);
                let upload_volume_factor = row
                    .remove("uploadvolumefactor")
                    .and_then(|v| extract::parse_f64_loose(&v))
                    .unwrap_or(1.0);

                Some(SearchResult {
                    title,
                    guid,
                    download_url,
                    size_bytes,
                    seeders,
                    leechers,
                    info_hash,
                    category_id,
                    publication_date,
                    indexer_id: self.indexer.id,
                    // NOTE: Cardigann definitions describe torrent trackers;
                    // there is no protocol field in the schema.
                    protocol: ReleaseProtocol::Torrent,
                    download_volume_factor,
                    upload_volume_factor,
                    custom_attrs: row.into_iter().collect(),
                })
            })
            .collect()
    }

    fn definition_caps(&self) -> IndexerCaps {
        const MODE_FUNCTIONS: &[(&str, &str)] = &[
            ("search", "search"),
            ("tv-search", "tvsearch"),
            ("movie-search", "movie"),
            ("music-search", "music"),
            ("book-search", "book"),
        ];
        IndexerCaps {
            server: ServerInfo {
                title: Some(self.definition.name.clone()),
                version: None,
            },
            limits: SearchLimits::default(),
            search_functions: MODE_FUNCTIONS
                .iter()
                .map(|(mode, function)| SearchFunction {
                    function_type: (*function).to_string(),
                    available: self.definition.caps.modes.contains_key(*mode),
                })
                .collect(),
            categories: categories::caps_categories(&self.definition.caps.categorymappings),
        }
    }
}

impl IndexerClient for CardigannClient {
    #[instrument(skip(self, ct), fields(indexer_id = self.indexer.id, indexer_name = %self.indexer.name))]
    async fn search(
        &self,
        query: &SearchQuery,
        ct: CancellationToken,
    ) -> Result<Vec<SearchResult>, SearchIndexerError> {
        let mappings = &self.definition.caps.categorymappings;
        let site_categories = categories::site_categories_for(mappings, &query.category_ids);
        if !query.category_ids.is_empty() && !mappings.is_empty() && site_categories.is_empty() {
            // WHY: the tracker carries none of the requested categories — an
            // empty result is the answer, not an error.
            return Ok(Vec::new());
        }

        let now = Zoned::now();
        let ctx = self.template_context(query, site_categories.clone(), &now)?;
        let unconstrained = query.category_ids.is_empty();

        let mut results = Vec::new();
        for path in &self.definition.search.paths {
            if !path_applies(path, &site_categories, unconstrained) {
                continue;
            }
            let url = self.build_search_url(path, &ctx)?;
            let body = self.fetch_text(url.as_str(), ct.clone()).await?;
            let rows = extract::extract_rows(&body, &self.definition, &ctx, &now).map_err(|e| {
                SearchIndexerError::ParseResponse {
                    url: redact_api_key(url.as_str()),
                    error: e,
                    location: snafu::Location::new(file!(), line!(), column!()),
                }
            })?;
            results.extend(self.rows_to_results(rows));
        }

        let limit = usize::try_from(query.limit).unwrap_or(usize::MAX);
        if limit > 0 && results.len() > limit {
            results.truncate(limit);
        }
        Ok(results)
    }

    #[instrument(skip(self, _ct), fields(indexer_id = self.indexer.id))]
    async fn caps(&self, _ct: CancellationToken) -> Result<IndexerCaps, SearchIndexerError> {
        Ok(self.definition_caps())
    }

    #[instrument(skip(self, ct), fields(indexer_id = self.indexer.id))]
    async fn test(&self, ct: CancellationToken) -> Result<IndexerStatus, SearchIndexerError> {
        // NOTE: login.test.selector checks are deferred — this probe only
        // verifies the page is reachable (HTTP success).
        let test_path = self
            .definition
            .login
            .as_ref()
            .and_then(|login| login.test.as_ref())
            .and_then(|test| test.path.clone());
        let probe = async {
            let url = match test_path {
                Some(path) => self
                    .base_url
                    .join(path.trim_start_matches('/'))
                    .map_err(|e| self.invalid(format!("login test path {path:?}: {e}")))?,
                None => self.base_url.clone(),
            };
            if self.indexer.cf_bypass {
                self.cf_proxy.get(url.as_str(), ct).await?;
                return Ok(());
            }
            let response = self.send(url.as_str(), ct).await?;
            response
                .error_for_status()
                .map(|_| ())
                .context(error::HttpRequestSnafu {
                    url: redact_api_key(url.as_str()),
                })
        };
        match probe.await {
            Ok(()) => Ok(IndexerStatus {
                healthy: true,
                caps: Some(self.definition_caps()),
                error: None,
            }),
            Err(e) => Ok(IndexerStatus {
                healthy: false,
                caps: None,
                error: Some(e.to_string()),
            }),
        }
    }

    #[instrument(skip(self, ct), fields(indexer_id = self.indexer.id))]
    async fn download(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<DownloadResponse, SearchIndexerError> {
        if url.starts_with("magnet:") {
            return Ok(DownloadResponse::MagnetUri(url.to_string()));
        }
        // SAFETY: download URLs originate in scraped third-party HTML —
        // validate scheme + resolved addresses before any fetch.
        validate_fetch_url(url).await?;

        let final_url = match &self.definition.download {
            Some(block) if block.selector.is_some() => {
                let page = self.fetch_text(url, ct.clone()).await?;
                let link = extract::extract_download_link(
                    &page,
                    block.selector.as_deref().unwrap_or_default(),
                    block.attribute.as_deref(),
                )
                .map_err(|e| self.invalid(format!("download: {e}")))?;
                let link = filters::apply(link, &block.filters, &Zoned::now())
                    .map_err(|e| self.invalid(format!("download filters: {e}")))?;
                let absolute = self
                    .absolutize(&link)
                    .map_err(|e| self.invalid(format!("download link {link:?}: {e}")))?;
                if absolute.starts_with("magnet:") {
                    return Ok(DownloadResponse::MagnetUri(absolute));
                }
                validate_fetch_url(&absolute).await?;
                absolute
            }
            _ => url.to_string(),
        };

        let bytes = self.fetch_bytes(&final_url, ct).await?;
        Ok(DownloadResponse::TorrentFile(bytes))
    }
}

fn resolve_base_url(
    indexer: &IndexerConfig,
    definition: &CardigannDefinition,
) -> Result<Url, SearchIndexerError> {
    let invalid = |reason: String| SearchIndexerError::DefinitionInvalid {
        definition_id: definition.id.clone(),
        reason,
        location: snafu::Location::new(file!(), line!(), column!()),
    };
    let mut url = match parse_absolute_http(&indexer.url) {
        Some(from_row) => from_row,
        None => {
            let link = definition
                .links
                .first()
                .ok_or_else(|| invalid("no links".to_string()))?;
            Url::parse(link).map_err(|e| invalid(format!("base url {link}: {e}")))?
        }
    };
    // WHY: Url::join replaces the last path segment unless the base ends in
    // "/" — a sub-path site link must keep its directory.
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

/// Parses `raw` as an absolute http(s) URL; anything else (definition ids,
/// relative paths, other schemes) yields `None`.
fn parse_absolute_http(raw: &str) -> Option<Url> {
    Url::parse(raw)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
}

/// Resolves the login block to an optional Cookie header value.
fn resolve_login(
    indexer: &IndexerConfig,
    definition: &CardigannDefinition,
) -> Result<Option<String>, SearchIndexerError> {
    let Some(login) = &definition.login else {
        return Ok(None);
    };
    // NOTE: Cardigann's default method when the login block omits one is
    // "form", which this engine defers.
    match login.method.as_deref().unwrap_or("form") {
        "none" => Ok(None),
        "cookie" => match &indexer.api_key {
            Some(cookie) if !cookie.trim().is_empty() => Ok(Some(cookie.clone())),
            _ => Err(SearchIndexerError::CookieAuthRequired {
                definition_id: definition.id.clone(),
                indexer_id: indexer.id,
                location: snafu::Location::new(file!(), line!(), column!()),
            }),
        },
        other => Err(SearchIndexerError::LoginUnsupported {
            definition_id: definition.id.clone(),
            method: other.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        }),
    }
}

fn path_applies(path: &SearchPath, site_categories: &[String], unconstrained: bool) -> bool {
    if path.categories.is_empty() || unconstrained {
        return true;
    }
    path.categories
        .iter()
        .any(|c| site_categories.contains(&c.0))
}

fn build_keywords(query: &SearchQuery) -> String {
    let mut keywords = query.query_text.clone().unwrap_or_default();
    match (query.season, query.episode) {
        (Some(season), Some(episode)) => {
            keywords.push_str(&format!(" S{season:02}E{episode:02}"));
        }
        (Some(season), None) => keywords.push_str(&format!(" S{season:02}")),
        // NOTE: an episode without a season has no SxxEyy rendering.
        (None, _) => {}
    }
    keywords.trim().to_string()
}

fn query_vars(query: &SearchQuery) -> BTreeMap<&'static str, String> {
    let mut vars = BTreeMap::new();
    vars.insert("Q", query.query_text.clone().unwrap_or_default());
    if let Some(season) = query.season {
        vars.insert("Season", season.to_string());
    }
    if let Some(episode) = query.episode {
        vars.insert("Ep", episode.to_string());
    }
    if let Some(imdb) = &query.imdb_id {
        vars.insert("IMDBID", imdb.clone());
        vars.insert("IMDBIDShort", imdb.trim_start_matches("tt").to_string());
    }
    if let Some(tvdb) = query.tvdb_id {
        vars.insert("TVDBID", tvdb.to_string());
    }
    if let Some(tmdb) = query.tmdb_id {
        vars.insert("TMDBID", tmdb.to_string());
    }
    if let Some(artist) = &query.artist {
        vars.insert("Artist", artist.clone());
    }
    if let Some(album) = &query.album {
        vars.insert("Album", album.clone());
    }
    if let Some(author) = &query.author {
        vars.insert("Author", author.clone());
    }
    vars.insert("Limit", query.limit.to_string());
    vars.insert("Offset", query.offset.to_string());
    vars
}

#[cfg(test)]
mod tests;
