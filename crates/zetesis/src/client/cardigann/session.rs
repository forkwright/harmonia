//! Per-indexer Cardigann login sessions: cookie store plus the interactive
//! (form/post/get) login flow.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use dashmap::DashMap;
use jiff::Zoned;
use reqwest::header;
use scraper::{Html, Selector};
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::instrument;
use url::Url;

use crate::client::cardigann::definition::{LoginBlock, LoginTest};
use crate::client::cardigann::{CardigannClient, extract, template::TemplateContext};
use crate::client::{SsrfGuardResolver, read_body_bounded};
use crate::error::{self, SearchIndexerError};

/// Upper bound on manual redirect follows during one login flow.
const MAX_LOGIN_REDIRECTS: usize = 5;

/// Login strategy resolved at client construction from the definition's
/// `login:` block plus the indexer row.
pub(crate) enum LoginMethod {
    /// No authentication.
    None,
    /// Static `Cookie` header from the indexer row's `api_key` column.
    Cookie(String),
    /// Session-based login executed against the site.
    Interactive { verb: LoginVerb },
}

/// HTTP shape of an interactive login.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoginVerb {
    /// GET the login page, harvest the form (hidden CSRF included), POST.
    Form,
    /// POST rendered inputs straight to `login.path`.
    Post,
    /// GET `login.path` with rendered inputs as query pairs.
    Get,
}

// WHY: manual Debug — the Cookie variant carries the row's session cookie.
impl fmt::Debug for LoginMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Cookie(_) => f.write_str("Cookie([redacted])"),
            Self::Interactive { verb } => write!(f, "Interactive({verb:?})"),
        }
    }
}

/// Interactive-login sessions keyed by indexer-instance id.
///
/// WHY: keyed per indexer id, NOT per host — two indexer rows may target the
/// same host with different accounts, and their sessions must never mix. The
/// Cloudflare `CookieStore` is host-keyed byparr state and stays
/// deliberately separate. In-memory only: a restart means a fresh login,
/// matching the CF-bypass cookie posture.
///
/// There is no login single-flight lock: two concurrent misses both log in
/// and the second `store` wins — benign, and lock-free keeps every reader
/// trivial (the per-indexer rate limiter damps the duplicate request).
#[derive(Default)]
pub struct SessionStore {
    sessions: DashMap<i64, Session>,
}

