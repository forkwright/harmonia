use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::path::PathBuf;

use serde_json::{Value, json};
use snafu::ResultExt;

use crate::cli::{CliMediaType, DbMigrateArgs, MigrateArgs, PlayArgs};
use crate::error::{HostError, OutputSnafu};

const SERVER_NAME: &str = "harmonia";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2025-06-18";

pub(crate) async fn run_stdio() -> Result<(), HostError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let reader = std::io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    run_stdio_with_io(reader, &mut writer).await
}

async fn run_stdio_with_io<R, W>(reader: R, writer: &mut W) -> Result<(), HostError>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line.context(OutputSnafu {
            operation: "read MCP request",
        })?;
        let Some(response) = handle_line(&line).await else {
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

async fn handle_line(line: &str) -> Option<Value> {
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

    handle_message(message).await
}

async fn handle_message(message: Value) -> Option<Value> {
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
            Some(success_response(id, call_tool_result(&message).await))
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
        "instructions": "Local MCP stdio surface for Harmonia maintenance operations. The HTTP API remains the canonical remote service API."
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            db_migrate_tool(),
            migrate_library_tool(),
            play_file_tool(),
            render_tool()
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

async fn call_tool_result(message: &Value) -> Value {
    let Some(name) = message.pointer("/params/name").and_then(Value::as_str) else {
        return tool_error("tools/call requires params.name");
    };
    let arguments = message
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match call_tool(name, &arguments).await {
        Ok(output) => tool_success(output),
        Err(message) => tool_error(message),
    }
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
    use super::*;

    #[tokio::test]
    async fn tools_list_includes_offline_command_tools() {
        let response = handle_message(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }))
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

        assert!(names.contains(&"harmonia_db_migrate"));
        assert!(names.contains(&"harmonia_migrate_library"));
        assert!(names.contains(&"harmonia_play_file"));
        assert!(names.contains(&"harmonia_render"));
    }

    #[tokio::test]
    async fn migrate_library_tool_runs_dry_run() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let album = source.path().join("Artist").join("Album");
        std::fs::create_dir_all(&album).unwrap();
        std::fs::write(album.join("01 Song.flac"), b"not real audio").unwrap();

        let response = handle_message(json!({
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
    async fn stdio_runner_writes_one_response_per_request_line() {
        let input = br#"{"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}
"#;
        let mut output = Vec::new();

        run_stdio_with_io(&input[..], &mut output).await.unwrap();

        let line = String::from_utf8(output).unwrap();
        let response: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(
            response.pointer("/result/serverInfo/name"),
            Some(&json!("harmonia"))
        );
    }
}
