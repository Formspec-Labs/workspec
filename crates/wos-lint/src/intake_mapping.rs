// Rust guideline compliant 2026-02-21
//! Shared qualification of the protected initial-case Mapping input.

use std::sync::LazyLock;

use serde_json::{Value, json};

static SOURCE_CONTEXT: LazyLock<formspec_lint::MappingSourceContext> =
    LazyLock::new(build_source_context);

/// Builds WOS's host-owned input shape, separate from the Mapping's Definition identity.
///
/// The source reader still owns full accepted-evidence validation. Opaque retained
/// evidence is declared at its original member; nested answer paths are checked
/// against the genuine selected Definition by Formspec, never a generated one.
///
/// # Panics
/// Panics if a bundled owner schema contains invalid JSON or lacks its declared properties.
pub fn source_context() -> formspec_lint::MappingSourceContext {
    SOURCE_CONTEXT.clone()
}

fn build_source_context() -> formspec_lint::MappingSourceContext {
    let evidence: Value =
        serde_json::from_str(include_str!("../../../schemas/api/case-file.schema.json"))
            .expect("canonical accepted evidence schema is valid JSON");
    let handoff: Value = serde_json::from_str(include_str!(
        "../../../../formspec/schemas/intake-handoff.schema.json"
    ))
    .expect("canonical handoff schema is valid JSON");
    let response: Value = serde_json::from_str(include_str!(
        "../../../../formspec/schemas/response.schema.json"
    ))
    .expect("canonical response schema is valid JSON");
    let mut schema = declared_shape(&evidence["$defs"]["AcceptedIntakeEvidence"]);
    schema["$id"] = json!("https://wos-spec.org/schemas/initial-intake-mapping-context/1.0");
    // These three descriptor schemas are the only WOS-local references reachable
    // from accepted evidence. Read models and HTTP resources remain separate.
    schema["$defs"] = json!({
        "AcceptedAuthoredArtifact":declared_shape(&evidence["$defs"]["AcceptedAuthoredArtifact"]),
        "AcceptedSignatureVerification":declared_shape(&evidence["$defs"]["AcceptedSignatureVerification"]),
        "AcceptedAttachmentDescriptor":declared_shape(&evidence["$defs"]["AcceptedAttachmentDescriptor"]),
        "Handoff": local_resource(handoff, "#/$defs/Handoff"),
        "Response": local_resource(response, "#/$defs/Response"),
    });
    schema["properties"]["handoff"] = json!({"$ref":"#/$defs/Handoff"});
    schema["properties"]["response"] = json!({"$ref":"#/$defs/Response"});
    schema["properties"]["caseReservation"] = json!({"type":"object","required":["caseLedgerId","caseReference"],
        "properties":{"caseLedgerId":{"type":"string","minLength":1},"caseReference":{"type":"string","minLength":1}},"additionalProperties":false});
    // The source reader already qualifies the complete evidence record. This
    // checker requires the protected inputs used to bind and execute a Mapping.
    schema["required"] = json!(["handoff", "response", "definition", "caseReservation"]);
    formspec_lint::MappingSourceContext {
        schema,
        data_path: "response.data".into(),
    }
}

// Reuse declared property/item shapes without reimplementing the source reader's
// conditional validation. This is a host path schema, not a second source admission.
fn declared_shape(schema: &Value) -> Value {
    let mut shape = serde_json::Map::new();
    for keyword in ["type", "$ref", "additionalProperties"] {
        if let Some(value) = schema.get(keyword) {
            shape.insert(keyword.into(), value.clone());
        }
    }
    for keyword in ["properties", "$defs"] {
        if let Some(values) = schema.get(keyword).and_then(Value::as_object) {
            shape.insert(
                keyword.into(),
                Value::Object(
                    values
                        .iter()
                        .map(|(key, value)| (key.clone(), declared_shape(value)))
                        .collect(),
                ),
            );
        }
    }
    if let Some(items) = schema.get("items") {
        shape.insert("items".into(), declared_shape(items));
    }
    if let Some(alternatives) = schema.get("oneOf").and_then(Value::as_array) {
        shape.insert(
            "oneOf".into(),
            Value::Array(alternatives.iter().map(declared_shape).collect()),
        );
    }
    Value::Object(shape)
}

// Bundle owner-declared shapes into one resource for the existing path resolver.
// Only schema fragment references change; source documents remain immutable.
fn local_resource(schema: Value, root: &str) -> Value {
    let mut schema = declared_shape(&schema);
    fn relocate(value: &mut Value, root: &str) {
        match value {
            Value::Object(object) => {
                if let Some(Value::String(reference)) = object.get_mut("$ref")
                    && let Some(fragment) = reference.strip_prefix('#')
                {
                    *reference = format!("{root}{fragment}");
                }
                for child in object.values_mut() {
                    relocate(child, root);
                }
            }
            Value::Array(values) => {
                for child in values {
                    relocate(child, root);
                }
            }
            _ => {}
        }
    }
    schema
        .as_object_mut()
        .expect("canonical schema is an object")
        .remove("$id");
    relocate(&mut schema, root);
    schema
}

/// Checks the immutable Mapping against its actual Definition and protected input representation.
pub fn lint(mapping: &Value, definition: &Value) -> formspec_lint::LintResult {
    formspec_lint::lint_with_options(
        mapping,
        &formspec_lint::LintOptions {
            mode: formspec_lint::LintMode::Runtime,
            definition_document: Some(definition.clone()),
            mapping_source_context: Some(source_context()),
            ..Default::default()
        },
    )
}

/// Checks protected input bindings before initial-case Mapping execution.
///
/// The caller adds its own reservation after refusing a source-supplied one.
/// Original evidence hashes and authorization remain the source reader's responsibility.
///
/// # Errors
/// Refuses missing context, foreign handoffs/responses, and differing Definition pins.
pub fn validate_input(
    input: &Value,
    definition_url: &str,
    definition_version: &str,
    response_ref: &str,
) -> Result<(), String> {
    for member in ["handoff", "response", "definition", "caseReservation"] {
        if !input.get(member).is_some_and(Value::is_object) {
            return Err(format!(
                "initial case Mapping input /{member} must be a protected object"
            ));
        }
    }
    formspec_lint::validate_mapping_source(&source_context().schema, input)?;
    for (path, expected) in [
        ("/handoff/definitionRef/url", definition_url),
        ("/handoff/definitionRef/version", definition_version),
        ("/response/definitionUrl", definition_url),
        ("/response/definitionVersion", definition_version),
        ("/definition/url", definition_url),
        ("/definition/version", definition_version),
        ("/definition/document/url", definition_url),
        ("/definition/document/version", definition_version),
        ("/handoff/responseRef", response_ref),
    ] {
        if input.pointer(path).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "initial case Mapping input {path} differs from the protected selection"
            ));
        }
    }
    if input["definition"]["kind"] != "definition" {
        return Err("initial case Mapping input /definition/kind is not definition".into());
    }
    Ok(())
}
