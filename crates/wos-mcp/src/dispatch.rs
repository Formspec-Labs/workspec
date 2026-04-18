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

use crate::errors::{DispatchError, ToolError};
use crate::tools;

/// Route a tool call to its handler by name.
///
/// This is the unified entry point used by both the stdio server and any
/// in-workspace Rust caller that has the tool name as a runtime string.
/// Callers that know the tool name statically (e.g. tests) may also call
/// the handler function directly via `wos_mcp::tools::<handler>`.
pub async fn dispatch(
    tool_name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, DispatchError> {
    match tool_name {
        "wos_ping" => tools::ping(args).await,
        other => Err(ToolError::UnknownTool(other.to_string())),
    }
    .map_err(|source| DispatchError::ToolFailed {
        tool: tool_name.to_string(),
        source,
    })
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

    /// Verifies that an unknown tool name produces a typed `DispatchError`.
    #[tokio::test]
    async fn dispatch_unknown_tool_errors() {
        let err = dispatch("wos_nonexistent", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                DispatchError::ToolFailed {
                    ref source,
                    ..
                } if matches!(source, crate::errors::ToolError::UnknownTool(_))
            ),
            "expected UnknownTool error, got: {err}"
        );
    }
}
