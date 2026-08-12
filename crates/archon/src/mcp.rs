//! MCP stdio surface (#652, PRs 2-4 of the rmcp migration) — an rmcp server.
//!
//! The hand-rolled `for line in reader.lines()` loop this module used to
//! run was a single global critical section: one request completed before
//! the next byte was read, notifications were dropped unparsed, and
//! `initialize` echoed the client's protocol version instead of
//! negotiating. rmcp's `serve_inner` is the issue's design verbatim — an
//! in-flight registry keyed by JSON-RPC ID, each request spawned as its
//! own task, `notifications/cancelled` intercepted by the loop itself, and
//! a single serialized writer — so the stdio surface now IS an rmcp
//! server: config → `HarmoniaServer` → `.serve(stdio())` → `waiting()`.
//!
//! What did NOT change:
//! - The acquisition bridge protocol. `forward_to_bridge` still connects
//!   to the serve-hosted Unix socket per call (no pooling), writes one
//!   `tools/call` line, and awaits one response under `call_timeout` —
//!   only now the forwarded line is rebuilt from the typed params (proved
//!   lossless by the `mcp_params` round-trip tests) instead of forwarding
//!   the client's raw message verbatim.
//! - Tool-level failures stay `isError: true` envelopes with
//!   `structuredContent.ok`; only protocol failures become JSON-RPC
//!   errors.
//! - `harmonia_play_file`/`harmonia_render` remain as blocking one-call
//!   forms (cancellable through their request ct). The PR-4 lifecycle trios
//!   — `harmonia_play_file_start`/`_status`/`_stop` and
//!   `harmonia_render_start`/`_status`/`_stop` over the `mcp_ops` registry —
//!   are the preferred shape for long-running work (#652's "state rather
//!   than an indefinitely open RPC"); the blocking forms suit short work a
//!   caller wants to await in one call. Blocking calls do NOT join the
//!   registry: they are invisible to `*_status` and unstoppable via
//!   `*_stop` (cancel the call itself instead).
//!
//! What PR 3 changed — cancellation wiring (rmcp cancellation is
//! COOPERATIVE: the serve loop intercepts `notifications/cancelled` and
//! cancels that request's token promptly, but the handler future keeps
//! running unless it observes the token):
//! - Every tool method takes the request's `RequestContext.ct` as a
//!   trailing `CancellationToken` argument (rmcp's `FromContextPart`
//!   impls supply it from the tool-call context).
//! - Bridged acquisition calls select on the token around the bridge
//!   round-trip: a cancelled call drops the `forward_to_bridge` future —
//!   and with it the Unix socket — immediately instead of lingering to
//!   `call_timeout`, answering a tool-level "cancelled" envelope.
//! - `play_file` threads the token into `run_play`, whose event loop
//!   selects on it; cancellation runs the same `engine.stop()` teardown a
//!   finished track runs — the real stop path (decode/DSP tasks aborted
//!   and joined), proved by the play.rs falsifier test.
//! - `render` threads the token into `run_renderer_loop`, whose internal
//!   shutdown tree becomes a child of it; cancellation breaks the
//!   reconnect loop through the same shutdown path SIGINT/SIGTERM use,
//!   proved by the runner.rs falsifier test.
//!
//! Behavior changes made deliberately in PR 2 (witness-mandated
//! normalizations from the #700 review; see `mcp_params.rs` module docs
//! for the full schema-delta list):
//! - Non-object `arguments`: explicit `null`/absent still runs a tool with
//!   defaults (rmcp's `Option<JsonObject>` maps null to "no arguments"),
//!   but an array/scalar/`bool` `arguments` value no longer silently runs
//!   with defaults — it fails typed request dispatch and is answered as an
//!   unknown method (`-32601`) before any tool runs.
//! - Unknown tool names now get a protocol-level `-32602` "tool not found"
//!   instead of a tool-level `isError` envelope.
//! - Protocol version is negotiated (`min(client, server)`, server latest
//!   `2025-11-25`), not echoed; a client asking for an unknown newer
//!   version gets the server's latest back.
//! - The advertised `inputSchema` for each tool is the schemars-generated
//!   schema of its `mcp_params` DTO, replacing the hand-written `json!`
//!   literals (whose advertised-vs-actual drift the DTO docs pin).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport::stdio;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt, tool, tool_router};
use serde_json::{Value, json};
use snafu::ResultExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use archon::mcp_params::{
    CancelDownloadParams, DbMigrateParams, EnqueueDownloadParams, ListDownloadsParams,
    MigrateLibraryParams, MigrateMediaType, PlayFileParams, PlayFileStatusParams,
    PlayFileStopParams, RenderParams, RenderStatusParams, RenderStopParams, SearchReleasesParams,
};

use crate::cli::{CliMediaType, DbMigrateArgs, MigrateArgs, PlayArgs};
use crate::error::{ConfigSnafu, HostError, McpSocketPathSnafu};
use crate::mcp_ops::{self, OpSlot, OpStatus};

const SERVER_NAME: &str = "harmonia";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTRUCTIONS: &str = "Local MCP stdio surface for Harmonia. Offline tools run in-process with no server required: harmonia_db_migrate, harmonia_migrate_library, and playback/renderer control — the harmonia_play_file_start/status/stop and harmonia_render_start/status/stop lifecycle ops (preferred for long-running work: start returns an op id immediately, one op per kind at a time), plus the blocking one-call forms harmonia_play_file and harmonia_render for short work awaited in a single call. The 4 acquisition tools (search_releases, enqueue_download, list_downloads, cancel_download) forward to a running 'harmonia serve' over its local acquisition bridge socket — if the server isn't running, those calls return a tool-level error naming the socket. The HTTP API remains the canonical remote service API.";

/// Resolved once at process start — the acquisition bridge socket path and
/// per-call deadline. The 4 offline tools never touch this; the 4
/// acquisition tools forward through it on every call (no pooling).
#[derive(Clone)]
pub(crate) struct McpContext {
    pub(crate) socket_path: PathBuf,
    pub(crate) call_timeout: Duration,
}

