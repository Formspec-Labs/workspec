//! `wos_ping` — health-check tool. Returns `{"pong": true}`.
//!
//! Used in scaffold tests and transport smoke tests to verify the
//! JSON-RPC-2.0 loop and in-process dispatch both work end-to-end.

use crate::errors::ToolError;

/// Health-check tool. Accepts any arguments, always returns `{"pong": true}`.
pub async fn ping(_args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
    Ok(serde_json::json!({"pong": true}))
}
