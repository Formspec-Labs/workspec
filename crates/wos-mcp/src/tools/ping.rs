//! `wos_ping` — health-check tool. Returns `{"pong": true}`.
//!
//! Used in scaffold tests and transport smoke tests to verify the
//! JSON-RPC-2.0 loop and in-process dispatch both work end-to-end.

use crate::errors::ToolError;
use crate::registry::ProjectRegistry;

/// Health-check tool. Accepts any arguments, always returns `{"pong": true}`.
///
/// The `registry` and `project_id` arguments are part of the shared
/// handler signature from the plan; `wos_ping` is a project-less tool
/// and ignores them. Keeping the signature aligned now means Task 3
/// handlers plug in without reshaping call sites.
pub async fn ping(
    _registry: &ProjectRegistry,
    _project_id: &str,
    _args: serde_json::Value,
) -> Result<serde_json::Value, ToolError> {
    Ok(serde_json::json!({"pong": true}))
}