impl McpContext {
    fn from_config(config: &horismos::Config) -> Result<Self, HostError> {
        Ok(Self {
            socket_path: archon::mcp_bridge::resolve_socket_path(config)
                .context(McpSocketPathSnafu)?,
            call_timeout: Duration::from_secs(config.mcp.call_timeout_secs),
        })
    }
}

#[cfg(test)]
impl Default for McpContext {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("harmonia-mcp.sock"),
            call_timeout: Duration::from_secs(120),
        }
    }
}

pub(crate) async fn run_stdio(config_path: PathBuf) -> Result<(), HostError> {
    let (config, warnings) =
        horismos::load_config(Some(config_path.as_path())).context(ConfigSnafu)?;
    for w in &warnings {
        tracing::warn!(field = %w.field, "{}", w.message);
    }
    let ctx = McpContext::from_config(&config)?;

    let service = HarmoniaServer::new(ctx)
        .serve(stdio())
        .await
        .map_err(|e| HostError::Mcp {
            message: format!("MCP server initialization failed: {e}"),
            location: snafu::location!(),
        })?;
    service.waiting().await.map_err(|e| HostError::Mcp {
        message: format!("MCP serve loop failed: {e}"),
        location: snafu::location!(),
    })?;
    Ok(())
}

/// The rmcp stdio server — 14 tools as `#[tool]` methods over the typed
/// `mcp_params` DTOs. Holds the bridge context plus the two lifecycle op
/// slots (`mcp_ops::OpSlot`, one per long-running kind); the 4 short offline
/// tools build their CLI args per call exactly as the CLI subcommands do.
#[derive(Clone)]
pub(crate) struct HarmoniaServer {
    ctx: McpContext,
    playback: Arc<Mutex<OpSlot>>,
    renderer: Arc<Mutex<OpSlot>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl HarmoniaServer {
    fn new(ctx: McpContext) -> Self {
        Self {
            ctx,
            playback: Arc::new(Mutex::new(OpSlot::new("playback"))),
            renderer: Arc::new(Mutex::new(OpSlot::new("renderer"))),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "harmonia_db_migrate",
        title = "Run Harmonia database migrations",
        description = "Apply embedded SQLite migrations using a harmonia.toml config path."
    )]
    async fn db_migrate(
        &self,
        Parameters(params): Parameters<DbMigrateParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut output = Vec::new();
        let config = params
            .config
            .unwrap_or_else(|| PathBuf::from("harmonia.toml"));
        match crate::db::run_db_migrate(DbMigrateArgs { config }, &mut output).await {
            Ok(()) => Ok(text_output(output).map_or_else(tool_error, tool_success)),
            Err(e) => Ok(tool_error(e.to_string())),
        }
    }

    #[tool(
        name = "harmonia_migrate_library",
        title = "Migrate a legacy media library",
        description = "Run the canonical storage migrator for music, books, audiobooks, or podcasts."
    )]
    async fn migrate_library(
        &self,
        Parameters(params): Parameters<MigrateLibraryParams>,
    ) -> Result<CallToolResult, McpError> {
        let media_type = match params.media_type {
            MigrateMediaType::Music => CliMediaType::Music,
            MigrateMediaType::Books => CliMediaType::Books,
            MigrateMediaType::Audiobooks => CliMediaType::Audiobooks,
            MigrateMediaType::Podcasts => CliMediaType::Podcasts,
            // WHY: the DTO enum is non_exhaustive across the lib/bin boundary —
            // a future variant becomes a tool error here, not a compile break.
            other => return Ok(tool_error(format!("unsupported media_type: {other:?}"))),
        };
        let mut output = Vec::new();
        match crate::migrate::run_migrate(
            MigrateArgs {
                source: params.source,
                target: params.target,
                media_type,
                dry_run: params.dry_run,
                copy: params.copy,
            },
            &mut output,
        )
        .await
        {
            Ok(()) => Ok(text_output(output).map_or_else(tool_error, tool_success)),
            Err(e) => Ok(tool_error(e.to_string())),
        }
    }

    #[tool(
        name = "harmonia_play_file",
        title = "Play a local audio file",
        description = "Play a local file through the Akouo audio engine, blocking until playback stops; cancelling the call stops playback. Prefer the harmonia_play_file_start/status/stop lifecycle ops for long tracks or anything you may stop later — they keep the connection free and represent the work as an op you can poll. This blocking form suits short clips you want to await in one call. A blocking call does not join the lifecycle registry: it is invisible to harmonia_play_file_status and cannot be stopped via harmonia_play_file_stop."
    )]
    async fn play_file(
        &self,
        Parameters(params): Parameters<PlayFileParams>,
        ct: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        let mut output = Vec::new();
        match crate::play::run_play(
            PlayArgs {
                file: params.file,
                device: params.device,
            },
            &mut output,
            ct,
        )
        .await
        {
            Ok(()) => Ok(text_output(output)
                .map(|text| {
                    if text.trim().is_empty() {
                        "playback completed".to_string()
                    } else {
                        text
                    }
                })
                .map_or_else(tool_error, tool_success)),
            Err(e) => Ok(tool_error(e.to_string())),
        }
    }

    #[tool(
        name = "harmonia_render",
        title = "Run Harmonia renderer mode",
        description = "Run the headless renderer loop, blocking while the renderer is active; cancelling the call stops the renderer. Prefer the harmonia_render_start/status/stop lifecycle ops to run the renderer as a managed op you can monitor and stop with separate calls. A blocking call does not join the lifecycle registry: it is invisible to harmonia_render_status and cannot be stopped via harmonia_render_stop."
    )]
    async fn render(
        &self,
        Parameters(params): Parameters<RenderParams>,
        ct: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        match crate::render::run_render(
            crate::render::RenderArgs {
                server: params.server,
                cert_dir: params
                    .cert_dir
                    .unwrap_or_else(crate::paths::default_renderer_cert_dir),
                name: params.name,
                config_path: params.config,
            },
            ct,
        )
        .await
        {
            Ok(()) => Ok(tool_success("renderer completed".to_string())),
            Err(e) => Ok(tool_error(e.to_string())),
        }
    }

    // ── Lifecycle ops (#652 PR 4): the long-runners as registry state ──────

    #[tool(
        name = "harmonia_play_file_start",
        title = "Start playing a local audio file",
        description = "Start playing a local file through the Akouo audio engine as a background op and return its op id immediately. One playback op at a time: a start while one is running is refused, not replaced. Poll harmonia_play_file_status for the exit summary; stop playback with harmonia_play_file_stop."
    )]
    async fn play_file_start(
        &self,
        Parameters(params): Parameters<PlayFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut slot = self.playback.lock().await;
        match slot.start(|ct| mcp_ops::spawn_playback(params, ct)).await {
            Ok(op_id) => Ok(tool_success_data(
                format!("playback started as op {op_id}"),
                json!({ "state": "running", "op_id": op_id.as_str() }),
            )),
            Err(message) => Ok(tool_error(message)),
        }
    }

    #[tool(
        name = "harmonia_play_file_status",
        title = "Report playback op status",
        description = "Report the playback op's state: running, exited (with the exit summary), or idle. With op_id omitted, describes the running op or the most recent exit; an op_id must name one of those two — anything else is a tool error."
    )]
    async fn play_file_status(
        &self,
        Parameters(params): Parameters<PlayFileStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut slot = self.playback.lock().await;
        match slot.status(params.op_id.as_deref()).await {
            Ok(status) => Ok(status_result("playback", status)),
            Err(message) => Ok(tool_error(message)),
        }
    }

    #[tool(
        name = "harmonia_play_file_stop",
        title = "Stop the playback op",
        description = "Cancel the running playback op's token and await its teardown (the engine stop path that releases the audio device), then return the exit summary. Stopping when no playback is running — or with an op_id that is not the running op — is a tool error and stops nothing."
    )]
    async fn play_file_stop(
        &self,
        Parameters(params): Parameters<PlayFileStopParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut slot = self.playback.lock().await;
        match slot.stop(params.op_id.as_deref()).await {
            Ok(exit) => Ok(tool_success_data(
                format!("stopped playback op {}: {}", exit.op_id, exit.summary),
                json!({ "state": "exited", "op_id": exit.op_id.as_str(), "summary": exit.summary }),
            )),
            Err(message) => Ok(tool_error(message)),
        }
    }

    #[tool(
        name = "harmonia_render_start",
        title = "Start the Harmonia renderer",
        description = "Start the headless renderer loop as a background op and return its op id immediately. One renderer op at a time: a start while one is running is refused, not replaced. Poll harmonia_render_status for the exit summary; stop the renderer with harmonia_render_stop."
    )]
    async fn render_start(
        &self,
        Parameters(params): Parameters<RenderParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut slot = self.renderer.lock().await;
        match slot.start(|ct| mcp_ops::spawn_renderer(params, ct)).await {
            Ok(op_id) => Ok(tool_success_data(
                format!("renderer started as op {op_id}"),
                json!({ "state": "running", "op_id": op_id.as_str() }),
            )),
            Err(message) => Ok(tool_error(message)),
        }
    }

    #[tool(
        name = "harmonia_render_status",
        title = "Report renderer op status",
        description = "Report the renderer op's state: running, exited (with the exit summary), or idle. With op_id omitted, describes the running op or the most recent exit; an op_id must name one of those two — anything else is a tool error."
    )]
    async fn render_status(
        &self,
        Parameters(params): Parameters<RenderStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut slot = self.renderer.lock().await;
        match slot.status(params.op_id.as_deref()).await {
            Ok(status) => Ok(status_result("renderer", status)),
            Err(message) => Ok(tool_error(message)),
        }
    }

    #[tool(
        name = "harmonia_render_stop",
        title = "Stop the renderer op",
        description = "Cancel the running renderer op's token and await its teardown (the same shutdown path SIGINT/SIGTERM take), then return the exit summary. Stopping when no renderer is running — or with an op_id that is not the running op — is a tool error and stops nothing."
    )]
    async fn render_stop(
        &self,
        Parameters(params): Parameters<RenderStopParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut slot = self.renderer.lock().await;
        match slot.stop(params.op_id.as_deref()).await {
            Ok(exit) => Ok(tool_success_data(
                format!("stopped renderer op {}: {}", exit.op_id, exit.summary),
                json!({ "state": "exited", "op_id": exit.op_id.as_str(), "summary": exit.summary }),
            )),
            Err(message) => Ok(tool_error(message)),
        }
    }

    #[tool(
        name = "harmonia_search_releases",
        title = "Search acquisition indexers",
        description = "Search configured indexers for a release across media types. Each result carries a release_id usable with harmonia_enqueue_download; download URLs are credential-redacted. Requires a running 'harmonia serve'."
    )]
    async fn search_releases(
        &self,
        Parameters(params): Parameters<SearchReleasesParams>,
        ct: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        Ok(self
            .call_via_bridge("harmonia_search_releases", &params, ct)
            .await)
    }

    #[tool(
        name = "harmonia_enqueue_download",
        title = "Enqueue a download",
        description = "Enqueue exactly one of a cached search result (release_id) or a magnet URI, against an existing want. release_id resolves its credentialed download URL server-side — the credential never crosses this tool boundary. Requires a running 'harmonia serve'."
    )]
    async fn enqueue_download(
        &self,
        Parameters(params): Parameters<EnqueueDownloadParams>,
        ct: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        Ok(self
            .call_via_bridge("harmonia_enqueue_download", &params, ct)
            .await)
    }

    #[tool(
        name = "harmonia_list_downloads",
        title = "List queued and active downloads",
        description = "List download_queue rows, optionally filtered by status or id. download_url is credential-redacted. Requires a running 'harmonia serve'."
    )]
    async fn list_downloads(
        &self,
        Parameters(params): Parameters<ListDownloadsParams>,
        ct: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        Ok(self
            .call_via_bridge("harmonia_list_downloads", &params, ct)
            .await)
    }

    #[tool(
        name = "harmonia_cancel_download",
        title = "Cancel a queued or active download",
        description = "Cancels the queue item, stopping a live download when one is active. Requires a running 'harmonia serve'."
    )]
    async fn cancel_download(
        &self,
        Parameters(params): Parameters<CancelDownloadParams>,
        ct: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        Ok(self
            .call_via_bridge("harmonia_cancel_download", &params, ct)
            .await)
    }

    /// Forwards one acquisition call to the serve-hosted bridge and maps
    /// the outcome to the tool envelope — transport failures (server down,
    /// hung, protocol violation) are tool-level `isError` results with the
    /// same actionable texts the hand-rolled surface produced.
    async fn call_via_bridge<P: serde::Serialize>(
        &self,
        name: &'static str,
        params: &P,
        ct: CancellationToken,
    ) -> CallToolResult {
        let arguments = match serde_json::to_value(params) {
            Ok(arguments) => arguments,
            Err(e) => return tool_error(format!("serialize bridge arguments: {e}")),
        };
        // WHY the literal id: one request line in, one response line out
        // per connection — the response is matched positionally, never by
        // id, and the stdio side only extracts result/error.
        let message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        // WHY the select: rmcp cancellation is cooperative — the handler
        // future keeps running after `notifications/cancelled` unless it
        // observes `ct`. Dropping the `forward_to_bridge` future mid-flight
        // drops its UnixStream, so a cancelled call closes the bridge
        // socket immediately instead of lingering to `call_timeout`.
        let forwarded = tokio::select! {
            biased;
            _ = ct.cancelled() => Err(BridgeCallError::Cancelled),
            result = forward_to_bridge(&self.ctx, &message) => result,
        };
        match forwarded {
            Ok(response) => {
                if let Some(result) = response.get("result") {
                    // WHY: the bridge only ever emits the
                    // `tool_success_json`/`tool_error` envelope shapes, both
                    // of which deserialize as `CallToolResult` verbatim —
                    // content + structuredContent + isError survive intact.
                    serde_json::from_value::<CallToolResult>(result.clone()).unwrap_or_else(|_| {
                        tool_error("malformed response from the harmonia server's MCP bridge")
                    })
                } else if let Some(error) = response.get("error") {
                    let text = error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("mcp bridge returned an error");
                    tool_error(text.to_string())
                } else {
                    tool_error("malformed response from the harmonia server's MCP bridge")
                }
            }
            Err(BridgeCallError::Cancelled) => tool_error(
                "call cancelled by the client; the MCP bridge connection was dropped".to_string(),
            ),
            Err(BridgeCallError::Unavailable) => tool_error(format!(
                "harmonia server is not running (socket {} unavailable); start 'harmonia serve'",
                self.ctx.socket_path.display()
            )),
            Err(BridgeCallError::Timeout) => tool_error(format!(
                "harmonia server did not respond within {}s on the MCP bridge; it may be overloaded",
                self.ctx.call_timeout.as_secs()
            )),
            Err(BridgeCallError::Protocol(detail)) => {
                tool_error(format!("MCP bridge protocol error: {detail}"))
            }
        }
    }
}

