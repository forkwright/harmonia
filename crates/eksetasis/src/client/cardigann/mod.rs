//! Cardigann definition-driven indexer client.
//!
//! Executes Prowlarr-compatible YAML definitions (loaded at startup from
//! `cardigann_definitions_dir`) against HTML trackers that lack a native
//! Torznab/Newznab API: templated search URLs (Go-template subset with
//! `if`/`range` blocks and per-row `.Result` field references), CSS row/field
//! selectors, a filter pipeline, and category mapping. Login support covers
//! `none`, `cookie`, and the interactive `form`/`post`/`get` methods
//! (per-indexer session cookies in [`session::SessionStore`]).

pub mod categories;
pub mod definition;
mod extract;
mod filters;
mod json_extract;
mod session;
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
    IndexerClient, IndexerConfig, read_body_bounded, read_body_bytes_bounded, redact_secrets,
    validate_fetch_url,
};
use crate::error::{self, SearchIndexerError};
use crate::types::{
    DownloadResponse, IndexerCaps, IndexerStatus, ReleaseProtocol, SearchFunction, SearchLimits,
    SearchQuery, SearchResult, ServerInfo,
};
use definition::{CardigannDefinition, SearchPath};
pub use session::SessionStore;
use session::{LoginMethod, LoginVerb};
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
        sessions: Arc<SessionStore>,
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
            sessions,
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
    /// Resolved login strategy (none / static cookie / interactive).
    login: LoginMethod,
    /// Per-indexer interactive-login session store, owned by the service and
    /// shared across every ephemeral client for one indexer.
    sessions: Arc<SessionStore>,
    /// Rendered `login.path` joined on the site base (interactive methods
    /// only). Also the staleness comparison target — a search that ends up
    /// back here means the session expired.
    login_url: Option<Url>,
}

impl std::fmt::Debug for CardigannClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // WHY: `login` has a redacting Debug and `sessions` prints cookie
        // names only — no credential or cookie value can reach this output.
        f.debug_struct("CardigannClient")
            .field("indexer_id", &self.indexer.id)
            .field("definition_id", &self.definition.id)
            .field("base_url", &self.base_url.as_str())
            .field("login", &self.login)
            .finish_non_exhaustive()
    }
}

