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
    assert_eq!(
        data.get("specversion").and_then(|v| v.as_str()),
        Some("1.0")
    );
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

    let data = consumed
        .data
        .as_ref()
        .expect("EventConsumed must carry data");
    assert_eq!(
        data.get("id").and_then(|v| v.as_str()),
        Some("evt-external-001")
    );
    assert_eq!(
        data.get("specversion").and_then(|v| v.as_str()),
        Some("1.0")
    );
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

// ── NB.4 Tool binding tests ──────────────────────────────────────

/// INT-TOOL-001: tool binding fires, emits ToolInvoked provenance, maps output to case state.
#[test]
fn int_tool_001_tool_invoked_provenance_present() {
    let fixture_json = include_str!("fixtures/INT-TOOL-001-happy.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-TOOL-001 failed:\n{}",
        result.failures.join("\n")
    );

    let invoked = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::ToolInvoked)
        .expect("ToolInvoked provenance must be present");

    let data = invoked.data.as_ref().expect("ToolInvoked must carry data");
    assert_eq!(
        data.get("toolId").and_then(|v| v.as_str()),
        Some("risk-analysis-v2")
    );
    assert_eq!(data.get("outcome").and_then(|v| v.as_str()), Some("ok"));
}

/// INT-TOOL-002: tool binding with response contract — ContractValidation provenance present.
#[test]
fn int_tool_002_contract_validation_on_tool_response() {
    let fixture_json = include_str!("fixtures/INT-TOOL-002-pin-mismatch.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-TOOL-002 failed:\n{}",
        result.failures.join("\n")
    );

    let contract_record = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::ContractValidation)
        .expect("ContractValidation provenance must be present");

    let data = contract_record.data.as_ref().expect("must carry data");
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("response"));
    assert_eq!(
        data.get("contractRef").and_then(|v| v.as_str()),
        Some("urn:test:scoring-response-v1")
    );
}

// ── NB.4 Arazzo sequence tests ───────────────────────────────────

/// INT-ARAZZO-001: 3-step sequence — all steps succeed, ArazzoStep records in order.
#[test]
fn int_arazzo_001_all_steps_succeed_in_order() {
    let fixture_json = include_str!("fixtures/INT-ARAZZO-001-happy.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-ARAZZO-001 failed:\n{}",
        result.failures.join("\n")
    );

    let arazzo_steps: Vec<_> = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::ArazzoStep)
        .collect();

    assert_eq!(arazzo_steps.len(), 3, "expected 3 ArazzoStep records");

    let step_ids: Vec<&str> = arazzo_steps
        .iter()
        .map(|p| {
            p.data
                .as_ref()
                .and_then(|d| d.get("stepId"))
                .and_then(|v| v.as_str())
                .expect("ArazzoStep must carry stepId")
        })
        .collect();

    assert_eq!(step_ids, vec!["validate", "transform", "persist"]);
}

/// INT-ARAZZO-002: mid-sequence failure — step 1 ok, step 2 failed, step 3 absent.
#[test]
fn int_arazzo_002_mid_sequence_failure_halts_sequence() {
    let fixture_json = include_str!("fixtures/INT-ARAZZO-002-mid-sequence-failure.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    // The fixture expects the transition to fail (no expected_transitions), so
    // run_fixture may return Ok with no transitions, or Err from the engine.
    let result = run_fixture(fixture_json, &base_dir);

    let provenance = match result {
        Ok(result) => result.provenance,
        Err(err) => {
            // Engine error is acceptable — the sequence halted.
            let msg = err.to_string();
            assert!(
                msg.contains("failed") || msg.contains("contract"),
                "unexpected error: {msg}"
            );
            return;
        }
    };

    // When the engine does not error out, verify the provenance stream.
    let step_records: Vec<_> = provenance
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::ArazzoStep)
        .collect();

    // step-one must be ok.
    let step_one = step_records
        .iter()
        .find(|p| {
            p.data
                .as_ref()
                .and_then(|d| d.get("stepId"))
                .and_then(|v| v.as_str())
                == Some("step-one")
        })
        .expect("step-one ArazzoStep must be present");
    assert_eq!(
        step_one
            .data
            .as_ref()
            .and_then(|d| d.get("outcome"))
            .and_then(|v| v.as_str()),
        Some("ok")
    );

    // step-two must be failed.
    let step_two = step_records
        .iter()
        .find(|p| {
            p.data
                .as_ref()
                .and_then(|d| d.get("stepId"))
                .and_then(|v| v.as_str())
                == Some("step-two")
        })
        .expect("step-two ArazzoStep must be present");
    assert_eq!(
        step_two
            .data
            .as_ref()
            .and_then(|d| d.get("outcome"))
            .and_then(|v| v.as_str()),
        Some("failed")
    );

    // step-three must be absent.
    let step_three_count = step_records
        .iter()
        .filter(|p| {
            p.data
                .as_ref()
                .and_then(|d| d.get("stepId"))
                .and_then(|v| v.as_str())
                == Some("step-three")
        })
        .count();
    assert_eq!(
        step_three_count, 0,
        "step-three must not be attempted after step-two failure"
    );
}

