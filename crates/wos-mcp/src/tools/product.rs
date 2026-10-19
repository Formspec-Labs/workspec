//! Small native edits; payload shapes are owned by the Workflow schema.
use crate::{errors::ToolError, registry::ProjectRegistry};
use serde_json::{Value, json};
use wos_authoring::{AuthoringResult, WosProject};

fn project<'a>(
    registry: &'a mut ProjectRegistry,
    args: &Value,
) -> Result<&'a mut WosProject, ToolError> {
    registry.get_mut(string(args, "project_id")?)
}
fn string<'a>(args: &'a Value, name: &str) -> Result<&'a str, ToolError> {
    args.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::MissingArgument(name.into()))
}
fn applied(result: AuthoringResult) -> Result<Value, ToolError> {
    result
        .map(|result| json!({"applied":result.label}))
        .map_err(|error| ToolError::InvalidArguments(error.message))
}
pub async fn metadata(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    let fields = args["fields"]
        .as_object()
        .ok_or_else(|| ToolError::InvalidArguments("fields must be an object".into()))?
        .clone();
    applied(project(registry, &args)?.set_native_fields(fields))
}
pub async fn case_field(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(
        project(registry, &args)?
            .put_case_field(string(&args, "name")?.into(), args["definition"].clone()),
    )
}
pub async fn contract(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(
        project(registry, &args)?
            .put_contract(string(&args, "name")?.into(), args["contract"].clone()),
    )
}
pub async fn state(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(
        project(registry, &args)?
            .put_state(string(&args, "pointer")?.into(), args["state"].clone()),
    )
}
pub async fn state_action(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(project(registry, &args)?.append_state_action(
        string(&args, "pointer")?.into(),
        string(&args, "phase")?.into(),
        args["value"].clone(),
    ))
}
pub async fn governance(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(
        project(registry, &args)?.set_native_fields(serde_json::Map::from_iter([(
            "governance".into(),
            args["value"].clone(),
        )])),
    )
}
pub async fn agent(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(project(registry, &args)?.put_agent(args["value"].clone()))
}
pub async fn remove_agent(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(project(registry, &args)?.remove_agent(string(&args, "agent_id")?.into()))
}
pub async fn agent_constraint(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(project(registry, &args)?.add_agent_constraint(
        string(&args, "agent_id")?.into(),
        args["constraint"].clone(),
    ))
}
pub async fn ai_oversight(
    registry: &mut ProjectRegistry,
    _: &str,
    args: Value,
) -> Result<Value, ToolError> {
    applied(
        project(registry, &args)?.set_native_fields(serde_json::Map::from_iter([(
            "aiOversight".into(),
            args["value"].clone(),
        )])),
    )
}
