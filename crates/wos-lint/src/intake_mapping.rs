// Rust guideline compliant 2026-02-21
//! Shared qualification of the protected initial-case Mapping input.

use std::sync::LazyLock;

use serde_json::Value;

static SOURCE_CONTEXT: LazyLock<formspec_lint::MappingSourceContext> = LazyLock::new(|| {
    let context: InitialIntakeMappingContext = serde_json::from_str(include_str!(
        "../../../schemas/lint/initial-intake-mapping-context.json"
    ))
    .expect("bundled initial intake Mapping context is valid JSON");
    formspec_lint::MappingSourceContext {
        schema: context.schema,
        data_path: context.data_path,
    }
});

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitialIntakeMappingContext {
    schema: Value,
    data_path: String,
}

/// Loads the WOS input shape used to check initial-case Mapping paths.
///
/// The source reader still owns full accepted-evidence validation. Opaque retained
/// evidence is declared at its original member; nested answer paths are checked
/// against the genuine selected Definition by Formspec, never a generated one.
///
/// # Panics
/// Panics if the bundled context is invalid JSON or lacks its declared fields.
pub fn source_context() -> formspec_lint::MappingSourceContext {
    SOURCE_CONTEXT.clone()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn initial_intake_accepts_a_response_with_a_warning_explanation() {
        let url = "https://example.org/intake";
        let version = "1.0.0";
        let response_ref = "urn:example:response:1";
        let input = json!({
            "handoff": {"definitionRef": {"url": url, "version": version}, "responseRef": response_ref},
            "response": {
                "definitionUrl": url, "definitionVersion": version, "data": {"amount": 10},
                "explanations": [{
                    "shapeId": "amountWarning", "path": "amount", "text": "Estimate confirmed",
                    "basis": format!("sha256:{}", "a".repeat(64)),
                    "author": {"id": "respondent"}, "recorded": "2026-01-01T00:00:00Z"
                }]
            },
            "definition": {"kind": "definition", "url": url, "version": version,
                           "document": {"url": url, "version": version}},
            "caseReservation": {"caseLedgerId": "ledger-1", "caseReference": "case-1"}
        });
        validate_input(&input, url, version, response_ref)
            .expect("accept the Response explanation");
    }

    #[test]
    fn initial_intake_mapping_context_matches_owner_schemas() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/lint/initial-intake-mapping-context.json");
        let rebuilt = build_source_context();
        let expected = json!({"schema": rebuilt.schema, "dataPath": rebuilt.data_path});
        let generated = serde_json::to_string_pretty(&expected).expect("serialize context") + "\n";
        let write = std::env::var_os("WOS_WRITE_INITIAL_INTAKE_MAPPING_CONTEXT").is_some();
        if write {
            std::fs::create_dir_all(path.parent().expect("context path has a parent"))
                .expect("create the context directory");
            std::fs::write(&path, &generated).expect("write the checked-in context");
        }
        let checked_in = std::fs::read_to_string(path).expect("read the checked-in context");
        assert_eq!(checked_in, generated);
        // Write mode can replace the file after the embedded bytes were compiled.
        if !write {
            let loaded = source_context();
            assert_eq!(loaded.schema, rebuilt.schema);
            assert_eq!(loaded.data_path, rebuilt.data_path);
        }
    }
}