impl ServerHandler for HarmoniaServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, SERVER_VERSION))
            .with_instructions(INSTRUCTIONS.to_string())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: self.tool_router.list_all(),
            meta: None,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }
}

// ── Acquisition-tool forwarding over the serve-hosted bridge socket ────────

enum BridgeCallError {
    Unavailable,
    Timeout,
    Cancelled,
    Protocol(String),
}

/// Connects to the bridge socket per call (no pooling), writes one
/// `tools/call` request line, and awaits exactly one response line under
/// `ctx.call_timeout`.
///
/// Cancel-safe by construction: the stream is a plain local, so dropping
/// this future mid-flight (the caller's `ct` select) closes the socket —
/// there is no partially-sent state to clean up beyond that.
async fn forward_to_bridge(ctx: &McpContext, message: &Value) -> Result<Value, BridgeCallError> {
    let connect = tokio::net::UnixStream::connect(&ctx.socket_path);
    let mut stream = tokio::time::timeout(ctx.call_timeout, connect)
        .await
        .map_err(|_| BridgeCallError::Timeout)?
        .map_err(|_| BridgeCallError::Unavailable)?;

    let mut line =
        serde_json::to_string(message).map_err(|e| BridgeCallError::Protocol(e.to_string()))?;
    line.push('\n');
    tokio::time::timeout(ctx.call_timeout, stream.write_all(line.as_bytes()))
        .await
        .map_err(|_| BridgeCallError::Timeout)?
        .map_err(|e| BridgeCallError::Protocol(e.to_string()))?;

    let mut reader = tokio::io::BufReader::new(&mut stream);
    let mut response_line = String::new();
    tokio::time::timeout(ctx.call_timeout, reader.read_line(&mut response_line))
        .await
        .map_err(|_| BridgeCallError::Timeout)?
        .map_err(|e| BridgeCallError::Protocol(e.to_string()))?;

    if response_line.trim().is_empty() {
        return Err(BridgeCallError::Protocol(
            "empty response from bridge".to_string(),
        ));
    }
    serde_json::from_str(&response_line).map_err(|e| BridgeCallError::Protocol(e.to_string()))
}

