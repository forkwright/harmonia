//! Serve-hosted MCP acquisition bridge (#609).
//!
//! `harmonia mcp` is a bare stdio process with no config, DB, or services —
//! the live `DynSearchService`/`DynQueueManager` only exist inside a running
//! `harmonia serve`. Enqueue/cancel MUST reach the RUNNING syntaxis service
//! or the row is invisible until restart (the #499/#469 defect class), so
//! the stdio surface cannot be a thin in-process adapter for these tools —
//! it reaches this Unix-domain-socket bridge instead, spawned by `serve`
//! right after `AppState` construction and holding Arc clones of the SAME
//! services `AppState` got.
//!
//! Wire protocol: newline-delimited JSON-RPC 2.0 — one `tools/call` request
//! line in, one response line out per connection request (mirrors the
//! stdio surface's own framing so both sides share one mental model).
//!
//! Auth: filesystem, not HTTP. The socket is 0600, owned by the process
//! that ran `harmonia serve`; anyone who can connect can already read
//! `harmonia.toml` and the database file. `structuredContent`/`text`
//! payloads still run every `download_url` through `paroche::redact` — the
//! MCP client is an LLM whose transcripts leave the box, so a raw indexer
//! passkey must never reach it even though the local-operator trust model
//! doesn't otherwise gate this surface.
//!
//! The socket lives in a DEDICATED `harmonia-mcp/` runtime subdirectory
//! (chmod'd 0700) beside `database.db_path` — never in `db_path`'s own
//! directory, which may be shared with other users/processes we must not
//! chmod (#609 adversarial review finding).

// NOTE: `Permissions`/`PermissionsExt` live at module scope — `from_mode` is
// a pure value builder (not I/O), and keeping the `std::fs::` path out of the
// async `spawn` body avoids a false-positive blocking-io-in-async lint on the
// mode constructor there; the actual filesystem writes all go through
// `tokio::fs`.
#[cfg(unix)]
use std::fs::Permissions;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apotheke::DbPools;
use apotheke::repo::want::{self, Release};
use paroche::state::{DynQueueManager, DynSearchService, EnqueueItem, ServiceError};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::Instrument;
use url::Url;
use uuid::Uuid;

const ACQUISITION_TOOLS: [&str; 4] = [
    "harmonia_search_releases",
    "harmonia_enqueue_download",
    "harmonia_list_downloads",
    "harmonia_cancel_download",
];

// WHY: `releases.indexer_id` is a plain NOT NULL i64, not an FK — a
// client-supplied magnet URI never came from a cataloged indexer, so it
// gets this sentinel rather than a fabricated real indexer id. Real
// `indexers.id` values are SQLite rowids starting at 1, never 0.
const MANUAL_MAGNET_INDEXER_ID: i64 = 0;

/// True for the 4 tool names the bridge serves — the stdio surface uses
/// this to decide "forward to the bridge" vs. "run in-process."
pub fn is_acquisition_tool(name: &str) -> bool {
    ACQUISITION_TOOLS.contains(&name)
}

/// Narrow context handed to the bridge listener — Arc clones of exactly
/// what `AppState` got for `search`/`queue`/`db`. No locks, nothing held
/// across an `.await`.
#[derive(Clone)]
pub struct BridgeContext {
    pub search: Arc<dyn DynSearchService>,
    pub queue: Arc<dyn DynQueueManager>,
    pub db: Arc<DbPools>,
}

impl BridgeContext {
    /// Assembles the bridge context from the live acquisition services.
    pub fn new(
        search: Arc<dyn DynSearchService>,
        queue: Arc<dyn DynQueueManager>,
        db: Arc<DbPools>,
    ) -> Self {
        Self { search, queue, db }
    }

    /// Dispatches one tool call to its handler. Exposed directly (not only
    /// via the socket) so dispatch-level tests can drive it without a real
    /// UDS connection. Pure — no I/O beyond the service calls themselves.
    pub async fn dispatch(&self, name: &str, arguments: &Value) -> Value {
        match name {
            "harmonia_search_releases" => handle_search(arguments, self).await,
            "harmonia_enqueue_download" => handle_enqueue(arguments, self).await,
            "harmonia_list_downloads" => handle_list(arguments, self).await,
            "harmonia_cancel_download" => handle_cancel(arguments, self).await,
            other => tool_error(format!("unknown acquisition tool: {other}")),
        }
    }
}

/// Derives the bridge socket path FROM config — the SAME derivation both
/// `harmonia serve` (binds) and `harmonia mcp` (connects) run
/// independently, so the two processes agree without operator wiring.
/// `mcp.socket_path` overrides; otherwise a DEDICATED `harmonia-mcp/`
/// runtime subdirectory beside `database.db_path`, holding
/// `harmonia-mcp.sock`.
///
/// WHY a dedicated subdir, not a sibling file: `spawn` chmods the socket's
/// parent directory `0700` so only the operator can enter it. `db_path`'s
/// own directory may be shared (other files, other users) — chmodding it
/// would be wrong (and its failure would be a false-fatal serve-startup
/// error). A subdirectory we alone create is ours to chmod.
pub fn resolve_socket_path(config: &horismos::Config) -> PathBuf {
    config.mcp.socket_path.clone().unwrap_or_else(|| {
        let parent = config
            .database
            .db_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        parent.join("harmonia-mcp").join("harmonia-mcp.sock")
    })
}

// ── JSON-RPC / MCP tool-result envelope helpers ─────────────────────────────
//
// Shared by this module's own per-connection protocol AND the stdio surface
// (`archon::mcp_bridge::{...}`, called from the bin crate's mcp.rs) — one
// definition of what a JSON-RPC response / MCP tool result looks like.

pub fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// A tool-level failure (`isError: true`) — a normal JSON-RPC SUCCESS
/// response whose result reports the tool itself failed. Distinct from a
/// JSON-RPC protocol error: a bad release_id is the caller's input, not a
/// broken wire.
pub fn tool_error(message: impl Into<String>) -> Value {
    let message = message.into();
    json!({
        "content": [{ "type": "text", "text": message }],
        "structuredContent": { "ok": false },
        "isError": true
    })
}

/// A tool success carrying a structured JSON payload — `structuredContent`
/// holds it verbatim; `text` is its compact serialization for clients that
/// only render the text block.
pub fn tool_success_json(payload: Value) -> Value {
    let text = serde_json::to_string(&payload).unwrap_or_else(|_| payload.to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": payload,
        "isError": false
    })
}

fn service_error_to_tool_error(error: ServiceError) -> Value {
    match error {
        ServiceError::NotFound => tool_error("not found"),
        ServiceError::NotAvailable => {
            tool_error("the backing acquisition service is not available")
        }
        ServiceError::InvalidInput(message) => tool_error(message),
        ServiceError::Internal(message) => {
            // WHY: the tool result carries no detail for an internal fault —
            // mirrors paroche::error's ParocheError::Internal mapping: the
            // detail lands in the log or it is lost entirely.
            tracing::error!(%message, "mcp acquisition bridge: service call failed");
            tool_error("internal service error — see server logs")
        }
        // WHY: ServiceError is #[non_exhaustive] — a future variant degrades
        // to a generic internal error rather than failing to build.
        other => {
            tracing::error!(?other, "mcp acquisition bridge: unclassified service error");
            tool_error("internal service error — see server logs")
        }
    }
}

