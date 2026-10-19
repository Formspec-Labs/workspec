// Rust guideline compliant 2026-02-21
//! Native effects remain distinct from ordinary event data and unsupported work.

#[path = "support/provenance_controls.rs"]
mod controls;

use serde_json::{Value, json};
use wos_conformance::studio_api::{ObservationExpectation, prepare_replay, replay};

#[test]
fn literal_operation_controls_observe_saved_effects_state_and_provenance() {
    let corpus = controls::corpus();
    for vector in corpus["vectors"].as_array().unwrap() {
        let prepared = prepare_replay(controls::request(vector));
        if let Some(reason) = vector["expected"]["preparationRefusalContains"].as_str() {
            let error = prepared.unwrap_err().to_string();
            assert!(error.contains(reason), "{}: {error}", vector["id"]);
            controls::retain_observation("shared", vector, &json!({"preparationError":error}));
            continue;
        }
        let result = serde_json::to_value(replay(&prepared.unwrap()).unwrap()).unwrap();
        controls::retain_observation("shared", vector, &result);
        controls::assert_observations(vector, &result);
    }
}

#[test]
fn missing_service_effect_stays_missing_and_cannot_be_filled_from_expectation() {
    let corpus = controls::corpus();
    let vector = &corpus["vectors"][0];
    let mut request = controls::request(vector);
    request.expectations.push(ObservationExpectation {
        pointer: "/finalSnapshot/stepResults/0/output".into(),
        expected: json!({"forged":true}),
    });
    let result = replay(&prepare_replay(request).unwrap()).unwrap();
    assert_eq!(result.status, "refused");
    assert_eq!(
        serde_json::to_value(result.final_snapshot).unwrap()["stepResults"],
        json!([])
    );
    assert_eq!(result.differences[0].actual, None);
    assert!(
        result
            .refusals
            .iter()
            .any(|reason| reason.contains("missing-observation"))
    );
}

#[test]
fn required_governance_on_selected_migration_target_refuses_before_initialization() {
    let corpus = controls::corpus();
    let mut vector = corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|vector| vector["id"] == "native-migration")
        .unwrap()
        .clone();
    vector["dependencies"][0]["document"]["governance"] =
        json!({"dueProcess":{"adverseDecisionPolicy":{"noticeRequired":true}}});
    let error = prepare_replay(controls::request(&vector))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unsupported-policy-changing-migration"),
        "{error}"
    );
    // The shared facade already refuses policy-changing migration. Exercise the
    // native selected-version admission too, without weakening that frozen gate.
    let mut native = vector["fixture"].clone();
    native["documents"] = json!({"kernel":"inline","kernel@2.0.0":"inline"});
    native["inline_documents"] =
        json!({"kernel":vector["candidate"],"kernel@2.0.0":vector["dependencies"][0]["document"]});
    let fixture = serde_json::from_value(native).unwrap();
    let error = wos_conformance::WorkflowEngine::new(&fixture)
        .err()
        .expect("unsupported selected target")
        .to_string();
    assert!(error.contains("noticeRequired"), "{error}");
}

#[test]
fn false_guard_ordinary_adverse_name_cannot_create_a_notice_without_an_operation() {
    let corpus = controls::corpus();
    let mut vector = corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|vector| vector["id"] == "false-guard-no-notice")
        .unwrap()
        .clone();
    vector["candidate"]
        .as_object_mut()
        .unwrap()
        .remove("governance");
    vector["expected"] = json!({"status":"pass","configuration":["open"],"caseState":{"evidence":"before"},"stepResults":[],"provenanceCounts":{"noticeSent":0}});
    let result: Value =
        serde_json::to_value(replay(&prepare_replay(controls::request(&vector)).unwrap()).unwrap())
            .unwrap();
    controls::assert_observations(&vector, &result);
    assert!(
        result["attempts"][0]["drains"][0]["guardEvaluations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|guard| guard["result"] == false)
    );
}

