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
async fn handle_request(
    registry: &mut ProjectRegistry,
    req: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
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
                    },
                    {
                        "name": "wos_create_kernel",
                        "description": "Create a new empty WOS kernel project. Returns {\"project_id\": \"<uuid>\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "wos_load_document",
                        "description": "Parse and register a WOS kernel document from inline JSON text or a file path. Returns {\"project_id\": \"<uuid>\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "json": { "type": "string", "description": "WOS kernel document as a JSON string." },
                                "path": { "type": "string", "description": "Path to a WOS kernel JSON file." }
                            },
                            "required": []
                        }
                    },
                    {
                        "name": "wos_export_document",
                        "description": "Serialize a registered project back to a JSON string. Returns {\"document\": \"<json-string>\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." }
                            },
                            "required": ["project_id"]
                        }
                    },
                    {
                        "name": "wos_describe_document",
                        "description": "Return summary counts for a registered project: state_count, transition_count, actor_count, impact_level, ai_agent_count.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." }
                            },
                            "required": ["project_id"]
                        }
                    },
                    {
                        "name": "wos_add_state",
                        "description": "Add a top-level state to a registered project. Returns {\"state_id\": \"<id>\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "state_id": { "type": "string", "description": "Unique state identifier." },
                                "kind": { "type": "string", "enum": ["atomic", "compound", "parallel", "final"], "description": "State kind; defaults to atomic." },
                                "label": { "type": "string", "description": "Optional human-readable description stored on the state." },
                                "metadata": { "type": "object", "description": "Optional metadata stored under state.extensions.x-meta." }
                            },
                            "required": ["project_id", "state_id"]
                        }
                    },
                    {
                        "name": "wos_add_transition",
                        "description": "Add a transition between two existing states. Returns {\"from\": \"...\", \"to\": \"...\", \"trigger\": \"...\", \"event\": ...}. Use `trigger` for a legacy string event, or `event` for a typed TransitionEvent object (not both).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "from": { "type": "string", "description": "Source state identifier." },
                                "to": { "type": "string", "description": "Target state identifier." },
                                "trigger": { "type": "string", "description": "Optional legacy string event name (mutually exclusive with `event`)." },
                                "event": { "description": "Optional typed TransitionEvent object (mutually exclusive with non-empty `trigger`)." },
                                "guard": { "type": "string", "description": "Optional FEL guard expression." }
                            },
                            "required": ["project_id", "from", "to"]
                        }
                    },
                    {
                        "name": "wos_set_initial_state",
                        "description": "Set the lifecycle initial state for a registered project. The state must already exist. Returns {\"state_id\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "state_id": { "type": "string", "description": "Identifier of an existing state to set as initial." }
                            },
                            "required": ["project_id", "state_id"]
                        }
                    },
                    {
                        "name": "wos_remove_state",
                        "description": "Remove a state and all transitions referencing it. Returns {\"state_id\": \"...\", \"transitions_removed\": N}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "state_id": { "type": "string", "description": "Identifier of the state to remove." }
                            },
                            "required": ["project_id", "state_id"]
                        }
                    },
                    {
                        "name": "wos_add_actor",
                        "description": "Declare a human or system actor on a registered project. Returns {\"actor_id\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "actor_id": { "type": "string", "description": "Unique actor identifier." },
                                "kind": { "type": "string", "enum": ["human", "system"], "description": "Actor kind; defaults to human." }
                            },
                            "required": ["project_id", "actor_id"]
                        }
                    },
                    {
                        "name": "wos_add_actor_extension",
                        "description": "Attach an x-prefixed extension key to an existing actor (kernel §10.6). Returns {\"actor_id\": \"...\", \"key\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "actor_id": { "type": "string", "description": "Identifier of an existing actor." },
                                "key": { "type": "string", "description": "Extension key; must start with x-." },
                                "value": { "description": "JSON value to store under the extension key." }
                            },
                            "required": ["project_id", "actor_id", "key", "value"]
                        }
                    },
                    // ── Task 5: Governance + AI tools ─────────────────────────────────
                    {
                        "name": "wos_add_due_process_path",
                        "description": "Record a due-process path under x-wos-governance.dueProcessPaths. Returns {\"path_id\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id":  { "type": "string", "description": "UUID of the open project." },
                                "path_id":     { "type": "string", "description": "Unique path identifier." },
                                "description": { "type": "string", "description": "Human-readable description of the due-process path." },
                                "steps":       { "type": "array", "items": { "type": "string" }, "description": "Ordered list of step identifiers." }
                            },
                            "required": ["project_id", "path_id", "description"]
                        }
                    },
                    {
                        "name": "wos_add_assertion_gate",
                        "description": "Register an assertion gate in x-wos-governance.assertionGates. Returns {\"gate_id\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "gate_id":    { "type": "string", "description": "Unique gate identifier." },
                                "assertion":  { "type": "string", "description": "FEL expression that must hold." },
                                "transition": { "type": "string", "description": "Lifecycle transition event this gate guards." }
                            },
                            "required": ["project_id", "gate_id", "assertion", "transition"]
                        }
                    },
                    {
                        "name": "wos_set_impact_level",
                        "description": "Set the document-level impact classification (kernel §S6). Returns {\"level\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "level": { "type": "string", "enum": ["rights-impacting", "safety-impacting", "operational", "informational"], "description": "Impact level variant." }
                            },
                            "required": ["project_id", "level"]
                        }
                    },
                    {
                        "name": "wos_add_ai_agent",
                        "description": "Register an AI agent under x-wos-ai.agents. AI agents are NOT actors — they route through x-wos-ai. Returns {\"agent_id\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id":   { "type": "string", "description": "UUID of the open project." },
                                "agent_id":     { "type": "string", "description": "Unique agent identifier." },
                                "role":         { "type": "string", "description": "Role description." },
                                "model":        { "type": "string", "description": "Model identifier string." },
                                "capabilities": { "type": "array", "items": { "type": "string" }, "description": "Capability strings." }
                            },
                            "required": ["project_id", "agent_id", "role", "model"]
                        }
                    },
                    {
                        "name": "wos_add_deontic_constraint",
                        "description": "Append a structured deontic constraint under x-wos-ai.deonticConstraints. Returns {\"constraint_id\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id":    { "type": "string", "description": "UUID of the open project." },
                                "constraint_id": { "type": "string", "description": "Unique constraint identifier." },
                                "target":        { "type": "string", "description": "Actor or scope this constraint targets." },
                                "modality":      { "type": "string", "enum": ["must", "must_not", "may"], "description": "Deontic modality." },
                                "action":        { "type": "string", "description": "Action description." }
                            },
                            "required": ["project_id", "constraint_id", "target", "modality", "action"]
                        }
                    },
                    // ── Task 6: Validation + query tools ──────────────────────────────
                    {
                        "name": "wos_lint",
                        "description": "Export project and lint it. Returns {\"diagnostics\": [...], \"error_count\": N, \"warning_count\": N}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." }
                            },
                            "required": ["project_id"]
                        }
                    },
                    {
                        "name": "wos_run_conformance",
                        "description": "Run a conformance fixture and return the ConformanceTrace as JSON. Returns {\"passed\": bool, \"failures\": [...], \"trace\": {...}}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id":   { "type": "string", "description": "UUID of the open project (unused; conformance operates on fixture_json)." },
                                "fixture_json": { "type": "string", "description": "Inline conformance fixture JSON string." },
                                "base_dir":     { "type": "string", "description": "Base directory for resolving document paths in the fixture (defaults to '.')." }
                            },
                            "required": ["fixture_json"]
                        }
                    },
                    {
                        "name": "wos_preview_state_graph",
                        "description": "Construct a state graph string from the project's states and transitions. Returns {\"graph\": \"...\", \"format\": \"mermaid|dot\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "format":     { "type": "string", "enum": ["mermaid", "dot"], "description": "Output format; defaults to mermaid." }
                            },
                            "required": ["project_id"]
                        }
                    },
                    {
                        "name": "wos_search",
                        "description": "Linear substring search over states, transitions, actors, or deontic constraints. Returns {\"matches\": [...], \"kind\": \"...\", \"query\": \"...\"}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the open project." },
                                "kind":  { "type": "string", "enum": ["state", "transition", "actor", "constraint"], "description": "Entity kind to search." },
                                "query": { "type": "string", "description": "Substring to match against id and label fields." }
                            },
                            "required": ["project_id", "kind", "query"]
                        }
                    },
                    {
                        "name": "wos_list_projects",
                        "description": "List all open project UUIDs in the current session. Returns {\"projects\": [...], \"count\": N}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "wos_close_project",
                        "description": "Close (remove) an open project from the session registry. Returns {\"project_id\": \"...\", \"closed\": true}.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string", "description": "UUID of the project to close." }
                            },
                            "required": ["project_id"]
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
                Err(e @ wos_mcp::errors::DispatchError::ToolFailed { .. }) => JsonRpcResponse::ok(
                    id,
                    serde_json::json!({
                        "isError": true,
                        "content": [{"type": "text", "text": e.to_string()}]
                    }),
                ),
            }
        }

        other => JsonRpcResponse::err(id, METHOD_NOT_FOUND, format!("method not found: {other}")),
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

    // One registry per process. The stdio loop is single-threaded so a plain
    // mutable reference suffices — no locking needed.
    let mut registry = ProjectRegistry::new();

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

        let response: Option<JsonRpcResponse> =
            match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(req) => handle_request(&mut registry, req).await,
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
