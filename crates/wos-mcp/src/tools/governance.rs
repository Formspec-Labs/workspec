//! Governance and AI tool handlers (Task 5).
//!
//! Each handler is a thin delegate to `WosProject` authoring helpers.
//! No business logic lives here — all invariants are enforced by
//! `wos-authoring`.
//!
//! # Tools in this module
//!
//! - `wos_add_due_process_path` — write a due-process path to `x-wos-governance`.
//! - `wos_add_assertion_gate`   — register an assertion gate in `x-wos-governance`.
//! - `wos_set_impact_level`     — set the document-level impact classification.
//! - `wos_add_ai_agent`         — register an AI agent in `x-wos-ai.agents`.
//! - `wos_add_deontic_constraint` — append a structured deontic constraint.

use serde_json::Value;
use wos_authoring::ImpactLevel;

use crate::errors::ToolError;
use crate::registry::ProjectRegistry;

// ── wos_add_due_process_path ──────────────────────────────────────────────────

/// Record a due-process path under `x-wos-governance.dueProcesePaths`.
///
/// Args:
/// ```json
/// {
///   "project_id":   "...",
///   "path_id":      "...",
///   "description":  "...",
///   "steps":        ["step1", "step2"]
/// }
/// ```
/// Returns `{"path_id": "..."}`.
pub async fn wos_add_due_process_path(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let path_id = require_string_arg(&args, "path_id")?;
    let description = require_string_arg(&args, "description")?;
    let steps = parse_string_array(&args, "steps")?;

    let project = registry.get_mut(pid)?;
    project
        .add_due_process_path(path_id, description, steps)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "path_id": path_id }))
}

// ── wos_add_assertion_gate ────────────────────────────────────────────────────

/// Add an assertion gate to `x-wos-governance.assertionGates`.
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "gate_id":    "...",
///   "assertion":  "<FEL expression>",
///   "transition": "<event name>"
/// }
/// ```
/// Returns `{"gate_id": "..."}`.
pub async fn wos_add_assertion_gate(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let gate_id = require_string_arg(&args, "gate_id")?;
    let assertion = require_string_arg(&args, "assertion")?;
    let transition = require_string_arg(&args, "transition")?;

    let project = registry.get_mut(pid)?;
    project
        .add_assertion_gate(gate_id, assertion, transition)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "gate_id": gate_id }))
}

// ── wos_set_impact_level ──────────────────────────────────────────────────────

/// Set the document-level impact classification (kernel §S6).
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "level": "rights-impacting|safety-impacting|operational|informational"
/// }
/// ```
/// Returns `{"level": "..."}`.
pub async fn wos_set_impact_level(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let level_str = require_string_arg(&args, "level")?;
    let level = parse_impact_level(level_str)?;

    let project = registry.get_mut(pid)?;
    project
        .set_impact_level(level)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "level": level_str }))
}

// ── wos_add_ai_agent ──────────────────────────────────────────────────────────

/// Register an AI agent under `x-wos-ai.agents`.
///
/// AI agents are NOT actors (kernel §S3). They live in `x-wos-ai.agents` so
/// that the `ActorKind` invariant (Human | System only) is preserved.
///
/// Args:
/// ```json
/// {
///   "project_id":   "...",
///   "agent_id":     "...",
///   "role":         "...",
///   "model":        "...",
///   "capabilities": ["cap1", "cap2"]
/// }
/// ```
/// Returns `{"agent_id": "..."}`.
pub async fn wos_add_ai_agent(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let agent_id = require_string_arg(&args, "agent_id")?;
    let role = require_string_arg(&args, "role")?;
    let model = require_string_arg(&args, "model")?;
    let capabilities = parse_string_array(&args, "capabilities")?;

    let project = registry.get_mut(pid)?;
    project
        .add_ai_agent(agent_id, role, model, capabilities)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "agent_id": agent_id }))
}

// ── wos_add_deontic_constraint ────────────────────────────────────────────────

