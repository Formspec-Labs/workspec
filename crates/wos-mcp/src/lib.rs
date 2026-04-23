//! `wos-mcp` — MCP adapter exposing WOS authoring operations as JSON-RPC-2.0 tools.
//!
//! Two entry points, one set of handlers:
//! 1. **Stdio binary** (`wos-mcp`): wraps handlers in JSON-RPC-2.0 over stdin/stdout.
//! 2. **Library function** (`wos_mcp::dispatch::dispatch`): called directly by
//!    in-workspace Rust crates with no protocol overhead.

pub mod dispatch;
pub mod errors;
pub mod registry;
pub mod tools;
