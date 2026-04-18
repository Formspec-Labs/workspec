//! Integration tests for the `wos-mcp` stdio binary.
//!
//! Each test spawns the compiled binary as a child process, drives a valid
//! JSON-RPC-2.0 sequence via stdin, and asserts that responses on stdout are
//! well-formed and contain expected fields.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Build the binary path from the current CARGO_MANIFEST_DIR, falling back
/// to discovering it through `cargo build`.
fn binary_path() -> std::path::PathBuf {
    // When running under `cargo test`, the binary has already been built.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut path = std::path::PathBuf::from(manifest_dir);
    // Walk up to workspace root, then into the target directory.
    path.pop(); // crates/wos-mcp -> crates
    path.pop(); // crates -> workspace root
    path.push("target");
    path.push("debug");
    path.push("wos-mcp");
    path
}

/// Send a sequence of newline-delimited JSON requests to the binary and
/// collect the corresponding responses.
fn run_sequence(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let binary = binary_path();

    // Build the binary if it doesn't exist yet (e.g. `cargo test --no-build`
    // is not used, so this should already exist; but be defensive).
    assert!(
        binary.exists(),
        "wos-mcp binary not found at {binary:?}. Run `cargo build -p wos-mcp` first."
    );

    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn wos-mcp binary");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    // Write all requests, then close stdin to signal EOF.
    for req in requests {
        let line = serde_json::to_string(req).unwrap();
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin); // EOF → binary's read loop exits

    let reader = BufReader::new(stdout);
    let responses: Vec<serde_json::Value> = reader
        .lines()
        .map(|l| {
            let line = l.expect("failed to read stdout line");
            serde_json::from_str(&line).expect("response is not valid JSON")
        })
        .collect();

    let _status = child.wait().expect("failed to wait for child");

    responses
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Full MCP session: initialize → tools/list → tools/call wos_ping.
#[test]
fn full_mcp_session() {
    let requests = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {"name": "test-client", "version": "0.0.1"},
                "capabilities": {}
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "wos_ping",
                "arguments": {}
            }
        }),
    ];

    let responses = run_sequence(&requests);
    assert_eq!(responses.len(), 3, "expected 3 responses");

    // initialize response
    let init = &responses[0];
    assert_eq!(init["jsonrpc"], "2.0");
    assert_eq!(init["id"], 1);
    assert!(init["result"]["serverInfo"]["name"].as_str().is_some());
    assert_eq!(init["result"]["serverInfo"]["name"], "wos-mcp");

    // tools/list response
    let list = &responses[1];
    assert_eq!(list["jsonrpc"], "2.0");
    assert_eq!(list["id"], 2);
    let tools = list["result"]["tools"].as_array().expect("tools must be an array");
    assert!(
        tools.iter().any(|t| t["name"] == "wos_ping"),
        "wos_ping must appear in tools/list"
    );

    // tools/call wos_ping response
    let call = &responses[2];
    assert_eq!(call["jsonrpc"], "2.0");
    assert_eq!(call["id"], 3);
    assert!(call["error"].is_null(), "wos_ping must not return an error");
    // The content is a JSON-encoded string; check it contains "pong"
    let content_text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("content[0].text must be a string");
    let pong: serde_json::Value =
        serde_json::from_str(content_text).expect("content text must be valid JSON");
    assert_eq!(pong["pong"], true, "wos_ping must return {{\"pong\": true}}");
}

/// JSON-RPC-2.0 §4.1 and MCP both forbid responses to notifications.
/// A notification is a request WITHOUT an `id` field (not `id: null`).
/// `notifications/initialized` is the canonical MCP post-handshake notification.
#[test]
fn notification_produces_no_response() {
    let requests = vec![
        // Notification — absence of `id` means no response expected.
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        // Follow-up request WITH `id` — this must produce a response, and
        // the response for it must be the ONLY line on stdout.
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/list",
            "params": {}
        }),
    ];

    let responses = run_sequence(&requests);
    assert_eq!(
        responses.len(),
        1,
        "notification must produce zero responses; only the tools/list request should reply"
    );
    assert_eq!(responses[0]["id"], 42);
}

/// Unknown method returns a JSON-RPC method-not-found error.
#[test]
fn unknown_method_returns_error() {
    let requests = vec![serde_json::json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "totally/unknown",
        "params": {}
    })];

    let responses = run_sequence(&requests);
    assert_eq!(responses.len(), 1);

    let resp = &responses[0];
    assert_eq!(resp["id"], 99);
    assert!(
        resp["error"]["code"].as_i64().is_some(),
        "must have an error code"
    );
    assert_eq!(resp["error"]["code"], -32601); // METHOD_NOT_FOUND
}

/// Calling `tools/call` with an unknown tool name is a routing failure:
/// per MCP, this is a JSON-RPC error with `-32602 INVALID_PARAMS` (the
/// `name` parameter references a tool that does not exist).
#[test]
fn unknown_tool_name_returns_invalid_params_error() {
    let requests = vec![serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "wos_does_not_exist",
            "arguments": {}
        }
    })];

    let responses = run_sequence(&requests);
    assert_eq!(responses.len(), 1);

    let resp = &responses[0];
    assert_eq!(resp["id"], 10);
    assert_eq!(
        resp["error"]["code"], -32602,
        "unknown tool must map to INVALID_PARAMS, not INTERNAL_ERROR"
    );
}
