//! Validation and query tool handlers (Task 6).
//!
//! - `wos_lint`              — export project, lint, return diagnostics summary.
//! - `wos_run_conformance`   — run a conformance fixture, return ConformanceTrace JSON.
//! - `wos_preview_state_graph` — generate Mermaid or DOT state graph string.
//! - `wos_search`            — linear substring search over states/transitions/actors/constraints.
//! - `wos_list_projects`     — list all open project UUIDs.
//! - `wos_close_project`     — close (remove) an open project.

use serde_json::Value;
use wos_authoring::TransitionEvent;

use crate::errors::ToolError;
use crate::registry::ProjectRegistry;

// ── wos_lint ──────────────────────────────────────────────────────────────────

/// Export the project to JSON and lint it. Returns a diagnostics summary.
///
/// Args: `{"project_id": "..."}`
///
/// Returns:
/// ```json
/// {
///   "diagnostics": [{ "severity": "error"|"warning", "message": "...", "path": "..." }],
///   "error_count": N,
///   "warning_count": N
/// }
/// ```
pub async fn wos_lint(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let project = registry.get(pid)?;

    let doc = project.snapshot();
    let json = serde_json::to_string(&doc)
        .map_err(|e| ToolError::Internal(format!("serialization failed: {e}")))?;

    let diagnostics = wos_lint::lint_document(&json)
        .map_err(|e| ToolError::Internal(format!("lint engine error: {e}")))?;

    let mut error_count: u64 = 0;
    let mut warning_count: u64 = 0;

    let diag_values: Vec<Value> = diagnostics
        .iter()
        .map(|d| {
            let severity_str = if d.severity == wos_lint::LintSeverity::Error {
                error_count += 1;
                "error"
            } else {
                warning_count += 1;
                "warning"
            };
            serde_json::json!({
                "severity": severity_str,
                "message": d.message,
                "path": d.path,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "diagnostics": diag_values,
        "error_count": error_count,
        "warning_count": warning_count,
    }))
}

// ── wos_run_conformance ───────────────────────────────────────────────────────

/// Run a conformance fixture and return the ConformanceTrace as JSON.
///
/// The caller supplies `fixture_json` — an inline conformance fixture document.
/// Document paths inside the fixture are resolved relative to `base_dir`
/// (defaults to `"."` when absent).
///
/// Args:
/// ```json
/// {
///   "project_id":    "...",
///   "fixture_json":  "<conformance-fixture-json-string>",
///   "base_dir":      "<optional-path>"
/// }
/// ```
///
/// Returns the JSON-serialized `ConformanceTrace` plus a `passed` boolean.
pub async fn wos_run_conformance(
    _registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let fixture_json = require_string_arg(&args, "fixture_json")?;
    let base_dir = args.get("base_dir").and_then(Value::as_str).unwrap_or(".");

    let (result, trace) =
        wos_conformance::run_fixture_with_trace(fixture_json, base_dir).map_err(|e| match e {
            wos_conformance::ConformanceError::Engine(_) => {
                ToolError::Internal(format!("conformance engine error: {e}"))
            }
            _ => ToolError::InvalidArguments(format!("conformance error: {e}")),
        })?;

    let trace_value = serde_json::to_value(&trace)
        .map_err(|e| ToolError::Internal(format!("trace serialization failed: {e}")))?;

    Ok(serde_json::json!({
        "passed": result.passed,
        "failures": result.failures,
        "trace": trace_value,
    }))
}

// ── wos_preview_state_graph ───────────────────────────────────────────────────

/// Construct a state graph string directly from states + transitions.
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "format":     "mermaid" | "dot"
/// }
/// ```
/// Returns `{"graph": "<string>", "format": "mermaid|dot"}`.
pub async fn wos_preview_state_graph(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("mermaid");

    if !matches!(format, "mermaid" | "dot") {
        return Err(ToolError::InvalidArguments(format!(
            "unknown format '{format}'; expected mermaid | dot"
        )));
    }

    let project = registry.get(pid)?;
    let doc = project.snapshot();

    let graph = match format {
        "dot" => build_dot_graph(&doc),
        _ => build_mermaid_graph(&doc),
    };

    Ok(serde_json::json!({ "graph": graph, "format": format }))
}

/// Build a Mermaid state diagram string.
fn build_mermaid_graph(doc: &wos_authoring::KernelDocument) -> String {
    let mut lines = vec!["stateDiagram-v2".to_string()];

    if !doc.lifecycle.initial_state.is_empty() {
        lines.push(format!(
            "    [*] --> {}",
            sanitize_id(&doc.lifecycle.initial_state)
        ));
    }

    for (state_id, state) in &doc.lifecycle.states {
        let sid = sanitize_id(state_id);

        // Mark final states.
        if state.kind == wos_authoring::StateKind::Final {
            lines.push(format!("    {} --> [*]", sid));
        }

        for transition in &state.transitions {
            let tid = sanitize_id(&transition.target);
            let label = match &transition.event {
                None => String::new(),
                Some(ev) => {
                    let n = ev.authoring_display_label();
                    if n.is_empty() {
                        String::new()
                    } else {
                        format!(" : {n}")
                    }
                }
            };
            lines.push(format!("    {} --> {}{}", sid, tid, label));
        }
    }

    lines.join("\n")
}

/// Build a Graphviz DOT digraph string. No external graphviz dependency needed —
/// we produce the DOT source only; rendering is the caller's responsibility.
fn build_dot_graph(doc: &wos_authoring::KernelDocument) -> String {
    let mut lines = vec![
        "digraph workflow {".to_string(),
        "  rankdir=LR;".to_string(),
    ];

    if !doc.lifecycle.initial_state.is_empty() {
        let sid = sanitize_dot_id(&doc.lifecycle.initial_state);
        lines.push("  __start__ [shape=point];".to_string());
        lines.push(format!("  __start__ -> {};", sid));
    }

    for (state_id, state) in &doc.lifecycle.states {
        let sid = sanitize_dot_id(state_id);
        let shape = if state.kind == wos_authoring::StateKind::Final {
            "doublecircle"
        } else {
            "circle"
        };
        lines.push(format!(
            "  {} [shape={} label=\"{}\"];",
            sid, shape, state_id
        ));

        for transition in &state.transitions {
            let tid = sanitize_dot_id(&transition.target);
            let label = match &transition.event {
                None => String::new(),
                Some(ev) => {
                    let n = ev.authoring_display_label();
                    if n.is_empty() {
                        String::new()
                    } else {
                        format!(" [label=\"{n}\"]")
                    }
                }
            };
            lines.push(format!("  {} -> {}{};", sid, tid, label));
        }
    }

    lines.push("}".to_string());
    lines.join("\n")
}

/// Sanitize a state ID for use in Mermaid (spaces → underscores).
fn sanitize_id(id: &str) -> String {
    id.replace(' ', "_")
}

/// Sanitize a state ID for use in DOT (non-alphanumeric → underscore).
fn sanitize_dot_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// ── wos_search ────────────────────────────────────────────────────────────────

/// Linear substring search over states, transitions, actors, or constraints.
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "kind":       "state" | "transition" | "actor" | "constraint",
///   "query":      "substring"
/// }
/// ```
/// Returns `{"matches": [...], "kind": "...", "query": "..."}`.
pub async fn wos_search(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let kind = require_string_arg(&args, "kind")?;
    let query = require_string_arg(&args, "query")?;

    if !matches!(kind, "state" | "transition" | "actor" | "constraint") {
        return Err(ToolError::InvalidArguments(format!(
            "unknown kind '{kind}'; expected state | transition | actor | constraint"
        )));
    }

    let project = registry.get(pid)?;
    let doc = project.snapshot();
    let q = query.to_lowercase();

    let matches: Vec<Value> = match kind {
        "state" => doc
            .lifecycle
            .states
            .iter()
            .filter(|(id, state)| {
                id.to_lowercase().contains(&q)
                    || state
                        .description
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false)
            })
            .map(|(id, state)| {
                serde_json::json!({
                    "id": id,
                    "kind": format!("{:?}", state.kind).to_lowercase(),
                    "description": state.description,
                })
            })
            .collect(),

        "transition" => {
            let mut results = Vec::new();
            for (state_id, state) in &doc.lifecycle.states {
                for transition in &state.transitions {
                    let event_matches = transition.event.as_ref().is_some_and(|e| {
                        e.runtime_dispatch_label().to_lowercase().contains(&q)
                            || e.authoring_display_label().to_lowercase().contains(&q)
                            || serde_json::to_string(e)
                                .map(|s| s.to_lowercase().contains(&q))
                                .unwrap_or(false)
                    });
                    if event_matches
                        || transition.target.to_lowercase().contains(&q)
                        || state_id.to_lowercase().contains(&q)
                    {
                        let event_value = transition
                            .event
                            .as_ref()
                            .and_then(|e| serde_json::to_value(e).ok());
                        let event_label = transition
                            .event
                            .as_ref()
                            .map(TransitionEvent::runtime_dispatch_label);
                        results.push(serde_json::json!({
                            "from": state_id,
                            "to": transition.target,
                            "event": event_value,
                            "event_label": event_label,
                            "guard": transition.guard,
                        }));
                    }
                }
            }
            results
        }

        "actor" => doc
            .actors
            .iter()
            .filter(|a| {
                a.id.to_lowercase().contains(&q)
                    || a.description
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false)
            })
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "kind": format!("{:?}", a.kind).to_lowercase(),
                    "description": a.description,
                })
            })
            .collect(),

        "constraint" => {
            // Search deontic constraints under x-wos-ai.deonticConstraints.
            let constraints: Vec<Value> = doc
                .extensions
                .get("x-wos-ai")
                .and_then(|v| v.get("deonticConstraints"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            constraints
                .iter()
                .filter(|c| {
                    let id = c.get("id").and_then(Value::as_str).unwrap_or("");
                    let action = c.get("action").and_then(Value::as_str).unwrap_or("");
                    let target = c.get("target").and_then(Value::as_str).unwrap_or("");
                    id.to_lowercase().contains(&q)
                        || action.to_lowercase().contains(&q)
                        || target.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        }

        _ => unreachable!(),
    };

    Ok(serde_json::json!({
        "matches": matches,
        "kind": kind,
        "query": query,
    }))
}

// ── wos_list_projects ─────────────────────────────────────────────────────────

/// List all open project UUIDs.
///
/// Args: `{}`
///
/// Returns `{"projects": ["<uuid>", ...], "count": N}`.
pub async fn wos_list_projects(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    _args: Value,
) -> Result<Value, ToolError> {
    let ids: Vec<String> = registry.list().iter().map(|id| id.to_string()).collect();
    let count = ids.len();
    Ok(serde_json::json!({ "projects": ids, "count": count }))
}

// ── wos_close_project ─────────────────────────────────────────────────────────

/// Close (remove) an open project from the registry.
///
/// Args: `{"project_id": "..."}`
///
/// Returns `{"project_id": "...", "closed": true}`.
pub async fn wos_close_project(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    // Verify it exists before closing so we can return a meaningful error.
    registry.get(pid)?;
    registry.close(pid);
    Ok(serde_json::json!({ "project_id": pid, "closed": true }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn require_string_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    match args.get(key) {
        None => Err(ToolError::MissingArgument(key.to_string())),
        Some(v) => v.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
            ToolError::InvalidArguments(format!("'{key}' must be a non-empty string"))
        }),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wos_authoring::{ActorKind, ImpactLevel, StateKind, TransitionEvent, WosProject};

    fn create_project(registry: &mut ProjectRegistry) -> String {
        let project = WosProject::new_kernel();
        registry.insert(project).unwrap().to_string()
    }

    fn create_populated_project(registry: &mut ProjectRegistry) -> String {
        let mut project = WosProject::new(ImpactLevel::Operational, "test");
        project.add_state("draft", StateKind::Atomic).unwrap();
        project.add_state("done", StateKind::Final).unwrap();
        project
            .add_transition("draft", "done", Some("submit".to_string()), None)
            .unwrap();
        project.add_actor("reviewer", ActorKind::Human).unwrap();
        project.set_initial_state("draft").unwrap();
        registry.insert(project).unwrap().to_string()
    }

    // ── wos_lint ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn lint_empty_project_returns_diagnostics() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_lint(&mut registry, "", json!({ "project_id": pid }))
            .await
            .unwrap();

        assert!(result["diagnostics"].is_array());
        assert!(result["error_count"].is_number());
        assert!(result["warning_count"].is_number());
    }

    // ── wos_preview_state_graph ───────────────────────────────────────────

    #[tokio::test]
    async fn preview_mermaid_contains_state_names() {
        let mut registry = ProjectRegistry::new();
        let pid = create_populated_project(&mut registry);

        let result = wos_preview_state_graph(
            &mut registry,
            "",
            json!({ "project_id": pid, "format": "mermaid" }),
        )
        .await
        .unwrap();

        let graph = result["graph"].as_str().unwrap();
        assert!(graph.contains("draft"));
        assert!(graph.contains("done"));
        assert!(graph.contains("submit"));
        assert!(graph.contains("stateDiagram-v2"));
    }

    #[tokio::test]
    async fn preview_dot_contains_state_names() {
        let mut registry = ProjectRegistry::new();
        let pid = create_populated_project(&mut registry);

        let result = wos_preview_state_graph(
            &mut registry,
            "",
            json!({ "project_id": pid, "format": "dot" }),
        )
        .await
        .unwrap();

        let graph = result["graph"].as_str().unwrap();
        assert!(graph.contains("digraph"));
        assert!(graph.contains("draft"));
        assert!(graph.contains("done"));
    }

    #[tokio::test]
    async fn preview_unknown_format_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_populated_project(&mut registry);

        let err = wos_preview_state_graph(
            &mut registry,
            "",
            json!({ "project_id": pid, "format": "svg" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_search ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn search_states_finds_substring_match() {
        let mut registry = ProjectRegistry::new();
        let pid = create_populated_project(&mut registry);

        let result = wos_search(
            &mut registry,
            "",
            json!({ "project_id": pid, "kind": "state", "query": "dra" }),
        )
        .await
        .unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["id"], json!("draft"));
    }

    #[tokio::test]
    async fn search_transitions_finds_event_match() {
        let mut registry = ProjectRegistry::new();
        let pid = create_populated_project(&mut registry);

        let result = wos_search(
            &mut registry,
            "",
            json!({ "project_id": pid, "kind": "transition", "query": "sub" }),
        )
        .await
        .unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["event_label"], json!("submit"));
        assert_eq!(matches[0]["event"]["kind"], json!("message"));
        assert_eq!(matches[0]["event"]["name"], json!("submit"));
    }

    #[tokio::test]
    async fn search_transitions_matches_correlation_in_typed_event() {
        let mut registry = ProjectRegistry::new();
        let mut project = WosProject::new(ImpactLevel::Operational, "typed-search");
        project.add_state("a", StateKind::Atomic).unwrap();
        project.add_state("b", StateKind::Atomic).unwrap();
        project
            .add_transition_typed(
                "a",
                "b",
                Some(TransitionEvent::Message {
                    name: "ping".into(),
                    correlation_key: Some("ck-99".into()),
                    data: None,
                }),
                None,
            )
            .unwrap();
        let pid = registry.insert(project).unwrap().to_string();

        let result = wos_search(
            &mut registry,
            "",
            json!({ "project_id": pid, "kind": "transition", "query": "ck-99" }),
        )
        .await
        .unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["event_label"], json!("ping"));
        assert_eq!(matches[0]["event"]["correlationKey"], json!("ck-99"));
    }

    #[tokio::test]
    async fn search_actors_finds_id_match() {
        let mut registry = ProjectRegistry::new();
        let pid = create_populated_project(&mut registry);

        let result = wos_search(
            &mut registry,
            "",
            json!({ "project_id": pid, "kind": "actor", "query": "review" }),
        )
        .await
        .unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["id"], json!("reviewer"));
    }

    #[tokio::test]
    async fn search_unknown_kind_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let err = wos_search(
            &mut registry,
            "",
            json!({ "project_id": pid, "kind": "milestone", "query": "x" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_list_projects ─────────────────────────────────────────────────

    #[tokio::test]
    async fn list_projects_returns_all_ids() {
        let mut registry = ProjectRegistry::new();
        let pid1 = create_project(&mut registry);
        let pid2 = create_project(&mut registry);

        let result = wos_list_projects(&mut registry, "", json!({}))
            .await
            .unwrap();

        let projects = result["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 2);
        let ids: Vec<&str> = projects.iter().filter_map(Value::as_str).collect();
        assert!(ids.contains(&pid1.as_str()));
        assert!(ids.contains(&pid2.as_str()));
        assert_eq!(result["count"], json!(2));
    }

    #[tokio::test]
    async fn list_projects_empty_registry() {
        let mut registry = ProjectRegistry::new();

        let result = wos_list_projects(&mut registry, "", json!({}))
            .await
            .unwrap();

        assert_eq!(result["count"], json!(0));
        assert_eq!(result["projects"].as_array().unwrap().len(), 0);
    }

    // ── wos_close_project ─────────────────────────────────────────────────

    #[tokio::test]
    async fn close_project_removes_from_registry() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_close_project(&mut registry, "", json!({ "project_id": pid }))
            .await
            .unwrap();

        assert_eq!(result["closed"], json!(true));
        assert!(registry.get(&pid).is_err());
    }

    #[tokio::test]
    async fn close_project_unknown_id_errors() {
        let mut registry = ProjectRegistry::new();

        let err = wos_close_project(
            &mut registry,
            "",
            json!({ "project_id": "00000000-0000-0000-0000-000000000000" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::ProjectNotFound(_)));
    }
}
