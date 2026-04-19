//! Lifecycle tool handlers: add/remove states, transitions, and initial state.
//!
//! Each handler is a thin wrapper over the corresponding `WosProject` method.
//! Argument unpacking and result packing live here; all document logic lives
//! in `wos-authoring`.

use serde_json::Value;
use wos_authoring::StateKind;

use crate::errors::ToolError;
use crate::registry::ProjectRegistry;

// ── wos_add_state ─────────────────────────────────────────────────────────────

/// Add a top-level state to a registered project.
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "state_id":   "...",
///   "kind":       "atomic|compound|parallel|final",  // optional, default "atomic"
///   "label":      "...",                             // optional description
///   "metadata":   {...}                              // optional x-meta extension value
/// }
/// ```
/// Returns `{"state_id": "<state_id>"}` on success.
pub async fn wos_add_state(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let state_id = require_string_arg(&args, "state_id")?;
    let kind = parse_state_kind(&args)?;
    let label = args.get("label").and_then(Value::as_str).map(str::to_string);
    let metadata = args.get("metadata").cloned();

    let project = registry.get_mut(pid)?;
    project
        .add_state_described(state_id, kind, label, metadata)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "state_id": state_id }))
}

// ── wos_add_transition ────────────────────────────────────────────────────────

/// Add a transition between two existing states.
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "from":       "...",
///   "to":         "...",
///   "trigger":    "...",  // optional event name
///   "guard":      "..."   // optional guard FEL expression
/// }
/// ```
/// Returns `{"from": "...", "to": "...", "trigger": "..."}` on success.
pub async fn wos_add_transition(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let from = require_string_arg(&args, "from")?;
    let to = require_string_arg(&args, "to")?;
    let trigger = args.get("trigger").and_then(Value::as_str).map(str::to_string);
    let guard = args.get("guard").and_then(Value::as_str).map(str::to_string);

    let project = registry.get_mut(pid)?;
    project
        .add_transition(from, to, trigger.clone(), guard)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({
        "from": from,
        "to": to,
        "trigger": trigger.unwrap_or_default(),
    }))
}

// ── wos_set_initial_state ─────────────────────────────────────────────────────

/// Set the initial state of the lifecycle.
///
/// The state must already exist. Errors if it does not.
///
/// Args: `{"project_id": "...", "state_id": "..."}`.
/// Returns `{"state_id": "..."}` on success.
pub async fn wos_set_initial_state(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let state_id = require_string_arg(&args, "state_id")?;

    let project = registry.get_mut(pid)?;
    project
        .set_initial_state(state_id)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "state_id": state_id }))
}

// ── wos_remove_state ──────────────────────────────────────────────────────────