/// Append a structured deontic constraint under `x-wos-ai.deonticConstraints`.
///
/// Args:
/// ```json
/// {
///   "project_id":     "...",
///   "constraint_id":  "...",
///   "target":         "...",
///   "modality":       "must|must_not|may",
///   "action":         "..."
/// }
/// ```
/// Returns `{"constraint_id": "..."}`.
pub async fn wos_add_deontic_constraint(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let constraint_id = require_string_arg(&args, "constraint_id")?;
    let target = require_string_arg(&args, "target")?;
    let modality = require_string_arg(&args, "modality")?;
    let action = require_string_arg(&args, "action")?;

    let project = registry.get_mut(pid)?;
    project
        .add_deontic_constraint(constraint_id, target, modality, action)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "constraint_id": constraint_id }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a required non-empty string argument.
fn require_string_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    match args.get(key) {
        None => Err(ToolError::MissingArgument(key.to_string())),
        Some(v) => v
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::InvalidArguments(format!("'{key}' must be a non-empty string"))),
    }
}

/// Extract an optional array of strings, defaulting to empty if absent.
fn parse_string_array(args: &Value, key: &str) -> Result<Vec<String>, ToolError> {
    match args.get(key) {
        None => Ok(Vec::new()),
        Some(v) => {
            let arr = v
                .as_array()
                .ok_or_else(|| ToolError::InvalidArguments(format!("'{key}' must be an array")))?;
            arr.iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| ToolError::InvalidArguments(format!("'{key}' must be an array of strings")))
                })
                .collect()
        }
    }
}

