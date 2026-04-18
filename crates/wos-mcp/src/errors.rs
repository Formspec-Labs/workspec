//! Typed error hierarchy for `wos-mcp`.
//!
//! `ToolError` — per-handler errors returned by individual tool functions.
//! `DispatchError` — wraps `ToolError` with the tool name for the dispatch layer.
//! `ServerError` — transport-level errors for the stdio JSON-RPC loop.

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

    /// The tool was called with an unknown tool name.
    #[error("unknown tool: {0}")]
    UnknownTool(String),

    /// An internal logic error occurred inside the tool.
    #[error("tool internal error: {0}")]
    Internal(String),
}

/// Error returned by `dispatch::dispatch` — wraps `ToolError` with the tool name.
#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("tool '{tool}' failed: {source}")]
    ToolFailed {
        tool: String,
        #[source]
        source: ToolError,
    },
}

/// Transport-level errors for the JSON-RPC-2.0 stdio loop.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("dispatch error: {0}")]
    Dispatch(#[from] DispatchError),
}