// ── Tool handlers (pure — no I/O beyond the service calls themselves) ──────

async fn handle_search(arguments: &Value, ctx: &BridgeContext) -> Value {
    let request: paroche::routes::search::SearchRequest =
        match serde_json::from_value(arguments.clone()) {
            Ok(r) => r,
            Err(e) => return tool_error(format!("invalid search arguments: {e}")),
        };
    // WHY: builds the SAME query JSON the HTTP route builds
    // (paroche::routes::search::search) — the search service reads generic
    // JSON keys, not the SearchRequest type, so round-tripping through the
    // route's own DTO keeps the two surfaces from drifting apart.
    let query = match serde_json::to_value(&request) {
        Ok(v) => v,
        Err(e) => return tool_error(format!("failed to encode search query: {e}")),
    };
    match ctx.search.search(query).await {
        Ok(mut results) => {
            // WHY: redaction stays ON for MCP output — the client transcript
            // leaves the box (an LLM), so a raw indexer apikey/passkey must
            // never reach it, exactly like the HTTP /api/v1/search route.
            paroche::redact::redact_download_urls_in_json(&mut results);
            tool_success_json(results)
        }
        Err(error) => service_error_to_tool_error(error),
    }
}

#[derive(serde::Deserialize)]
struct EnqueueArguments {
    release_id: Option<String>,
    magnet: Option<String>,
    #[serde(default = "default_enqueue_priority")]
    priority: u8,
    want_id: Option<String>,
}

fn default_enqueue_priority() -> u8 {
    4
}

async fn handle_enqueue(arguments: &Value, ctx: &BridgeContext) -> Value {
    let args: EnqueueArguments = match serde_json::from_value(arguments.clone()) {
        Ok(a) => a,
        Err(e) => return tool_error(format!("invalid enqueue arguments: {e}")),
    };

    let (release_id, download_url, protocol, info_hash, title, size_bytes, indexer_id) =
        match (args.release_id.as_deref(), args.magnet.as_deref()) {
            (Some(_), Some(_)) => {
                return tool_error("exactly one of release_id or magnet is required, not both");
            }
            (None, None) => return tool_error("exactly one of release_id or magnet is required"),
            (Some(raw_release_id), None) => {
                let release_id = match Uuid::parse_str(raw_release_id) {
                    Ok(id) => id,
                    Err(_) => return tool_error("release_id must be a valid UUID"),
                };
                // WHY: the #608 seam — resolves the credentialed download
                // URL server-side so the indexer credential never crosses
                // this tool boundary.
                match ctx.search.resolve_release(release_id).await {
                    Ok(resolved) => (
                        release_id,
                        resolved.download_url,
                        resolved.protocol,
                        resolved.info_hash,
                        resolved.title,
                        resolved.size_bytes,
                        resolved.indexer_id,
                    ),
                    Err(ServiceError::NotFound) => {
                        return tool_error(format!(
                            "release {release_id} not found or its search result cache entry \
                             expired — search again"
                        ));
                    }
                    Err(error) => return service_error_to_tool_error(error),
                }
            }
            (None, Some(magnet)) => {
                let parsed = match Url::parse(magnet) {
                    Ok(u) => u,
                    Err(_) => return tool_error("magnet is not a valid URI"),
                };
                if parsed.scheme() != "magnet" {
                    return tool_error(
                        "magnet must be a magnet: URI — raw http(s) URLs are rejected; use \
                         release_id for credentialed indexer results",
                    );
                }
                // WHY: `dn` (display name) / `xl` (exact length) are the
                // magnet URI spec's own metadata query params — reused as
                // the persisted release's title/size so a self-supplied
                // magnet gets a real `releases` row too, not just the
                // cache-backed release_id arm (#651).
                let mut title = None;
                let mut size_bytes = None;
                for (key, value) in parsed.query_pairs() {
                    match key.as_ref() {
                        "dn" => title = Some(value.into_owned()),
                        "xl" => size_bytes = value.parse::<u64>().ok(),
                        _ => {}
                    }
                }
                let title = title.unwrap_or_else(|| "magnet download".to_string());
                (
                    Uuid::now_v7(),
                    magnet.to_string(),
                    "torrent".to_string(),
                    None,
                    title,
                    size_bytes,
                    MANUAL_MAGNET_INDEXER_ID,
                )
            }
        };

    // SAFETY: the by-reference path must not become an SSRF bypass — the
    // RESOLVED url is validated the same as a client-supplied one, never
    // trusted just because it came from the server-side cache (mirrors
    // paroche::routes::download::enqueue_download).
    if let Err(error) = paroche::net_validate::validate_download_url(&download_url).await {
        return tool_error(error.to_string());
    }

    // WHY: the tool no longer mints a want id for a caller that omits one
    // (#651) — an invented id can never resolve against `wants`, so the
    // production `ImportAdapter` deterministically fails after the transfer
    // completes. The caller must reference a want it already created.
    let Some(raw_want_id) = args.want_id.as_deref() else {
        return tool_error(
            "want_id is required and must reference an existing want row — create the want \
             first; this tool no longer invents an unresolvable id",
        );
    };
    let want_id = match Uuid::parse_str(raw_want_id) {
        Ok(id) => id,
        Err(_) => return tool_error("want_id must be a valid UUID"),
    };

    match want::get_want(&ctx.db.read, want_id.as_bytes().as_ref()).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return tool_error(format!(
                "want {want_id} not found — create it before enqueueing"
            ));
        }
        Err(e) => return tool_error(format!("failed to look up want {want_id}: {e}")),
    }

    // INVARIANT: the `releases` row for `release_id` must exist BEFORE this
    // handler calls `queue.enqueue()` below — `syntaxis::ImportService`'s
    // production adapter resolves both `want_id` and `release_id` against
    // `apotheke::repo::want::{get_want,get_release}` once the transfer
    // completes (#651). A search result's `release_id` otherwise lives only
    // in eksetasis's in-memory cache; persisting it here, ahead of enqueue,
    // is what makes it a durable identifier rather than a process-local key.
    match want::get_release(&ctx.db.read, release_id.as_bytes().as_ref()).await {
        Ok(Some(existing)) if existing.want_id == want_id.as_bytes().to_vec() => {
            // NOTE: idempotent retry — this exact (release, want) pair is
            // already persisted (e.g. a prior enqueue attempt failed after
            // this insert but before the queue write) — reuse it.
        }
        Ok(Some(_other_want)) => {
            return tool_error(format!(
                "release {release_id} is already recorded under a different want"
            ));
        }
        Ok(None) => {
            let release = Release {
                id: release_id.as_bytes().to_vec(),
                want_id: want_id.as_bytes().to_vec(),
                indexer_id,
                title,
                size_bytes: i64::try_from(size_bytes.unwrap_or(0)).unwrap_or(i64::MAX),
                // WHY: no pre-download quality-assessment pipeline scores a
                // raw search hit or magnet yet — kritike::assessment scores
                // already-imported file tags (kathodos::sidecar output), not
                // release-name text. 0 is a neutral placeholder, not a
                // measured quality claim; upgrade decisions over this row
                // are unaffected since they compare against OTHER releases
                // of the same want, not an absolute threshold.
                quality_score: 0,
                custom_format_score: 0,
                download_url: download_url.clone(),
                protocol: protocol.clone(),
                info_hash: info_hash.clone(),
                found_at: jiff::Timestamp::now().to_string(),
                grabbed_at: None,
                rejected_reason: None,
            };
            if let Err(e) = want::insert_release(&ctx.db.write, &release).await {
                return tool_error(format!("failed to persist release {release_id}: {e}"));
            }
        }
        Err(e) => {
            return tool_error(format!(
                "failed to check for an existing release record: {e}"
            ));
        }
    }

    let queue_id = Uuid::now_v7();
    let priority = args.priority.clamp(1, 4);

    if let Err(error) = ctx
        .queue
        .enqueue(EnqueueItem {
            queue_id,
            want_id,
            release_id,
            download_url,
            protocol,
            priority,
            info_hash,
        })
        .await
    {
        return service_error_to_tool_error(error);
    }

    match paroche::routes::download::fetch_download(&ctx.db.read, queue_id).await {
        Ok(Some(row)) => match serde_json::to_value(row) {
            Ok(payload) => tool_success_json(payload),
            Err(e) => tool_error(format!("failed to encode the persisted queue row: {e}")),
        },
        Ok(None) => tool_error("queue row vanished immediately after insert"),
        Err(e) => tool_error(format!("failed to read back the persisted queue row: {e}")),
    }
}

