//! Tool handler modules. Each module exposes one or more `async fn` tool handlers.
//!
//! Tool handlers are pure async functions: they accept a `serde_json::Value`
//! argument bag and return `Result<serde_json::Value, ToolError>`. They contain
//! no MCP protocol awareness — that lives in `server.rs` and `dispatch.rs`.

mod document;
mod ping;

pub use document::{
    wos_create_kernel, wos_describe_document, wos_export_document, wos_load_document,
};
pub use ping::ping;
