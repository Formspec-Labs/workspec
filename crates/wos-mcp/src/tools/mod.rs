//! Tool handler modules. Each module exposes one or more `async fn` tool handlers.
//!
//! Tool handlers are pure async functions: they accept a `serde_json::Value`
//! argument bag and return `Result<serde_json::Value, ToolError>`. They contain
//! no MCP protocol awareness — that lives in `server.rs` and `dispatch.rs`.

mod ping;

pub use ping::ping;