/// INT-ARAZZO-003: ArazzoStep records appear in step order, before binding-level DataMapping.
#[test]
fn int_arazzo_003_step_provenance_precedes_binding_level_data_mapping() {
    let fixture_json = include_str!("fixtures/INT-ARAZZO-003-step-provenance-ordering.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-ARAZZO-003 failed:\n{}",
        result.failures.join("\n")
    );

    // Find the last ArazzoStep position.
    let last_arazzo_pos = result
        .provenance
        .iter()
        .rposition(|p| p.record_kind == ProvenanceKind::ArazzoStep)
        .expect("ArazzoStep records must be present");

    // Find the binding-level DataMapping position.
    let binding_mapping_pos = result
        .provenance
        .iter()
        .position(|p| {
            p.record_kind == ProvenanceKind::DataMapping
                && p.data
                    .as_ref()
                    .and_then(|d| d.get("phase"))
                    .and_then(|v| v.as_str())
                    == Some("binding-level")
        })
        .expect("binding-level DataMapping must be present");

    assert!(
        last_arazzo_pos < binding_mapping_pos,
        "all ArazzoStep records must appear before the binding-level DataMapping record"
    );

    // Also verify step-a appears before step-b appears before step-c.
    let step_positions: Vec<(&str, usize)> = ["step-a", "step-b", "step-c"]
        .iter()
        .map(|&id| {
            let pos = result
                .provenance
                .iter()
                .position(|p| {
                    p.record_kind == ProvenanceKind::ArazzoStep
                        && p.data
                            .as_ref()
                            .and_then(|d| d.get("stepId"))
                            .and_then(|v| v.as_str())
                            == Some(id)
                })
                .unwrap_or_else(|| panic!("ArazzoStep for {id} must be present"));
            (id, pos)
        })
        .collect();

    assert!(
        step_positions[0].1 < step_positions[1].1,
        "step-a must appear before step-b"
    );
    assert!(
        step_positions[1].1 < step_positions[2].1,
        "step-b must appear before step-c"
    );
}

// ── NB.4 Policy-engine tests ─────────────────────────────────────

/// INT-POLICY-001: allow decision — PolicyDecision provenance with decision='allow'.
#[test]
fn int_policy_001_allow_decision_emits_provenance() {
    let fixture_json = include_str!("fixtures/INT-POLICY-001-allow.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-POLICY-001 failed:\n{}",
        result.failures.join("\n")
    );

    let policy_record = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::PolicyDecision)
        .expect("PolicyDecision provenance must be present");

    let data = policy_record.data.as_ref().expect("must carry data");
    assert_eq!(data.get("decision").and_then(|v| v.as_str()), Some("allow"));
    assert_eq!(data.get("reasonsCount").and_then(|v| v.as_u64()), Some(0));
}