/// Parse the `level` string into an `ImpactLevel`.
///
/// Accepts the kebab-case variants used by the kernel spec:
/// `rights-impacting`, `safety-impacting`, `operational`, `informational`.
fn parse_impact_level(s: &str) -> Result<ImpactLevel, ToolError> {
    match s {
        "rights-impacting" => Ok(ImpactLevel::RightsImpacting),
        "safety-impacting" => Ok(ImpactLevel::SafetyImpacting),
        "operational" => Ok(ImpactLevel::Operational),
        "informational" => Ok(ImpactLevel::Informational),
        other => Err(ToolError::InvalidArguments(format!(
            "unknown impact level '{other}'; expected \
             rights-impacting | safety-impacting | operational | informational"
        ))),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_project(registry: &mut ProjectRegistry) -> String {
        let project = wos_authoring::WosProject::new_kernel();
        registry.insert(project).unwrap().to_string()
    }

    // ── wos_add_due_process_path ──────────────────────────────────────────

    #[tokio::test]
    async fn add_due_process_path_stores_in_extension() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_add_due_process_path(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "path_id": "appealPath",
                "description": "Standard appeal process",
                "steps": ["file", "review", "decision"]
            }),
        )
        .await
        .unwrap();

        assert_eq!(result["path_id"], json!("appealPath"));

        let doc = registry.get(&pid).unwrap().snapshot();
        let gov = &doc.extensions["x-wos-governance"];
        assert_eq!(gov["dueProcesePaths"]["appealPath"]["description"], "Standard appeal process");
        let steps = gov["dueProcesePaths"]["appealPath"]["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 3);
    }

    #[tokio::test]
    async fn add_due_process_path_duplicate_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        wos_add_due_process_path(
            &mut registry,
            "",
            json!({ "project_id": pid, "path_id": "p1", "description": "x", "steps": [] }),
        )
        .await
        .unwrap();

        let err = wos_add_due_process_path(
            &mut registry,
            "",
            json!({ "project_id": pid, "path_id": "p1", "description": "y", "steps": [] }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_add_assertion_gate ────────────────────────────────────────────

    #[tokio::test]
    async fn add_assertion_gate_stores_in_extension() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_add_assertion_gate(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "gate_id": "incomeCheck",
                "assertion": "caseFile.income > 0",
                "transition": "approve"
            }),
        )
        .await
        .unwrap();

        assert_eq!(result["gate_id"], json!("incomeCheck"));

        let doc = registry.get(&pid).unwrap().snapshot();
        let gov = &doc.extensions["x-wos-governance"];
        assert_eq!(gov["assertionGates"]["incomeCheck"]["assertion"], "caseFile.income > 0");
        assert_eq!(gov["assertionGates"]["incomeCheck"]["transition"], "approve");
    }

    // ── wos_set_impact_level ──────────────────────────────────────────────

    #[tokio::test]
    async fn set_impact_level_updates_document() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_set_impact_level(
            &mut registry,
            "",
            json!({ "project_id": pid, "level": "rights-impacting" }),
        )
        .await
        .unwrap();

        assert_eq!(result["level"], json!("rights-impacting"));

        let doc = registry.get(&pid).unwrap().snapshot();
        assert_eq!(doc.impact_level, Some(ImpactLevel::RightsImpacting));
    }

    #[tokio::test]
    async fn set_impact_level_rejects_unknown_variant() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let err = wos_set_impact_level(
            &mut registry,
            "",
            json!({ "project_id": pid, "level": "critical" }),
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ToolError::InvalidArguments(_)),
            "obsolete variant 'critical' must be rejected; got {err:?}"
        );
    }

    #[tokio::test]
    async fn set_impact_level_accepts_all_four_variants() {
        for level in ["rights-impacting", "safety-impacting", "operational", "informational"] {
            let mut registry = ProjectRegistry::new();
            let pid = create_project(&mut registry);
            wos_set_impact_level(
                &mut registry,
                "",
                json!({ "project_id": pid, "level": level }),
            )
            .await
            .unwrap_or_else(|e| panic!("level '{level}' must be accepted; got {e}"));
        }
    }

    // ── wos_add_ai_agent ──────────────────────────────────────────────────

    #[tokio::test]
    async fn add_ai_agent_stores_in_extension() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_add_ai_agent(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "agent_id": "reviewBot",
                "role": "Automated reviewer",
                "model": "claude-3-5-sonnet",
                "capabilities": ["read_case_file", "submit_review"]
            }),
        )
        .await
        .unwrap();

        assert_eq!(result["agent_id"], json!("reviewBot"));

        let doc = registry.get(&pid).unwrap().snapshot();
        let agents = doc.extensions["x-wos-ai"]["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["id"], json!("reviewBot"));
        assert_eq!(agents[0]["model"], json!("claude-3-5-sonnet"));
        let caps = agents[0]["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
    }

    #[tokio::test]
    async fn add_ai_agent_duplicate_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        wos_add_ai_agent(
            &mut registry,
            "",
            json!({ "project_id": pid, "agent_id": "bot", "role": "r", "model": "m", "capabilities": [] }),
        )
        .await
        .unwrap();

        let err = wos_add_ai_agent(
            &mut registry,
            "",
            json!({ "project_id": pid, "agent_id": "bot", "role": "r2", "model": "m2", "capabilities": [] }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_add_deontic_constraint ────────────────────────────────────────

    #[tokio::test]
    async fn add_deontic_constraint_stores_in_extension() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_add_deontic_constraint(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "constraint_id": "mustNotAutoApprove",
                "target": "ai-agents",
                "modality": "must_not",
                "action": "auto-approve"
            }),
        )
        .await
        .unwrap();

        assert_eq!(result["constraint_id"], json!("mustNotAutoApprove"));

        let doc = registry.get(&pid).unwrap().snapshot();
        let constraints = doc.extensions["x-wos-ai"]["deonticConstraints"]
            .as_array()
            .unwrap();
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0]["modality"], json!("must_not"));
        assert_eq!(constraints[0]["target"], json!("ai-agents"));
    }

    #[tokio::test]
    async fn add_deontic_constraint_rejects_invalid_modality() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let err = wos_add_deontic_constraint(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "constraint_id": "c1",
                "target": "all",
                "modality": "should",
                "action": "review"
            }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }
}
