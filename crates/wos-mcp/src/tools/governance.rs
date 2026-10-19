// Rust guideline compliant 2026-02-21
//! Native Workflow impact classification.
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

fn require_string_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    match args.get(key) {
        None => Err(ToolError::MissingArgument(key.to_string())),
        Some(v) => v.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
            ToolError::InvalidArguments(format!("'{key}' must be a non-empty string"))
        }),
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