#[test]
fn full_governance_profiles_cannot_be_verified_by_empty_fixtures_or_caller_rule_lists() {
    use wos_conformance::{
        ClaimStatus, ProcessorClaims, ProcessorManifest, run_profile_against_fixtures,
        verify_processor_manifest,
    };
    let root = tempfile::tempdir().unwrap();
    let corpus = controls::corpus();
    let candidate = &corpus["vectors"][0]["candidate"];
    for rule in [
        "G-002", "G-006", "G-007", "G-010", "G-016", "G-017", "G-018",
    ] {
        let fixture = json!({"id":rule,"rule":rule,"description":"Empty successful fixture is not positive governance evidence","documents":{"kernel":"inline"},"inline_documents":{"kernel":candidate},"event_sequence":[],"expected_transitions":[]});
        std::fs::write(
            root.path().join(format!("{rule}.json")),
            fixture.to_string(),
        )
        .unwrap();
    }
    for name in ["Governance Basic", "Governance Complete"] {
        for required in [None, Some(&[] as &[&str]), Some(&["G-007"] as &[&str])] {
            let (passed, message) =
                run_profile_against_fixtures(root.path(), name, |_| true, required);
            assert!(!passed, "{name}: {message}");
            assert!(message.contains("unsupported"), "{message}");
        }
    }
    let report = verify_processor_manifest(
        &ProcessorManifest {
            processor_name: "label attack".into(),
            claims: ProcessorClaims {
                governance_basic: true,
                governance_complete: true,
                ..Default::default()
            },
            ..Default::default()
        },
        root.path(),
    )
    .unwrap();
    for rule in ["G-051", "G-052"] {
        assert_eq!(report.claim(rule).unwrap().status, ClaimStatus::Failed);
        let metadata = wos_conformance::rules::all_rules()
            .iter()
            .find(|metadata| metadata.id == rule)
            .unwrap();
        assert!(metadata.fixtures.is_empty());
        assert!(matches!(
            metadata.graduation,
            wos_conformance::rules::Graduation::Draft
        ));
    }
}

#[test]
fn descriptive_and_empty_governance_does_not_block_ordinary_native_actions() {
    let corpus = controls::corpus();
    let mut vector = corpus["vectors"][0].clone();
    vector["candidate"]["governance"] = json!({"dueProcess":{"description":"Metadata, no configured operation"},"reviewProtocols":[],"pipelines":[],"holds":{},"taskCatalog":[{"verificationMethod":"source-grounding","slaDefinitions":[],"warningThresholds":[],"escalationChain":[],"breachPolicy":{}}],"policyParameters":{"description":"Static metadata","value":30,"parameters":{},"bindings":{}}});
    let result =
        serde_json::to_value(replay(&prepare_replay(controls::request(&vector)).unwrap()).unwrap())
            .unwrap();
    controls::assert_observations(&vector, &result);
}

#[test]
fn ordinary_business_data_never_manufactures_completed_operation_records() {
    let corpus = controls::corpus();
    let vector = &corpus["vectors"][0];
    let result = replay(&prepare_replay(controls::request(vector)).unwrap()).unwrap();
    let records = serde_json::to_value(result.provenance).unwrap();
    for (kind, count) in vector["expected"]["provenanceCounts"].as_object().unwrap() {
        let actual = records
            .as_array()
            .unwrap()
            .iter()
            .filter(|record| record["recordKind"] == *kind)
            .count();
        assert_eq!(
            actual,
            count.as_u64().unwrap() as usize,
            "business data manufactured {kind}: {records}"
        );
    }
}

#[test]
fn ordinary_state_names_do_not_execute_compensation() {
    let corpus = controls::corpus();
    let vector = corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|vector| vector["id"] == "ordinary-compensation-names")
        .unwrap();
    let result = replay(&prepare_replay(controls::request(vector)).unwrap()).unwrap();
    assert!(
        !result
            .provenance
            .as_ref()
            .unwrap()
            .iter()
            .any(|record| matches!(
                record.record_kind,
                wos_conformance::ProvenanceKind::CompensationExecuted
                    | wos_conformance::ProvenanceKind::CompensationScopeBoundary
            )),
        "ordinary state names manufactured compensation execution"
    );
}

#[test]
fn active_temporal_tables_refuse_while_static_metadata_remains_usable() {
    let corpus = controls::corpus();
    let table = json!({"threshold":{"resolutionDateRef":"caseFile.date","values":[{"effectiveDate":"2025-01-01","value":30}]}});
    for field in ["parameters", "bindings"] {
        for embedded in [false, true] {
            let mut vector = corpus["vectors"][0].clone();
            let mut governance = json!({});
            if embedded {
                governance["policyParameters"] = json!({field:table.clone()});
            } else {
                governance[field] = table.clone();
            }
            vector["candidate"]["governance"] = governance.clone();
            let error = prepare_replay(controls::request(&vector))
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("effective-date") && error.contains(field),
                "{error}"
            );
            // The selected companion role is extracted to its governance interior.
            // Use the native facade directly so selection cannot be bypassed by
            // shared preparation restrictions on unrelated companion dependencies.
            let mut fixture = corpus["vectors"][0]["fixture"].clone();
            fixture["documents"]["kernel"] = json!("inline");
            fixture["documents"]["policyParameters"] = json!("inline");
            fixture["inline_documents"]["kernel"] = corpus["vectors"][0]["candidate"].clone();
            fixture["inline_documents"]["policyParameters"] =
                json!({"$wosWorkflow":"1.0","governance":governance});
            let native = serde_json::from_value(fixture).unwrap();
            let error = wos_conformance::WorkflowEngine::new(&native)
                .err()
                .expect("selected temporal table refusal")
                .to_string();
            assert!(
                error.contains("/companions/policyParameters") && error.contains(field),
                "{error}"
            );
        }
    }
}