const KNOWN_DOWNLOAD_STATUSES: [&str; 6] = [
    "queued",
    "downloading",
    "post_processing",
    "importing",
    "completed",
    "failed",
];

// WHY: bounds an unbounded client-supplied limit from reaching the DB —
// `get_queue_snapshot` has no cap at all; the MCP surface is a new attack
// surface (an LLM-driven caller), so it gets a defensive ceiling.
const MAX_LIST_LIMIT: u32 = 500;

#[derive(serde::Deserialize)]
struct ListArguments {
    status: Option<String>,
    id: Option<String>,
    #[serde(default = "default_list_limit")]
    limit: u32,
}

fn default_list_limit() -> u32 {
    50
}

async fn handle_list(arguments: &Value, ctx: &BridgeContext) -> Value {
    let args: ListArguments = match serde_json::from_value(arguments.clone()) {
        Ok(a) => a,
        Err(e) => return tool_error(format!("invalid list arguments: {e}")),
    };
    if let Some(status) = args.status.as_deref()
        && !KNOWN_DOWNLOAD_STATUSES.contains(&status)
    {
        return tool_error(format!(
            "status must be one of {KNOWN_DOWNLOAD_STATUSES:?}; got {status}"
        ));
    }
    let id = match args.id.as_deref().map(Uuid::parse_str).transpose() {
        Ok(id) => id,
        Err(_) => return tool_error("id must be a valid UUID"),
    };
    let limit = args.limit.clamp(1, MAX_LIST_LIMIT);

    match paroche::routes::download::list_downloads(&ctx.db.read, args.status.as_deref(), id, limit)
        .await
    {
        Ok(rows) => tool_success_json(json!({ "downloads": rows })),
        Err(e) => tool_error(format!("failed to list downloads: {e}")),
    }
}

#[derive(serde::Deserialize)]
struct CancelArguments {
    id: String,
}

async fn handle_cancel(arguments: &Value, ctx: &BridgeContext) -> Value {
    let args: CancelArguments = match serde_json::from_value(arguments.clone()) {
        Ok(a) => a,
        Err(e) => return tool_error(format!("invalid cancel arguments: {e}")),
    };
    let id = match Uuid::parse_str(&args.id) {
        Ok(id) => id,
        Err(_) => return tool_error("id must be a valid UUID"),
    };
    match ctx.queue.cancel(id).await {
        Ok(()) => tool_success_json(json!({ "ok": true, "id": id.to_string() })),
        Err(ServiceError::NotFound) => tool_error(format!("download {id} not found")),
        Err(error) => service_error_to_tool_error(error),
    }
}

// ── Wire protocol (one request line -> one response line) ──────────────────

/// Handles one raw JSON-RPC request line, returning the encoded response
/// line (no trailing newline).
pub async fn handle_request_line(line: &str, ctx: &BridgeContext) -> String {
    let message: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return encode(&error_response(
                Value::Null,
                -32700,
                format!("parse error: {e}"),
            ));
        }
    };
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    if message.get("method").and_then(Value::as_str) != Some("tools/call") {
        return encode(&error_response(
            id,
            -32601,
            "the mcp acquisition bridge only serves tools/call".to_string(),
        ));
    }
    let Some(name) = message.pointer("/params/name").and_then(Value::as_str) else {
        return encode(&success_response(
            id,
            tool_error("tools/call requires params.name"),
        ));
    };
    let arguments = message
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let result = ctx.dispatch(name, &arguments).await;
    encode(&success_response(id, result))
}

fn encode(value: &Value) -> String {
    // WHY: infallible for these envelope shapes in practice; a stray
    // serialize failure still degrades to a valid, if generic, line rather
    // than panicking a connection handler.
    serde_json::to_string(value).unwrap_or_else(|_| {
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"internal encode error"}}"#
            .to_string()
    })
}

// ── UDS listener (server side) ──────────────────────────────────────────────

// WHY: a `tools/call` request line is tiny (a UUID + a handful of scalar
// fields) — this is the wire-frame CEILING, not an expected size, sibling
// to `MAX_LIST_LIMIT`'s defensive-cap discipline above. Without it, a
// connection that streams bytes with no newline grows the request buffer
// unbounded until it OOMs the whole `serve` process (HTTP API, downloads,
// everything) — one malicious/broken MCP client should only be able to
// cost itself a closed connection, never the process.
#[cfg(unix)]
const MAX_REQUEST_BYTES: usize = 256 * 1024;

#[cfg(unix)]
async fn handle_connection(
    stream: tokio::net::UnixStream,
    ctx: BridgeContext,
    shutdown: tokio_util::sync::CancellationToken,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut buf: Vec<u8> = Vec::new();
    loop {
        buf.clear();
        // WHY: the shutdown race sits AT the read point (a request
        // boundary) only — once `handle_request_line` below is polled, this
        // loop no longer selects against `shutdown`, so an in-flight
        // enqueue/cancel always finishes its dispatch and writes its
        // response before the connection can be torn down.
        //
        // `.take(..)` is re-created every iteration so the byte budget is
        // per-REQUEST, not per-connection — a long-lived connection may
        // send many small requests without ever tripping the ceiling.
        let mut limited = (&mut reader).take(MAX_REQUEST_BYTES as u64);
        let read = tokio::select! {
            () = shutdown.cancelled() => return,
            read = limited.read_until(b'\n', &mut buf) => read,
        };
        let n = match read {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(error = %e, "mcp bridge connection read error");
                return;
            }
        };
        if n == 0 {
            return; // EOF
        }
        if buf.last() != Some(&b'\n') {
            // WHY: no newline within MAX_REQUEST_BYTES means either the
            // wire-frame ceiling was hit or the peer vanished mid-frame — a
            // conforming client (this bridge's own `mcp.rs` forwarder
            // included) always terminates a request with `\n`, so either
            // way the frame is unusable. Reject and close now rather than
            // keep reading further (the whole point of the cap).
            let response = encode(&error_response(
                Value::Null,
                -32600,
                format!("request exceeds the {MAX_REQUEST_BYTES}-byte wire-frame limit"),
            ));
            // WHY: best-effort notify then close — a write failure here
            // changes nothing (we return regardless), so it is logged at
            // debug, never propagated.
            let notify = async {
                writer.write_all(response.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await
            };
            if let Err(e) = notify.await {
                tracing::debug!(error = %e, "mcp bridge: could not send oversized-frame error to peer");
            }
            tracing::warn!(
                limit = MAX_REQUEST_BYTES,
                "mcp bridge closed a connection whose request exceeded the wire-frame ceiling"
            );
            return;
        }
        let line = match std::str::from_utf8(&buf) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "mcp bridge connection received a non-utf8 line");
                return;
            }
        };
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        let response = handle_request_line(trimmed, &ctx).await;
        if writer.write_all(response.as_bytes()).await.is_err()
            || writer.write_all(b"\n").await.is_err()
            || writer.flush().await.is_err()
        {
            tracing::warn!("mcp bridge connection write error");
            return;
        }
    }
}

