// MCP transport: hand-rolled. rust-mcp-sdk v0.9.0 exists but pulls in
// hyper/axum/reqwest/SSL — far too heavy for a pure stdio adapter. The
// JSON-RPC-2.0 stdio protocol is ~100 LOC to hand-roll and carries no
// ecosystem risk. See crates/wos-mcp/README.md for the full rationale.

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
const INTERNAL_ERROR: i32 = -32603;

// ── Routing ──────────────────────────────────────────────────────────────────

/// Process one JSON-RPC request and return the response value.
async fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone().unwrap_or(Value::Null);

    debug!(method = %req.method, "handling request");

    match req.method.as_str() {
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

        // MCP notifications that require no response body.
        "notifications/initialized" => JsonRpcResponse::ok(id, Value::Null),

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

            match wos_mcp::dispatch::dispatch(&tool_name, args).await {
                Ok(result) => JsonRpcResponse::ok(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": result.to_string()}]
                    }),
                ),
                Err(e) => JsonRpcResponse::err(id, INTERNAL_ERROR, e.to_string()),
            }
        }

        other => JsonRpcResponse::err(
            id,
            METHOD_NOT_FOUND,
            format!("method not found: {other}"),
        ),
    }
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

        let response: JsonRpcResponse = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => handle_request(req).await,
            Err(e) => JsonRpcResponse::err(
                Value::Null,
                -32700, // Parse error
                format!("parse error: {e}"),
            ),
        };

        let serialized = serde_json::to_string(&response).expect("response serialization failed");
        writeln!(stdout, "{serialized}").expect("stdout write failed");
        stdout.flush().expect("stdout flush failed");
    }
}