struct Session {
    cookies: BTreeMap<String, String>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Cookie` header value for the indexer's live session, if any.
    ///
    /// INVARIANT: the DashMap guard is dropped before this returns — callers
    /// can never hold a shard lock across an await (search.rs discipline).
    pub fn get_cookie_header(&self, indexer_id: i64) -> Option<String> {
        let session = self.sessions.get(&indexer_id)?;
        if session.cookies.is_empty() {
            return None;
        }
        Some(format_cookie_header(&session.cookies))
    }

    pub fn store(&self, indexer_id: i64, cookies: BTreeMap<String, String>) {
        self.sessions.insert(indexer_id, Session { cookies });
    }

    pub fn invalidate(&self, indexer_id: i64) {
        self.sessions.remove(&indexer_id);
    }
}

// WHY: manual Debug — cookie VALUES are session credentials; only the
// indexer ids and cookie names are printable.
impl fmt::Debug for SessionStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for entry in &self.sessions {
            map.entry(
                entry.key(),
                &entry.value().cookies.keys().collect::<Vec<_>>(),
            );
        }
        map.finish()
    }
}

/// A parsed login `<form>`: its `action` and every named input's value
/// (hidden CSRF tokens included).
struct LoginForm {
    action: Option<String>,
    inputs: Vec<(String, String)>,
}

impl CardigannClient {
    /// Establishes a live session for interactive login methods; a no-op for
    /// `none`/`cookie` and when the store already has a session.
    pub(crate) async fn ensure_session(
        &self,
        ct: CancellationToken,
    ) -> Result<(), SearchIndexerError> {
        let LoginMethod::Interactive { verb } = &self.login else {
            return Ok(());
        };
        let verb = *verb;
        if self.sessions.get_cookie_header(self.indexer.id).is_some() {
            return Ok(());
        }
        self.login(verb, ct).await.map(|_| ())
    }

    /// One full login: page fetch + form harvest (form verb), credential
    /// submit, manual redirect chase, error-block check, `login.test`
    /// check, session store.
    #[instrument(
        skip(self, ct),
        fields(indexer_id = self.indexer.id, definition_id = %self.definition.id)
    )]
    /// Runs one full login and returns the `Cookie` header it established, so
    /// a caller can use the fresh session directly without re-reading the
    /// shared store (which a concurrent client's `invalidate` may have wiped).
    pub(super) async fn login(
        &self,
        verb: LoginVerb,
        ct: CancellationToken,
    ) -> Result<Option<String>, SearchIndexerError> {
        let (Some(login), Some(login_url)) =
            (self.definition.login.as_ref(), self.login_url.as_ref())
        else {
            // INVARIANT: construction sets both for every interactive method.
            return Err(self.login_failed("interactive login without a login block".to_string()));
        };
        let ctx = self.login_template_context();
        let client = self.login_http_client()?;
        let mut cookies: BTreeMap<String, String> = BTreeMap::new();

        // WHY: inputs render PLAIN — encoding happens exactly once at
        // body/query serialization below; render_url here would
        // double-encode credentials.
        let mut inputs: Vec<(String, String)> = Vec::new();
        let submit_url = match verb {
            LoginVerb::Form => {
                let label = redact_query(login_url);
                let response = self
                    .login_send(client.get(login_url.clone()), &label, &ct)
                    .await?;
                harvest_cookies(response.headers(), &mut cookies);
                let response = self
                    .follow_login_redirects(&client, response, &mut cookies, &ct)
                    .await?;
                let page_url = response.url().clone();
                let body = read_body_bounded(response, &label, self.config.max_response_body_bytes)
                    .await?;
                let form = parse_login_form(&body, login.form.as_deref().unwrap_or("form"))
                    .map_err(|e| self.invalid(e))?
                    .ok_or_else(|| {
                        self.login_failed("login form selector matched nothing".to_string())
                    })?;
                inputs = form.inputs;
                for (key, value) in &login.inputs {
                    let rendered = ctx
                        .render(&value.0)
                        .map_err(|e| self.invalid(format!("login input {key}: {e}")))?;
                    override_input(&mut inputs, key, rendered);
                }
                self.resolve_submit_url(login, &ctx, &page_url, form.action.as_deref())?
            }
            LoginVerb::Post | LoginVerb::Get => {
                for (key, value) in &login.inputs {
                    let rendered = ctx
                        .render(&value.0)
                        .map_err(|e| self.invalid(format!("login input {key}: {e}")))?;
                    inputs.push((key.clone(), rendered));
                }
                self.resolve_submit_url(login, &ctx, login_url, None)?
            }
        };

        // WHY: snapshot the cookies harvested BEFORE credentials are sent (the
        // form-page GET and its redirects — e.g. an anonymous PHPSESSID). A
        // rejected password commonly re-renders 200 with that same cookie
        // still set, so a non-empty jar alone is not proof of a successful
        // login; the success gate below requires a login.test pass or a
        // cookie the credential exchange itself set or rotated.
        let pre_auth_cookies = cookies.clone();

        let label = redact_query(&submit_url);
        let request = match verb {
            LoginVerb::Get => {
                let mut url = submit_url;
                if !inputs.is_empty() {
                    // WHY: append_pair percent-encodes each credential value;
                    // the inputs above were rendered unencoded.
                    let mut pairs = url.query_pairs_mut();
                    for (key, value) in &inputs {
                        pairs.append_pair(key, value);
                    }
                }
                client.get(url)
            }
            LoginVerb::Form | LoginVerb::Post => {
                let body: String = url::form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(inputs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                    .finish();
                client
                    .post(submit_url)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(body)
            }
        };
        let request = match cookie_header(&cookies) {
            Some(cookie) => request.header(header::COOKIE, cookie),
            None => request,
        };
        let response = self.login_send(request, &label, &ct).await?;
        harvest_cookies(response.headers(), &mut cookies);
        let response = self
            .follow_login_redirects(&client, response, &mut cookies, &ct)
            .await?;

        let status = response.status();
        let final_label = redact_query(response.url());
        let body =
            read_body_bounded(response, &final_label, self.config.max_response_body_bytes).await?;
        for block in &login.error {
            // NOTE: the reason carries only the site's own message — never
            // the submitted credentials or request body.
            if let Some(message) = extract::extract_error_message(
                &body,
                &block.selector,
                block.message.as_ref(),
                &ctx,
                &Zoned::now(),
            )
            .map_err(|e| self.invalid(format!("login error block: {e}")))?
            {
                return Err(self.login_failed(message));
            }
        }
        if !status.is_success() {
            return Err(self.login_failed(format!("login response status {status}")));
        }

        let mut verified_by_test = false;
        if let Some(test) = &login.test
            && let Some(selector) = &test.selector
        {
            self.verify_login_test(&client, test, selector, &mut cookies, &ct)
                .await?;
            verified_by_test = true;
        }

        if cookies.is_empty() {
            // WHY: a cookieless "success" cannot authenticate later requests;
            // storing it would silently re-run this flow on every search.
            return Err(
                self.login_failed("login flow completed without any session cookie".to_string())
            );
        }
        // WHY: reject a "success" that rests only on pre-credential cookies.
        // A real login either passes login.test or sets/rotates a cookie
        // during the credential exchange; when it does neither, the password
        // was rejected (or the definition lacks a login.test to confirm it) —
        // storing the anonymous cookie would silently serve public/empty
        // results forever with no failure signal to the operator.
        let auth_cookie_set = cookies
            .iter()
            .any(|(name, value)| pre_auth_cookies.get(name) != Some(value));
        if !verified_by_test && !auth_cookie_set {
            return Err(self.login_failed(
                "login set no post-authentication cookie and the definition has no login.test \
                 to confirm success; the credentials were likely rejected"
                    .to_string(),
            ));
        }
        let stored = cookie_header(&cookies);
        self.sessions.store(self.indexer.id, cookies);
        Ok(stored)
    }

    /// Resolves where credentials are submitted: `login.submitpath`
    /// (rendered, joined on the site base), else the form's `action`
    /// resolved against the login-page URL, else the login-page URL itself.
    ///
    /// SECURITY GATE (credential-exfiltration chokepoint): the resolved host
    /// must equal the site base host. A hostile definition or a compromised
    /// login page must not point the credential submit off-origin — upstream
    /// Jackett does not enforce this; this engine deliberately does.
    fn resolve_submit_url(
        &self,
        login: &LoginBlock,
        ctx: &TemplateContext,
        page_url: &Url,
        action: Option<&str>,
    ) -> Result<Url, SearchIndexerError> {
        let submit = if let Some(submitpath) = &login.submitpath {
            let rendered = ctx
                .render_url(submitpath)
                .map_err(|e| self.invalid(format!("login submitpath: {e}")))?;
            self.base_url
                .join(rendered.trim_start_matches('/'))
                .map_err(|e| self.invalid(format!("login submitpath {rendered:?}: {e}")))?
        } else if let Some(action) = action.map(str::trim).filter(|a| !a.is_empty()) {
            page_url.join(action).map_err(|e| {
                self.login_failed(format!(
                    "login form action {action:?} does not resolve: {e}"
                ))
            })?
        } else {
            page_url.clone()
        };
        self.require_same_host(&submit, "login submit")?;
        Ok(submit)
    }

    /// Rejects any login-flow target that leaves the indexer's host.
    fn require_same_host(&self, url: &Url, what: &str) -> Result<(), SearchIndexerError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(self.login_failed(format!(
                "{what} target must be http(s), got scheme {:?}",
                url.scheme()
            )));
        }
        if url.host() != self.base_url.host() {
            return Err(self.login_failed(format!(
                "{what} target host {:?} differs from the indexer host {:?}; \
                 refusing to send credentials off-site",
                url.host_str().unwrap_or("<none>"),
                self.base_url.host_str().unwrap_or("<none>")
            )));
        }
        Ok(())
    }

    /// One-shot no-redirect client for the login flow.
    ///
    /// WHY: the shared client auto-follows redirects and drops each hop's
    /// Set-Cookie; login must own every hop. Mirrors `build_http_client`
    /// (timeout + SSRF-guard resolver); a throwaway client per login is fine
    /// — this runs once per session lifetime. No `unwrap_or_default`
    /// fallback: a default client follows redirects and drops the SSRF
    /// guard, so a build failure must fail closed.
    fn login_http_client(&self) -> Result<reqwest::Client, SearchIndexerError> {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(self.timeout)
            .dns_resolver(Arc::new(SsrfGuardResolver))
            .build()
            .map_err(|e| self.login_failed(format!("login HTTP client build failed: {e}")))
    }

    /// Sends one login-flow request, honoring cancellation.
    ///
    /// NOTE: `url_label` is pre-redacted (query stripped) — get-method login
    /// URLs carry credentials in the query string.
    async fn login_send(
        &self,
        request: reqwest::RequestBuilder,
        url_label: &str,
        ct: &CancellationToken,
    ) -> Result<reqwest::Response, SearchIndexerError> {
        let fut = request.send();
        tokio::select! {
            result = fut => result.context(error::HttpRequestSnafu { url: url_label.to_string() }),
            () = ct.cancelled() => Err(SearchIndexerError::Cancelled {
                url: url_label.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            }),
        }
    }

    /// Chases 3xx responses by hand: same-host enforced, at most
    /// [`MAX_LOGIN_REDIRECTS`] hops, cookies harvested per hop.
    ///
    /// WHY: manual on purpose — reqwest's auto-redirect discards each hop's
    /// Set-Cookie, and login flows routinely set the session cookie on the
    /// 302 itself.
    async fn follow_login_redirects(
        &self,
        client: &reqwest::Client,
        mut response: reqwest::Response,
        cookies: &mut BTreeMap<String, String>,
        ct: &CancellationToken,
    ) -> Result<reqwest::Response, SearchIndexerError> {
        let mut hops = 0;
        while response.status().is_redirection() {
            let Some(location) = response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
            else {
                // NOTE: a 3xx without Location is a final response.
                return Ok(response);
            };
            hops += 1;
            if hops > MAX_LOGIN_REDIRECTS {
                return Err(
                    self.login_failed(format!("login exceeded {MAX_LOGIN_REDIRECTS} redirects"))
                );
            }
            let next = response.url().join(&location).map_err(|e| {
                self.login_failed(format!("login redirect target {location:?}: {e}"))
            })?;
            self.require_same_host(&next, "login redirect")?;
            let label = redact_query(&next);
            let mut request = client.get(next);
            if let Some(cookie) = cookie_header(cookies) {
                request = request.header(header::COOKIE, cookie);
            }
            response = self.login_send(request, &label, ct).await?;
            harvest_cookies(response.headers(), cookies);
        }
        Ok(response)
    }

    /// `login.test` assertion: GET the test path with the fresh cookies; the
    /// selector must match or the login is treated as failed.
    ///
    /// WHY: many trackers answer a failed login with HTTP 200 — only a
    /// content assertion can tell a session apart from a login page.
    async fn verify_login_test(
        &self,
        client: &reqwest::Client,
        test: &LoginTest,
        selector: &str,
        cookies: &mut BTreeMap<String, String>,
        ct: &CancellationToken,
    ) -> Result<(), SearchIndexerError> {
        let url = match &test.path {
            Some(path) => self
                .base_url
                .join(path.trim_start_matches('/'))
                .map_err(|e| self.invalid(format!("login test path {path:?}: {e}")))?,
            None => self.base_url.clone(),
        };
        let label = redact_query(&url);
        let mut request = client.get(url);
        if let Some(cookie) = cookie_header(cookies) {
            request = request.header(header::COOKIE, cookie);
        }
        let response = self.login_send(request, &label, ct).await?;
        harvest_cookies(response.headers(), cookies);
        let response = self
            .follow_login_redirects(client, response, cookies, ct)
            .await?;
        let body = read_body_bounded(response, &label, self.config.max_response_body_bytes).await?;
        let matched = extract::selector_matches(&body, selector)
            .map_err(|e| self.invalid(format!("login test: {e}")))?;
        if !matched {
            return Err(self.login_failed(
                "login test selector matched nothing — login likely failed".to_string(),
            ));
        }
        Ok(())
    }
}

/// Parses the login page's `<form>` and its named inputs.
///
/// WHY: sync on purpose — `scraper::Html` is `!Send` and must be created and
/// dropped between awaits (see extract.rs).
fn parse_login_form(body: &str, form_selector: &str) -> Result<Option<LoginForm>, String> {
    let document = Html::parse_document(body);
    let selector =
        Selector::parse(form_selector).map_err(|e| format!("login form {form_selector:?}: {e}"))?;
    let Some(form) = document.select(&selector).next() else {
        return Ok(None);
    };
    let input_selector =
        Selector::parse("input[name]").map_err(|e| format!("input selector: {e}"))?;
    let mut inputs = Vec::new();
    for input in form.select(&input_selector) {
        let Some(name) = input.value().attr("name") else {
            continue;
        };
        // WHY: a valueless input (empty username/password field) contributes
        // an empty string the definition's login.inputs then overlays.
        let value = input.value().attr("value").unwrap_or("");
        override_input(&mut inputs, name, value.to_string());
    }
    Ok(Some(LoginForm {
        action: form.value().attr("action").map(str::to_string),
        inputs,
    }))
}

/// Replaces the named input in place (keeping page order) or appends it.
fn override_input(inputs: &mut Vec<(String, String)>, name: &str, value: String) {
    match inputs.iter_mut().find(|(n, _)| n == name) {
        Some((_, v)) => *v = value,
        None => inputs.push((name.to_string(), value)),
    }
}

/// Accumulates `name=value` pairs from every `Set-Cookie` header; malformed
/// headers are skipped.
fn harvest_cookies(headers: &header::HeaderMap, cookies: &mut BTreeMap<String, String>) {
    for value in headers.get_all(header::SET_COOKIE) {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        let pair = raw.split(';').next().unwrap_or(raw);
        let Some((name, val)) = pair.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        cookies.insert(name.to_string(), val.trim().to_string());
    }
}

fn cookie_header(cookies: &BTreeMap<String, String>) -> Option<String> {
    if cookies.is_empty() {
        return None;
    }
    Some(format_cookie_header(cookies))
}

fn format_cookie_header(cookies: &BTreeMap<String, String>) -> String {
    cookies
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// URL rendered for logs/errors with its query stripped.
///
/// WHY: get-method login URLs carry credentials in the query; even the
/// apikey-focused `redact_api_key` would leak them.
fn redact_query(url: &Url) -> String {
    let mut clean = url.clone();
    clean.set_query(None);
    clean.to_string()
}