/// Remove a state and all transitions that reference it.
///
/// Inbound transitions from other states that target the removed state are
/// pruned automatically. The response includes the count of transitions removed.
///
/// Args: `{"project_id": "...", "state_id": "..."}`.
/// Returns `{"state_id": "...", "transitions_removed": N}`.
pub async fn wos_remove_state(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let state_id = require_string_arg(&args, "state_id")?;

    // Count transitions before removal so we can report the delta.
    let transitions_before = count_all_transitions(registry, pid)?;

    let project = registry.get_mut(pid)?;
    project
        .remove_state(state_id)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    let transitions_after = count_all_transitions(registry, pid)?;
    let transitions_removed = transitions_before.saturating_sub(transitions_after);

    Ok(serde_json::json!({
        "state_id": state_id,
        "transitions_removed": transitions_removed,
    }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a required string argument from the args object.
fn require_string_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::MissingArgument(key.to_string()))
}

/// Parse the optional `kind` argument to `StateKind`, defaulting to `Atomic`.
fn parse_state_kind(args: &Value) -> Result<StateKind, ToolError> {
    match args.get("kind").and_then(Value::as_str) {
        None | Some("atomic") => Ok(StateKind::Atomic),
        Some("compound") => Ok(StateKind::Compound),
        Some("parallel") => Ok(StateKind::Parallel),
        Some("final") => Ok(StateKind::Final),
        Some(other) => Err(ToolError::InvalidArguments(format!(
            "unknown state kind '{other}'; expected atomic|compound|parallel|final"
        ))),
    }
}

/// Sum all transition counts across all states in the project.
fn count_all_transitions(registry: &ProjectRegistry, pid: &str) -> Result<usize, ToolError> {
    let project = registry.get(pid)?;
    let doc = project.snapshot();
    Ok(doc
        .lifecycle
        .states
        .values()
        .map(|s| s.transitions.len())
        .sum())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── wos_add_state ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn add_state_inserts_state() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        let result = wos_add_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "draft" }),
        )
        .await
        .unwrap();

        assert_eq!(result["state_id"], json!("draft"));
        let project = registry.get(&pid).unwrap();
        assert!(project.snapshot().lifecycle.states.contains_key("draft"));
    }

    #[tokio::test]
    async fn add_state_with_label_and_metadata() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        wos_add_state(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "state_id": "review",
                "label": "Under Review",
                "metadata": { "sla_days": 5 }
            }),
        )
        .await
        .unwrap();

        let project = registry.get(&pid).unwrap();
        let doc = project.snapshot();
        let state = &doc.lifecycle.states["review"];
        assert_eq!(state.description.as_deref(), Some("Under Review"));
        assert_eq!(state.extensions["x-meta"]["sla_days"], json!(5));
    }

    #[tokio::test]
    async fn add_state_duplicate_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        wos_add_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "s1" }),
        )
        .await
        .unwrap();

        let err = wos_add_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "s1" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn add_state_invalid_kind_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        let err = wos_add_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "s1", "kind": "agent" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_add_transition ────────────────────────────────────────────────

    #[tokio::test]
    async fn add_transition_links_two_states() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        add_state(&mut registry, &pid, "draft").await;
        add_state(&mut registry, &pid, "approved").await;

        let result = wos_add_transition(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "from": "draft",
                "to": "approved",
                "trigger": "approve"
            }),
        )
        .await
        .unwrap();

        assert_eq!(result["from"], json!("draft"));
        assert_eq!(result["to"], json!("approved"));

        let project = registry.get(&pid).unwrap();
        let transitions = &project.snapshot().lifecycle.states["draft"].transitions;
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].target, "approved");
        assert_eq!(transitions[0].event, "approve");
    }

    #[tokio::test]
    async fn add_transition_unknown_state_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        add_state(&mut registry, &pid, "draft").await;

        let err = wos_add_transition(
            &mut registry,
            "",
            json!({ "project_id": pid, "from": "draft", "to": "ghost" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_set_initial_state ─────────────────────────────────────────────

    #[tokio::test]
    async fn set_initial_state_updates_document() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;
        add_state(&mut registry, &pid, "draft").await;

        let result = wos_set_initial_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "draft" }),
        )
        .await
        .unwrap();

        assert_eq!(result["state_id"], json!("draft"));
        let project = registry.get(&pid).unwrap();
        assert_eq!(project.snapshot().lifecycle.initial_state, "draft");
    }

    #[tokio::test]
    async fn set_initial_state_nonexistent_state_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        let err = wos_set_initial_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "ghost" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_remove_state ──────────────────────────────────────────────────

    #[tokio::test]
    async fn remove_state_returns_transition_count() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;
        add_state(&mut registry, &pid, "a").await;
        add_state(&mut registry, &pid, "b").await;
        add_state(&mut registry, &pid, "c").await;

        // Two transitions target "b": a→b and c→b.
        add_transition(&mut registry, &pid, "a", "b", "go").await;
        add_transition(&mut registry, &pid, "c", "b", "go").await;

        let result = wos_remove_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "b" }),
        )
        .await
        .unwrap();

        assert_eq!(result["state_id"], json!("b"));
        // Both inbound transitions (a→b, c→b) should be counted as removed.
        assert_eq!(result["transitions_removed"], json!(2));

        let project = registry.get(&pid).unwrap();
        let doc = project.snapshot();
        assert!(!doc.lifecycle.states.contains_key("b"));
        // Inbound transitions pruned from a and c.
        assert!(doc.lifecycle.states["a"].transitions.is_empty());
        assert!(doc.lifecycle.states["c"].transitions.is_empty());
    }

    #[tokio::test]
    async fn remove_state_nonexistent_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry).await;

        let err = wos_remove_state(
            &mut registry,
            "",
            json!({ "project_id": pid, "state_id": "ghost" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── Test helpers ──────────────────────────────────────────────────────

    async fn create_project(registry: &mut ProjectRegistry) -> String {
        let project = wos_authoring::WosProject::new_kernel();
        registry.insert(project).unwrap().to_string()
    }

    async fn add_state(registry: &mut ProjectRegistry, pid: &str, state_id: &str) {
        wos_add_state(
            registry,
            "",
            json!({ "project_id": pid, "state_id": state_id }),
        )
        .await
        .unwrap();
    }

    async fn add_transition(
        registry: &mut ProjectRegistry,
        pid: &str,
        from: &str,
        to: &str,
        trigger: &str,
    ) {
        wos_add_transition(
            registry,
            "",
            json!({
                "project_id": pid,
                "from": from,
                "to": to,
                "trigger": trigger
            }),
        )
        .await
        .unwrap();
    }
}
