// Rust guideline compliant 2026-02-21

//! Round-trip deserialization tests for WOS Kernel Documents.
//!
//! Verifies that [`KernelDocument`] can deserialize every valid kernel
//! fixture without data loss. Each test loads a fixture, deserializes
//! it, and asserts key structural properties.

use std::fs;
use wos_core::KernelDocument;

/// Loads and deserializes a kernel fixture by filename.
fn load_fixture(name: &str) -> KernelDocument {
    let path = format!(
        "{}/fixtures/kernel/{name}",
        env!("CARGO_MANIFEST_DIR").replace("/crates/wos-core", "")
    );
    let json = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {name}: {e}"))
}

#[test]
fn purchase_order_approval_round_trips() {
    let doc = load_fixture("purchase-order-approval.json");
    assert_eq!(doc.wos_kernel, "1.0");
    assert_eq!(doc.title.as_deref(), Some("Purchase Order Approval"));
    assert_eq!(doc.impact_level, Some(wos_core::ImpactLevel::Operational));
    assert_eq!(doc.actors.len(), 3);
    assert_eq!(doc.lifecycle.initial_state, "submitted");
    assert!(doc.lifecycle.states.contains_key("submitted"));
    assert!(doc.lifecycle.states.contains_key("approved"));
    assert!(doc.lifecycle.states.contains_key("rejected"));
    assert!(doc.contracts.contains_key("purchaseOrderForm"));
    assert_eq!(doc.contracts["purchaseOrderForm"].binding, "formspec");
    assert!(doc.execution.is_some());
    let exec = doc.execution.as_ref().unwrap();
    assert_eq!(exec.workflow_timeout.as_deref(), Some("P90D"));
}

#[test]
fn benefits_adjudication_round_trips() {
    let doc = load_fixture("benefits-adjudication.json");
    assert_eq!(doc.wos_kernel, "1.0");
    assert_eq!(
        doc.impact_level,
        Some(wos_core::ImpactLevel::RightsImpacting)
    );
    assert!(!doc.actors.is_empty());
    assert!(doc.lifecycle.states.contains_key("intake"));
    assert!(doc.contracts.contains_key("applicationForm"));
    assert!(doc.execution.is_some());
}

#[test]
fn medicaid_redetermination_round_trips() {
    let doc = load_fixture("medicaid-redetermination.json");
    assert_eq!(doc.wos_kernel, "1.0");
    assert_eq!(
        doc.impact_level,
        Some(wos_core::ImpactLevel::RightsImpacting)
    );
    assert!(doc.lifecycle.states.len() > 5);
    assert!(doc.contracts.contains_key("applicationForm"));
}

#[test]
fn case_relationship_appeal_round_trips() {
    let doc = load_fixture("case-relationship-appeal.json");
    assert_eq!(doc.wos_kernel, "1.0");
    let case_file = doc.case_file.as_ref().expect("case_file present");
    assert!(
        !case_file.relationships.is_empty(),
        "should have case relationships"
    );
}

#[test]
fn new_phase2_fields_round_trip() {
    // Verify evaluationMode and maxRelationshipEventDepth deserialize correctly.
    let json = r#"{
        "$wosKernel": "1.0",
        "evaluationMode": "continuous",
        "maxRelationshipEventDepth": 5,
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": { "type": "atomic" },
                "end": { "type": "final" }
            }
        }
    }"#;
    let doc: wos_core::KernelDocument = serde_json::from_str(json).unwrap();
    assert_eq!(
        doc.evaluation_mode,
        Some(wos_core::EvaluationMode::Continuous)
    );
    assert_eq!(doc.max_relationship_event_depth, Some(5));
}

#[test]
fn evaluation_mode_defaults_absent() {
    // When evaluationMode is absent, it should be None (default event-driven).
    let json = r#"{
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "s",
            "states": { "s": { "type": "atomic" } }
        }
    }"#;
    let doc: wos_core::KernelDocument = serde_json::from_str(json).unwrap();
    assert!(doc.evaluation_mode.is_none());
    assert!(doc.max_relationship_event_depth.is_none());
}

#[test]
fn contract_reference_typed() {
    let json = r#"{
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "s",
            "states": { "s": { "type": "atomic" } }
        },
        "contracts": {
            "myForm": {
                "binding": "formspec",
                "ref": "urn:formspec:test:1.0",
                "description": "Test contract"
            }
        },
        "execution": {
            "workflowTimeout": "P90D",
            "compensable": true
        }
    }"#;
    let doc: wos_core::KernelDocument = serde_json::from_str(json).unwrap();
    let contract = &doc.contracts["myForm"];
    assert_eq!(contract.binding, "formspec");
    assert_eq!(contract.reference, "urn:formspec:test:1.0");
    assert_eq!(contract.description.as_deref(), Some("Test contract"));
    let exec = doc.execution.as_ref().unwrap();
    assert!(exec.compensable);
}

#[test]
fn non_kernel_fixtures_do_not_parse() {
    // These files in fixtures/kernel/ are NOT KernelDocuments.
    // Verify they fail to parse as KernelDocument.
    let non_kernel = [
        "invalid-documents.json",
        "benefits-correspondence-metadata.json",
        "purchase-order-provenance.json",
    ];
    for name in non_kernel {
        let path = format!(
            "{}/fixtures/kernel/{name}",
            env!("CARGO_MANIFEST_DIR").replace("/crates/wos-core", "")
        );
        let json = fs::read_to_string(&path).unwrap();
        let result: Result<KernelDocument, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "{name} should not parse as KernelDocument");
    }
}
