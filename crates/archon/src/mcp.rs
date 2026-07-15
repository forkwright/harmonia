use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use snafu::ResultExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::cli::{CliMediaType, DbMigrateArgs, MigrateArgs, PlayArgs};
use crate::error::{ConfigSnafu, HostError, OutputSnafu};

const SERVER_NAME: &str = "harmonia";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Resolved once at process start — the acquisition bridge socket path and
/// per-call deadline. The 4 offline tools never touch this; the 4
/// acquisition tools forward through it on every call (no pooling).
#[derive(Clone)]
pub(crate) struct McpContext {
    pub(crate) socket_path: PathBuf,
    pub(crate) call_timeout: Duration,
}

impl McpContext {
    fn from_config(config: &horismos::Config) -> Self {
        Self {
            socket_path: archon::mcp_bridge::resolve_socket_path(config),
            call_timeout: Duration::from_secs(config.mcp.call_timeout_secs),
        }
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
    let ctx = McpContext::from_config(&config);

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let reader = std::io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    run_stdio_with_io(reader, &mut writer, &ctx).await
}

async fn run_stdio_with_io<R, W>(
    reader: R,
    writer: &mut W,
    ctx: &McpContext,
) -> Result<(), HostError>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line.context(OutputSnafu {
            operation: "read MCP request",
        })?;
        let Some(response) = handle_line(&line, ctx).await else {
            continue;
        };
        let encoded = serde_json::to_string(&response).map_err(|e| HostError::Mcp {
            message: format!("serialize response: {e}"),
            location: snafu::location!(),
        })?;
        writeln!(writer, "{encoded}").context(OutputSnafu {
            operation: "write MCP response",
        })?;
        writer.flush().context(OutputSnafu {
            operation: "flush MCP response",
        })?;
    }

    Ok(())
}

async fn handle_line(line: &str, ctx: &McpContext) -> Option<Value> {
    let message = match serde_json::from_str::<Value>(line) {
        Ok(message) => message,
        Err(e) => {
            return Some(error_response(
                Value::Null,
                -32700,
                format!("parse error: {e}"),
            ));
        }
    };

    handle_message(message, ctx).await
}

async fn handle_message(message: Value, ctx: &McpContext) -> Option<Value> {
    if message.is_array() {
        return Some(error_response(
            Value::Null,
            -32600,
            "JSON-RPC batch requests are not supported".to_string(),
        ));
    }

    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str);

    match (id, method) {
        (None, Some("notifications/initialized")) => None,
        (None, Some(_)) => None,
        (Some(id), Some("initialize")) => Some(success_response(id, initialize_result(&message))),
        (Some(id), Some("ping")) => Some(success_response(id, json!({}))),
        (Some(id), Some("tools/list")) => Some(success_response(id, tools_list_result())),
        (Some(id), Some("tools/call")) => {
            Some(success_response(id, call_tool_result(&message, ctx).await))
        }
        (Some(id), Some(method)) => Some(error_response(
            id,
            -32601,
            format!("method not found: {method}"),
        )),
        (Some(id), None) => Some(error_response(id, -32600, "missing method".to_string())),
        (None, None) => None,
    }
}

fn initialize_result(message: &Value) -> Value {
    let requested = message
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_VERSION);

    json!({
        "protocolVersion": requested,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION
        },
        "instructions": "Local MCP stdio surface for Harmonia. The 4 offline tools (db_migrate, migrate_library, play_file, render) run in-process with no server required. The 4 acquisition tools (search_releases, enqueue_download, list_downloads, cancel_download) forward to a running 'harmonia serve' over its local acquisition bridge socket — if the server isn't running, those calls return a tool-level error naming the socket. The HTTP API remains the canonical remote service API."
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            db_migrate_tool(),
            migrate_library_tool(),
            play_file_tool(),
            render_tool(),
            search_releases_tool(),
            enqueue_download_tool(),
            list_downloads_tool(),
            cancel_download_tool()
        ]
    })
}

fn db_migrate_tool() -> Value {
    json!({
        "name": "harmonia_db_migrate",
        "title": "Run Harmonia database migrations",
        "description": "Apply embedded SQLite migrations using a harmonia.toml config path.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "config": {
                    "type": "string",
                    "description": "Path to harmonia.toml. Defaults to harmonia.toml."
                }
            },
            "additionalProperties": false
        }
    })
}

fn migrate_library_tool() -> Value {
    json!({
        "name": "harmonia_migrate_library",
        "title": "Migrate a legacy media library",
        "description": "Run the canonical storage migrator for music, books, audiobooks, or podcasts.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "source": { "type": "string", "description": "Source directory containing legacy media." },
                "target": { "type": "string", "description": "Target directory for canonical output." },
                "media_type": {
                    "type": "string",
                    "enum": ["music", "books", "audiobooks", "podcasts"]
                },
                "dry_run": { "type": "boolean", "default": false },
                "copy": { "type": "boolean", "default": false }
            },
            "required": ["source", "target", "media_type"],
            "additionalProperties": false
        }
    })
}