// ── Tool-result envelopes ──────────────────────────────────────────────────

/// A successful tool call: text content plus the `structuredContent.ok`
/// marker the stdio surface has always carried.
fn tool_success(output: String) -> CallToolResult {
    let mut result = CallToolResult::success(vec![Content::text(output)]);
    result.structured_content = Some(json!({ "ok": true }));
    result
}

/// `tool_success` with extra structured fields merged in — the lifecycle
/// ops report `state`/`op_id`/`summary` a client can match on without
/// parsing the text line.
fn tool_success_data(output: String, data: Value) -> CallToolResult {
    let mut result = CallToolResult::success(vec![Content::text(output)]);
    let mut structured = json!({ "ok": true });
    if let (Some(fields), Value::Object(extra)) = (structured.as_object_mut(), data) {
        fields.extend(extra);
    }
    result.structured_content = Some(structured);
    result
}

/// Renders a registry status into the tool envelope: a text line for logs
/// plus the structured `state` (`running`/`exited`/`idle`) clients poll on.
fn status_result(kind: &str, status: OpStatus) -> CallToolResult {
    match status {
        OpStatus::Idle => tool_success_data(
            format!("no {kind} op is running and none has exited"),
            json!({ "state": "idle" }),
        ),
        OpStatus::Running { op_id } => tool_success_data(
            format!("{kind} op {op_id} is running"),
            json!({ "state": "running", "op_id": op_id.as_str() }),
        ),
        OpStatus::Exited { op_id, summary } => tool_success_data(
            format!("{kind} op {op_id} exited: {summary}"),
            json!({ "state": "exited", "op_id": op_id.as_str(), "summary": summary }),
        ),
    }
}