// WHY: bounded shutdown grace — long enough for a real tool call (indexer
// fan-out, a DB write) to finish, short enough that a wedged connection
// cannot hang process shutdown forever. Best-effort past this point: we log
// and unlink anyway rather than block shutdown indefinitely.
#[cfg(unix)]
const SHUTDOWN_DRAIN_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

#[cfg(unix)]
async fn run_accept_loop(
    listener: tokio::net::UnixListener,
    ctx: BridgeContext,
    shutdown: tokio_util::sync::CancellationToken,
    socket_path: PathBuf,
) {
    // WHY: shutdown used to join only THIS accept loop — spawned
    // per-connection tasks were dropped mid-call, silently losing an
    // in-flight enqueue/cancel (#609 adversarial review finding). Tracking
    // every connection task here lets shutdown DRAIN them before the socket
    // is unlinked.
    let tracker = tokio_util::task::TaskTracker::new();
    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _addr)) => {
                        let conn_ctx = ctx.clone();
                        let conn_shutdown = shutdown.clone();
                        tracker.spawn(
                            handle_connection(stream, conn_ctx, conn_shutdown)
                                .instrument(tracing::info_span!("mcp_bridge_conn")),
                        );
                    }
                    Err(e) => tracing::warn!(error = %e, "mcp bridge accept error"),
                }
            }
        }
    }

    tracker.close();
    if tokio::time::timeout(SHUTDOWN_DRAIN_GRACE, tracker.wait())
        .await
        .is_err()
    {
        tracing::warn!(
            grace_secs = SHUTDOWN_DRAIN_GRACE.as_secs(),
            "mcp bridge: connections still in flight after the shutdown grace period; \
             unlinking the socket anyway"
        );
    }

    // WHY: best-effort — a failed remove leaves a stale file that the NEXT
    // startup's stale-socket unlink (in `spawn`) cleans up; not worth a
    // panic on shutdown.
    tokio::fs::remove_file(&socket_path).await.ok();
    tracing::info!(socket = %socket_path.display(), "mcp acquisition bridge stopped");
}

/// Binds the acquisition bridge's Unix domain socket and spawns its accept
/// loop. Bind is FATAL — mirrors the HTTP listener's startup posture: a
/// server that cannot bind its configured surface must not come up
/// half-alive. The socket's parent directory is a dedicated runtime dir
/// this process owns (see `resolve_socket_path`) — created and `chmod`'d
/// `0700`; a failure here is a legitimate fatal (it means another user owns
/// what should be our runtime dir). The socket file itself is `chmod`'d
/// `0600` after bind, belt-and-suspenders on top of the directory
/// permission. If a socket already exists at the target path, it is
/// LIVENESS-CHECKED, not blindly unlinked: a successful connect means
/// another `harmonia serve` instance is already bound there (fatal — its
/// live socket is left untouched); a refused connect means the inode is
/// stale and is removed before rebinding.
#[cfg(unix)]
pub async fn spawn(
    socket_path: PathBuf,
    ctx: BridgeContext,
    shutdown: tokio_util::sync::CancellationToken,
) -> Result<tokio::task::JoinHandle<()>, std::io::Error> {
    if let Some(parent) = socket_path.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await?;
        tokio::fs::set_permissions(parent, Permissions::from_mode(0o700)).await?;
    }
    if tokio::fs::try_exists(&socket_path).await.unwrap_or(false) {
        // WHY: a SECOND `harmonia serve` started against the same config
        // must not silently sever a FIRST, still-live instance's MCP
        // surface by unlinking its socket out from under it. Attempting a
        // connect is the standard stale-socket protocol: `Ok` proves a
        // peer is actually listening (not just that a stale inode exists),
        // `Err` (connection refused / no such process) proves it is dead.
        match tokio::net::UnixStream::connect(&socket_path).await {
            Ok(_live) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    format!(
                        "another harmonia serve instance is already bound to {}",
                        socket_path.display()
                    ),
                ));
            }
            Err(_) => {
                tokio::fs::remove_file(&socket_path).await?;
            }
        }
    }
    let listener = tokio::net::UnixListener::bind(&socket_path)?;
    tokio::fs::set_permissions(&socket_path, Permissions::from_mode(0o600)).await?;

    tracing::info!(socket = %socket_path.display(), "mcp acquisition bridge listening");

    Ok(tokio::spawn(
        run_accept_loop(listener, ctx, shutdown, socket_path)
            .instrument(tracing::info_span!("mcp_bridge")),
    ))
}

