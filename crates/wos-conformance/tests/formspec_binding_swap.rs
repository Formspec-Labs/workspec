// Rust guideline compliant 2026-04-14

//! Integration test for binding swap based on fixture `binding` field.

use wos_conformance::ConformanceFixture;

fn minimal_fixture_with_binding(binding: &str) -> ConformanceFixture {
    let json = serde_json::json!({
        "binding": binding,
        "id": "binding-swap-test",
        "rule": "test",
        "description": "binding swap test",
        "documents": { "kernel": "inline" },
        "inline_documents": {
            "kernel": {
                "$wosWorkflow": "1.0",
                "url": "urn:test:kernel",
                "version": "1.0.0",
                "title": "Binding Swap Test",
                "description": "Minimal kernel for binding swap tests",
                "status": "active",
                "impactLevel": "operational",
                "actors": [{ "id": "worker", "type": "human", "description": "Test actor" }],
                "lifecycle": {
                    "initialState": "s0",
                    "states": {
                        "s0": { "type": "atomic", "transitions": [] }
                    }
                },
                "caseFile": { "fields": {} },
                "execution": {
                    "workflowTimeout": "P90D",
                    "defaultTaskTimeout": "P7D",
                    "instanceVersioning": "pinned"
                }
            }
        },
        "initial_case_state": {},
        "event_sequence": [],
        "expected_transitions": [],
        "expected_provenance": [],
    });
    serde_json::from_value(json).unwrap()
}

#[test]
fn engine_accepts_formspec_binding_fixture() {
    let fx = minimal_fixture_with_binding("formspec");
    let mut engine = wos_conformance::WorkflowEngine::new(&fx).expect("engine init");
    let result = engine.execute(&fx).expect("execute");
    assert_eq!(result.binding_used.as_deref(), Some("formspec"));
}

#[test]
fn engine_defaults_to_conformance_binding() {
    let fx = minimal_fixture_with_binding("conformance");
    let mut engine = wos_conformance::WorkflowEngine::new(&fx).expect("engine init");
    let result = engine.execute(&fx).expect("execute");
    assert_eq!(result.binding_used.as_deref(), Some("formspec"));
}