fn play_file_tool() -> Value {
    json!({
        "name": "harmonia_play_file",
        "title": "Play a local audio file",
        "description": "Play a local file through the Akouo audio engine. This blocks until playback stops.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "Path to the audio file." },
                "device": { "type": "string", "description": "Optional audio output device name." }
            },
            "required": ["file"],
            "additionalProperties": false
        }
    })
}

fn render_tool() -> Value {
    json!({
        "name": "harmonia_render",
        "title": "Run Harmonia renderer mode",
        "description": "Run the headless renderer loop. This blocks while the renderer is active.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "server": { "type": "string", "description": "Optional host:port server address." },
                "cert_dir": {
                    "type": "string",
                    "description": "Directory for TLS certificates and pairing credentials. Defaults to $XDG_CONFIG_HOME/harmonia/renderer."
                },
                "name": { "type": "string", "description": "Optional renderer display name." },
                "config": { "type": "string", "description": "Optional renderer TOML config path." }
            },
            "additionalProperties": false
        }
    })
}

fn search_releases_tool() -> Value {
    json!({
        "name": "harmonia_search_releases",
        "title": "Search acquisition indexers",
        "description": "Search configured indexers for a release across media types. Each result carries a release_id usable with harmonia_enqueue_download; download URLs are credential-redacted. Requires a running 'harmonia serve'.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query_text": { "type": "string" },
                "media_type": { "type": "string", "enum": ["any", "tv", "movie", "music", "book"] },
                "artist": { "type": "string" },
                "album": { "type": "string" },
                "author": { "type": "string" },
                "imdb_id": { "type": "string" },
                "tvdb_id": { "type": "integer" },
                "tmdb_id": { "type": "integer" },
                "season": { "type": "integer" },
                "episode": { "type": "integer" },
                "category_ids": { "type": "array", "items": { "type": "integer" } },
                "limit": { "type": "integer", "default": 100 },
                "offset": { "type": "integer", "default": 0 }
            },
            "additionalProperties": false
        }
    })
}

fn enqueue_download_tool() -> Value {
    json!({
        "name": "harmonia_enqueue_download",
        "title": "Enqueue a download",
        "description": "Enqueue exactly one of a cached search result (release_id) or a magnet URI. release_id resolves its credentialed download URL server-side — the credential never crosses this tool boundary. Requires a running 'harmonia serve'.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "release_id": { "type": "string", "description": "A release_id from a prior harmonia_search_releases result." },
                "magnet": { "type": "string", "description": "A magnet: URI. Raw http(s) URLs are rejected — use release_id for credentialed indexer results." },
                "priority": { "type": "integer", "minimum": 1, "maximum": 4, "default": 4 },
                "want_id": { "type": "string", "description": "Optional want row UUID to associate; a fresh UUIDv7 is generated when omitted." }
            },
            "additionalProperties": false
        }
    })
}

fn list_downloads_tool() -> Value {
    json!({
        "name": "harmonia_list_downloads",
        "title": "List queued and active downloads",
        "description": "List download_queue rows, optionally filtered by status or id. download_url is credential-redacted. Requires a running 'harmonia serve'.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "status": { "type": "string", "enum": ["queued", "downloading", "post_processing", "importing", "completed", "failed"] },
                "id": { "type": "string", "description": "Filter to one download_queue row id." },
                "limit": { "type": "integer", "default": 50 }
            },
            "additionalProperties": false
        }
    })
}

fn cancel_download_tool() -> Value {
    json!({
        "name": "harmonia_cancel_download",
        "title": "Cancel a queued or active download",
        "description": "Cancels the queue item, stopping a live download when one is active. Requires a running 'harmonia serve'.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "The download_queue row id (from harmonia_enqueue_download or harmonia_list_downloads)." }
            },
            "required": ["id"],
            "additionalProperties": false
        }
    })
}

async fn call_tool_result(message: &Value, ctx: &McpContext) -> Value {
    let Some(name) = message.pointer("/params/name").and_then(Value::as_str) else {
        return tool_error("tools/call requires params.name");
    };

    if archon::mcp_bridge::is_acquisition_tool(name) {
        return call_via_bridge(ctx, message).await;
    }

    let arguments = message
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match call_tool(name, &arguments).await {
        Ok(output) => tool_success(output),
        Err(message) => tool_error(message),
    }
}

// ── Acquisition-tool forwarding over the serve-hosted bridge socket ────────

enum BridgeCallError {
    Unavailable,
    Timeout,
    Protocol(String),
}