#[cfg(not(unix))]
pub async fn spawn(
    _socket_path: PathBuf,
    _ctx: BridgeContext,
    _shutdown: tokio_util::sync::CancellationToken,
) -> Result<tokio::task::JoinHandle<()>, std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the MCP acquisition bridge requires a Unix domain socket; unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;
    use std::time::Duration;

    use apotheke::migrate::MIGRATOR;
    use apotheke::repo::want::Want;
    use paroche::state::{ResolvedRelease, ServiceFut};
    use sqlx::SqlitePool;

    use super::*;

    // ── Stub search / queue services ─────────────────────────────────────

    /// A cached search hit's resolvable form — everything the real
    /// `eksetasis` results cache would carry for a `release_id`, so
    /// `handle_enqueue`'s release-persistence step has real data to write.
    #[derive(Clone)]
    struct ResolveEntry {
        download_url: String,
        protocol: String,
        info_hash: Option<String>,
        title: String,
        size_bytes: Option<u64>,
        indexer_id: i64,
    }

    impl ResolveEntry {
        fn new(download_url: impl Into<String>, protocol: impl Into<String>) -> Self {
            Self {
                download_url: download_url.into(),
                protocol: protocol.into(),
                info_hash: None,
                title: "Test.Release.Title".to_string(),
                size_bytes: Some(1_000_000),
                indexer_id: 1,
            }
        }
    }

    struct StubSearch {
        results: Value,
        resolve: HashMap<Uuid, ResolveEntry>,
    }

    impl DynSearchService for StubSearch {
        fn search(&self, _query: Value) -> ServiceFut<Value> {
            let results = self.results.clone();
            Box::pin(async move { Ok(results) })
        }
        fn test_indexer(&self, _indexer_id: i64) -> ServiceFut<Value> {
            Box::pin(async { Ok(json!({})) })
        }
        fn refresh_caps(&self, _indexer_id: i64) -> ServiceFut<Value> {
            Box::pin(async { Ok(json!({})) })
        }
        fn cached_results(&self, _query_id: Uuid) -> ServiceFut<Value> {
            Box::pin(async { Err(ServiceError::NotAvailable) })
        }
        fn resolve_release(&self, release_id: Uuid) -> ServiceFut<ResolvedRelease> {
            let found = self.resolve.get(&release_id).cloned().map(|entry| {
                ResolvedRelease {
                    download_url: entry.download_url,
                    protocol: entry.protocol,
                    info_hash: entry.info_hash,
                    indexer_id: entry.indexer_id,
                    title: entry.title,
                    size_bytes: entry.size_bytes,
                }
            });
            Box::pin(async move { found.ok_or(ServiceError::NotFound) })
        }
    }

    /// Records every enqueue/cancel and, when given a pool, PERSISTS the
    /// enqueued row exactly as the real syntaxis service does — so
    /// `handle_enqueue`'s "re-read the persisted row, redacted" step has a
    /// row to read (the WHY the response can be checked for redaction).
    #[derive(Default)]
    struct StubQueue {
        enqueued: Mutex<Vec<EnqueueItem>>,
        cancelled: Mutex<Vec<Uuid>>,
        known: Mutex<std::collections::HashSet<Uuid>>,
        persist: Option<SqlitePool>,
        /// ORDERING PROBE (#651): for each `enqueue()` call, whether a
        /// `releases` row for that item's `release_id` already existed AT
        /// THE MOMENT this stub ran. An end-state check alone ("does the
        /// row exist once dispatch returns") would still pass if
        /// `handle_enqueue` persisted the release AFTER calling
        /// `queue.enqueue()` — this catches exactly that reordering.
        release_existed_at_enqueue: Arc<Mutex<Vec<bool>>>,
    }

    impl StubQueue {
        fn with_pool(pool: SqlitePool) -> Self {
            Self {
                persist: Some(pool),
                ..Self::default()
            }
        }
    }

    impl DynQueueManager for StubQueue {
        fn enqueue(&self, item: EnqueueItem) -> ServiceFut<()> {
            self.known.lock().unwrap().insert(item.queue_id);
            let pool = self.persist.clone();
            self.enqueued.lock().unwrap().push(item.clone());
            let release_existed = Arc::clone(&self.release_existed_at_enqueue);
            Box::pin(async move {
                if let Some(pool) = pool {
                    let existed: i64 =
                        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM releases WHERE id = ?)")
                            .bind(item.release_id.as_bytes().as_slice())
                            .fetch_one(&pool)
                            .await
                            .unwrap_or(0);
                    release_existed.lock().unwrap().push(existed != 0);

                    sqlx::query(
                        "INSERT INTO download_queue (id, want_id, release_id, download_url, \
                         protocol, priority, info_hash, status) \
                         VALUES (?, ?, ?, ?, ?, ?, ?, 'queued')",
                    )
                    .bind(item.queue_id.as_bytes().as_slice())
                    .bind(item.want_id.as_bytes().as_slice())
                    .bind(item.release_id.as_bytes().as_slice())
                    .bind(item.download_url.as_str())
                    .bind(item.protocol.as_str())
                    .bind(i64::from(item.priority))
                    .bind(item.info_hash.as_deref())
                    .execute(&pool)
                    .await
                    .map_err(|e| ServiceError::Internal(e.to_string()))?;
                }
                Ok(())
            })
        }
        fn cancel(&self, queue_id: Uuid) -> ServiceFut<()> {
            let found = self.known.lock().unwrap().contains(&queue_id);
            if found {
                self.cancelled.lock().unwrap().push(queue_id);
            }
            Box::pin(async move {
                if found {
                    Ok(())
                } else {
                    Err(ServiceError::NotFound)
                }
            })
        }
        fn reprioritize(&self, _queue_id: Uuid, _priority: u8) -> ServiceFut<()> {
            Box::pin(async { Ok(()) })
        }
    }

    async fn test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite opens");
        MIGRATOR.run(&pool).await.expect("migrations run");
        pool
    }

    async fn test_ctx(search: StubSearch, queue: StubQueue) -> BridgeContext {
        let pool = test_db().await;
        ctx_with_pool(pool, search, Arc::new(queue))
    }

    /// Builds a context whose `db` is the SAME pool a persisting StubQueue
    /// writes to — so an enqueue's row is visible to the read-back. Keeps
    /// the concrete `Arc<StubQueue>` type at the call site for inspection.
    fn ctx_with_pool(pool: SqlitePool, search: StubSearch, queue: Arc<StubQueue>) -> BridgeContext {
        BridgeContext {
            search: Arc::new(search),
            queue,
            db: Arc::new(DbPools {
                read: pool.clone(),
                write: pool,
            }),
        }
    }

    fn empty_search() -> StubSearch {
        StubSearch {
            results: json!({ "results": [] }),
            resolve: HashMap::new(),
        }
    }

    /// Persists a real `wants` row (and, on first call, its
    /// `quality_profiles` prerequisite via the migrator's seed data) and
    /// returns its id — the ONLY way `handle_enqueue` will now accept a
    /// `want_id` (#651: the tool no longer invents one).
    async fn seed_want(pool: &SqlitePool) -> Uuid {
        let profile_id: i64 =
            sqlx::query_scalar("SELECT id FROM quality_profiles WHERE media_type = 'music' LIMIT 1")
                .fetch_one(pool)
                .await
                .expect("migrator seeds a default music quality profile");
        let want_id = Uuid::now_v7();
        want::insert_want(
            pool,
            &Want {
                id: want_id.as_bytes().to_vec(),
                media_type: "music_album".to_string(),
                title: "Test Want".to_string(),
                registry_id: None,
                quality_profile_id: profile_id,
                status: "searching".to_string(),
                source: None,
                source_ref: None,
                added_at: jiff::Timestamp::now().to_string(),
                fulfilled_at: None,
            },
        )
        .await
        .expect("seed want inserts");
        want_id
    }

    // ── tools/list membership ────────────────────────────────────────────

    #[test]
    fn is_acquisition_tool_covers_exactly_the_four_names() {
        assert!(is_acquisition_tool("harmonia_search_releases"));
        assert!(is_acquisition_tool("harmonia_enqueue_download"));
        assert!(is_acquisition_tool("harmonia_list_downloads"));
        assert!(is_acquisition_tool("harmonia_cancel_download"));
        assert!(!is_acquisition_tool("harmonia_db_migrate"));
        assert!(!is_acquisition_tool("unknown_tool"));
    }

    // ── search ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn search_redacts_credentialed_download_urls() {
        let search = StubSearch {
            results: json!({
                "results": [{
                    "release_id": Uuid::now_v7().to_string(),
                    "download_url": "https://indexer.example/dl?apikey=SECRETVALUE",
                    "title": "Some Album"
                }]
            }),
            resolve: HashMap::new(),
        };
        let ctx = test_ctx(search, StubQueue::default()).await;

        let result = ctx
            .dispatch(
                "harmonia_search_releases",
                &json!({ "query_text": "test", "media_type": "music" }),
            )
            .await;

        assert_eq!(result["isError"], false);
        let rendered = serde_json::to_string(&result).unwrap();
        assert!(
            !rendered.contains("SECRETVALUE"),
            "no unredacted credential may appear anywhere in the tool result: {rendered}"
        );
        assert!(rendered.contains("REDACTED"));
    }

    #[tokio::test]
    async fn search_rejects_structurally_invalid_arguments() {
        // WHY: media_type VALUE validation is the search service's job
        // (serve.rs parse_search_media_type) — the bridge's own guarantee is
        // rejecting a structurally-invalid argument (a wrong-typed field)
        // before it ever reaches the service.
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch(
                "harmonia_search_releases",
                &json!({ "limit": "not-a-number" }),
            )
            .await;
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("invalid search arguments")
        );
    }

    // ── enqueue ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn enqueue_rejects_neither_release_id_nor_magnet() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx.dispatch("harmonia_enqueue_download", &json!({})).await;
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("exactly one")
        );
    }

    #[tokio::test]
    async fn enqueue_rejects_both_release_id_and_magnet() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({
                    "release_id": Uuid::now_v7().to_string(),
                    "magnet": "magnet:?xt=urn:btih:abc"
                }),
            )
            .await;
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not both")
        );
    }

    #[tokio::test]
    async fn enqueue_rejects_http_url_in_magnet_arm() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({ "magnet": "https://example.com/credentialed?apikey=SECRET" }),
            )
            .await;
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("magnet:")
        );
    }

    #[tokio::test]
    async fn enqueue_rejects_private_tracker_magnet() {
        // WHY: validate_download_url rejects a magnet whose tr= trackers all
        // resolve to non-public address space — this exercises the SSRF
        // guard staying wired on the magnet arm.
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({ "magnet": "magnet:?xt=urn:btih:abc&tr=http://127.0.0.1:6969/announce" }),
            )
            .await;
        assert_eq!(result["isError"], true);
    }

    #[tokio::test]
    async fn enqueue_release_id_not_found_reports_not_found() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({ "release_id": Uuid::now_v7().to_string() }),
            )
            .await;
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not found")
        );
    }

    #[tokio::test]
    async fn enqueue_release_id_resolves_persists_and_redacts_response() {
        // WHY: a magnet-form resolved URL validates without DNS (no tr=
        // trackers), keeping the test network-free while still carrying a
        // credential-shaped `apikey=` param that redaction must strip.
        let release_id = Uuid::now_v7();
        let credentialed =
            "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd&apikey=SECRETVALUE";
        let mut resolve = HashMap::new();
        resolve.insert(release_id, ResolveEntry::new(credentialed, "torrent"));
        let search = StubSearch {
            results: json!({}),
            resolve,
        };
        let pool = test_db().await;
        let want_id = seed_want(&pool).await;
        let queue = Arc::new(StubQueue::with_pool(pool.clone()));
        let ctx = ctx_with_pool(pool, search, Arc::clone(&queue));

        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({
                    "release_id": release_id.to_string(),
                    "want_id": want_id.to_string(),
                    "priority": 3
                }),
            )
            .await;

        assert_eq!(result["isError"], false, "{result:?}");
        // The QUEUE received the UNREDACTED, credentialed URL.
        let enqueued = queue.enqueued.lock().unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].download_url, credentialed);
        assert_eq!(enqueued[0].priority, 3);
        drop(enqueued);
        // The RESPONSE is redacted — no credential anywhere in it.
        let rendered = serde_json::to_string(&result).unwrap();
        assert!(
            !rendered.contains("SECRETVALUE"),
            "the enqueue response must never carry the unredacted credential: {rendered}"
        );
        assert!(rendered.contains("REDACTED"));
        assert_eq!(result["structuredContent"]["priority"], 3);
        assert!(
            !result["structuredContent"]["id"]
                .as_str()
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn enqueue_rejects_missing_want_id() {
        // WHY: #651 — omitting want_id must be a hard rejection, never a
        // silently minted, unresolvable id.
        let release_id = Uuid::now_v7();
        let mut resolve = HashMap::new();
        resolve.insert(
            release_id,
            ResolveEntry::new(
                "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd",
                "torrent",
            ),
        );
        let search = StubSearch {
            results: json!({}),
            resolve,
        };
        let ctx = test_ctx(search, StubQueue::default()).await;

        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({ "release_id": release_id.to_string() }),
            )
            .await;

        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("want_id is required")
        );
    }

    #[tokio::test]
    async fn enqueue_rejects_unknown_want_id() {
        // WHY: #651 — a well-formed but non-existent want_id must be
        // rejected too, not accepted and later fail import.
        let release_id = Uuid::now_v7();
        let mut resolve = HashMap::new();
        resolve.insert(
            release_id,
            ResolveEntry::new(
                "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd",
                "torrent",
            ),
        );
        let search = StubSearch {
            results: json!({}),
            resolve,
        };
        let ctx = test_ctx(search, StubQueue::default()).await;

        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({
                    "release_id": release_id.to_string(),
                    "want_id": Uuid::now_v7().to_string()
                }),
            )
            .await;

        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not found")
        );
    }

    #[tokio::test]
    async fn enqueue_persists_the_release_row_before_calling_queue_enqueue() {
        // WHY: #651's core ordering/durability assertion — an end-state
        // check ("does a releases row exist once dispatch returns") would
        // still pass if the persist call were moved to AFTER
        // `queue.enqueue()`. StubQueue::enqueue records whether the row
        // already existed the INSTANT it ran, so this fails under that
        // reordering even though the final state would look identical.
        let release_id = Uuid::now_v7();
        let mut resolve = HashMap::new();
        resolve.insert(
            release_id,
            ResolveEntry::new(
                "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd",
                "torrent",
            ),
        );
        let search = StubSearch {
            results: json!({}),
            resolve,
        };
        let pool = test_db().await;
        let want_id = seed_want(&pool).await;
        let queue = Arc::new(StubQueue::with_pool(pool.clone()));
        let ctx = ctx_with_pool(pool.clone(), search, Arc::clone(&queue));

        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({
                    "release_id": release_id.to_string(),
                    "want_id": want_id.to_string()
                }),
            )
            .await;

        assert_eq!(result["isError"], false, "{result:?}");
        let ordering_flags = queue.release_existed_at_enqueue.lock().unwrap().clone();
        assert_eq!(
            ordering_flags,
            vec![true],
            "the releases row must be written before queue.enqueue() is called, not after"
        );

        let persisted_want_id: Vec<u8> =
            sqlx::query_scalar("SELECT want_id FROM releases WHERE id = ?")
                .bind(release_id.as_bytes().as_slice())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(persisted_want_id, want_id.as_bytes().to_vec());
    }

    #[tokio::test]
    async fn enqueue_release_id_retry_reuses_the_persisted_release_row() {
        // WHY: a retry (e.g. after a transient queue failure) must not
        // fail on a duplicate-key insert against an already-persisted
        // release — the second call reuses the row idempotently.
        let release_id = Uuid::now_v7();
        let mut resolve = HashMap::new();
        resolve.insert(
            release_id,
            ResolveEntry::new(
                "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd",
                "torrent",
            ),
        );
        let search = StubSearch {
            results: json!({}),
            resolve,
        };
        let pool = test_db().await;
        let want_id = seed_want(&pool).await;
        let queue = Arc::new(StubQueue::with_pool(pool.clone()));
        let ctx = ctx_with_pool(pool.clone(), search, Arc::clone(&queue));
        let args = json!({
            "release_id": release_id.to_string(),
            "want_id": want_id.to_string()
        });

        let first = ctx.dispatch("harmonia_enqueue_download", &args).await;
        assert_eq!(first["isError"], false, "{first:?}");
        let second = ctx.dispatch("harmonia_enqueue_download", &args).await;
        assert_eq!(second["isError"], false, "{second:?}");

        let release_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM releases WHERE id = ?")
            .bind(release_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(release_count, 1, "the retry must not duplicate the release row");
    }

    #[tokio::test]
    async fn enqueue_magnet_arm_persists_a_release_row_from_dn_and_xl() {
        // WHY: #651's release-row gap is identical on the magnet arm — a
        // self-supplied magnet also needs a durable `releases` row before
        // enqueue, derived from the magnet URI's own dn/xl metadata params.
        let search = empty_search();
        let pool = test_db().await;
        let want_id = seed_want(&pool).await;
        let queue = Arc::new(StubQueue::with_pool(pool.clone()));
        let ctx = ctx_with_pool(pool.clone(), search, Arc::clone(&queue));

        let result = ctx
            .dispatch(
                "harmonia_enqueue_download",
                &json!({
                    "magnet": "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd&dn=My.Release&xl=123456",
                    "want_id": want_id.to_string()
                }),
            )
            .await;

        assert_eq!(result["isError"], false, "{result:?}");
        let row: (String, i64, i64) =
            sqlx::query_as("SELECT title, size_bytes, indexer_id FROM releases WHERE want_id = ?")
                .bind(want_id.as_bytes().as_slice())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "My.Release");
        assert_eq!(row.1, 123_456);
        assert_eq!(row.2, MANUAL_MAGNET_INDEXER_ID);
    }

    // ── list ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_rejects_unknown_status() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch(
                "harmonia_list_downloads",
                &json!({ "status": "not-a-status" }),
            )
            .await;
        assert_eq!(result["isError"], true);
    }

    #[tokio::test]
    async fn list_filters_by_status_and_redacts() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let queued_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO download_queue (id, want_id, release_id, download_url, protocol, \
             priority, status) VALUES (?, ?, ?, ?, 'torrent', 4, 'queued')",
        )
        .bind(queued_id.as_bytes().as_slice())
        .bind(Uuid::now_v7().as_bytes().as_slice())
        .bind(Uuid::now_v7().as_bytes().as_slice())
        .bind("magnet:?xt=urn:btih:abc&apikey=SECRETVALUE")
        .execute(&ctx.db.read)
        .await
        .unwrap();
        let completed_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO download_queue (id, want_id, release_id, download_url, protocol, \
             priority, status) VALUES (?, ?, ?, ?, 'torrent', 4, 'completed')",
        )
        .bind(completed_id.as_bytes().as_slice())
        .bind(Uuid::now_v7().as_bytes().as_slice())
        .bind(Uuid::now_v7().as_bytes().as_slice())
        .bind("magnet:?xt=urn:btih:def")
        .execute(&ctx.db.read)
        .await
        .unwrap();

        let result = ctx
            .dispatch("harmonia_list_downloads", &json!({ "status": "queued" }))
            .await;

        assert_eq!(result["isError"], false);
        let downloads = result["structuredContent"]["downloads"].as_array().unwrap();
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0]["id"], queued_id.to_string());
        let rendered = serde_json::to_string(&result).unwrap();
        assert!(!rendered.contains("SECRETVALUE"));
    }

    // ── cancel ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn cancel_rejects_invalid_uuid() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch("harmonia_cancel_download", &json!({ "id": "not-a-uuid" }))
            .await;
        assert_eq!(result["isError"], true);
    }

    #[tokio::test]
    async fn cancel_unknown_id_reports_not_found() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let result = ctx
            .dispatch(
                "harmonia_cancel_download",
                &json!({ "id": Uuid::now_v7().to_string() }),
            )
            .await;
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not found")
        );
    }

    #[tokio::test]
    async fn cancel_known_id_succeeds() {
        let queue = StubQueue::default();
        let queue_id = Uuid::now_v7();
        queue.known.lock().unwrap().insert(queue_id);
        let ctx = test_ctx(empty_search(), queue).await;

        let result = ctx
            .dispatch(
                "harmonia_cancel_download",
                &json!({ "id": queue_id.to_string() }),
            )
            .await;

        assert_eq!(result["isError"], false);
        assert_eq!(result["structuredContent"]["ok"], true);
        assert_eq!(result["structuredContent"]["id"], queue_id.to_string());
    }

    // ── wire protocol round trip over a real UDS socket ──────────────────

    #[tokio::test]
    async fn real_socket_round_trip_serves_a_tool_call() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("bridge.sock");
        let shutdown = tokio_util::sync::CancellationToken::new();
        let handle = spawn(socket_path.clone(), ctx, shutdown.clone())
            .await
            .expect("bridge binds");

        let mut stream = tokio::net::UnixStream::connect(&socket_path)
            .await
            .expect("connects to the real socket");
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "harmonia_list_downloads", "arguments": {} }
        });
        let mut line = serde_json::to_string(&request).unwrap();
        line.push('\n');
        stream.write_all(line.as_bytes()).await.unwrap();

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.unwrap();
        let response: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["result"]["isError"], false);

        shutdown.cancel();
        handle.await.unwrap();
        assert!(
            !socket_path.exists(),
            "the socket must be unlinked on shutdown"
        );
    }

    // ── FIX 1: bounded request read ──────────────────────────────────────

    #[tokio::test]
    async fn oversized_request_is_rejected_and_the_bridge_survives() {
        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("bridge.sock");
        let shutdown = tokio_util::sync::CancellationToken::new();
        let handle = spawn(socket_path.clone(), ctx, shutdown.clone())
            .await
            .expect("bridge binds");

        let mut stream = tokio::net::UnixStream::connect(&socket_path)
            .await
            .expect("connects to the real socket");
        // WHY: bytes with NO trailing newline, bigger than MAX_REQUEST_BYTES
        // — exercises the read cap without needing a syntactically valid
        // (but merely oversized) JSON body.
        let oversized = vec![b'a'; MAX_REQUEST_BYTES + 1024];
        stream.write_all(&oversized).await.unwrap();

        let mut reader = BufReader::new(&mut stream);
        let mut response_line = String::new();
        tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut response_line))
            .await
            .expect("bridge must respond before the timeout")
            .unwrap();
        let response: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["error"]["code"], -32600);
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("exceeds"),
            "{response:?}"
        );

        // the bridge then closes the connection — a further read is EOF.
        let mut trailing = String::new();
        let eof = reader.read_line(&mut trailing).await.unwrap();
        assert_eq!(
            eof, 0,
            "the bridge must close the connection after an oversized request"
        );

        // the bridge process itself must still be alive — a FRESH
        // connection on the same socket must still be served normally.
        let mut stream2 = tokio::net::UnixStream::connect(&socket_path)
            .await
            .expect("the bridge must still be listening after one bad connection");
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "harmonia_list_downloads", "arguments": {} }
        });
        let mut line = serde_json::to_string(&request).unwrap();
        line.push('\n');
        stream2.write_all(line.as_bytes()).await.unwrap();
        let mut reader2 = BufReader::new(stream2);
        let mut response_line2 = String::new();
        reader2.read_line(&mut response_line2).await.unwrap();
        let response2: Value = serde_json::from_str(&response_line2).unwrap();
        assert_eq!(response2["result"]["isError"], false, "{response2:?}");

        shutdown.cancel();
        handle.await.unwrap();
    }

    // ── FIX 3: liveness check before unlinking a socket ──────────────────

    #[tokio::test]
    async fn spawn_refuses_a_second_instance_on_the_same_socket() {
        let ctx1 = test_ctx(empty_search(), StubQueue::default()).await;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("bridge.sock");
        let shutdown = tokio_util::sync::CancellationToken::new();
        let handle = spawn(socket_path.clone(), ctx1, shutdown.clone())
            .await
            .expect("first instance binds");

        let ctx2 = test_ctx(empty_search(), StubQueue::default()).await;
        let second_shutdown = tokio_util::sync::CancellationToken::new();
        let result = spawn(socket_path.clone(), ctx2, second_shutdown).await;
        assert!(
            result.is_err(),
            "a second instance must refuse to bind over a LIVE socket"
        );

        // the FIRST instance's socket must still be reachable — a failed
        // second-instance bind attempt must not have unlinked it.
        let mut stream = tokio::net::UnixStream::connect(&socket_path)
            .await
            .expect("the first instance's socket must remain live and untouched");
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "harmonia_list_downloads", "arguments": {} }
        });
        let mut line = serde_json::to_string(&request).unwrap();
        line.push('\n');
        stream.write_all(line.as_bytes()).await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.unwrap();
        let response: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["result"]["isError"], false, "{response:?}");

        shutdown.cancel();
        handle.await.unwrap();
    }

    // ── FIX 4: shutdown drains in-flight connections ─────────────────────

    /// A queue whose `cancel` sleeps before resolving — simulates a tool
    /// call genuinely in flight when shutdown fires mid-dispatch.
    struct SlowCancelQueue {
        known: HashSet<Uuid>,
        delay: Duration,
    }

    impl DynQueueManager for SlowCancelQueue {
        fn enqueue(&self, _item: EnqueueItem) -> ServiceFut<()> {
            Box::pin(async { Ok(()) })
        }
        fn cancel(&self, queue_id: Uuid) -> ServiceFut<()> {
            let found = self.known.contains(&queue_id);
            let delay = self.delay;
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                if found {
                    Ok(())
                } else {
                    Err(ServiceError::NotFound)
                }
            })
        }
        fn reprioritize(&self, _queue_id: Uuid, _priority: u8) -> ServiceFut<()> {
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test]
    async fn shutdown_drains_an_inflight_call_before_unlinking() {
        let queue_id = Uuid::now_v7();
        let mut known = HashSet::new();
        known.insert(queue_id);
        let queue = SlowCancelQueue {
            known,
            delay: Duration::from_millis(300),
        };
        let pool = test_db().await;
        let ctx = BridgeContext {
            search: Arc::new(empty_search()),
            queue: Arc::new(queue),
            db: Arc::new(DbPools {
                read: pool.clone(),
                write: pool,
            }),
        };
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("bridge.sock");
        let shutdown = tokio_util::sync::CancellationToken::new();
        let handle = spawn(socket_path.clone(), ctx, shutdown.clone())
            .await
            .expect("bridge binds");

        let mut stream = tokio::net::UnixStream::connect(&socket_path)
            .await
            .expect("connects to the real socket");
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "harmonia_cancel_download", "arguments": { "id": queue_id.to_string() } }
        });
        let mut line = serde_json::to_string(&request).unwrap();
        line.push('\n');
        stream.write_all(line.as_bytes()).await.unwrap();

        // WHY: give the accept loop time to actually dispatch the request
        // (start the slow `cancel()` future) before triggering shutdown —
        // the race must genuinely exercise "in flight," not "not yet
        // started."
        tokio::time::sleep(Duration::from_millis(50)).await;
        shutdown.cancel();

        let mut reader = BufReader::new(&mut stream);
        let mut response_line = String::new();
        tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut response_line))
            .await
            .expect(
                "the in-flight call's response must still arrive — shutdown must not abort \
                 mid-dispatch",
            )
            .unwrap();
        let response: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["result"]["isError"], false, "{response:?}");
        assert_eq!(response["result"]["structuredContent"]["ok"], true);

        // the bridge must still have completed its full shutdown (accept
        // loop stopped, socket unlinked) within the grace period.
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("shutdown must complete within the drain grace period")
            .unwrap();
        assert!(!socket_path.exists());
    }

    // ── FIX 2: dedicated socket subdirectory ──────────────────────────────

    #[tokio::test]
    async fn socket_lives_in_a_dedicated_0700_subdir_with_a_0600_socket_and_both_sides_agree() {
        let dir = tempfile::tempdir().unwrap();
        let config = horismos::Config {
            database: horismos::DatabaseConfig {
                db_path: dir.path().join("harmonia.sqlite"),
                ..horismos::DatabaseConfig::default()
            },
            ..horismos::Config::default()
        };

        // WHY: `resolve_socket_path` is the ONE shared derivation both
        // `harmonia serve` (this call, standing in for the serve side) and
        // `harmonia mcp` (this second call, standing in for the stdio
        // client) run independently — calling it twice from the same
        // config proves they land on the identical path.
        let serve_side = resolve_socket_path(&config);
        let mcp_side = resolve_socket_path(&config);
        assert_eq!(
            serve_side, mcp_side,
            "serve and mcp must derive the identical socket path from the same config"
        );
        assert_eq!(
            serve_side,
            dir.path().join("harmonia-mcp").join("harmonia-mcp.sock")
        );
        assert_ne!(
            serve_side.parent(),
            Some(dir.path()),
            "the socket's parent must be a DEDICATED subdirectory, never db_path's own directory"
        );

        let ctx = test_ctx(empty_search(), StubQueue::default()).await;
        let shutdown = tokio_util::sync::CancellationToken::new();
        let handle = spawn(serve_side.clone(), ctx, shutdown.clone())
            .await
            .expect("bridge binds");

        let subdir_meta = tokio::fs::metadata(serve_side.parent().unwrap())
            .await
            .unwrap();
        assert_eq!(subdir_meta.permissions().mode() & 0o777, 0o700);
        let socket_meta = tokio::fs::metadata(&serve_side).await.unwrap();
        assert_eq!(socket_meta.permissions().mode() & 0o777, 0o600);

        shutdown.cancel();
        handle.await.unwrap();
    }
}
