// Rust guideline compliant 2026-02-21
//! Validation and query tool handlers (Task 6).
//!
//! - `wos_lint`              — export project, lint, return diagnostics summary.
//! - `wos_run_conformance`   — replay the current native snapshot through the shared conformance API.
//! - `wos_preview_state_graph` — generate Mermaid or DOT state graph string.
//! - `wos_search`            — linear substring search over states/transitions/actors/constraints.
//! - `wos_list_projects`     — list all open project UUIDs.
//! - `wos_close_project`     — close (remove) an open project.

use serde_json::Value;
use wos_authoring::TransitionEvent;

use crate::errors::ToolError;
use crate::registry::ProjectRegistry;

// ── wos_lint ──────────────────────────────────────────────────────────────────

/// Validate the native candidate using canonical schemas and explicit static scope.
///
/// Args: `{"project_id": "...", "scope": "document", "resolved_dependencies": []}`
///
/// Returns:
/// ```json
/// {
///   "diagnostics": [{ "ruleId": "K-016", "severity": "error", "tier": "T1", "message": "...", "path": "/lifecycle/initialState" }],
///   "block_count": N,
///   "error_count": N,
///   "warning_count": N,
///   "info_count": N
/// }
/// ```
pub async fn wos_lint(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let args = crate::catalog::validate_arguments("wos_lint", &args)?;
    let pid = require_string_arg(&args, "project_id")?;
    let project = registry.get(pid)?;
    let scope: wos_lint::studio_api::ValidationScope =
        serde_json::from_value(args["scope"].clone())
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
    let dependencies: Vec<wos_lint::studio_api::ResolvedDependency> =
        serde_json::from_value(args["resolved_dependencies"].clone())
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
    let candidate = project.native_snapshot();
    let validation = wos_lint::studio_api::validate(wos_lint::studio_api::ValidationRequest {
        candidate: &candidate,
        dependencies: &dependencies,
        scope,
    });
    let mut result = lint_summary(&validation.diagnostics);
    result["validation"] =
        serde_json::to_value(validation).map_err(|error| ToolError::Internal(error.to_string()))?;
    Ok(result)
}

/// Serialize native diagnostics and count each severity without rewriting findings.
fn lint_summary(diagnostics: &[wos_lint::LintDiagnostic]) -> Value {
    let (mut block_count, mut error_count, mut warning_count, mut info_count) = (0, 0, 0, 0);
    for diagnostic in diagnostics {
        match diagnostic.severity {
            wos_lint::LintSeverity::Block => block_count += 1,
            wos_lint::LintSeverity::Error => error_count += 1,
            wos_lint::LintSeverity::Warning => warning_count += 1,
            wos_lint::LintSeverity::Info => info_count += 1,
        }
    }
    serde_json::json!({
        "diagnostics": diagnostics,
        "block_count": block_count,
        "error_count": error_count,
        "warning_count": warning_count,
        "info_count": info_count,
    })
}

// ── wos_run_conformance ───────────────────────────────────────────────────────