async fn call_via_bridge(ctx: &McpContext, message: &Value) -> Value {
    match forward_to_bridge(ctx, message).await {
        Ok(response) => {
            if let Some(result) = response.get("result") {
                result.clone()
            } else if let Some(error) = response.get("error") {
                let text = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("mcp bridge returned an error");
                archon::mcp_bridge::tool_error(text.to_string())
            } else {
                archon::mcp_bridge::tool_error(
                    "malformed response from the harmonia server's MCP bridge",
                )
            }
        }
        Err(BridgeCallError::Unavailable) => archon::mcp_bridge::tool_error(format!(
            "harmonia server is not running (socket {} unavailable); start 'harmonia serve'",
            ctx.socket_path.display()
        )),
        Err(BridgeCallError::Timeout) => archon::mcp_bridge::tool_error(format!(
            "harmonia server did not respond within {}s on the MCP bridge; it may be overloaded",
            ctx.call_timeout.as_secs()
        )),
        Err(BridgeCallError::Protocol(detail)) => {
            archon::mcp_bridge::tool_error(format!("MCP bridge protocol error: {detail}"))
        }
    }
}

/// Connects to the bridge socket per call (no pooling), writes the ORIGINAL
/// `tools/call` request line unchanged, and awaits exactly one response line
/// under `ctx.call_timeout`.
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

async fn call_tool(name: &str, arguments: &Value) -> Result<String, String> {
    match name {
        "harmonia_db_migrate" => call_db_migrate(arguments).await,
        "harmonia_migrate_library" => call_migrate_library(arguments).await,
        "harmonia_play_file" => call_play_file(arguments).await,
        "harmonia_render" => call_render(arguments).await,
        _ => Err(format!("unknown tool: {name}")),
    }
}

async fn call_db_migrate(arguments: &Value) -> Result<String, String> {
    let mut output = Vec::new();
    let config =
        optional_path(arguments, "config")?.unwrap_or_else(|| PathBuf::from("harmonia.toml"));
    crate::db::run_db_migrate(DbMigrateArgs { config }, &mut output)
        .await
        .map_err(|e| e.to_string())?;
    string_output(output)
}

async fn call_migrate_library(arguments: &Value) -> Result<String, String> {
    let source = required_path(arguments, "source")?;
    let target = required_path(arguments, "target")?;
    let media_type = required_media_type(arguments, "media_type")?;
    let dry_run = optional_bool(arguments, "dry_run")?;
    let copy = optional_bool(arguments, "copy")?;

    let mut output = Vec::new();
    crate::migrate::run_migrate(
        MigrateArgs {
            source,
            target,
            media_type,
            dry_run,
            copy,
        },
        &mut output,
    )
    .await
    .map_err(|e| e.to_string())?;
    string_output(output)
}

async fn call_play_file(arguments: &Value) -> Result<String, String> {
    let file = required_path(arguments, "file")?;
    let device = optional_string(arguments, "device")?;
    let mut output = Vec::new();
    crate::play::run_play(PlayArgs { file, device }, &mut output)
        .await
        .map_err(|e| e.to_string())?;
    string_output_or_default(output, "playback completed")
}

async fn call_render(arguments: &Value) -> Result<String, String> {
    let server = optional_socket_addr(arguments, "server")?;
    let cert_dir = optional_path(arguments, "cert_dir")?
        .unwrap_or_else(crate::paths::default_renderer_cert_dir);
    let name = optional_string(arguments, "name")?;
    let config_path = optional_path(arguments, "config")?;

    crate::render::run_render(crate::render::RenderArgs {
        server,
        cert_dir,
        name,
        config_path,
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok("renderer completed".to_string())
}

fn required_path(arguments: &Value, key: &'static str) -> Result<PathBuf, String> {
    required_string(arguments, key).map(PathBuf::from)
}

fn optional_path(arguments: &Value, key: &'static str) -> Result<Option<PathBuf>, String> {
    Ok(optional_string(arguments, key)?.map(PathBuf::from))
}

fn required_media_type(arguments: &Value, key: &'static str) -> Result<CliMediaType, String> {
    match required_string(arguments, key)?.as_str() {
        "music" => Ok(CliMediaType::Music),
        "books" => Ok(CliMediaType::Books),
        "audiobooks" => Ok(CliMediaType::Audiobooks),
        "podcasts" => Ok(CliMediaType::Podcasts),
        other => Err(format!(
            "{key} must be one of music, books, audiobooks, podcasts; got {other}"
        )),
    }
}

fn optional_socket_addr(
    arguments: &Value,
    key: &'static str,
) -> Result<Option<SocketAddr>, String> {
    optional_string(arguments, key)?
        .map(|raw| {
            raw.parse::<SocketAddr>()
                .map_err(|e| format!("{key} must be a socket address: {e}"))
        })
        .transpose()
}

fn required_string(arguments: &Value, key: &'static str) -> Result<String, String> {
    optional_string(arguments, key)?.ok_or_else(|| format!("missing required argument: {key}"))
}

fn optional_string(arguments: &Value, key: &'static str) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

fn optional_bool(arguments: &Value, key: &'static str) -> Result<bool, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(format!("{key} must be a boolean")),
    }
}

fn string_output(output: Vec<u8>) -> Result<String, String> {
    String::from_utf8(output).map_err(|e| format!("tool output was not valid UTF-8: {e}"))
}

fn string_output_or_default(output: Vec<u8>, default: &'static str) -> Result<String, String> {
    let output = string_output(output)?;
    if output.trim().is_empty() {
        Ok(default.to_string())
    } else {
        Ok(output)
    }
}

fn tool_success(output: String) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": output
            }
        ],
        "structuredContent": {
            "ok": true
        },
        "isError": false
    })
}

