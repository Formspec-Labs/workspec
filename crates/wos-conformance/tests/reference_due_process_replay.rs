// Rust guideline compliant 2026-02-21
//! Authored stable recipes execute the actual signed governed native journey.
use serde_json::{Value, json};
use wos_conformance::studio_api::{
    ReferenceReplayAdapters, ReplayRequest, prepare_reference_replay, replay_reference,
};
fn inputs() -> (ReplayRequest, ReferenceReplayAdapters) {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/reference-due-process-replay.json")).unwrap();
    (
        serde_json::from_value(original["request"].clone()).unwrap(),
        serde_json::from_value(original["adapters"].clone()).unwrap(),
    )
}

#[test]
fn only_exact_empty_custody_forms_admit_and_preserve_distinct_bytes() {
    let (request, adapters) = inputs();
    let mut identities = Vec::new();
    for custody in [json!({}), json!({"anchorRequirements": []})] {
        let mut request = request.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        candidate["custody"] = custody;
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let prepared = prepare_reference_replay(request, adapters.clone()).unwrap();
        identities.push(prepared.identity().to_owned());
        assert!(replay_reference(&prepared).unwrap().passed);
    }
    assert_ne!(identities[0], identities[1]);
    for custody in [
        Value::Null,
        json!({"anchorRequirements": null}),
        json!({"anchorRequirements": {}}),
        json!({"anchorRequirements": [{"on": "effect", "anchor": "required"}]}),
        json!({"trustProfileRef": ""}),
        json!({"exportBundleRef": ""}),
        json!({"unknown": []}),
    ] {
        let mut request = request.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        candidate["custody"] = custody;
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(prepare_reference_replay(request, adapters.clone()).is_err());
    }
}

fn service_inputs() -> (ReplayRequest, ReferenceReplayAdapters) {
    let (mut request, mut adapters) = inputs();
    let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
    candidate["caseFile"]["fields"]["external"] = json!({"type":"number"});
    candidate["caseFile"]["fields"]["fraud"] = json!({"type":"string"});
    let actions = candidate["lifecycle"]["states"]["pending"]["onEntry"]
        .as_array_mut()
        .unwrap();
    actions.push(json!({"action":"invokeService","serviceRef":"income-service","idempotencyKey":"income-one"}));
    actions.push(json!({"action":"invokeService","serviceRef":"fraud-policy"}));
    candidate["extensions"] = json!({"x-wos-reference-invocations":{"profile":"referenceInvocations/1","bindings":[
        {"id":"income-service","kind":"requestResponse","descriptor":{"kind":"openapi","documentRef":"urn:fixture:income-openapi","operationRef":"verifyIncome"},"inputMapping":{"probe":"42"},"outputBinding":{"caseFile.external":"$.verified"}},
        {"id":"fraud-policy","kind":"opaPolicy","descriptor":{"kind":"opa","endpoint":"urn:fixture:fraud-opa"},"inputMapping":{"income":"caseFile.income"},"outputBinding":{"caseFile.fraud":"$.decision"}}
    ]}});
    request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
    adapters.invocations=serde_json::from_value(json!([
        {"id":"income-fixture","serviceRef":"income-service","expectedInput":{"probe":42},"response":{"verified":145},"maxInvocations":1},
        {"id":"fraud-fixture","serviceRef":"fraud-policy","expectedInput":{"income":150},"response":{"result":true,"reasons":[],"obligations":[]},"maxInvocations":1}
    ])).unwrap();
    (request, adapters)
}

#[test]
fn selected_service_and_opa_execute_native_handlers_and_retain_actual_associations() {
    let (request, adapters) = service_inputs();
    let prepared = prepare_reference_replay(request, adapters).unwrap();
    let actual = replay_reference(&prepared).unwrap();
    assert!(actual.passed, "{actual:#?}");
    let saved = actual.final_snapshot.unwrap();
    assert_eq!(saved.case_state["external"], json!(145));
    assert_eq!(saved.case_state["fraud"], json!("allow"));
    assert_eq!(saved.step_results.len(), 2);
    let association = saved
        .step_results
        .iter()
        .find(|step| step.service_ref == "income-service")
        .unwrap()
        .invocation_association
        .as_ref()
        .unwrap();
    assert_eq!(association.input, json!({"probe":42}));
    assert!(!association.adapter_identity.is_empty());
}

