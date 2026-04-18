// MCP transport: hand-rolled JSON-RPC-2.0 over stdio (~100 LOC). The
// original rationale ("rust-mcp-sdk v0.9.0 pulls hyper/axum/reqwest/SSL")
// was retracted 2026-04-18: `default-features = false, features = ["stdio"]`
// avoids those deps. See the TODO at crates/wos-mcp/Cargo.toml for the
// feature analysis and crates/wos-mcp/README.md for the current rationale.

//! `wos-mcp` binary — JSON-RPC-2.0 stdio server for the WOS MCP interface.
//!
//! Reads newline-delimited JSON-RPC-2.0 requests from stdin, routes to tool
//! handlers via `wos_mcp::dispatch`, and writes JSON-RPC-2.0 responses to
//! stdout. Shuts down gracefully on EOF.

use std::io::{BufRead, Write};

use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use wos_mcp::registry::ProjectRegistry;

// ── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "wos-mcp", about = "WOS MCP stdio server")]
struct Cli {}

// ── JSON-RPC-2.0 types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    // Accepted for protocol conformance; not used for routing.
    #[serde(default)]
    _jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// JSON-RPC-2.0 standard error codes
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;

// ── Routing ──────────────────────────────────────────────────────────────────

/// Process one JSON-RPC request and return the response value, or `None` if
/// the incoming message was a JSON-RPC-2.0 notification (no `id` field).
///
/// Per JSON-RPC-2.0 §4.1 and MCP spec, notifications MUST NOT receive a
/// response — the server silently consumes them.
async fn handle_request(registry: &ProjectRegistry, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    // Notification detection: absence of `id` field is the JSON-RPC-2.0
    // signal that the client does not want a response. A present-but-null
    // `id` is still a request (legal, though unusual).
    let Some(id) = req.id.clone() else {
        debug!(method = %req.method, "received notification — no response emitted");
        return None;
    };

    debug!(method = %req.method, "handling request");

    Some(match req.method.as_str() {
        // MCP protocol handshake — return server capabilities.
        "initialize" => JsonRpcResponse::ok(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "wos-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),

        // Return the current tool catalog.
        "tools/list" => JsonRpcResponse::ok(
            id,
            serde_json::json!({
                "tools": [
                    {
                        "name": "wos_ping",
                        "description": "Health-check tool. Returns {\"pong\": true}. Used in scaffold tests and transport smoke tests.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    }
                ]
            }),
        ),

        // Route tool calls through the unified dispatch function.
        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let args = req
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            // `project_id` lives inside `arguments` for handlers that need it.
            // Pulled out here so the handler signature stays uniform across all
            // tools — project-less tools (e.g. `wos_ping`) just ignore it.
            let project_id = args
                .get("project_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            match wos_mcp::dispatch::dispatch(registry, &tool_name, &project_id, args).await {
                Ok(result) => JsonRpcResponse::ok(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": result.to_string()}]
                    }),
                ),
                // Unknown tool name: routing failure → JSON-RPC error.
                // Per MCP spec, the `name` field points to a missing tool,
                // which is a parameter-level fault (INVALID_PARAMS).
                Err(e @ wos_mcp::errors::DispatchError::UnknownTool(_)) => {
                    JsonRpcResponse::err(id, INVALID_PARAMS, e.to_string())
                }
                // Tool execution failure: per MCP spec, return a SUCCESSFUL
                // JSON-RPC response whose `result` carries `isError: true`
                // and the error message in the content array. This lets the
                // client model tool failures separately from protocol faults.
                Err(e @ wos_mcp::errors::DispatchError::ToolFailed { .. }) => {
                    JsonRpcResponse::ok(
                        id,
                        serde_json::json!({
                            "isError": true,
                            "content": [{"type": "text", "text": e.to_string()}]
                        }),
                    )
                }
            }
        }

        other => JsonRpcResponse::err(
            id,
            METHOD_NOT_FOUND,
            format!("method not found: {other}"),
        ),
    })
}

// ── Main loop ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let _cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // One registry per process; Task 3 will populate it with `WosProject`
    // entries. Held behind a plain `&` borrow — the stdio loop is
    // single-threaded, so no locking is needed yet.
    let registry = ProjectRegistry::new();

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("stdin read error: {e}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response: Option<JsonRpcResponse> = match serde_json::from_str::<JsonRpcRequest>(
            trimmed,
        ) {
            Ok(req) => handle_request(&registry, req).await,
            Err(e) => Some(JsonRpcResponse::err(
                Value::Null,
                -32700, // Parse error
                format!("parse error: {e}"),
            )),
        };

        // JSON-RPC-2.0 notifications produce no output at all.
        let Some(response) = response else {
            continue;
        };

        let serialized = serde_json::to_string(&response).expect("response serialization failed");
        writeln!(stdout, "{serialized}").expect("stdout write failed");
        stdout.flush().expect("stdout flush failed");
    }
}