fn tool_error(message: impl Into<String>) -> Value {
    let message = message.into();
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "structuredContent": {
            "ok": false
        },
        "isError": true
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use paroche::state::{
        DynQueueManager, DynSearchService, EnqueueItem, ResolvedRelease, ServiceError, ServiceFut,
    };
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn tools_list_includes_all_eight_tools() {
        let response = handle_message(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list"
            }),
            &McpContext::default(),
        )
        .await
        .expect("request should produce a response");

        let tools = response
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .expect("tools/list response should include tools");
        let names: Vec<_> = tools
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect();

        assert_eq!(names.len(), 8, "{names:?}");
        assert!(names.contains(&"harmonia_db_migrate"));
        assert!(names.contains(&"harmonia_migrate_library"));
        assert!(names.contains(&"harmonia_play_file"));
        assert!(names.contains(&"harmonia_render"));
        assert!(names.contains(&"harmonia_search_releases"));
        assert!(names.contains(&"harmonia_enqueue_download"));
        assert!(names.contains(&"harmonia_list_downloads"));
        assert!(names.contains(&"harmonia_cancel_download"));
    }

    #[tokio::test]
    async fn migrate_library_tool_runs_dry_run() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let album = source.path().join("Artist").join("Album");
        std::fs::create_dir_all(&album).unwrap();
        std::fs::write(album.join("01 Song.flac"), b"not real audio").unwrap();

        let response = handle_message(
            json!({
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
            }),
            &McpContext::default(),
        )
        .await
        .expect("request should produce a response");

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(false))
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .expect("tool response should include text");
        assert!(
            text.contains("Dry run"),
            "expected dry-run output, got: {text}"
        );
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
        let response = handle_message(
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": {
                    "name": "harmonia_db_migrate",
                    "arguments": { "config": "/nonexistent/harmonia.toml" }
                }
            }),
            &ctx,
        )
        .await
        .expect("request should produce a response");

        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .expect("tool response should include text");
        assert!(!text.contains("MCP bridge"), "{text}");
        assert!(!text.contains("harmonia server is not running"), "{text}");
    }

    #[tokio::test]
    async fn stdio_runner_writes_one_response_per_request_line() {
        let input = br#"{"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}
"#;
        let mut output = Vec::new();

        run_stdio_with_io(&input[..], &mut output, &McpContext::default())
            .await
            .unwrap();

        let line = String::from_utf8(output).unwrap();
        let response: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(
            response.pointer("/result/serverInfo/name"),
            Some(&json!("harmonia"))
        );
    }

    // ── Acquisition-tool bridge forwarding (stdio side) ─────────────────────

    #[tokio::test]
    async fn acquisition_tool_call_reports_actionable_error_when_socket_absent() {
        let ctx = McpContext {
            socket_path: PathBuf::from("/nonexistent/harmonia-mcp-test.sock"),
            call_timeout: Duration::from_secs(2),
        };
        let response = handle_message(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "harmonia_search_releases", "arguments": {} }
            }),
            &ctx,
        )
        .await
        .expect("request should produce a response");

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true))
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("harmonia server is not running"), "{text}");
        assert!(text.contains("harmonia serve"), "{text}");
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
        let response = handle_message(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "harmonia_list_downloads", "arguments": {} }
            }),
            &ctx,
        )
        .await
        .expect("request should produce a response");

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(true))
        );
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("did not respond"), "{text}");
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
        let response = handle_message(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "harmonia_list_downloads", "arguments": {} }
            }),
            &ctx,
        )
        .await
        .expect("request should produce a response");

        assert_eq!(
            response.pointer("/result/isError"),
            Some(&Value::Bool(false)),
            "{response:?}"
        );
        assert_eq!(
            response.pointer("/result/structuredContent/downloads"),
            Some(&json!([]))
        );

        shutdown.cancel();
        handle.await.unwrap();
    }
}