/// Replay the selected project's current immutable snapshot through WosRuntime.
///
/// `fixture_json` supplies reviewed events and expectations. Its document roles
/// assert the selected snapshot/dependencies; they cannot substitute documents.
/// Optional `resolved_dependencies` contain role, URL, version and `document_json`.
/// `required_observations`, `expected_observations` and `clock_start_ms` pass
/// directly to the shared replay owner. Mutable file paths are unsupported.
///
/// # Errors
///
/// Returns invalid arguments for unsupported inputs or replay refusal, and the
/// native project lookup error when the selected project does not exist.
pub async fn wos_run_conformance(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    use wos_conformance::studio_api::{
        ObservationExpectation, ReplayRequest, ResolvedDependency, prepare_replay, replay,
    };
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct DependencyInput {
        role: String,
        url: String,
        version: Option<String>,
        document_json: String,
    }
    // Direct handler callers and dispatch use the same declaration and defaults.
    let args = crate::catalog::validate_arguments("wos_run_conformance", &args)?;
    let project_id = require_string_arg(&args, "project_id")?;
    let candidate = registry.get(project_id)?.native_snapshot();
    let candidate_bytes = serde_json::to_vec(&candidate)
        .map_err(|error| ToolError::Internal(format!("snapshot serialization failed: {error}")))?;
    let fixture_json = require_string_arg(&args, "fixture_json")?.to_string();
    let parse_error = |error: serde_json::Error| {
        ToolError::InvalidArguments(format!("invalid replay input: {error}"))
    };
    let dependencies: Vec<DependencyInput> =
        serde_json::from_value(args["resolved_dependencies"].clone()).map_err(parse_error)?;
    let required_observations: Vec<String> =
        serde_json::from_value(args["required_observations"].clone()).map_err(parse_error)?;
    let expectations: Vec<ObservationExpectation> =
        serde_json::from_value(args["expected_observations"].clone()).map_err(parse_error)?;
    let clock_start_ms: u64 =
        serde_json::from_value(args["clock_start_ms"].clone()).map_err(parse_error)?;
    let request = ReplayRequest {
        candidate_bytes,
        dependencies: dependencies
            .into_iter()
            .map(|input| ResolvedDependency {
                role: input.role,
                url: input.url,
                version: input.version,
                bytes: input.document_json.into_bytes(),
            })
            .collect(),
        fixture_json,
        clock_start_ms,
        required_observations,
        expectations,
    };
    let prepared =
        prepare_replay(request).map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
    let result = replay(&prepared)
        .map_err(|error| ToolError::InvalidArguments(format!("replay failed: {error}")))?;
    serde_json::to_value(result)
        .map_err(|error| ToolError::Internal(format!("replay serialization failed: {error}")))
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
    let args = crate::catalog::validate_arguments("wos_preview_state_graph", &args)?;
    let pid = require_string_arg(&args, "project_id")?;
    let format = require_string_arg(&args, "format")?;

    let project = registry.get(pid)?;
    let doc = project
        .typed_snapshot()
        .map_err(|error| ToolError::ValidationFailed(error.message))?;

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
    let q = query.to_lowercase();
    if kind == "constraint" {
        let document = project.native_snapshot();
        let matches: Vec<Value> = wos_authoring::native_rule_sources(&document)
            .into_iter()
            .filter(|source| source.value.to_string().to_lowercase().contains(&q))
            .map(|source| serde_json::json!({"path":source.pointer,"value":source.value}))
            .collect();
        return Ok(serde_json::json!({"matches":matches,"kind":kind,"query":query}));
    }
    let doc = project
        .typed_snapshot()
        .map_err(|error| ToolError::ValidationFailed(error.message))?;
    let matches: Vec<Value> = match kind {
        "state" => doc
            .lifecycle
            .states
            .iter()
            .filter(|(id, state)| {
                id.to_lowercase().contains(&q)
                    || state
                        .description
                        .as_ref()
                        .and_then(Value::as_str)
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
                        .as_ref()
                        .and_then(Value::as_str)
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

    #[test]
    fn shared_constructed_block_vector_preserves_metadata_and_count() {
        let diagnostic: wos_lint::LintDiagnostic = serde_json::from_str(include_str!(
            "../../../wos-lint/tests/fixtures/shared-validation/constructed-block.json"
        ))
        .unwrap();
        let expected = serde_json::to_value(&diagnostic).unwrap();
        let result = lint_summary(&[diagnostic]);
        assert_eq!(result["diagnostics"][0], expected);
        assert_eq!(result["block_count"], 1);
        assert_eq!(result["error_count"], 0);
    }

    #[test]
    fn lint_summary_preserves_all_native_severities_and_metadata() {
        let diagnostics: Vec<_> = [
            wos_lint::LintSeverity::Block,
            wos_lint::LintSeverity::Error,
            wos_lint::LintSeverity::Warning,
            wos_lint::LintSeverity::Info,
        ]
        .into_iter()
        .map(|severity| wos_lint::LintDiagnostic {
            rule_id: "PUB-LINT-001",
            severity,
            tier: wos_lint::Tier::T2,
            path: "/governance/findings/0".into(),
            message: "Review this finding.".into(),
            suggested_fix: Some(wos_lint::SuggestedFix::RemoveProperty {
                path: "/governance/findings/0".into(),
            }),
            related_docs: vec!["LINT-MATRIX.md#PUB-LINT-001".into()],
            source: None,
        })
        .collect();
        let result = lint_summary(&diagnostics);
        assert_eq!(
            result["diagnostics"],
            serde_json::to_value(&diagnostics).unwrap()
        );
        for count in ["block_count", "error_count", "warning_count", "info_count"] {
            assert_eq!(
                result[count], 1,
                "{count} must count only its native severity"
            );
        }
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
