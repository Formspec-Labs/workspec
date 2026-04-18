//! Typed error hierarchy for `wos-mcp`.
//!
//! `ToolError` — per-handler errors returned by individual tool functions.
//! `DispatchError` — classifies a failed dispatch call into:
//!   * `UnknownTool` — the tool name does not exist (routing failure).
//!   * `ToolFailed` — a known tool executed and returned an error.
//!
//! The distinction matters at the JSON-RPC boundary. Per the MCP spec an
//! unknown tool is a JSON-RPC error (`-32602 INVALID_PARAMS`), while a
//! tool-execution failure is returned as a *successful* JSON-RPC response
//! with `result: { isError: true, content: [...] }`. `server.rs` relies on
//! these variants to pick the right shape.
//!
//! Transport-level errors (stdin/stdout I/O, JSON parse) are handled
//! inline in `server.rs` — the previous `ServerError` wrapper had no
//! consumers and added a type without adding information.

use thiserror::Error;

/// Error returned by a single tool handler.
#[derive(Debug, Error)]
pub enum ToolError {
    /// The tool received arguments it does not accept.
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    /// A required argument field was missing.
    #[error("missing required argument: {0}")]
    MissingArgument(String),

    /// An internal logic error occurred inside the tool.
    #[error("tool internal error: {0}")]
    Internal(String),
}

/// Error returned by `dispatch::dispatch`.
///
/// Two cases, deliberately separate so the transport can choose between
/// "JSON-RPC error response" and "JSON-RPC success with isError=true":
#[derive(Debug, Error)]
pub enum DispatchError {
    /// No handler is registered for the supplied tool name. Routing failure.
    #[error("unknown tool: {0}")]
    UnknownTool(String),

    /// The tool was found and ran, but returned an error.
    #[error("tool '{tool}' failed: {source}")]
    ToolFailed {
        tool: String,
        #[source]
        source: ToolError,
    },
}

