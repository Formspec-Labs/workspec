//! Native Workflow impact classification and agents convenience edits.
use crate::{errors::ToolError, registry::ProjectRegistry};
use serde_json::Value;
use wos_authoring::ImpactLevel;

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

fn require_string_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    match args.get(key) {
        None => Err(ToolError::MissingArgument(key.to_string())),
        Some(v) => v.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
            ToolError::InvalidArguments(format!("'{key}' must be a non-empty string"))
        }),
    }
}

/// Extract an optional array of strings, defaulting to empty if absent.
fn parse_string_array(args: &Value, key: &str) -> Result<Vec<String>, ToolError> {
    match args.get(key) {
        None => serde_json::from_value(
            crate::catalog::default_argument("wos_add_ai_agent", key)
                .expect("declared capabilities default"),
        )
        .map_err(|error| ToolError::Internal(error.to_string())),
        Some(v) => {
            let arr = v
                .as_array()
                .ok_or_else(|| ToolError::InvalidArguments(format!("'{key}' must be an array")))?;
            arr.iter()
                .map(|item| {
                    item.as_str().map(str::to_string).ok_or_else(|| {
                        ToolError::InvalidArguments(format!("'{key}' must be an array of strings"))
                    })
                })
                .collect()
        }
    }
}

/// Parse the `level` string into an `ImpactLevel`.
///
/// Accepts the kebab-case variants used by the kernel spec:
/// `rights-impacting`, `safety-impacting`, `operational`, `informational`.
fn parse_impact_level(value: &str) -> Result<ImpactLevel, ToolError> {
    serde_json::from_value(serde_json::json!(value))
        .map_err(|error| ToolError::InvalidArguments(error.to_string()))
}