#[test]
fn selected_service_cannot_grant_capabilities_or_write_protected_effects() {
    let (request, adapters) = service_inputs();
    for mutate in 0..4 {
        let mut request = request.clone();
        let mut adapters = adapters.clone();
        let mut candidate: Value = serde_json::from_slice(&request.candidate_bytes).unwrap();
        match mutate {
            0 => {
                candidate["extensions"]["x-wos-reference-invocations"]["bindings"][0]["outputBinding"] =
                    json!({"caseFile.level":"$.verified"})
            }
            1 => adapters.invocations[1].response = json!({"decision":"permit"}),
            2 => {
                adapters.invocations.pop();
            }
            _ => {
                candidate["extensions"]["x-wos-reference-invocations"]["bindings"][0]["descriptor"]
                    ["kind"] = json!("opa")
            }
        }
        request.candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(
            prepare_reference_replay(request, adapters).is_err(),
            "mutation {mutate}"
        );
    }
    let (request, mut adapters) = service_inputs();
    adapters.invocations[0].expected_input = json!({"probe":43});
    let actual = replay_reference(&prepare_reference_replay(request, adapters).unwrap()).unwrap();
    assert!(!actual.passed);
    assert!(
        actual
            .final_snapshot
            .is_none_or(|saved| saved.case_state.get("external").is_none_or(Value::is_null))
    );
}

#[test]
fn declared_native_time_advances_retain_original_bytes_and_execute_real_clock() {
    use wos_conformance::studio_api::NativeTimeAdvance;
    let (mut request, mut adapters) = inputs();
    let mut fixture: Value = serde_json::from_str(&request.fixture_json).unwrap();
    fixture["event_sequence"][4]
        .as_object_mut()
        .unwrap()
        .remove("delay");
    request.fixture_json = fixture.to_string();
    let original = request.fixture_json.clone();
    adapters.time_advances.push(NativeTimeAdvance {
        before_event_index: 4,
        duration: "P10D".into(),
    });
    let prepared = prepare_reference_replay(request.clone(), adapters.clone()).unwrap();
    assert_eq!(prepared.request().fixture_json, original);
    let actual = replay_reference(&prepared).unwrap();
    assert!(actual.passed, "{actual:#?}");
    assert!(
        actual
            .inputs
            .iter()
            .any(|input| input.role == "loweredFixture")
    );
    assert!(
        actual
            .attempts
            .iter()
            .any(|attempt| attempt.kind == "clock" && attempt.clock_ms == 864_000_000)
    );
    for bad in ["P0D", "P01D", "PT24H", "P99999999999999999999999D"] {
        adapters.time_advances[0].duration = bad.into();
        assert!(prepare_reference_replay(request.clone(), adapters.clone()).is_err());
    }
    adapters.time_advances[0].duration = "P10D".into();
    adapters.time_advances[0].before_event_index = 8;
    assert!(prepare_reference_replay(request.clone(), adapters.clone()).is_err());
    adapters.time_advances[0].before_event_index = 4;
    adapters
        .time_advances
        .push(adapters.time_advances[0].clone());
    assert!(prepare_reference_replay(request, adapters).is_err());
    let (request, mut adapters) = inputs();
    adapters.time_advances.push(NativeTimeAdvance {
        before_event_index: 4,
        duration: "P10D".into(),
    });
    assert!(prepare_reference_replay(request, adapters).is_err());
}

