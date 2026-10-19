//! Owner-schema fragments used by authoring payloads and the MCP declarations.

use crate::AuthoringDiagnostic;
use serde_json::{Value, json};
use std::sync::LazyLock;

/// Schema resolution is local even when dependency features enable HTTP retrieval.
struct OfflineRetriever;
impl jsonschema::Retrieve for OfflineRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!("schema resource is not distributed locally: {uri}").into())
    }
}

static WORKFLOW_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../schemas/wos-workflow.schema.json"))
        .expect("bundled workflow schema is JSON")
});

/// Borrow the owning Workflow schema without maintaining a second field list.
pub fn workflow_schema() -> &'static Value {
    &WORKFLOW_SCHEMA
}

/// Reference a fragment in the owning Workflow schema.
pub fn fragment_reference(pointer: &str) -> Value {
    json!({"$ref":format!("{}#{pointer}", workflow_schema()["$id"].as_str().expect("owning schema has an id"))})
}

/// Validate a payload against one owning-schema fragment.
///
/// # Errors
/// Returns a diagnostic for an unknown fragment or invalid payload.
pub fn validate_fragment(pointer: &str, value: &Value) -> Result<(), AuthoringDiagnostic> {
    if workflow_schema().pointer(pointer).is_none() {
        return Err(AuthoringDiagnostic::error(
            pointer,
            "unknown owning schema fragment",
        ));
    }
    let id = workflow_schema()["$id"]
        .as_str()
        .expect("owning schema has an id");
    let registry = jsonschema::Registry::new()
        .retriever(OfflineRetriever)
        .add(id, workflow_schema().clone())
        .expect("bundled schema resource is valid")
        .prepare()
        .expect("bundled schema references resolve");
    let validator = jsonschema::options()
        .with_registry(&registry)
        .build(&fragment_reference(pointer))
        .expect("owning schema fragment compiles");
    if let Some(cause) = validator.iter_errors(value).next() {
        return Err(AuthoringDiagnostic::error(pointer, cause.to_string()));
    }
    Ok(())
}

/// Validate one declared top-level field as an incremental payload.
///
/// # Errors
/// Returns a diagnostic for an unknown field or invalid payload.
pub fn validate_workflow_field(field: &str, value: &Value) -> Result<(), AuthoringDiagnostic> {
    let pointer = format!(
        "/properties/{}",
        field.replace('~', "~0").replace('/', "~1")
    );
    validate_fragment(&pointer, value)
}

/// Native action phases supported by the state-action edit.
///
/// The names come from the owning State schema rather than another key list.
pub fn state_action_phases() -> Vec<&'static str> {
    workflow_schema()["$defs"]["State"]["properties"]
        .as_object()
        .expect("State properties are declared")
        .iter()
        .filter(|(name, definition)| {
            name.starts_with("on")
                && definition.get("type").and_then(Value::as_str) == Some("array")
        })
        .map(|(name, _)| name.as_str())
        .collect()
}
