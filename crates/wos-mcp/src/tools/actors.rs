//! Actor tool handlers: declare actors and attach actor-level extensions.
//!
//! Each handler is a thin wrapper over the corresponding `WosProject` method.
//! Per the WOS kernel spec (§S3), actors are `Human` or `System` only — AI
//! agents route through `x-wos-ai.agents`, not through new `ActorKind` variants.

use serde_json::Value;
use wos_authoring::ActorKind;

use crate::errors::ToolError;
use crate::registry::ProjectRegistry;

// ── wos_add_actor ─────────────────────────────────────────────────────────────

/// Declare an actor on a registered project.
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "actor_id":   "...",
///   "kind":       "human|system"  // optional, default "human"
/// }
/// ```
/// Returns `{"actor_id": "..."}` on success.
pub async fn wos_add_actor(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let actor_id = require_string_arg(&args, "actor_id")?;
    let kind = parse_actor_kind(&args)?;

    let project = registry.get_mut(pid)?;
    project
        .add_actor(actor_id, kind)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "actor_id": actor_id }))
}

// ── wos_add_actor_extension ───────────────────────────────────────────────────

/// Attach an extension key to an existing actor (kernel §10.6 actorExtension).
///
/// Args:
/// ```json
/// {
///   "project_id": "...",
///   "actor_id":   "...",
///   "key":        "x-...",  // must start with "x-"
///   "value":      <any JSON value>
/// }
/// ```
/// Returns `{"actor_id": "...", "key": "x-..."}` on success.
pub async fn wos_add_actor_extension(
    registry: &mut ProjectRegistry,
    _project_id: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let pid = require_string_arg(&args, "project_id")?;
    let actor_id = require_string_arg(&args, "actor_id")?;
    let key = require_string_arg(&args, "key")?;
    let value = args
        .get("value")
        .cloned()
        .ok_or_else(|| ToolError::MissingArgument("value".to_string()))?;

    let project = registry.get_mut(pid)?;
    project
        .add_actor_extension(actor_id, key, value)
        .map_err(|d| ToolError::InvalidArguments(d.message))?;

    Ok(serde_json::json!({ "actor_id": actor_id, "key": key }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a required non-empty string argument from the args object.
///
/// Returns `MissingArgument` when the key is absent.
/// Returns `InvalidArguments` when the key exists but is not a non-empty string,
/// so callers can distinguish a missing field from a type mismatch.
fn require_string_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    match args.get(key) {
        None => Err(ToolError::MissingArgument(key.to_string())),
        Some(v) => v.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
            ToolError::InvalidArguments(format!("'{key}' must be a non-empty string"))
        }),
    }
}

/// Parse the optional `kind` argument to `ActorKind`, defaulting to `Human`.
fn parse_actor_kind(args: &Value) -> Result<ActorKind, ToolError> {
    match args.get("kind").and_then(Value::as_str) {
        None | Some("human") => Ok(ActorKind::Human),
        Some("system") => Ok(ActorKind::System),
        Some(other) => Err(ToolError::InvalidArguments(format!(
            "unknown actor kind '{other}'; expected human|system \
             (AI agents use x-wos-ai.agents, not actor declarations)"
        ))),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── wos_add_actor ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn add_actor_human_appears_in_document() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let result = wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": pid, "actor_id": "reviewer" }),
        )
        .await
        .unwrap();

        assert_eq!(result["actor_id"], json!("reviewer"));
        let project = registry.get(&pid).unwrap();
        let actors = &project.snapshot().actors;
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].id, "reviewer");
        assert_eq!(actors[0].kind, ActorKind::Human);
    }

    #[tokio::test]
    async fn add_actor_system_kind() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": pid, "actor_id": "approval-service", "kind": "system" }),
        )
        .await
        .unwrap();

        let project = registry.get(&pid).unwrap();
        assert_eq!(project.snapshot().actors[0].kind, ActorKind::System);
    }

    #[tokio::test]
    async fn add_actor_invalid_kind_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let err = wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": pid, "actor_id": "bot", "kind": "agent" }),
        )
        .await
        .unwrap_err();

        // "agent" is not a valid ActorKind — AI agents route through x-wos-ai.
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn add_actor_duplicate_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": pid, "actor_id": "reviewer" }),
        )
        .await
        .unwrap();

        let err = wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": pid, "actor_id": "reviewer" }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── wos_add_actor_extension ───────────────────────────────────────────

    #[tokio::test]
    async fn add_actor_extension_stores_value() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": pid, "actor_id": "reviewer" }),
        )
        .await
        .unwrap();

        let result = wos_add_actor_extension(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "actor_id": "reviewer",
                "key": "x-department",
                "value": "finance"
            }),
        )
        .await
        .unwrap();

        assert_eq!(result["actor_id"], json!("reviewer"));
        assert_eq!(result["key"], json!("x-department"));

        let project = registry.get(&pid).unwrap();
        let actor = &project.snapshot().actors[0];
        assert_eq!(actor.extensions["x-department"], json!("finance"));
    }

    #[tokio::test]
    async fn add_actor_extension_bad_key_prefix_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": pid, "actor_id": "reviewer" }),
        )
        .await
        .unwrap();

        let err = wos_add_actor_extension(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "actor_id": "reviewer",
                "key": "department",
                "value": "finance"
            }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn add_actor_extension_unknown_actor_errors() {
        let mut registry = ProjectRegistry::new();
        let pid = create_project(&mut registry);

        let err = wos_add_actor_extension(
            &mut registry,
            "",
            json!({
                "project_id": pid,
                "actor_id": "ghost",
                "key": "x-dept",
                "value": "finance"
            }),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    // ── require_string_arg error classification ───────────────────────────

    /// Absent key must produce `MissingArgument`, not `InvalidArguments`.
    #[tokio::test]
    async fn missing_project_id_gives_missing_argument_error() {
        let mut registry = ProjectRegistry::new();
        // No project_id in args at all.
        let err = wos_add_actor(&mut registry, "", json!({ "actor_id": "a1" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::MissingArgument(_)),
            "absent key must yield MissingArgument; got {err:?}"
        );
    }

    /// Present key with wrong type must produce `InvalidArguments`, not
    /// `MissingArgument`.
    #[tokio::test]
    async fn wrong_type_project_id_gives_invalid_arguments_error() {
        let mut registry = ProjectRegistry::new();
        // project_id is a boolean, not a string.
        let err = wos_add_actor(
            &mut registry,
            "",
            json!({ "project_id": true, "actor_id": "a1" }),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, ToolError::InvalidArguments(_)),
            "wrong-type key must yield InvalidArguments; got {err:?}"
        );
    }

    // ── Test helpers ──────────────────────────────────────────────────────

    fn create_project(registry: &mut ProjectRegistry) -> String {
        let project = wos_authoring::WosProject::new_kernel();
        registry.insert(project).unwrap().to_string()
    }
}
