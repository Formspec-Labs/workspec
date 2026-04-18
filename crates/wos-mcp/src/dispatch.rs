//! In-process tool dispatch — the library entry point for in-workspace Rust callers.
//!
//! `wos-synth-core`, `wos-bench`, and tests call `dispatch(tool_name, args).await`
//! directly as a plain library function. No subprocess spawn, no JSON-RPC overhead,
//! no sockets. The same handler functions used here also power the stdio binary in
//! `server.rs`.
//!
//! # Adding a new tool
//! 1. Implement the handler in `src/tools/<module>.rs`.
//! 2. `pub use` it from `src/tools/mod.rs`.
//! 3. Add a match arm here.

use crate::errors::DispatchError;
use crate::registry::ProjectRegistry;
use crate::tools;

/// Route a tool call to its handler by name.
///
/// This is the unified entry point used by both the stdio server and any
/// in-workspace Rust caller that has the tool name as a runtime string.
/// Callers that know the tool name statically (e.g. tests) may also call
/// the handler function directly via `wos_mcp::tools::<handler>`.
///
/// Handler signature matches
/// `thoughts/plans/2026-04-17-wos-mcp-crate.md`:
/// `(&ProjectRegistry, &str, Value) -> Result<Value, ToolError>`.
/// Most handlers extract a project from `registry` by `project_id`;
/// project-less tools (e.g. `wos_ping`, `wos_list_projects`) ignore it.
///
/// Returns `DispatchError::UnknownTool` for routing failures and
/// `DispatchError::ToolFailed` for errors raised by the handler itself.
/// The stdio transport uses this distinction to map the former to a
/// JSON-RPC error and the latter to an `isError: true` result.
pub async fn dispatch(
    registry: &ProjectRegistry,
    tool_name: &str,
    project_id: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, DispatchError> {
    match tool_name {
        "wos_ping" => tools::ping(registry, project_id, args).await.map_err(|source| {
            DispatchError::ToolFailed {
                tool: tool_name.to_string(),
                source,
            }
        }),
        other => Err(DispatchError::UnknownTool(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that calling `dispatch` with `"wos_ping"` returns `{"pong": true}`.
    /// This is a library call — no subprocess, no JSON-RPC round-trip.
    #[tokio::test]
    async fn dispatch_wos_ping_returns_pong() {
        let registry = ProjectRegistry::new();
        let result = dispatch(&registry, "wos_ping", "", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!({"pong": true}));
    }

    /// Verifies that an unknown tool name produces `DispatchError::UnknownTool`
    /// (routing failure), NOT `DispatchError::ToolFailed` (execution failure).
    /// The distinction matters — the transport maps them to different
    /// JSON-RPC response shapes.
    #[tokio::test]
    async fn dispatch_unknown_tool_errors() {
        let registry = ProjectRegistry::new();
        let err = dispatch(&registry, "wos_nonexistent", "", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DispatchError::UnknownTool(ref name) if name == "wos_nonexistent"),
            "expected DispatchError::UnknownTool, got: {err}"
        );
    }

    /// Handler signature parity with the plan: `wos_ping` accepts
    /// `(&ProjectRegistry, &str, Value)` and ignores the first two args.
    #[tokio::test]
    async fn wos_ping_handler_accepts_registry_and_project_id() {
        let registry = ProjectRegistry::new();
        let result = tools::ping(&registry, "ignored-id", serde_json::json!({"anything": 1}))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!({"pong": true}));
    }
}
