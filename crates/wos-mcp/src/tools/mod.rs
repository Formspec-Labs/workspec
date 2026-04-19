//! Tool handler modules. Each module exposes one or more `async fn` tool handlers.
//!
//! Tool handlers are pure async functions: they accept a `serde_json::Value`
//! argument bag and return `Result<serde_json::Value, ToolError>`. They contain
//! no MCP protocol awareness — that lives in `server.rs` and `dispatch.rs`.

mod actors;
mod document;
mod governance;
mod lifecycle;
mod ping;
mod query;

pub use actors::{wos_add_actor, wos_add_actor_extension};
pub use document::{
    wos_create_kernel, wos_describe_document, wos_export_document, wos_load_document,
};
pub use governance::{
    wos_add_ai_agent, wos_add_assertion_gate, wos_add_deontic_constraint,
    wos_add_due_process_path, wos_set_impact_level,
};
pub use lifecycle::{wos_add_state, wos_add_transition, wos_remove_state, wos_set_initial_state};
pub use ping::ping;
pub use query::{
    wos_close_project, wos_list_projects, wos_lint, wos_preview_state_graph, wos_run_conformance,
    wos_search,
};