/// INT-POLICY-002: deny decision with reasons — reasonsCount reflects the payload.
#[test]
fn int_policy_002_deny_decision_with_reasons() {
    let fixture_json = include_str!("fixtures/INT-POLICY-002-deny.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-POLICY-002 failed:\n{}",
        result.failures.join("\n")
    );

    let policy_record = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::PolicyDecision)
        .expect("PolicyDecision provenance must be present");

    let data = policy_record.data.as_ref().expect("must carry data");
    assert_eq!(data.get("decision").and_then(|v| v.as_str()), Some("deny"));
    assert_eq!(data.get("reasonsCount").and_then(|v| v.as_u64()), Some(2));
}

/// INT-POLICY-003: indeterminate — decision='indeterminate', not coerced to allow or deny.
#[test]
fn int_policy_003_indeterminate_not_coerced() {
    let fixture_json = include_str!("fixtures/INT-POLICY-003-indeterminate.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-POLICY-003 failed:\n{}",
        result.failures.join("\n")
    );

    let policy_record = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::PolicyDecision)
        .expect("PolicyDecision provenance must be present");

    let data = policy_record.data.as_ref().expect("must carry data");
    let decision = data.get("decision").and_then(|v| v.as_str());
    assert_eq!(decision, Some("indeterminate"));
    // Explicitly verify it was NOT coerced.
    assert_ne!(
        decision,
        Some("allow"),
        "indeterminate must not be coerced to allow"
    );
    assert_ne!(
        decision,
        Some("deny"),
        "indeterminate must not be coerced to deny"
    );
}

/// INT-POLICY-004: OPA adapter — engineType='opa', result:true normalizes to decision='allow'.
#[test]
fn int_policy_004_opa_adapter_normalizes_result_true_to_allow() {
    let fixture_json = include_str!("fixtures/INT-POLICY-004-opa-adapter.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-POLICY-004 failed:\n{}",
        result.failures.join("\n")
    );

    let policy_record = result
        .provenance
        .iter()
        .find(|p| p.record_kind == ProvenanceKind::PolicyDecision)
        .expect("PolicyDecision provenance must be present");

    let data = policy_record.data.as_ref().expect("must carry data");
    assert_eq!(data.get("engineType").and_then(|v| v.as_str()), Some("opa"));
    assert_eq!(data.get("decision").and_then(|v| v.as_str()), Some("allow"));
    assert_eq!(data.get("reasonsCount").and_then(|v| v.as_u64()), Some(1));
}

/// INT-POLICY-005: unknown engineType is rejected — the permissive wildcard fallback is gone.
#[test]
fn int_policy_005_unknown_engine_type_is_rejected() {
    let fixture_json = include_str!("fixtures/INT-POLICY-005-unknown-engine-type-rejected.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir);

    match result {
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("unknown engineType") || msg.contains("xacml"),
                "expected 'unknown engineType' or engine name in error, got: {msg}"
            );
        }
        Ok(ok) => {
            // If run_fixture did not propagate the engine error, the fixture must
            // have produced no PolicyDecision provenance — the binding must have failed.
            let policy_count = ok
                .provenance
                .iter()
                .filter(|p| p.record_kind == ProvenanceKind::PolicyDecision)
                .count();
            assert_eq!(
                policy_count, 0,
                "unknown engineType must not produce a PolicyDecision record"
            );
        }
    }
}

/// INT-CALLBACK-003 (tightened): verify the exact provenance counts after an uncorrelated drop.
///
/// The fixture fires outbound (→ 1 CallbackPending) then delivers an uncorrelated inbound.
/// After both events: CallbackPending count == 1 and CallbackReceived count == 0.
#[test]
fn int_callback_003_provenance_counts_after_uncorrelated_drop() {
    let fixture_json = include_str!("fixtures/INT-CALLBACK-003-uncorrelated-drop.json");
    let base_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));

    let result = run_fixture(fixture_json, &base_dir).expect("run_fixture failed");

    assert!(
        result.passed,
        "INT-CALLBACK-003 failed:\n{}",
        result.failures.join("\n")
    );

    let pending_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::CallbackPending)
        .count();
    assert_eq!(
        pending_count, 1,
        "exactly one CallbackPending expected (from the outbound fire only, not from the uncorrelated inbound)"
    );

    let received_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::CallbackReceived)
        .count();
    assert_eq!(
        received_count, 0,
        "uncorrelated inbound must not produce CallbackReceived provenance"
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
