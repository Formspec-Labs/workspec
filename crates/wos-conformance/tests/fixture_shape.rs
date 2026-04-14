// Rust guideline compliant 2026-04-14

//! Fixture schema shape tests — verify deserialization defaults.

use wos_conformance::ConformanceFixture;

#[test]
fn fixture_defaults_binding_to_conformance() {
    let json = serde_json::json!({
        "id": "test",
        "rule": "test",
        "description": "test fixture",
        "documents": { "kernel": "inline" },
        "initial_case_state": {},
        "event_sequence": [],
        "expected_transitions": [],
        "expected_provenance": [],
    });
    let fx: ConformanceFixture = serde_json::from_value(json).unwrap();
    assert_eq!(fx.binding.as_deref(), Some("conformance"));
}

#[test]
fn fixture_accepts_formspec_binding() {
    let json = serde_json::json!({
        "binding": "formspec",
        "id": "test",
        "rule": "test",
        "description": "test fixture",
        "documents": { "kernel": "inline" },
        "initial_case_state": {},
        "event_sequence": [],
        "expected_transitions": [],
        "expected_provenance": [],
    });
    let fx: ConformanceFixture = serde_json::from_value(json).unwrap();
    assert_eq!(fx.binding.as_deref(), Some("formspec"));
}
