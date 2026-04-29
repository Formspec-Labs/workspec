//! Document-management tool handlers.
//!
//! These four handlers create, load, export, and describe WOS kernel projects
//! held in the `ProjectRegistry`. Each is a thin wrapper over `WosProject` /
//! `wos-authoring` helpers — no business logic lives here.
//!
//! # Project-less vs project-scoped handlers
//!
//! `wos_create_kernel` and `wos_load_document` do not take an existing
//! `project_id` (they create new entries). The dispatch layer passes an empty
//! string for `project_id` to these tools, consistent with the convention
//! established by `wos_ping`. The comment on each function documents this.
//!
//! `wos_export_document` and `wos_describe_document` require a valid
//! `project_id` in `args`.

use std::fs;

use serde_json::Value;
use wos_authoring::{KernelDocument, WosProject};

use crate::errors::ToolError;
use crate::registry::ProjectRegistry;

// ── wos_create_kernel ─────────────────────────────────────────────────────────

/// Create a new empty WOS kernel project and register it.
///
/// The `project_id` argument is unused (this tool creates a new project).
/// Returns `{"project_id": "<uuid>"}`.
pub async fn wos_create_kernel(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    _args: Value,
) -> Result<Value, ToolError> {
    let project = WosProject::new_kernel();
    let id = registry.insert(project)?;
    Ok(serde_json::json!({ "project_id": id.to_string() }))
}

// ── wos_load_document ─────────────────────────────────────────────────────────

/// Parse and register a WOS kernel document from JSON text or a file path.
///
/// Accepts either:
/// - `{"json": "<wos-kernel-json-string>"}` — parse inline JSON text.
/// - `{"path": "<file-path>"}` — read the file at `path`, then parse.
///
/// Validates the parsed document with `wos_lint::lint_document` and returns
/// `ToolError::ValidationFailed` (with the first error-level diagnostic) if
/// any errors are found. Warnings do not block registration.
///
/// On success returns `{"project_id": "<uuid>"}`.
pub async fn wos_load_document(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let json_text = resolve_json_text(&args)?;

    // Run lint to detect structural errors before registering the project.
    let diagnostics = wos_lint::lint_document(&json_text)
        .map_err(|e| ToolError::ValidationFailed(e.to_string()))?;

    // Treat any error-severity diagnostic as a blocking validation failure.
    if let Some(first_error) = diagnostics
        .iter()
        .find(|d| d.severity == wos_lint::LintSeverity::Error)
    {
        return Err(ToolError::ValidationFailed(first_error.message.clone()));
    }

    // Deserialize into a typed document, then wrap in a WosProject.
    let document: KernelDocument = serde_json::from_str(&json_text)
        .map_err(|e| ToolError::ValidationFailed(format!("deserialization failed: {e}")))?;

    let project = WosProject::from_document(document);
    let id = registry.insert(project)?;
    Ok(serde_json::json!({ "project_id": id.to_string() }))
}

/// Resolve the JSON text from `{"json": "..."}` or `{"path": "..."}` args.
fn resolve_json_text(args: &Value) -> Result<String, ToolError> {
    if let Some(inline) = args.get("json").and_then(Value::as_str) {
        return Ok(inline.to_string());
    }

    if let Some(path) = args.get("path").and_then(Value::as_str) {
        return fs::read_to_string(path)
            .map_err(|e| ToolError::InvalidArguments(format!("cannot read '{path}': {e}")));
    }

    Err(ToolError::MissingArgument(
        "wos_load_document requires either 'json' or 'path'".to_string(),
    ))
}

// ── wos_export_document ───────────────────────────────────────────────────────

/// Serialize a registered project back to a JSON string.
///
/// Returns `{"document": "<json-string>"}`.
pub async fn wos_export_document(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_project_id(&args)?;
    let project = registry.get(pid)?;
    let document = project.snapshot();
    let json = serde_json::to_string(&document)
        .map_err(|e| ToolError::Internal(format!("serialization failed: {e}")))?;
    Ok(serde_json::json!({ "document": json }))
}

// ── wos_describe_document ─────────────────────────────────────────────────────

