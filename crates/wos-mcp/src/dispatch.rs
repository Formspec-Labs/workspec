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
use crate::registry::ProjectRegistry;
use crate::tools;

/// Route a tool call to its handler by name.
///
/// This is the unified entry point used by both the stdio server and any
/// in-workspace Rust caller that has the tool name as a runtime string.
/// Callers that know the tool name statically (e.g. tests) may also call
/// the handler function directly via `wos_mcp::tools::<handler>`.
///
/// Handler signature convention per the plan:
/// `(&mut ProjectRegistry, &str, Value) -> Result<Value, ToolError>`.
///
/// `project_id` is extracted from `args` by the server before calling
/// dispatch, and re-passed here so project-scoped handlers (wos_export,
/// wos_describe) can locate their project. Project-less handlers
/// (wos_ping, wos_create_kernel, wos_load_document) receive an empty
/// string and ignore it.
///
/// Returns `DispatchError::UnknownTool` for routing failures and
/// `DispatchError::ToolFailed` for errors raised by the handler itself.
/// The stdio transport uses this distinction to map the former to a
/// JSON-RPC error and the latter to an `isError: true` result.
pub async fn dispatch(
    registry: &mut ProjectRegistry,
    tool_name: &str,
    project_id: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, DispatchError> {
    match tool_name {
        "wos_ping" => tools::ping(registry, project_id, args).await.map_err(|source| {
            DispatchError::ToolFailed {
                tool: tool_name.to_string(),
                source,
            }
        }),
        "wos_create_kernel" => {
            tools::wos_create_kernel(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_load_document" => {
            tools::wos_load_document(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_export_document" => {
            tools::wos_export_document(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_describe_document" => {
            tools::wos_describe_document(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_add_state" => {
            tools::wos_add_state(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_add_transition" => {
            tools::wos_add_transition(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_set_initial_state" => {
            tools::wos_set_initial_state(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_remove_state" => {
            tools::wos_remove_state(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_add_actor" => {
            tools::wos_add_actor(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
        "wos_add_actor_extension" => {
            tools::wos_add_actor_extension(registry, project_id, args)
                .await
                .map_err(|source| DispatchError::ToolFailed {
                    tool: tool_name.to_string(),
                    source,
                })
        }
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
        let mut registry = ProjectRegistry::new();
        let result = dispatch(&mut registry, "wos_ping", "", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!({"pong": true}));
    }

    /// Verifies that an unknown tool name produces `DispatchError::UnknownTool`
    /// (routing failure), NOT `DispatchError::ToolFailed` (execution failure).
    /// The distinction matters — the transport maps them to different
    /// JSON-RPC response shapes.
    #[tokio::test]
    async fn dispatch_unknown_tool_errors() {
        let mut registry = ProjectRegistry::new();
        let err = dispatch(&mut registry, "wos_nonexistent", "", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DispatchError::UnknownTool(ref name) if name == "wos_nonexistent"),
            "expected DispatchError::UnknownTool, got: {err}"
        );
    }

    /// Handler signature parity with the plan: `wos_ping` accepts
    /// `(&mut ProjectRegistry, &str, Value)` and ignores the first two args.
    #[tokio::test]
    async fn wos_ping_handler_accepts_registry_and_project_id() {
        let mut registry = ProjectRegistry::new();
        let result = tools::ping(&mut registry, "ignored-id", serde_json::json!({"anything": 1}))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!({"pong": true}));
    }

    /// create → export round-trip through the dispatch function.
    #[tokio::test]
    async fn dispatch_create_and_export_round_trip() {
        let mut registry = ProjectRegistry::new();

        // Create a new project.
        let create = dispatch(&mut registry, "wos_create_kernel", "", serde_json::json!({}))
            .await
            .unwrap();
        let pid = create["project_id"].as_str().unwrap().to_string();

        // Export it back through dispatch.
        let export = dispatch(
            &mut registry,
            "wos_export_document",
            &pid,
            serde_json::json!({ "project_id": &pid }),
        )
        .await
        .unwrap();

        let doc_json = export["document"].as_str().expect("must have document key");
        let parsed: serde_json::Value = serde_json::from_str(doc_json).unwrap();
        assert_eq!(parsed["$wosKernel"], serde_json::json!("1.0"));
    }

    /// describe through dispatch returns the expected shape.
    #[tokio::test]
    async fn dispatch_describe_document_returns_shape() {
        let mut registry = ProjectRegistry::new();
        let create = dispatch(&mut registry, "wos_create_kernel", "", serde_json::json!({}))
            .await
            .unwrap();
        let pid = create["project_id"].as_str().unwrap().to_string();

        let desc = dispatch(
            &mut registry,
            "wos_describe_document",
            &pid,
            serde_json::json!({ "project_id": &pid }),
        )
        .await
        .unwrap();

        assert!(desc["state_count"].is_number());
        assert!(desc["transition_count"].is_number());
        assert!(desc["actor_count"].is_number());
        assert!(desc["impact_level"].is_string());
        assert!(desc["ai_agent_count"].is_number());
    }

    /// Plan §4.6 round-trip: create → add 3 states → add 2 transitions →
    /// set initial state → export → parse → assert 3 states and 2 transitions.
    #[tokio::test]
    async fn lifecycle_round_trip() {
        let mut registry = ProjectRegistry::new();

        // Step 1: create kernel.
        let create = dispatch(&mut registry, "wos_create_kernel", "", serde_json::json!({}))
            .await
            .unwrap();
        let pid = create["project_id"].as_str().unwrap().to_string();

        // Step 2: add three states.
        for state_id in ["draft", "review", "approved"] {
            dispatch(
                &mut registry,
                "wos_add_state",
                &pid,
                serde_json::json!({ "project_id": &pid, "state_id": state_id }),
            )
            .await
            .unwrap();
        }

        // Step 3: add two transitions.
        dispatch(
            &mut registry,
            "wos_add_transition",
            &pid,
            serde_json::json!({
                "project_id": &pid,
                "from": "draft",
                "to": "review",
                "trigger": "submit"
            }),
        )
        .await
        .unwrap();

        dispatch(
            &mut registry,
            "wos_add_transition",
            &pid,
            serde_json::json!({
                "project_id": &pid,
                "from": "review",
                "to": "approved",
                "trigger": "approve"
            }),
        )
        .await
        .unwrap();

        // Step 4: set initial state.
        dispatch(
            &mut registry,
            "wos_set_initial_state",
            &pid,
            serde_json::json!({ "project_id": &pid, "state_id": "draft" }),
        )
        .await
        .unwrap();

        // Step 5: export and parse.
        let export = dispatch(
            &mut registry,
            "wos_export_document",
            &pid,
            serde_json::json!({ "project_id": &pid }),
        )
        .await
        .unwrap();
        let doc_json = export["document"].as_str().unwrap();
        let doc: serde_json::Value = serde_json::from_str(doc_json).unwrap();

        // Step 6: assert three states.
        let states = doc["lifecycle"]["states"].as_object().unwrap();
        assert_eq!(states.len(), 3, "expected 3 states");
        assert!(states.contains_key("draft"));
        assert!(states.contains_key("review"));
        assert!(states.contains_key("approved"));

        // Assert two transitions (one per source state that has them).
        let draft_transitions = doc["lifecycle"]["states"]["draft"]["transitions"]
            .as_array()
            .unwrap();
        let review_transitions = doc["lifecycle"]["states"]["review"]["transitions"]
            .as_array()
            .unwrap();
        assert_eq!(draft_transitions.len(), 1);
        assert_eq!(review_transitions.len(), 1);

        // Assert initial state is set.
        assert_eq!(doc["lifecycle"]["initialState"], "draft");
    }
}