impl CardigannClient {
    // WHY: an ephemeral per-search client wiring its live dependencies
    // (config, transport, proxy, timeout, indexer row, definition, shared
    // session store); a params struct would only relocate the same fields.
    #[expect(
        clippy::too_many_arguments,
        reason = "dependency-injection constructor; each arg is a distinct live dependency"
    )]
    pub fn new(
        config: Arc<SearchSubsystemConfig>,
        http_client: reqwest::Client,
        cf_proxy: Arc<dyn CloudflareProxy>,
        timeout: Duration,
        indexer: IndexerConfig,
        definition: Arc<CardigannDefinition>,
        sessions: Arc<SessionStore>,
    ) -> Result<Self, SearchIndexerError> {
        let base_url = resolve_base_url(&indexer, &definition)?;
        validate_settings(&indexer, &definition)?;
        let login = resolve_login(&indexer, &definition)?;
        if !matches!(login, LoginMethod::None) && indexer.cf_bypass {
            // WHY: the bypass proxy carries no request headers, so a session
            // or login cookie silently vanishes — fail loudly for EVERY
            // non-None login method, not just static cookies.
            return Err(SearchIndexerError::DefinitionUnsupported {
                definition_id: definition.id.clone(),
                feature: "cf_bypass combined with an authenticated login method".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
        if indexer.cf_bypass
            && definition
                .search
                .paths
                .iter()
                .any(|path| path.method.as_deref() == Some("post"))
        {
            // WHY: the bypass proxy is GET-only, so a POST search body cannot
            // be delivered through it — fail loudly rather than silently
            // issuing a bodyless GET.
            return Err(SearchIndexerError::DefinitionUnsupported {
                definition_id: definition.id.clone(),
                feature: "cf_bypass combined with POST search".to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
        let login_url = if matches!(login, LoginMethod::Interactive { .. }) {
            Some(resolve_login_url(&indexer, &definition, &base_url)?)
        } else {
            None
        };
        Ok(Self {
            config,
            http_client,
            cf_proxy,
            timeout,
            indexer,
            definition,
            base_url,
            login,
            sessions,
            login_url,
        })
    }

    /// Static `.Config` seed shared by search and login rendering: definition
    /// defaults, then the indexer's settings overrides, then the injected
    /// static cookie (cookie method only).
    fn config_seed(&self) -> BTreeMap<String, String> {
        build_config_seed(
            &self.definition,
            &self.indexer.settings,
            self.cookie_value(),
        )
    }

    /// The static `Cookie`-method value, if this indexer uses cookie login.
    fn cookie_value(&self) -> Option<&str> {
        match &self.login {
            LoginMethod::Cookie(cookie) => Some(cookie.as_str()),
            _ => None,
        }
    }

    /// A `.Config`-only template context for rendering `login.inputs` /
    /// `login.path` / error-message templates (no per-search keywords, no
    /// row scope).
    pub(crate) fn login_template_context(&self) -> TemplateContext {
        TemplateContext {
            keywords: String::new(),
            categories: Vec::new(),
            config: self.config_seed(),
            query: BTreeMap::new(),
            result: BTreeMap::new(),
        }
    }

    /// `Cookie` header for an ordinary request: the static cookie-method
    /// value, or the interactive session's cookies from the store.
    fn request_cookie_header(&self) -> Option<String> {
        match &self.login {
            LoginMethod::None => None,
            LoginMethod::Cookie(cookie) => Some(cookie.clone()),
            LoginMethod::Interactive { .. } => self.sessions.get_cookie_header(self.indexer.id),
        }
    }

    fn login_failed(&self, reason: String) -> SearchIndexerError {
        SearchIndexerError::LoginFailed {
            definition_id: self.definition.id.clone(),
            indexer_id: self.indexer.id,
            reason,
            location: snafu::Location::new(file!(), line!(), column!()),
        }
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
        let mut ctx = TemplateContext {
            keywords: build_keywords(query),
            categories: site_categories,
            config: self.config_seed(),
            query: query_vars(query),
            result: BTreeMap::new(),
        };
        // WHY: keywordsfilter args are templates (search scope — `.Result`
        // is rejected at load here), so they render before the pipeline runs.
        let specs = template::render_specs(&self.definition.search.keywordsfilters, &ctx)
            .map_err(|e| self.invalid(format!("keywordsfilters: {e}")))?;
        ctx.keywords = filters::apply(std::mem::take(&mut ctx.keywords), &specs, now)
            .map_err(|e| self.invalid(format!("keywordsfilters: {e}")))?;
        Ok(ctx)
    }

    /// Builds the search request for a path: the target URL and, for a POST
    /// path, the `application/x-www-form-urlencoded` body. GET paths fold their
    /// inputs into the query string and carry no body.
    fn build_search_request(
        &self,
        path: &SearchPath,
        ctx: &TemplateContext,
    ) -> Result<(Url, Option<String>), SearchIndexerError> {
        let rendered = ctx
            .render_url(&path.path)
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

        let is_post = path.method.as_deref() == Some("post");

        let mut raw_parts: Vec<String> = Vec::new();
        let mut pairs: Vec<(&str, String)> = Vec::new();
        for (key, value) in inputs {
            if key == "$raw" {
                // WHY: $raw is spliced verbatim into the query via set_query
                // (which does NOT encode), so variable expansions must be
                // percent-encoded here exactly as the path template already is
                // via render_url — a keyword like "a&b=c" must not inject an
                // extra parameter. Only the .Keywords/.Query.* expansions
                // encode; the definition-author's literal $raw "&"/"=" survive.
                // $raw is rejected at load for POST paths, so this is GET-only.
                let rendered = ctx
                    .render_url(value)
                    .map_err(|e| self.invalid(format!("search input {key}: {e}")))?;
                raw_parts.push(rendered);
            } else {
                // WHY: non-$raw values pass through append_pair / form encoding
                // below, both of which percent-encode the whole value — render
                // unencoded here.
                let rendered = ctx
                    .render(value)
                    .map_err(|e| self.invalid(format!("search input {key}: {e}")))?;
                pairs.push((key, rendered));
            }
        }

        if is_post {
            // WHY: a POST search delivers its inputs as a form body (mirroring
            // the login POST), leaving the endpoint URL query-free. raw_parts
            // is empty here — $raw with POST is rejected at load.
            let body = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(pairs.iter().map(|(k, v)| (*k, v.as_str())))
                .finish();
            return Ok((url, Some(body)));
        }

        // WHY: query_pairs_mut leaves a spurious bare "?" behind even when
        // nothing is appended — only touch the query when pairs exist.
        if !pairs.is_empty() {
            let mut serializer = url.query_pairs_mut();
            for (key, value) in &pairs {
                serializer.append_pair(key, value);
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
        Ok((url, None))
    }

    /// Fires one request with the current cookie (static or session),
    /// honoring cancellation. No status interpretation.
    async fn send_once(
        &self,
        url: &str,
        body: Option<&str>,
        ct: &CancellationToken,
    ) -> Result<reqwest::Response, SearchIndexerError> {
        self.send_once_with_cookie(url, body, self.request_cookie_header(), ct)
            .await
    }

    /// Like `send_once` but with an explicit `Cookie` header, so the post-login
    /// retry uses the session `login` just established rather than re-reading
    /// the store (which a racing `invalidate` could empty between the store
    /// and the read, spuriously sending the retry cookieless).
    async fn send_once_with_cookie(
        &self,
        url: &str,
        body: Option<&str>,
        cookie: Option<String>,
        ct: &CancellationToken,
    ) -> Result<reqwest::Response, SearchIndexerError> {
        // WHY: a form body switches the verb to POST (application/
        // x-www-form-urlencoded), mirroring the login submit; None stays GET.
        let mut request = match body {
            Some(form) => self
                .http_client
                .post(url)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(form.to_owned())
                .timeout(self.timeout),
            None => self.http_client.get(url).timeout(self.timeout),
        };
        if let Some(cookie) = cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        let fut = request.send();
        tokio::select! {
            result = fut => result.context(error::HttpRequestSnafu { url: redact_secrets(url) }),
            () = ct.cancelled() => Err(SearchIndexerError::Cancelled {
                url: redact_secrets(url),
                location: snafu::Location::new(file!(), line!(), column!()),
            }),
        }
    }

    fn rate_limited(&self, response: &reqwest::Response) -> SearchIndexerError {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        SearchIndexerError::RateLimited {
            indexer_id: self.indexer.id,
            retry_after_seconds: retry_after,
            location: snafu::Location::new(file!(), line!(), column!()),
        }
    }

    fn auth_failed(&self) -> SearchIndexerError {
        SearchIndexerError::AuthFailed {
            indexer_id: self.indexer.id,
            location: snafu::Location::new(file!(), line!(), column!()),
        }
    }

    /// True when a fetched response looks like lost authentication: a 401/403,
    /// or a final URL whose path equals the rendered login path (the tracker
    /// silently 200-redirected the request to its login page).
    ///
    /// WHY: an expired session otherwise 200s a login page and the indexer
    /// yields zero rows forever with no signal.
    fn looks_like_auth_loss(&self, response: &reqwest::Response) -> bool {
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return true;
        }
        self.login_url
            .as_ref()
            .is_some_and(|login_url| response.url().path() == login_url.path())
    }

    async fn send(
        &self,
        url: &str,
        body: Option<&str>,
        ct: CancellationToken,
    ) -> Result<reqwest::Response, SearchIndexerError> {
        let response = self.send_once(url, body, &ct).await?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(self.rate_limited(&response));
        }

        // WHY: interactive sessions expire mid-use — invalidate, re-login
        // ONCE, retry ONCE. A second auth loss is a real AuthFailed.
        if let LoginMethod::Interactive { verb } = &self.login {
            if self.looks_like_auth_loss(&response) {
                self.sessions.invalidate(self.indexer.id);
                // WHY: use the cookie login just established directly — a
                // concurrent client's invalidate can wipe the store between
                // login's store() and a fresh read, which would send the
                // retry cookieless and fail a healthy indexer.
                let established = self.login(*verb, ct.clone()).await?;
                let retry = self
                    .send_once_with_cookie(url, body, established, &ct)
                    .await?;
                if retry.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(self.rate_limited(&retry));
                }
                if self.looks_like_auth_loss(&retry) {
                    return Err(self.auth_failed());
                }
                return Ok(retry);
            }
            return Ok(response);
        }

        // None / Cookie: a 401/403 is an immediate auth failure.
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(self.auth_failed());
        }
        Ok(response)
    }

    async fn fetch_text(
        &self,
        url: &str,
        body: Option<&str>,
        ct: CancellationToken,
    ) -> Result<String, SearchIndexerError> {
        if self.indexer.cf_bypass {
            if body.is_some() {
                // WHY: defensive — POST + cf_bypass is rejected at construction;
                // the bypass proxy is GET-only and cannot carry a form body.
                return Err(self.invalid("POST search is unsupported with cf_bypass".to_string()));
            }
            let response = self.cf_proxy.get(url, ct).await?;
            return Ok(response.body);
        }
        let response = self.send(url, body, ct).await?;
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
        let response = self.send(url, None, ct).await?;
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

    /// Applies `search.rows.filters` (upstream's ParseRowFilters), dropping the
    /// rows a filter rejects. Only `andmatch` is implemented; load-time
    /// validation rejects any other rows filter.
    ///
    /// WHY here and not in the field-filter pipeline: a row filter decides
    /// whether the whole ROW survives, using the query keywords and the row's
    /// title — state the field pipeline never sees.
    fn apply_row_filters(
        &self,
        rows: Vec<BTreeMap<String, String>>,
        ctx: &TemplateContext,
        query: &SearchQuery,
    ) -> Result<Vec<BTreeMap<String, String>>, SearchIndexerError> {
        let specs = &self.definition.search.rows.filters;
        if specs.is_empty() || self.id_search_natively_supported(query) {
            return Ok(rows);
        }
        // WHY parse here as well as at load: the definition holds the YAML
        // form. Load rejected anything unusable, so this cannot fail in
        // practice — parsing keeps the decision below total over one filter
        // per search rather than re-reading names and arguments per row. The
        // args render as templates first (search scope — `.Result` is
        // rejected at load here).
        let specs = template::render_specs(specs, ctx).map_err(|e| self.invalid(e))?;
        let row_filters = filters::parse_row_filters(&specs).map_err(|e| self.invalid(e))?;
        // WHY hoisted: the keywords are the same for every row, so the tokens
        // are built once per search rather than re-split and re-folded per row.
        let token_sets: Vec<Vec<String>> = row_filters
            .iter()
            .map(|filter| match filter {
                filters::RowFilter::AndMatch { character_limit } => {
                    filters::andmatch_tokens(&ctx.keywords, *character_limit)
                }
            })
            .collect();

        let mut kept = Vec::with_capacity(rows.len());
        for row in rows {
            // WHY keep a titleless row: it cannot be keyword-matched, and
            // rows_to_results is where such a row is reported and skipped.
            // Dropping it here would silently remove the only signal that a
            // definition's title selector has stopped matching.
            let Some(title) = row.get("title") else {
                kept.push(row);
                continue;
            };
            if token_sets
                .iter()
                .all(|tokens| filters::andmatch_keeps(title, tokens))
            {
                kept.push(row);
            }
        }
        Ok(kept)
    }

    /// True when the query carries an id the definition advertises as a search
    /// parameter.
    ///
    /// WHY: upstream skips `andmatch` for such a search — an id-based query may
    /// carry no keywords at all, so AND-matching them against titles would drop
    /// every row.
    fn id_search_natively_supported(&self, query: &SearchQuery) -> bool {
        let advertises = |param: &str| {
            self.definition
                .caps
                .modes
                .values()
                .any(|params| params.iter().any(|declared| declared == param))
        };
        (query.imdb_id.is_some() && advertises("imdbid"))
            || (query.tmdb_id.is_some() && advertises("tmdbid"))
            || (query.tvdb_id.is_some() && advertises("tvdbid"))
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
        self.ensure_session(ct.clone()).await?;
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
            let (url, form_body) = self.build_search_request(path, &ctx)?;
            let text = self
                .fetch_text(url.as_str(), form_body.as_deref(), ct.clone())
                .await?;
            let is_json = path
                .response
                .as_ref()
                .and_then(|r| r.response_type.as_deref())
                == Some("json");
            let extracted = if is_json {
                json_extract::extract_rows_json(&text, &self.definition, &ctx, &now)
            } else {
                extract::extract_rows(&text, &self.definition, &ctx, &now)
            };
            let rows = extracted.map_err(|e| SearchIndexerError::ParseResponse {
                url: redact_secrets(url.as_str()),
                error: e,
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;
            let rows = self.apply_row_filters(rows, &ctx, query)?;
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
        let login_test = self
            .definition
            .login
            .as_ref()
            .and_then(|login| login.test.as_ref());
        let test_selector = login_test.and_then(|test| test.selector.clone());
        let test_path = login_test.and_then(|test| test.path.clone());
        let probe = async {
            // WHY: ensure_session logs the interactive indexer in (which
            // already runs login.test as part of the flow); for none/cookie
            // it is a no-op.
            self.ensure_session(ct.clone()).await?;
            let url = match &test_path {
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
            // WHY: when the definition declares a login.test selector, the
            // health check is a content assertion (a reachable login page is
            // NOT a healthy session); otherwise it stays a reachability probe.
            match &test_selector {
                Some(selector) => {
                    let response = self.send(url.as_str(), None, ct).await?;
                    let body = read_body_bounded(
                        response,
                        url.as_str(),
                        self.config.max_response_body_bytes,
                    )
                    .await?;
                    let matched = extract::selector_matches(&body, selector)
                        .map_err(|e| self.invalid(format!("login test: {e}")))?;
                    if matched {
                        Ok(())
                    } else {
                        Err(self.login_failed(
                            "login test selector matched nothing — login likely failed".to_string(),
                        ))
                    }
                }
                None => {
                    let response = self.send(url.as_str(), None, ct).await?;
                    response
                        .error_for_status()
                        .map(|_| ())
                        .context(error::HttpRequestSnafu {
                            url: redact_secrets(url.as_str()),
                        })
                }
            }
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

    // WHY: skip `url` — download URLs carry secrets (apikey/passkey in the
    // query) and #[instrument] would capture the raw value as a span field
    // visible to any event emitted in the span.
    #[instrument(skip(self, url, ct), fields(indexer_id = self.indexer.id))]
    async fn download(
        &self,
        url: &str,
        ct: CancellationToken,
    ) -> Result<DownloadResponse, SearchIndexerError> {
        if url.starts_with("magnet:") {
            return Ok(DownloadResponse::MagnetUri(url.to_string()));
        }
        // WHY: a gated .torrent needs the login cookie on the fetch; a magnet
        // above never touches the network, so the session is only ensured
        // once a real download is imminent.
        self.ensure_session(ct.clone()).await?;
        // SAFETY: download URLs originate in scraped third-party HTML —
        // validate scheme + resolved addresses before any fetch.
        validate_fetch_url(url).await?;

        let final_url = match &self.definition.download {
            Some(block) if block.selector.is_some() => {
                let page = self.fetch_text(url, None, ct.clone()).await?;
                let link = extract::extract_download_link(
                    &page,
                    block.selector.as_deref().unwrap_or_default(),
                    block.attribute.as_deref(),
                )
                .map_err(|e| self.invalid(format!("download: {e}")))?;
                // WHY: download filter args are templates in config scope
                // (there is no row or query at download time; `.Result` and
                // `.Query` render as absent).
                let specs = template::render_specs(&block.filters, &self.login_template_context())
                    .map_err(|e| self.invalid(format!("download filters: {e}")))?;
                let link = filters::apply(link, &specs, &Zoned::now())
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

/// Rejects an indexer row's `settings` overrides that the definition does not
/// declare, or whose value does not fit the declared `type`.
///
/// WHY: `cookie` is always sourced from the row's `api_key` — accepting it
/// here would let a settings override silently shadow the real session
/// cookie the login flow depends on.
fn validate_settings(
    indexer: &IndexerConfig,
    definition: &CardigannDefinition,
) -> Result<(), SearchIndexerError> {
    let invalid = |reason: String| SearchIndexerError::SettingsInvalid {
        definition_id: definition.id.clone(),
        indexer_id: indexer.id,
        reason,
        location: snafu::Location::new(file!(), line!(), column!()),
    };

    for (key, value) in &indexer.settings {
        if key == "cookie" {
            return Err(invalid(
                "\"cookie\" is a reserved setting name sourced from the indexer's \
                 api_key field, not overridable via settings"
                    .to_string(),
            ));
        }
        let Some(field) = definition.settings.iter().find(|s| &s.name == key) else {
            return Err(invalid(format!(
                "unknown setting {key:?} (not declared in this definition's settings)"
            )));
        };
        match field.field_type.as_deref() {
            Some("select") => {
                let known = field
                    .options
                    .as_ref()
                    .is_some_and(|options| options.keys().any(|opt| &opt.0 == value));
                if !known {
                    return Err(invalid(format!(
                        "setting {key:?} value {value:?} is not one of the declared options"
                    )));
                }
            }
            Some("checkbox") => {
                if value != "true" && value != "false" {
                    return Err(invalid(format!(
                        "setting {key:?} is a checkbox; value must be \"true\" or \"false\", \
                         got {value:?}"
                    )));
                }
            }
            Some("info") => {
                return Err(invalid(format!(
                    "setting {key:?} is type \"info\" and cannot be overridden"
                )));
            }
            // WHY: text/password/absent-type settings accept any string.
            None | Some(_) => {}
        }
    }
    Ok(())
}

/// Builds the static `.Config` seed: definition defaults, then the indexer's
/// settings overrides, then the injected static cookie (never
/// user-overridable — see `validate_settings`).
fn build_config_seed(
    definition: &CardigannDefinition,
    indexer_settings: &BTreeMap<String, String>,
    cookie: Option<&str>,
) -> BTreeMap<String, String> {
    let mut config = BTreeMap::new();
    for setting in &definition.settings {
        config.insert(
            setting.name.clone(),
            setting
                .default
                .as_ref()
                .map(|d| d.0.clone())
                .unwrap_or_default(),
        );
    }
    for (key, value) in indexer_settings {
        config.insert(key.clone(), value.clone());
    }
    if let Some(cookie) = cookie {
        config.insert("cookie".to_string(), cookie.to_string());
    }
    config
}

/// Resolves the login block to a [`LoginMethod`].
///
/// For interactive methods (form/post/get) every `.Config.<key>` referenced
/// in VALUE position by `login.inputs` must resolve to a non-empty value
/// after the settings overlay — a missing credential fails loud at
/// construction rather than sending an empty username/password to the
/// tracker. Keys referenced only inside `{{ if .Config.x }}` conditions are
/// exempt: an unset optional setting simply makes the branch false.
fn resolve_login(
    indexer: &IndexerConfig,
    definition: &CardigannDefinition,
) -> Result<LoginMethod, SearchIndexerError> {
    let Some(login) = &definition.login else {
        return Ok(LoginMethod::None);
    };
    // NOTE: Cardigann's default method when the login block omits one is
    // "form".
    let verb = match login.method.as_deref().unwrap_or("form") {
        "none" => return Ok(LoginMethod::None),
        "cookie" => {
            return match &indexer.api_key {
                Some(cookie) if !cookie.trim().is_empty() => {
                    Ok(LoginMethod::Cookie(cookie.clone()))
                }
                _ => Err(SearchIndexerError::CookieAuthRequired {
                    definition_id: definition.id.clone(),
                    indexer_id: indexer.id,
                    location: snafu::Location::new(file!(), line!(), column!()),
                }),
            };
        }
        "form" => LoginVerb::Form,
        "post" => LoginVerb::Post,
        "get" => LoginVerb::Get,
        other => {
            return Err(SearchIndexerError::LoginUnsupported {
                definition_id: definition.id.clone(),
                method: other.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            });
        }
    };

    let config = build_config_seed(definition, &indexer.settings, None);
    for value in login.inputs.values() {
        for key in template::config_keys(&value.0) {
            let resolved = config.get(&key).map(String::as_str).unwrap_or_default();
            if resolved.trim().is_empty() {
                return Err(SearchIndexerError::SettingsInvalid {
                    definition_id: definition.id.clone(),
                    indexer_id: indexer.id,
                    reason: format!(
                        "login requires a non-empty value for setting {key:?}; set it via the \
                         indexer's settings (e.g. username/password)"
                    ),
                    location: snafu::Location::new(file!(), line!(), column!()),
                });
            }
        }
    }
    Ok(LoginMethod::Interactive { verb })
}

/// Renders `login.path` (config-only context) and joins it on the site base.
fn resolve_login_url(
    indexer: &IndexerConfig,
    definition: &CardigannDefinition,
    base_url: &Url,
) -> Result<Url, SearchIndexerError> {
    let invalid = |reason: String| SearchIndexerError::DefinitionInvalid {
        definition_id: definition.id.clone(),
        reason,
        location: snafu::Location::new(file!(), line!(), column!()),
    };
    // INVARIANT: validate() rejects an interactive login without a login.path.
    let path = definition
        .login
        .as_ref()
        .and_then(|login| login.path.as_ref())
        .ok_or_else(|| invalid("interactive login requires login.path".to_string()))?;
    let cookie = match &indexer.api_key {
        Some(cookie) if !cookie.trim().is_empty() => Some(cookie.as_str()),
        _ => None,
    };
    let config = build_config_seed(definition, &indexer.settings, cookie);
    let ctx = TemplateContext {
        keywords: String::new(),
        categories: Vec::new(),
        config,
        query: BTreeMap::new(),
        result: BTreeMap::new(),
    };
    let rendered = ctx
        .render_url(path)
        .map_err(|e| invalid(format!("login path: {e}")))?;
    let resolved = base_url
        .join(rendered.trim_start_matches('/'))
        .map_err(|e| invalid(format!("login path {rendered:?}: {e}")))?;
    // SECURITY GATE: login.path must stay on the indexer's own host. A path
    // carrying its own scheme (e.g. `http://attacker.example/harvest`) makes
    // `Url::join` return it verbatim, ignoring base_url — which would point
    // the login-page GET (and the cookies it harvests) at an arbitrary host,
    // the very off-site credential exposure require_same_host guards on the
    // submit URL and every redirect hop. Fail loud at construction.
    if !matches!(resolved.scheme(), "http" | "https") || resolved.host() != base_url.host() {
        return Err(invalid(format!(
            "login path {rendered:?} resolves to host {:?}, off the indexer host {:?}; \
             refusing to fetch the login page off-site",
            resolved.host_str().unwrap_or("<none>"),
            base_url.host_str().unwrap_or("<none>")
        )));
    }
    Ok(resolved)
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