/// Return a summary of a registered project's current document state.
///
/// Counts top-level states, outgoing transitions across all states, declared
/// actors, and AI agents stored under the `x-wos-ai.agents` extension key.
///
/// Returns:
/// ```json
/// {
///   "state_count": N,
///   "transition_count": N,
///   "actor_count": N,
///   "impact_level": "operational",
///   "ai_agent_count": N
/// }
/// ```
pub async fn wos_describe_document(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_project_id(&args)?;
    let project = registry.get(pid)?;
    let doc = project.snapshot();

    let state_count = doc.lifecycle.states.len();

    let transition_count: usize = doc
        .lifecycle
        .states
        .values()
        .map(|s| s.transitions.len())
        .sum();

    let actor_count = doc.actors.len();

    let impact_level = doc
        .impact_level
        .map(|level| {
            // Serialize through serde to get the kebab-case string form the
            // kernel spec uses (e.g. "rights-impacting", "safety-impacting").
            serde_json::to_value(level)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_else(|| format!("{level:?}"))
        })
        .unwrap_or_else(|| "none".to_string());

    // AI agents live under x-wos-ai.agents (an array) when the document was
    // authored via add_extension_key or a companion AIIntegrationDocument.
    // Deontic constraints (x-wos-ai.deonticConstraints) are separate and not
    // counted here.
    // TODO(wos-mcp): replace with typed AIIntegrationDocument accessor once it lands.
    let ai_agent_count = doc
        .extensions
        .get("x-wos-ai")
        .and_then(|v| v.get("agents"))
        .and_then(Value::as_array)
        .map(|arr| arr.len())
        .unwrap_or(0);

    Ok(serde_json::json!({
        "state_count": state_count,
        "transition_count": transition_count,
        "actor_count": actor_count,
        "impact_level": impact_level,
        "ai_agent_count": ai_agent_count,
    }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the `project_id` string from the args object.
///
/// Returns `MissingArgument` when the key is absent.
/// Returns `InvalidArguments` when the key exists but is not a non-empty string,
/// so callers can distinguish a missing field from a type mismatch.
fn require_project_id(args: &Value) -> Result<&str, ToolError> {
    match args.get("project_id") {
        None => Err(ToolError::MissingArgument("project_id".to_string())),
        Some(v) => v.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
            ToolError::InvalidArguments("'project_id' must be a non-empty string".to_string())
        }),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wos_authoring::{ActorKind, ImpactLevel, StateKind};

    // ── wos_create_kernel ─────────────────────────────────────────────────

    #[tokio::test]
    async fn create_kernel_returns_project_id() {
        let mut registry = ProjectRegistry::new();
        let result = wos_create_kernel(&mut registry, "", json!({}))
            .await
            .unwrap();
        let pid = result["project_id"].as_str().expect("must have project_id");
        assert!(!pid.is_empty());
        // The returned id must be locatable in the registry.
        assert!(registry.get(pid).is_ok());
    }

    #[tokio::test]
    async fn create_kernel_enforces_project_cap() {
        let mut registry = ProjectRegistry::new();
        for _ in 0..20 {
            wos_create_kernel(&mut registry, "", json!({}))
                .await
                .unwrap();
        }
        let err = wos_create_kernel(&mut registry, "", json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::TooManyProjects));
    }

    // ── wos_export_document ───────────────────────────────────────────────

    #[tokio::test]
    async fn export_document_round_trip() {
        let mut registry = ProjectRegistry::new();
        let result = wos_create_kernel(&mut registry, "", json!({}))
            .await
            .unwrap();
        let pid = result["project_id"].as_str().unwrap().to_string();

        let export = wos_export_document(&mut registry, "", json!({ "project_id": pid }))
            .await
            .unwrap();

        let doc_str = export["document"].as_str().expect("must have document");
        // Document must deserialize back to a KernelDocument.
        let doc: KernelDocument = serde_json::from_str(doc_str)
            .expect("exported document must be valid KernelDocument JSON");
        assert_eq!(doc.wos_workflow, "1.0");
    }

    #[tokio::test]
    async fn export_document_unknown_project_errors() {
        let mut registry = ProjectRegistry::new();
        let err = wos_export_document(
            &mut registry,
            "",
            json!({ "project_id": "00000000-0000-0000-0000-000000000000" }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, ToolError::ProjectNotFound(_)));
    }

    #[tokio::test]
    async fn export_document_missing_project_id_errors() {
        let mut registry = ProjectRegistry::new();
        let err = wos_export_document(&mut registry, "", json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    // ── wos_describe_document ─────────────────────────────────────────────

    #[tokio::test]
    async fn describe_document_empty_project() {
        let mut registry = ProjectRegistry::new();
        let result = wos_create_kernel(&mut registry, "", json!({}))
            .await
            .unwrap();
        let pid = result["project_id"].as_str().unwrap().to_string();

        let desc = wos_describe_document(&mut registry, "", json!({ "project_id": pid }))
            .await
            .unwrap();

        assert_eq!(desc["state_count"], json!(0));
        assert_eq!(desc["transition_count"], json!(0));
        assert_eq!(desc["actor_count"], json!(0));
        assert_eq!(desc["impact_level"], json!("operational"));
        assert_eq!(desc["ai_agent_count"], json!(0));
    }

    #[tokio::test]
    async fn describe_document_populated_project() {
        let mut registry = ProjectRegistry::new();
        let mut project = WosProject::new(ImpactLevel::SafetyImpacting, "test");
        project.add_state("draft", StateKind::Atomic).unwrap();
        project.add_state("approved", StateKind::Final).unwrap();
        project
            .add_transition("draft", "approved", Some("approve".to_string()), None)
            .unwrap();
        project.add_actor("reviewer", ActorKind::Human).unwrap();

        let pid = registry.insert(project).unwrap().to_string();

        let desc = wos_describe_document(&mut registry, "", json!({ "project_id": pid }))
            .await
            .unwrap();

        assert_eq!(desc["state_count"], json!(2));
        assert_eq!(desc["transition_count"], json!(1));
        assert_eq!(desc["actor_count"], json!(1));
        assert_eq!(desc["impact_level"], json!("safety-impacting"));
        assert_eq!(desc["ai_agent_count"], json!(0));
    }

    // ── create + export round-trip (plan Step 3.6) ────────────────────────

    #[tokio::test]
    async fn create_and_export_round_trip() {
        let mut registry = ProjectRegistry::new();

        let create_result = wos_create_kernel(&mut registry, "", json!({}))
            .await
            .unwrap();
        let pid = create_result["project_id"].as_str().unwrap().to_string();

        let export_result = wos_export_document(&mut registry, "", json!({ "project_id": &pid }))
            .await
            .unwrap();
        let doc_json = export_result["document"].as_str().unwrap();

        // Re-parse to verify the exported JSON is a valid kernel document.
        let doc: serde_json::Value = serde_json::from_str(doc_json).unwrap();
        assert_eq!(doc["$wosWorkflow"], json!("1.0"));
    }

    // ── require_project_id error classification ───────────────────────────

    /// Absent `project_id` key must produce `MissingArgument`.
    #[tokio::test]
    async fn missing_project_id_gives_missing_argument_error() {
        let mut registry = ProjectRegistry::new();
        let err = wos_export_document(&mut registry, "", json!({}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::MissingArgument(_)),
            "absent project_id must yield MissingArgument; got {err:?}"
        );
    }

    /// `project_id` present but wrong type must produce `InvalidArguments`.
    #[tokio::test]
    async fn wrong_type_project_id_gives_invalid_arguments_error() {
        let mut registry = ProjectRegistry::new();
        let err = wos_export_document(&mut registry, "", json!({ "project_id": 123 }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::InvalidArguments(_)),
            "wrong-type project_id must yield InvalidArguments; got {err:?}"
        );
    }

    // ── wos_load_document ─────────────────────────────────────────────────

    #[tokio::test]
    async fn load_document_inline_json_registers_project() {
        let mut registry = ProjectRegistry::new();

        // Build a minimal valid kernel document that passes lint tier-1.
        let kernel_json = serde_json::json!({
            "$wosWorkflow": "1.0",
            "title": "loaded workflow",
            "impactLevel": "operational",
            "lifecycle": {
                "initialState": "draft",
                "states": {
                    "draft": { "type": "atomic" },
                    "done": { "type": "final" }
                }
            }
        })
        .to_string();

        let result = wos_load_document(&mut registry, "", json!({ "json": kernel_json }))
            .await
            .unwrap();

        let pid = result["project_id"].as_str().expect("must have project_id");
        let project = registry.get(pid).unwrap();
        let doc = project.snapshot();
        assert_eq!(doc.title.as_deref(), Some("loaded workflow"));
    }

    #[tokio::test]
    async fn load_document_missing_both_args_errors() {
        let mut registry = ProjectRegistry::new();
        let err = wos_load_document(&mut registry, "", json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[tokio::test]
    async fn load_document_invalid_json_errors() {
        let mut registry = ProjectRegistry::new();
        let err = wos_load_document(&mut registry, "", json!({ "json": "not json" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ValidationFailed(_)));
    }

    #[tokio::test]
    async fn load_document_path_branch_registers_project() {
        // Write a valid kernel document to a temp file and load it via the
        // {"path": "..."} branch of wos_load_document.
        let kernel_json = serde_json::json!({
            "$wosWorkflow": "1.0",
            "title": "path-loaded workflow",
            "impactLevel": "operational",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" },
                    "closed": { "type": "final" }
                }
            }
        })
        .to_string();

        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::fs::write(tmp.path(), &kernel_json).expect("write kernel json to temp file");

        let mut registry = ProjectRegistry::new();
        let result = wos_load_document(
            &mut registry,
            "",
            json!({ "path": tmp.path().to_str().expect("valid utf-8 path") }),
        )
        .await
        .unwrap();

        let pid = result["project_id"].as_str().expect("must have project_id");
        assert!(!pid.is_empty());

        let project = registry.get(pid).expect("project must be registered");
        let doc = project.snapshot();
        assert_eq!(doc.title.as_deref(), Some("path-loaded workflow"));
    }
}
