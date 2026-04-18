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
use crate::tools;

/// Route a tool call to its handler by name.
///
/// This is the unified entry point used by both the stdio server and any
/// in-workspace Rust caller that has the tool name as a runtime string.
/// Callers that know the tool name statically (e.g. tests) may also call
/// the handler function directly via `wos_mcp::tools::<handler>`.
///
/// Returns `DispatchError::UnknownTool` for routing failures and
/// `DispatchError::ToolFailed` for errors raised by the handler itself.
/// The stdio transport uses this distinction to map the former to a
/// JSON-RPC error and the latter to an `isError: true` result.
pub async fn dispatch(
    tool_name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, DispatchError> {
    match tool_name {
        "wos_ping" => tools::ping(args)
            .await
            .map_err(|source| DispatchError::ToolFailed {
                tool: tool_name.to_string(),
                source,
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
        let result = dispatch("wos_ping", serde_json::json!({})).await.unwrap();
        assert_eq!(result, serde_json::json!({"pong": true}));
    }

    /// Verifies that an unknown tool name produces `DispatchError::UnknownTool`
    /// (routing failure), NOT `DispatchError::ToolFailed` (execution failure).
    /// The distinction matters — the transport maps them to different
    /// JSON-RPC response shapes.
    #[tokio::test]
    async fn dispatch_unknown_tool_errors() {
        let err = dispatch("wos_nonexistent", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DispatchError::UnknownTool(ref name) if name == "wos_nonexistent"),
            "expected DispatchError::UnknownTool, got: {err}"
        );
    }
}
