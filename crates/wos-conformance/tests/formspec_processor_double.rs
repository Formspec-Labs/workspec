// Rust guideline compliant 2026-04-14

//! Tests for the FixtureFormspecProcessor test double.

use wos_conformance::formspec_processor::FixtureFormspecProcessor;
use wos_formspec_binding::FormspecProcessor;

#[test]
fn processor_validates_pinned_envelope() {
    let proc = FixtureFormspecProcessor::new("urn:fx:form", "1.0.0");
    let envelope = serde_json::json!({
        "status": "complete",
        "definitionUrl": "urn:fx:form",
        "definitionVersion": "1.0.0",
        "data": { "a": 1 }
    });
    let errs = proc.validate_envelope(&envelope).unwrap();
    assert!(errs.is_empty());
}

#[test]
fn processor_rejects_unpinned_envelope() {
    let proc = FixtureFormspecProcessor::new("urn:fx:form", "1.0.0");
    let envelope = serde_json::json!({ "status": "complete", "data": {} });
    let errs = proc.validate_envelope(&envelope).unwrap();
    assert!(errs.iter().any(|e| e["code"] == "envelope_missing_field"));
}

#[test]
fn processor_with_definition_errors_returns_canned_errors() {
    let proc = FixtureFormspecProcessor::with_definition_errors(
        "urn:fx:form",
        "1.0.0",
        vec![serde_json::json!({ "code": "definitionInvalid", "message": "missing field" })],
    );
    let result = proc
        .validate_definition("urn:fx:form", "1.0.0", &serde_json::json!({}))
        .unwrap();
    assert_eq!(
        result,
        Some(vec![
            serde_json::json!({ "code": "definitionInvalid", "message": "missing field" })
        ])
    );
}

#[test]
fn processor_without_definition_errors_returns_none() {
    let proc = FixtureFormspecProcessor::new("urn:fx:form", "1.0.0");
    let result = proc
        .validate_definition("urn:fx:form", "1.0.0", &serde_json::json!({}))
        .unwrap();
    assert_eq!(result, None);
}