#[test]
fn actual_subject_selectors_require_native_facts_and_preserve_task_artifact() {
    use wos_conformance::studio_api::{
        NativeComparison, NativeSelector, ObservationExpectation, compare_observation,
    };
    let (request, adapters) = inputs();
    let actual = replay_reference(&prepare_reference_replay(request, adapters).unwrap()).unwrap();
    let selector:NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":1,"kind":"taskSubmission","occurrence":0},"subject":{"kind":"task","taskRef":"supervisor-signature","taskOccurrence":0}})).unwrap();
    assert!(matches!(
        compare_observation(
            &actual,
            &selector,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("completed")
            }
        ),
        NativeComparison::Pass
    ));
    let NativeComparison::Fail { difference } = compare_observation(
        &actual,
        &selector,
        &ObservationExpectation {
            pointer: "/status".into(),
            expected: json!("invented-status"),
        },
    ) else {
        panic!("independent wrong expectation must fail");
    };
    assert_eq!(difference.actual, Some(json!("completed")));
    assert_eq!(difference.expected, json!("invented-status"));
    assert!(matches!(
        compare_observation(
            &actual,
            &selector,
            &ObservationExpectation {
                pointer: "/acceptedArtifact/response/data/affirmed".into(),
                expected: json!(true)
            }
        ),
        NativeComparison::Pass
    ));
    let mut missing = selector.clone();
    missing.attempt.occurrence = 1;
    assert!(matches!(
        compare_observation(
            &actual,
            &missing,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("completed")
            }
        ),
        NativeComparison::Refused { .. }
    ));
    let notice:NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":2,"kind":"governanceOperation","occurrence":0},"subject":{"kind":"notice","target":{"bindingRef":"adverse","firingOccurrence":0}}})).unwrap();
    assert!(matches!(
        compare_observation(
            &actual,
            &notice,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("qualified")
            }
        ),
        NativeComparison::Pass
    ));
    let wrong_kind:NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":2,"kind":"event","occurrence":0},"subject":{"kind":"notice","target":{"bindingRef":"adverse","firingOccurrence":0}}})).unwrap();
    assert!(matches!(
        compare_observation(
            &actual,
            &wrong_kind,
            &ObservationExpectation {
                pointer: "/status".into(),
                expected: json!("qualified")
            }
        ),
        NativeComparison::Refused { .. }
    ));
}

#[test]
fn recipe_and_adapter_forgery_refuse_before_execution() {
    let (request, adapters) = inputs();
    for mutate in 0..4 {
        let mut request = request.clone();
        let mut adapters = adapters.clone();
        let mut fixture: Value = serde_json::from_str(&request.fixture_json).unwrap();
        match mutate {
            0 => fixture["event_sequence"][2]["governance_recipe"]["target"] = Value::Null,
            1 => {
                fixture["event_sequence"][1]["task_submission"]["signature_recipe"]["signature"]["verified"] =
                    json!(true)
            }
            2 => {
                adapters.fixture_signing_capabilities[0].receipt_pkcs8_base64 = adapters
                    .fixture_signing_capabilities[0]
                    .pkcs8_base64
                    .clone()
            }
            _ => {
                fixture["event_sequence"][1]["task_submission"]["response"]["data"]["signatureTaskId"] =
                    json!("predicted-task")
            }
        }
        request.fixture_json = fixture.to_string();
        assert!(
            prepare_reference_replay(request, adapters).is_err(),
            "mutation {mutate}"
        );
    }
}
#[test]
fn actual_signed_reference_recipe_reaches_applied_after_real_hearing() {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/reference-due-process-replay.json")).unwrap();
    let request: ReplayRequest = serde_json::from_value(original["request"].clone()).unwrap();
    let adapters: ReferenceReplayAdapters =
        serde_json::from_value(original["adapters"].clone()).unwrap();
    let prepared = prepare_reference_replay(request, adapters).unwrap();
    let actual = replay_reference(&prepared).unwrap();
    if let Ok(path) = std::env::var("WOS_REFERENCE_REPLAY_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
    }
    assert!(
        actual.passed,
        "{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );
    assert_eq!(
        actual.execution["notifications"],
        "selected deterministic local fixture transport; native qualified-receipt admission; no hosted/provider delivery qualification"
    );
    assert_eq!(actual.execution["hosted"], false);
    assert_eq!(actual.execution["durable"], false);
    let saved = actual.final_snapshot.unwrap().governance_state;
    assert_eq!(
        saved["dueProcess"]["notices"][0]["status"],
        json!("qualified")
    );
    assert_eq!(
        saved["dueProcess"]["appeals"][0]["hearing"]["reviewerHuman"],
        json!("actor:human:reviewer")
    );
    assert!(actual.attempts[1].recipe_resolution.as_ref().unwrap()["signedResponse"]["authoredSignatures"].is_array());
}