/// A tool-level failure (`isError: true`) — a normal JSON-RPC SUCCESS
/// response whose result reports the tool itself failed, matching the
/// bridge's own `tool_error` envelope shape.
fn tool_error(message: impl Into<String>) -> CallToolResult {
    let mut result = CallToolResult::error(vec![Content::text(message.into())]);
    result.structured_content = Some(json!({ "ok": false }));
    result
}

fn text_output(output: Vec<u8>) -> Result<String, String> {
    String::from_utf8(output).map_err(|e| format!("tool output was not valid UTF-8: {e}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use paroche::state::{
        DynQueueManager, DynSearchService, EnqueueItem, ResolvedRelease, ServiceError, ServiceFut,
    };
    use rmcp::RoleServer;
    use rmcp::service::{RunningService, serve_server};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf};
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    use super::*;

    /// A test client speaking newline-delimited JSON-RPC over the client
    /// half of the in-memory duplex — the same framing a stdio client uses.
    struct TestClient {
        writer: WriteHalf<DuplexStream>,
        reader: BufReader<ReadHalf<DuplexStream>>,
    }

    impl TestClient {
        async fn send(&mut self, message: Value) {
            let mut line = serde_json::to_string(&message).unwrap();
            line.push('\n');
            self.writer.write_all(line.as_bytes()).await.unwrap();
            self.writer.flush().await.unwrap();
        }

        async fn next_message(&mut self) -> Value {
            self.next_message_within(Duration::from_secs(10)).await
        }

        async fn next_message_within(&mut self, timeout: Duration) -> Value {
            let mut line = String::new();
            tokio::time::timeout(timeout, self.reader.read_line(&mut line))
                .await
                .expect("server response timed out")
                .unwrap();
            serde_json::from_str(&line).unwrap()
        }

        /// One `tools/call` round trip; returns the raw response value.
        async fn call_tool(&mut self, id: i64, name: &str, arguments: Value) -> Value {
            self.send(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            }))
            .await;
            self.next_message().await
        }
    }

    /// Boots a `HarmoniaServer` on an in-memory duplex and drives the
    /// initialize handshake (requesting the older 2025-06-18 version) plus
    /// the initialized notification, returning the client end and the
    /// running service handle.
    async fn start_test_server(
        ctx: McpContext,
    ) -> (TestClient, RunningService<RoleServer, HarmoniaServer>) {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let server = HarmoniaServer::new(ctx);
        let boot = tokio::spawn(async move { serve_server(server, server_io).await });

        let (read_half, write_half) = tokio::io::split(client_io);
        let mut client = TestClient {
            writer: write_half,
            reader: BufReader::new(read_half),
        };
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0" }
                }
            }))
            .await;
        let initialize_response = client.next_message().await;
        assert_eq!(
            initialize_response.pointer("/result/serverInfo/name"),
            Some(&json!("harmonia")),
            "{initialize_response:?}"
        );
        let running = tokio::time::timeout(Duration::from_secs(5), boot)
            .await
            .expect("serve_server boot timed out")
            .unwrap()
            .expect("initialize handshake should succeed");
        client
            .send(json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }))
            .await;
        (client, running)
    }

    async fn stop_test_server(
        client: TestClient,
        running: RunningService<RoleServer, HarmoniaServer>,
    ) {
        drop(client);
        running.cancel().await.unwrap();
    }

    #[tokio::test]
    async fn initialize_negotiates_an_older_client_version() {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let server = HarmoniaServer::new(McpContext::default());
        let boot = tokio::spawn(async move { serve_server(server, server_io).await });

        let (read_half, write_half) = tokio::io::split(client_io);
        let mut client = TestClient {
            writer: write_half,
            reader: BufReader::new(read_half),
        };
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": "a",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0" }
                }
            }))
            .await;
        let response = client.next_message().await;
        assert_eq!(response.pointer("/id"), Some(&json!("a")), "{response:?}");
        assert_eq!(
            response.pointer("/result/serverInfo/name"),
            Some(&json!("harmonia")),
            "{response:?}"
        );
        // WHY: real negotiation, not the old echo — min(client, server) with
        // rmcp 1.7.0's latest (2025-11-25) yields the client's 2025-06-18.
        assert_eq!(
            response.pointer("/result/protocolVersion"),
            Some(&json!("2025-06-18")),
            "{response:?}"
        );
        assert!(
            response
                .pointer("/result/instructions")
                .and_then(Value::as_str)
                .unwrap()
                .contains("Local MCP stdio surface for Harmonia"),
            "{response:?}"
        );

        let running = tokio::time::timeout(Duration::from_secs(5), boot)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn tools_list_includes_all_fourteen_tools() {
        let (mut client, running) = start_test_server(McpContext::default()).await;

        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list"
            }))
            .await;
        let response = client.next_message().await;

        let tools = response
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .expect("tools/list response should include tools");
        let names: Vec<_> = tools
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect();

        assert_eq!(names.len(), 14, "{names:?}");
        assert!(names.contains(&"harmonia_db_migrate"));
        assert!(names.contains(&"harmonia_migrate_library"));
        assert!(names.contains(&"harmonia_play_file"));
        assert!(names.contains(&"harmonia_render"));
        assert!(names.contains(&"harmonia_play_file_start"));
        assert!(names.contains(&"harmonia_play_file_status"));
        assert!(names.contains(&"harmonia_play_file_stop"));
        assert!(names.contains(&"harmonia_render_start"));
        assert!(names.contains(&"harmonia_render_status"));
        assert!(names.contains(&"harmonia_render_stop"));
        assert!(names.contains(&"harmonia_search_releases"));
        assert!(names.contains(&"harmonia_enqueue_download"));
        assert!(names.contains(&"harmonia_list_downloads"));
        assert!(names.contains(&"harmonia_cancel_download"));
        // WHY: the advertised inputSchema is now schemars-generated from the
        // mcp_params DTOs — every tool must carry one, with its required set.
        let play = tools
            .iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some("harmonia_play_file"))
            .unwrap();
        assert_eq!(
            play.pointer("/inputSchema/required"),
            Some(&json!(["file"])),
            "{play:?}"
        );
        // The lifecycle start tools reuse the blocking tools' param DTOs, so
        // they advertise the same required sets; status/stop take only an
        // optional op_id and must require nothing.
        let play_start = tools
            .iter()
            .find(|tool| {
                tool.get("name").and_then(Value::as_str) == Some("harmonia_play_file_start")
            })
            .unwrap();
        assert_eq!(
            play_start.pointer("/inputSchema/required"),
            Some(&json!(["file"])),
            "{play_start:?}"
        );
        let play_stop = tools
            .iter()
            .find(|tool| {
                tool.get("name").and_then(Value::as_str) == Some("harmonia_play_file_stop")
            })
            .unwrap();
        assert!(
            play_stop.pointer("/inputSchema/required").is_none(),
            "status/stop params are all-optional: {play_stop:?}"
        );

        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn migrate_library_tool_runs_dry_run() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let album = source.path().join("Artist").join("Album");
        std::fs::create_dir_all(&album).unwrap();
        std::fs::write(album.join("01 Song.flac"), b"not real audio").unwrap();

        let (mut client, running) = start_test_server(McpContext::default()).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "harmonia_migrate_library",
                    "arguments": {
                        "source": source.path(),
                        "target": target.path(),
                        "media_type": "music",
                        "dry_run": true
                    }
                }
            }))
            .await;
        let response = client.next_message().await;

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(false)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .expect("tool response should include text");
        assert!(
            text.contains("Dry run"),
            "expected dry-run output, got: {text}"
        );

        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn offline_tool_never_touches_the_bridge() {
        // WHY: a socket_path that cannot exist proves the offline tool path
        // never dials it — any bridge involvement would surface as a
        // "harmonia server is not running" / "MCP bridge" message instead of
        // db_migrate's own (unrelated) config-load failure.
        let ctx = McpContext {
            socket_path: PathBuf::from("/nonexistent/harmonia-mcp-test.sock"),
            call_timeout: Duration::from_secs(2),
        };
        let (mut client, running) = start_test_server(ctx).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": {
                    "name": "harmonia_db_migrate",
                    "arguments": { "config": "/nonexistent/harmonia.toml" }
                }
            }))
            .await;
        let response = client.next_message().await;

        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .expect("tool response should include text");
        assert!(!text.contains("MCP bridge"), "{text}");
        assert!(!text.contains("harmonia server is not running"), "{text}");

        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn null_arguments_run_the_tool_with_defaults() {
        // WHY: pins the null half of the non-object-arguments normalization —
        // explicit `null` arguments deserialize as "no arguments" and the
        // tool runs with defaults (db_migrate then fails on the missing
        // default config, a TOOL error, not a protocol rejection).
        let (mut client, running) = start_test_server(McpContext::default()).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": { "name": "harmonia_db_migrate", "arguments": null }
            }))
            .await;
        let response = client.next_message().await;

        // WHY: assert only the ENVELOPE shape — the tool demonstrably RAN
        // with defaults (a result, not a protocol error). Whether the run
        // itself succeeds depends on the machine's config, so its outcome
        // and text stay unpinned.
        assert!(
            response.get("error").is_none(),
            "null arguments must not be a protocol rejection: {response:?}"
        );
        assert!(
            response.pointer("/result/isError").is_some(),
            "the tool ran and answered with a tool-level envelope: {response:?}"
        );

        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn non_object_arguments_are_rejected_as_a_protocol_error() {
        // WHY: pins the other half of the normalization — an array (or any
        // non-object) `arguments` value no longer silently runs the tool with
        // defaults, as the hand-rolled surface did. rmcp 1.7.0 cannot fit the
        // params into `CallToolRequestParams.arguments: Option<JsonObject>`,
        // so the request fails typed dispatch and is answered as an unknown
        // method (-32601 naming "tools/call") before any tool runs.
        let (mut client, running) = start_test_server(McpContext::default()).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": { "name": "harmonia_db_migrate", "arguments": [1, 2, 3] }
            }))
            .await;
        let response = client.next_message().await;

        assert_eq!(response.pointer("/id"), Some(&json!(6)), "{response:?}");
        assert!(
            response.get("error").is_some(),
            "non-object arguments must be a protocol rejection, not a tool run: {response:?}"
        );
        assert_eq!(
            response.pointer("/error/code"),
            Some(&json!(-32601)),
            "{response:?}"
        );

        // The connection survives a poisoned frame.
        client
            .send(json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" }))
            .await;
        let pong = client.next_message().await;
        assert_eq!(pong.pointer("/id"), Some(&json!(7)), "{pong:?}");
        assert_eq!(pong.pointer("/result"), Some(&json!({})), "{pong:?}");

        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn unknown_tool_name_is_a_protocol_error() {
        // WHY: the old surface returned a tool-level isError envelope for
        // unknown names; rmcp's router answers -32602 "tool not found".
        let (mut client, running) = start_test_server(McpContext::default()).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": { "name": "harmonia_nope", "arguments": {} }
            }))
            .await;
        let response = client.next_message().await;

        assert_eq!(
            response.pointer("/error/code"),
            Some(&json!(-32602)),
            "{response:?}"
        );

        stop_test_server(client, running).await;
    }

    // ── Acquisition-tool bridge forwarding (stdio side) ─────────────────────

    #[tokio::test]
    async fn acquisition_tool_call_reports_actionable_error_when_socket_absent() {
        let ctx = McpContext {
            socket_path: PathBuf::from("/nonexistent/harmonia-mcp-test.sock"),
            call_timeout: Duration::from_secs(2),
        };
        let (mut client, running) = start_test_server(ctx).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "harmonia_search_releases", "arguments": {} }
            }))
            .await;
        let response = client.next_message().await;

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("harmonia server is not running"), "{text}");
        assert!(text.contains("harmonia serve"), "{text}");

        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn acquisition_tool_call_times_out_when_bridge_hangs() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("hang.sock");
        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
        tokio::spawn(async move {
            // WHY: accept and hold the connection without ever writing a
            // response line — the client's call_timeout must fire on read.
            if let Ok((_stream, _addr)) = listener.accept().await {
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        let ctx = McpContext {
            socket_path,
            call_timeout: Duration::from_millis(100),
        };
        let (mut client, running) = start_test_server(ctx).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "harmonia_list_downloads", "arguments": {} }
            }))
            .await;
        let response = client.next_message().await;

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("did not respond"), "{text}");

        stop_test_server(client, running).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ping_is_answered_while_a_long_call_is_in_flight() {
        // WHY: the issue's core concurrency guarantee. The hand-rolled loop
        // awaited each request inline, so a ping behind a hung bridged call
        // could never be answered; rmcp spawns one task per request, so the
        // ping response must arrive first.
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("hang.sock");
        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
        tokio::spawn(async move {
            if let Ok((_stream, _addr)) = listener.accept().await {
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
        let ctx = McpContext {
            socket_path,
            call_timeout: Duration::from_secs(3),
        };
        let (mut client, running) = start_test_server(ctx).await;

        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": { "name": "harmonia_list_downloads", "arguments": {} }
            }))
            .await;
        // WHY: give the serve loop a beat to dispatch the call task before
        // the ping lands behind it on the wire.
        tokio::time::sleep(Duration::from_millis(100)).await;
        client
            .send(json!({ "jsonrpc": "2.0", "id": 3, "method": "ping" }))
            .await;

        let first = client.next_message_within(Duration::from_secs(2)).await;
        assert_eq!(
            first.pointer("/id"),
            Some(&json!(3)),
            "ping must be answered while the bridged call is still in flight: {first:?}"
        );
        assert_eq!(first.pointer("/result"), Some(&json!({})), "{first:?}");

        let second = client.next_message().await;
        assert_eq!(second.pointer("/id"), Some(&json!(2)), "{second:?}");
        assert_eq!(
            second.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "the hung bridged call ends in its timeout tool error: {second:?}"
        );

        stop_test_server(client, running).await;
    }

    // ── #652 PR 3 falsifier: cancellation is effective, not just delivered ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_notification_drops_a_hung_bridged_call() {
        use tokio::io::AsyncReadExt as _;

        // WHY: the memo §6 falsifier for bridged calls. rmcp's serve loop
        // intercepts `notifications/cancelled` and cancels the request's
        // token — but the handler future keeps running unless it observes
        // the token. This test proves the EFFECT: a bridge that never
        // answers would otherwise hold the call for the full 30s
        // call_timeout; the cancelled call must answer in ~ms and the
        // bridge side must observe its socket die.
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("hang.sock");
        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
        let (dropped_tx, dropped_rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            let Ok((mut stream, _addr)) = listener.accept().await else {
                return;
            };
            // Read until EOF and never answer: the request line arrives
            // first, then Ok(0) proves the stdio side dropped the socket.
            let mut buf = [0u8; 256];
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
            dropped_tx.send(()).ok();
        });

        let ctx = McpContext {
            socket_path,
            call_timeout: Duration::from_secs(30),
        };
        let (mut client, running) = start_test_server(ctx).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "tools/call",
                "params": { "name": "harmonia_list_downloads", "arguments": {} }
            }))
            .await;
        // WHY: give the serve loop a beat to dispatch the call task before
        // the ping and the cancel land behind it on the wire.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // The issue's core concurrency guarantee still holds mid-call.
        client
            .send(json!({ "jsonrpc": "2.0", "id": 11, "method": "ping" }))
            .await;
        let pong = client.next_message_within(Duration::from_secs(2)).await;
        assert_eq!(
            pong.pointer("/id"),
            Some(&json!(11)),
            "ping must be answered while the bridged call is in flight: {pong:?}"
        );

        client
            .send(json!({
                "jsonrpc": "2.0",
                "method": "notifications/cancelled",
                "params": { "requestId": 10, "reason": "test cancel" }
            }))
            .await;

        let response = client.next_message_within(Duration::from_secs(5)).await;
        assert_eq!(response.pointer("/id"), Some(&json!(10)), "{response:?}");
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "a cancelled bridged call is a tool-level error, not a protocol error: {response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("cancelled"), "{text}");

        // The effect that matters: the bridge socket died with the cancel,
        // long before the 30s call_timeout could have ended the call.
        tokio::time::timeout(Duration::from_secs(2), dropped_rx)
            .await
            .expect("the cancelled call must drop the bridge socket")
            .unwrap();

        stop_test_server(client, running).await;
    }

    // ── #652 PR 4: start/status/stop lifecycle ops over the op registry ────

    #[tokio::test]
    async fn play_file_lifecycle_reports_the_exit_of_a_missing_file() {
        // WHY: the playback registry path at protocol level, without audio
        // hardware — a missing file fails fast on any machine, so the op
        // exits on its own and the registry must reap it with the engine's
        // summary. The running/stop-semantics coverage that needs a
        // controllable long-runner lives in the mcp_ops unit tests (synthetic
        // work) and in play.rs's PR-3 falsifier (real cancellation effect).
        let (mut client, running) = start_test_server(McpContext::default()).await;

        let response = client
            .call_tool(
                20,
                "harmonia_play_file_start",
                json!({ "file": "/nonexistent/harmonia-pr4.flac" }),
            )
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(false)),
            "{response:?}"
        );
        assert_eq!(
            response.pointer("/result/structuredContent/op_id"),
            Some(&json!("playback-1")),
            "{response:?}"
        );
        assert_eq!(
            response.pointer("/result/structuredContent/state"),
            Some(&json!("running")),
            "{response:?}"
        );

        // Poll status until the lazy reap lands — the op's finish timing is
        // the engine's, the reap is the first status call after that.
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        let summary = loop {
            let response = client
                .call_tool(
                    21,
                    "harmonia_play_file_status",
                    json!({ "op_id": "playback-1" }),
                )
                .await;
            let state = response
                .pointer("/result/structuredContent/state")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if state == "exited" {
                break response
                    .pointer("/result/structuredContent/summary")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the missing-file op never exited: {response:?}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        assert!(
            !summary.is_empty(),
            "the exit must carry the engine's summary"
        );

        // An op id that never existed is a tool error, not a status.
        let response = client
            .call_tool(
                22,
                "harmonia_play_file_status",
                json!({ "op_id": "playback-zz" }),
            )
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("unknown playback op"), "{text}");

        // Stopping the already-exited op is a clean tool error.
        let response = client
            .call_tool(
                23,
                "harmonia_play_file_stop",
                json!({ "op_id": "playback-1" }),
            )
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("no playback op is running"), "{text}");

        stop_test_server(client, running).await;
    }

    #[tokio::test]
    async fn render_lifecycle_start_status_stop_round_trip() {
        // WHY: the full registry lifecycle at protocol level, hardware-free.
        // With credentials on disk and an explicit dead server address,
        // run_render enters the runner's connect/backoff cycle and stays
        // there (mDNS discovery is skipped, no audio device is touched) until
        // the registry's stop token breaks it through the same teardown path
        // PR 3's runner falsifier proved.
        let cert_dir = tempfile::tempdir().unwrap();
        crate::render::credentials::save_credentials(
            cert_dir.path(),
            &crate::render::credentials::RendererCredentials {
                api_key: "test-key".to_string(),
                server_fingerprint: "ab".repeat(32),
                server_name: "test-server".to_string(),
                paired_at: "2026-08-12T00:00:00Z".to_string(),
            },
        )
        .unwrap();
        let start_args = json!({ "server": "127.0.0.1:9", "cert_dir": cert_dir.path() });

        let (mut client, running) = start_test_server(McpContext::default()).await;

        let response = client
            .call_tool(30, "harmonia_render_start", start_args.clone())
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(false)),
            "{response:?}"
        );
        assert_eq!(
            response.pointer("/result/structuredContent/op_id"),
            Some(&json!("renderer-1")),
            "{response:?}"
        );
        assert_eq!(
            response.pointer("/result/structuredContent/state"),
            Some(&json!("running")),
            "{response:?}"
        );

        // A second start while renderer-1 runs is refused, not replaced.
        let response = client
            .call_tool(31, "harmonia_render_start", start_args)
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("already running"), "{text}");
        assert!(text.contains("renderer-1"), "{text}");

        // Status reports it running — by id and as "the current one".
        for arguments in [json!({ "op_id": "renderer-1" }), json!({})] {
            let response = client
                .call_tool(32, "harmonia_render_status", arguments)
                .await;
            assert_eq!(
                response.pointer("/result/structuredContent/state"),
                Some(&json!("running")),
                "{response:?}"
            );
            assert_eq!(
                response.pointer("/result/structuredContent/op_id"),
                Some(&json!("renderer-1")),
                "{response:?}"
            );
        }

        // A stop addressed to the wrong op refuses and stops nothing.
        let response = client
            .call_tool(
                33,
                "harmonia_render_stop",
                json!({ "op_id": "renderer-99" }),
            )
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("renderer-99"), "{text}");
        assert!(text.contains("renderer-1"), "{text}");
        let response = client
            .call_tool(34, "harmonia_render_status", json!({}))
            .await;
        assert_eq!(
            response.pointer("/result/structuredContent/state"),
            Some(&json!("running")),
            "the refused stop must leave the op running: {response:?}"
        );

        // The real stop cancels the op's token and awaits teardown.
        let response = client
            .call_tool(35, "harmonia_render_stop", json!({}))
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(false)),
            "{response:?}"
        );
        assert_eq!(
            response.pointer("/result/structuredContent/state"),
            Some(&json!("exited")),
            "{response:?}"
        );
        let summary = response
            .pointer("/result/structuredContent/summary")
            .and_then(Value::as_str)
            .unwrap();
        assert!(
            summary.contains("cancelled"),
            "the stop must name the stop cause: {summary}"
        );

        // Status reports the recorded exit; a second stop is a clean error.
        let response = client
            .call_tool(
                36,
                "harmonia_render_status",
                json!({ "op_id": "renderer-1" }),
            )
            .await;
        assert_eq!(
            response.pointer("/result/structuredContent/state"),
            Some(&json!("exited")),
            "{response:?}"
        );
        let response = client
            .call_tool(37, "harmonia_render_stop", json!({}))
            .await;
        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true)),
            "{response:?}"
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("no renderer op is running"), "{text}");

        stop_test_server(client, running).await;
    }

    // ── Full round trip: stdio surface -> real bridge over a real socket ───

    struct EmptySearch;
    impl DynSearchService for EmptySearch {
        fn search(&self, _query: Value) -> ServiceFut<Value> {
            Box::pin(async { Ok(json!({ "results": [] })) })
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
        fn resolve_release(&self, _release_id: Uuid) -> ServiceFut<ResolvedRelease> {
            Box::pin(async { Err(ServiceError::NotAvailable) })
        }
    }

    struct NoopQueue;
    impl DynQueueManager for NoopQueue {
        fn enqueue(&self, _item: EnqueueItem) -> ServiceFut<()> {
            Box::pin(async { Ok(()) })
        }
        fn cancel(&self, _queue_id: Uuid) -> ServiceFut<()> {
            Box::pin(async { Err(ServiceError::NotFound) })
        }
        fn reprioritize(&self, _queue_id: Uuid, _priority: u8) -> ServiceFut<()> {
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test]
    async fn acquisition_tool_call_round_trips_through_a_real_bridge() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        apotheke::migrate::MIGRATOR.run(&pool).await.unwrap();
        let db = Arc::new(apotheke::DbPools {
            read: pool.clone(),
            write: pool,
        });
        let bridge_ctx = archon::mcp_bridge::BridgeContext {
            search: Arc::new(EmptySearch),
            queue: Arc::new(NoopQueue),
            db,
        };
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("mcp.sock");
        let shutdown = CancellationToken::new();
        let handle = archon::mcp_bridge::spawn(socket_path.clone(), bridge_ctx, shutdown.clone())
            .await
            .expect("bridge binds");

        let ctx = McpContext {
            socket_path,
            call_timeout: Duration::from_secs(5),
        };
        let (mut client, running) = start_test_server(ctx).await;
        client
            .send(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "harmonia_list_downloads", "arguments": {} }
            }))
            .await;
        let response = client.next_message().await;

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(false)),
            "{response:?}"
        );
        assert_eq!(
            response.pointer("/result/structuredContent/downloads"),
            Some(&json!([]))
        );

        stop_test_server(client, running).await;
        shutdown.cancel();
        handle.await.unwrap();
    }
}
