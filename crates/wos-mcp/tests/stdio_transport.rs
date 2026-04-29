//! Integration tests for the `wos-mcp` stdio binary.
//!
//! Each test spawns the compiled binary as a child process, drives a valid
//! JSON-RPC-2.0 sequence via stdin, and asserts that responses on stdout are
//! well-formed and contain expected fields.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Wall-clock budget for the entire subprocess round-trip. If the binary
/// panics, deadlocks, or writes no stdout, the test fails with a clear
/// message instead of hanging the suite.
const RUN_TIMEOUT: Duration = Duration::from_secs(5);

/// Build the binary path from the current CARGO_MANIFEST_DIR, falling back
/// to discovering it through `cargo build`.
fn binary_path() -> std::path::PathBuf {
    // When running under `cargo nextest run`, the binary has already been built.
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
///
/// Enforces a wall-clock timeout (`RUN_TIMEOUT`) and captures stderr so
/// that a panicking, deadlocked, or silently-exiting binary surfaces a
/// useful failure message rather than hanging the test suite.
fn run_sequence(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let binary = binary_path();
    assert!(
        binary.exists(),
        "wos-mcp binary not found at {binary:?}. Run `cargo build -p wos-mcp` first."
    );

    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn wos-mcp binary");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();

    // Write all requests, then close stdin to signal EOF.
    for req in requests {
        let line = serde_json::to_string(req).unwrap();
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin); // EOF → binary's read loop exits

    // Read stdout on a worker thread so the main thread can enforce a
    // wall-clock timeout against it.
    let (tx, rx) = mpsc::channel();
    let reader_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let lines: Vec<String> = reader
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .expect("failed to read stdout line");
        let _ = tx.send(lines);
    });

    let lines = match rx.recv_timeout(RUN_TIMEOUT) {
        Ok(lines) => lines,
        Err(_) => {
            // Grab whatever stderr is available to surface the root cause
            // (panic message, etc.) before we kill the child.
            let mut stderr_buf = String::new();
            let _ = stderr.read_to_string(&mut stderr_buf);
            let _ = child.kill();
            panic!(
                "wos-mcp stdio round-trip timed out after {:?}.\nstderr:\n{stderr_buf}",
                RUN_TIMEOUT
            );
        }
    };
    let _ = reader_thread.join();

    let status = child.wait().expect("failed to wait for child");

    // If the binary exited non-zero, surface captured stderr so panic
    // messages from the server reach the test output.
    if !status.success() {
        let mut stderr_buf = String::new();
        let _ = stderr.read_to_string(&mut stderr_buf);
        panic!("wos-mcp exited with {status}.\nstderr:\n{stderr_buf}");
    }

    lines
        .into_iter()
        .map(|line| serde_json::from_str(&line).expect("response is not valid JSON"))
        .collect()
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
    let tools = list["result"]["tools"]
        .as_array()
        .expect("tools must be an array");
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
    assert_eq!(
        pong["pong"], true,
        "wos_ping must return {{\"pong\": true}}"
    );
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

/// `tools/list` must advertise exactly 22 tools:
///   1 ping + 4 document + 4 lifecycle + 2 actor + 5 gov/AI + 6 validation/query.
/// Any regression in the list manifest (adding or removing a tool without
/// updating this count) is caught here.
#[test]
fn tools_list_advertises_twenty_two_tools() {
    let requests = vec![serde_json::json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "tools/list",
        "params": {}
    })];

    let responses = run_sequence(&requests);
    assert_eq!(responses.len(), 1);

    let tools = responses[0]["result"]["tools"]
        .as_array()
        .expect("tools must be an array");

    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    assert_eq!(
        names.len(),
        22,
        "tools/list must advertise exactly 22 tools; got {}: {names:?}",
        names.len()
    );

    // Verify all Task-5 governance/AI tools are present.
    for expected in [
        "wos_add_due_process_path",
        "wos_add_assertion_gate",
        "wos_set_impact_level",
        "wos_add_ai_agent",
        "wos_add_deontic_constraint",
    ] {
        assert!(
            names.contains(&expected),
            "tools/list must include '{expected}'; got: {names:?}"
        );
    }

    // Verify all Task-6 validation/query tools are present.
    for expected in [
        "wos_lint",
        "wos_run_conformance",
        "wos_preview_state_graph",
        "wos_search",
        "wos_list_projects",
        "wos_close_project",
    ] {
        assert!(
            names.contains(&expected),
            "tools/list must include '{expected}'; got: {names:?}"
        );
    }
}

/// Verify that every tool name in `tools/list` has a registered dispatch handler.
///
/// This test calls every tool by name with deliberately invalid arguments (an
/// empty object). The expected outcome is either a successful result OR a
/// `ToolFailed` error — but never `UnknownTool` (which maps to INVALID_PARAMS
/// at the JSON-RPC layer). An INVALID_PARAMS error for a tool that was
/// advertised in `tools/list` is a catalog-dispatch mismatch.
#[test]
fn every_advertised_tool_has_a_dispatch_handler() {
    // First collect all tool names from tools/list.
    let list_requests = vec![serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    })];
    let list_responses = run_sequence(&list_requests);
    let tools = list_responses[0]["result"]["tools"]
        .as_array()
        .expect("tools must be an array");

    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect();

    assert!(
        !names.is_empty(),
        "tools/list must return at least one tool"
    );

    // Call each tool with an empty arguments object.
    //
    // Acceptable outcomes:
    //   - A successful result (`result` field present, no `error` field).
    //   - A tool-execution failure indicated by `isError: true` inside `result`
    //     (e.g., missing required arguments). These are application-level errors
    //     returned through the normal result channel, NOT JSON-RPC protocol errors.
    //
    // Unacceptable outcome:
    //   - A JSON-RPC protocol error (`error` field present). That would mean the
    //     server returned INVALID_PARAMS / UnknownTool for a name that was just
    //     advertised — a catalog–dispatch mismatch. `isError: true` in the result
    //     is fine; a top-level `error` field is not.
    let call_requests: Vec<serde_json::Value> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": i + 100,
                "method": "tools/call",
                "params": {
                    "name": name,
                    "arguments": {}
                }
            })
        })
        .collect();

    let call_responses = run_sequence(&call_requests);
    assert_eq!(call_responses.len(), names.len());

    for (name, response) in names.iter().zip(call_responses.iter()) {
        // A JSON-RPC level error (error field present) for a tool that exists
        // would mean UnknownTool: catalog–dispatch mismatch.
        assert!(
            response["error"].is_null(),
            "tool '{name}' is advertised in tools/list but returns a JSON-RPC error \
             (UnknownTool / catalog–dispatch mismatch): {}",
            response["error"]
        );
    }
}
