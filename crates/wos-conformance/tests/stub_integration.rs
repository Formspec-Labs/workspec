// Rust guideline compliant 2026-04-14

//! Regression tests for exercised contract and service stubs.
//! Also houses NB.3 CloudEvents conformance tests that require inspecting
//! provenance records beyond what fixture assertions support.

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

// ── NB.3 CloudEvents binding tests ──────────────────────────────

/// INT-EMIT-001: event-emit binding fires and emits EventEmitted provenance.
#[test]
fn int_emit_001_event_emitted_provenance_present() {
    let fixture_json = include_str!("fixtures/INT-EMIT-001-happy.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-EMIT-001 failed:\n{}",
        result.failures.join("\n")
    );

    let emitted_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::EventEmitted)
        .count();
    assert_eq!(emitted_count, 1, "expected exactly one EventEmitted record");
}

/// INT-EMIT-002: EventEmitted provenance contains the full CE envelope fields.
#[test]
fn int_emit_002_full_envelope_captured_in_provenance() {
    let fixture_json = include_str!("fixtures/INT-EMIT-002-envelope-captured.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-EMIT-002 failed:\n{}",
        result.failures.join("\n")
    );

    let emitted = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::EventEmitted)
        .expect("EventEmitted provenance must be present");

    let data = emitted.data.as_ref().expect("EventEmitted must carry data");
    assert_eq!(data.get("specversion").and_then(|v| v.as_str()), Some("1.0"));
    assert_eq!(
        data.get("source").and_then(|v| v.as_str()),
        Some("https://example.com/orders")
    );
    assert_eq!(
        data.get("type").and_then(|v| v.as_str()),
        Some("com.example.order.placed")
    );
    // id and subject must also be present
    assert!(data.get("id").is_some(), "envelope must carry id");
    assert!(data.get("subject").is_some(), "envelope must carry subject");
}

/// INT-EMIT-003: Custom subject override is used instead of the default format.
#[test]
fn int_emit_003_custom_subject_override() {
    let fixture_json = include_str!("fixtures/INT-EMIT-003-custom-subject.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-EMIT-003 failed:\n{}",
        result.failures.join("\n")
    );

    let emitted = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::EventEmitted)
        .expect("EventEmitted provenance must be present");

    let subject = emitted
        .data
        .as_ref()
        .and_then(|d| d.get("subject"))
        .and_then(|v| v.as_str())
        .expect("EventEmitted must carry subject");

    assert_eq!(subject, "custom/subject/override");
}

/// INT-CONSUME-001: inbound CloudEvent consumed, output binding applied, EventConsumed emitted.
#[test]
fn int_consume_001_event_consumed_and_state_updated() {
    let fixture_json = include_str!("fixtures/INT-CONSUME-001-happy.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-CONSUME-001 failed:\n{}",
        result.failures.join("\n")
    );

    let consumed = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::EventConsumed)
        .expect("EventConsumed provenance must be present");

    let data = consumed.data.as_ref().expect("EventConsumed must carry data");
    assert_eq!(data.get("id").and_then(|v| v.as_str()), Some("evt-external-001"));
    assert_eq!(data.get("specversion").and_then(|v| v.as_str()), Some("1.0"));
}

/// INT-CONSUME-002: inbound CloudEvent with empty id is rejected — engine errors with EventIngressInvalid.
#[test]
fn int_consume_002_empty_id_rejected_at_binding_boundary() {
    let fixture_json = include_str!("fixtures/INT-CONSUME-002-missing-id-rejected.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir);

    match result {
        Err(err) => {
            let msg = err.to_string();
            // May contain either the ingress-invalid marker OR the "empty required field" message.
            assert!(
                msg.contains("EventIngressInvalid") || msg.contains("empty required field"),
                "expected ingress rejection in error, got: {msg}"
            );
        }
        Ok(result) => {
            // If the fixture unexpectedly passes (no error), the transition must
            // not have occurred — no case-state update should have applied.
            let consumed_count = result
                .provenance
                .iter()
                .filter(|p| p.record_kind == ProvenanceKind::EventConsumed)
                .count();
            assert_eq!(
                consumed_count, 0,
                "rejected event must not produce EventConsumed provenance"
            );
        }
    }
}

/// INT-CALLBACK-001: outbound fires → CallbackPending; inbound resolves → CallbackReceived.
#[test]
fn int_callback_001_correlation_outbound_then_inbound() {
    let fixture_json = include_str!("fixtures/INT-CALLBACK-001-correlation.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-CALLBACK-001 failed:\n{}",
        result.failures.join("\n")
    );

    let pending = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::CallbackPending)
        .expect("CallbackPending provenance must be present");
    let received = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::CallbackReceived)
        .expect("CallbackReceived provenance must be present");

    let pending_subject = pending
        .data
        .as_ref()
        .and_then(|d| d.get("subject"))
        .and_then(|v| v.as_str())
        .expect("CallbackPending must carry subject");
    let received_subject = received
        .data
        .as_ref()
        .and_then(|d| d.get("correlationSubject"))
        .and_then(|v| v.as_str())
        .expect("CallbackReceived must carry correlationSubject");

    assert_eq!(
        pending_subject, received_subject,
        "pending and received subjects must match for correlation"
    );
}

/// INT-CALLBACK-002: full lifecycle — CallbackPending then CallbackReceived, state updated.
#[test]
fn int_callback_002_pending_to_received_lifecycle() {
    let fixture_json = include_str!("fixtures/INT-CALLBACK-002-pending-to-received.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-CALLBACK-002 failed:\n{}",
        result.failures.join("\n")
    );

    // Verify ordering: CallbackPending appears before CallbackReceived.
    let pending_pos = result
        .provenance
        .iter()
        .position(|p| p.record_kind == ProvenanceKind::CallbackPending)
        .expect("CallbackPending must be present");
    let received_pos = result
        .provenance
        .iter()
        .position(|p| p.record_kind == ProvenanceKind::CallbackReceived)
        .expect("CallbackReceived must be present");

    assert!(
        pending_pos < received_pos,
        "CallbackPending must appear before CallbackReceived in the provenance stream"
    );
}

/// INT-CALLBACK-003: inbound event with unknown subject is silently dropped, no case-state change.
#[test]
fn int_callback_003_uncorrelated_inbound_is_dropped() {
    let fixture_json = include_str!("fixtures/INT-CALLBACK-003-uncorrelated-drop.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-CALLBACK-003 failed:\n{}",
        result.failures.join("\n")
    );

    // Must not produce a CallbackReceived record for an unmatched subject.
    let received_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::CallbackReceived)
        .count();
    assert_eq!(
        received_count, 0,
        "unmatched inbound event must not produce CallbackReceived provenance"
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
