// Rust guideline compliant 2026-02-21

//! Regression tests for exercised contract and service stubs.

use wos_conformance::{ProvenanceKind, run_fixture};

#[test]
fn k031_contract_outcomes_are_emitted_in_contract_validation_provenance() {
    let fixture_json = include_str!("fixtures/k-031-contract-structured-results.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    let record = result
        .provenance
        .iter()
        .find(|record| record.record_kind == ProvenanceKind::ContractValidation)
        .expect("contract validation provenance should be present");
    let errors = record
        .data
        .as_ref()
        .and_then(|data| data.get("errors"))
        .and_then(serde_json::Value::as_array)
        .expect("contract validation provenance should carry structured errors");

    assert_eq!(
        errors,
        &vec![
            serde_json::json!("income: required field missing"),
            serde_json::json!("dependents: must be non-negative")
        ]
    );
}

#[test]
fn k024_step_result_persistence_comes_from_executed_service_invocation() {
    let fixture_json = include_str!("fixtures/k-024-persist-before-advance.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    let record = result
        .provenance
        .iter()
        .find(|record| record.record_kind == ProvenanceKind::StepResultPersisted)
        .expect("step-result persistence provenance should be present");

    assert_eq!(
        record.data.as_ref().and_then(|data| data.get("serviceRef")),
        Some(&serde_json::json!("verificationSystem"))
    );
}
