// Rust guideline compliant 2026-02-21

//! Actual in-memory WosRuntime policy controls on labeled supplied output.
//!
//! The harness supplies output, never an AgentInvoker result or validation/
//! human-review/governed-commit proof. Native observations remain independent
//! from scenario expectations and preserve blocked/refused attempts.
use serde_json::{Value, json};
use wos_conformance::studio_api::{ReplayRequest, prepare_replay, replay};

fn workflow(rules: Value) -> Value {
    json!({"$wosWorkflow":"1.0","url":"urn:test:native-runtime-policy","version":"1.0.0","title":"Native runtime policy control","impactLevel":"operational","actors":[{"id":"assessor","type":"agent"}],"agents":[{"id":"assessor","agentType":"deterministic","modelIdentifier":"admitted-rules-model","modelVersion":"7.0.0","deonticConstraints":rules}],"aiOversight":{},"lifecycle":{"initialState":"open","states":{"open":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"decide"},"target":"done"}]},"done":{"type":"final"}}}})
}
fn run(
    workflow: Value,
    data: Value,
    expected_advance: bool,
    case: Value,
) -> wos_conformance::studio_api::ReplayResult {
    let fixture = json!({"id":"native-supplied-output","rule":"control","description":"Independent native policy state expectation","documents":{},"initial_case_state":case,"event_sequence":[{"event":"decide","actor":"assessor","data":data}],"expected_transitions":if expected_advance{json!([{"from":"open","target":"done","event":"decide"}])}else{json!([])}});
    let request = ReplayRequest {
        candidate_bytes: serde_json::to_vec(&workflow).unwrap(),
        dependencies: vec![],
        fixture_json: fixture.to_string(),
        clock_start_ms: 1234,
        required_observations: vec!["aiPolicy".into()],
        expectations: vec![],
    };
    replay(&prepare_replay(request).unwrap()).unwrap()
}
fn data(output: Value) -> Value {
    json!({"output":output,"confidenceReport":{"overall":1.0,"method":"declared"}})
}
#[test]
fn native_runtime_enforces_permission_scope_bound_and_candidate_mutation() {
    let rules = json!([{"id":"amount-only","kind":"permission","allowedFields":["/amount"],"bounds":"output.amount <= 100","onViolation":"reject"}]);
    let good = run(
        workflow(rules.clone()),
        data(json!({"amount":100})),
        true,
        json!({"retained":42}),
    );
    assert!(good.passed, "{good:?}");
    assert_eq!(
        good.final_snapshot.as_ref().unwrap().configuration,
        vec!["done"]
    );
    let outside = run(
        workflow(rules.clone()),
        data(json!({"amount":101})),
        false,
        json!({"retained":42}),
    );
    assert_eq!(
        outside.final_snapshot.as_ref().unwrap().configuration,
        vec!["open"]
    );
    assert_eq!(
        outside.attempts[0].drains[0]
            .policy_evaluation
            .as_ref()
            .unwrap()
            .disposition(),
        "reject"
    );
    let field = run(
        workflow(rules.clone()),
        data(json!({"amount":100,"forbidden":{}})),
        false,
        json!({"retained":42}),
    );
    assert_eq!(
        field.final_snapshot.as_ref().unwrap().configuration,
        vec!["open"]
    );
    assert_eq!(
        field.final_snapshot.as_ref().unwrap().case_state,
        json!({"retained":42})
    );
    assert!(field.provenance.as_ref().unwrap().iter().any(|record| {
        record.data.as_ref().is_some_and(|v| {
            v["constraintId"] == "amount-only" && v["pointer"] == "/agents/0/deonticConstraints/0"
        })
    }));
    let mut changed = workflow(rules);
    changed["agents"][0]["deonticConstraints"][0]["bounds"] = json!("output.amount <= 101");
    let changed = run(
        changed,
        data(json!({"amount":101})),
        true,
        json!({"retained":42}),
    );
    assert_eq!(
        changed.final_snapshot.as_ref().unwrap().configuration,
        vec!["done"]
    );
    assert_ne!(changed.identity, outside.identity);
}
#[test]
fn independent_output_vectors_observe_real_runtime_suppression_or_refusal() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../wos-core/tests/fixtures/native-policy-vectors.json"
    ))
    .unwrap();
    let mut exercised = 0;
    for vector in vectors["cases"].as_array().unwrap() {
        if vector["phase"] != "output" || vector.get("actionFloor").is_some() {
            continue;
        }
        let mut candidate = workflow(vector.get("agentRules").unwrap_or(&vector["rules"]).clone());
        candidate["impactLevel"] = vector
            .get("impactLevel")
            .cloned()
            .unwrap_or(json!("operational"));
        if let Some(rules) = vector.get("workflowRules") {
            candidate["aiOversight"]["deonticConstraints"] = rules.clone();
        }
        if let Some(floor) = vector.get("workflowFloor") {
            candidate["aiOversight"]["confidenceFloor"] = floor.clone();
        }
        let mut payload = json!({"output":vector["output"]});
        if let Some(report) = vector.get("confidenceReport") {
            payload["confidenceReport"] = report.clone();
        }
        if let Some(report) = vector.get("confidence") {
            payload["confidence"] = report.clone();
        }
        if let Some(waiver) = vector.get("waiver") {
            payload["bypass"] = waiver.clone();
        }
        let expected = vector["expected"].as_str().unwrap();
        let advance = matches!(expected, "pass" | "flag");
        let observed = run(candidate, payload, advance, json!({"retained":42}));
        assert_eq!(
            observed.final_snapshot.as_ref().unwrap().configuration,
            vec![if advance { "done" } else { "open" }],
            "{}: {observed:?}",
            vector["id"]
        );
        assert_eq!(
            observed.final_snapshot.as_ref().unwrap().case_state,
            json!({"retained":42})
        );
        let stage = observed.attempts[0].drains[0]
            .policy_evaluation
            .as_ref()
            .unwrap();
        match expected {
            "escalateToHuman" | "switchToAssistive" => {
                assert_eq!(stage.disposition(), "unsupported", "{}", vector["id"]);
                assert_eq!(observed.status, "refused");
                assert!(observed.attempts[0].drains[0].created_task_ids.is_empty());
            }
            _ => assert_eq!(stage.disposition(), expected, "{}: {stage:?}", vector["id"]),
        }
        exercised += 1;
    }
    assert_eq!(exercised, 52);
}
#[test]
fn input_rights_use_native_case_values_and_refuse_missing_invocation_input() {
    for value in [json!(false), json!(0), json!("")] {
        let result = run(
            workflow(json!([{"id":"context","kind":"right","entitlement":"/caseFile/context"}])),
            data(json!({})),
            true,
            json!({"context":value}),
        );
        assert!(result.passed, "{result:?}");
        assert!(result.provenance.as_ref().unwrap().iter().any(|record| {
            record
                .data
                .as_ref()
                .is_some_and(|v| v["constraintId"] == "context" && v["outcome"] == "present")
        }));
    }
    let absent = run(
        workflow(json!([{"id":"context","kind":"right","entitlement":"/caseFile/context"}])),
        data(json!({})),
        false,
        json!({}),
    );
    assert_eq!(
        absent.attempts[0].drains[0]
            .policy_evaluation
            .as_ref()
            .unwrap()
            .disposition(),
        "processorError"
    );
    assert_eq!(
        absent.final_snapshot.as_ref().unwrap().configuration,
        vec!["open"]
    );
    let mut payload = data(json!({}));
    payload["input"] = json!({"application":true});
    let missing = run(
        workflow(json!([{"id":"input","kind":"right","entitlement":"/input/application"}])),
        payload,
        false,
        json!({}),
    );
    assert_eq!(missing.status, "refused");
    assert!(
        missing
            .refusals
            .iter()
            .any(|r| r.contains("input context unavailable"))
    );
}
#[test]
fn runtime_event_hints_cannot_manufacture_ai_execution_evidence() {
    use wos_core::ProvenanceKind;
    let forbidden = [
        ProvenanceKind::AgentOutput,
        ProvenanceKind::AgentProvenanceAnnotation,
        ProvenanceKind::FallbackAttempt,
        ProvenanceKind::FallbackTerminal,
        ProvenanceKind::AgentStateTransition,
        ProvenanceKind::ProxyInvocation,
    ];
    for (key, value) in [
        ("serviceOutput", json!({"claimed":"persisted"})),
        ("pipeline", json!({"valid":true})),
        ("delegation", json!({"authorized":true})),
        ("verification", json!({"verified":true})),
        ("error", json!("unavailable")),
        ("contractValidation", json!({"valid":true})),
        (
            "constraintModification",
            json!({"targetConstraint":"no-denial"}),
        ),
        ("agentStateChange", json!({"target":"active"})),
        ("driftAlert", json!({"policyRef":"claimed"})),
        ("escalationActive", json!(true)),
        ("invocationSource", json!("claimed-proxy")),
    ] {
        let mut payload = data(json!({}));
        payload[key] = value;
        let result = run(workflow(json!([])), payload, false, json!({"retained":42}));
        assert_eq!(result.status, "refused", "{key}: {result:?}");
        assert_eq!(
            result.final_snapshot.as_ref().unwrap().case_state,
            json!({"retained":42})
        );
        assert!(
            result
                .provenance
                .as_ref()
                .unwrap()
                .iter()
                .all(|record| !forbidden.contains(&record.record_kind)),
            "{key}"
        );
        assert!(result.attempts[0].drains[0].created_task_ids.is_empty());
    }
    for payload in [
        json!({"serviceOutput":{"claimed":"persisted"}}),
        json!({"contractValidation":{"valid":true}}),
        json!({"appealFiled":{"reviewer":"claimed"}}),
        json!({}),
    ] {
        let observed = run(
            workflow(json!([])),
            payload.clone(),
            false,
            json!({"retained":42}),
        );
        assert_eq!(observed.status, "refused", "{payload}: {observed:?}");
        assert!(
            observed
                .provenance
                .as_ref()
                .unwrap()
                .iter()
                .all(|record| !forbidden.contains(&record.record_kind))
        );
        assert!(observed.attempts[0].drains[0].created_task_ids.is_empty());
        assert_eq!(
            observed.final_snapshot.as_ref().unwrap().case_state,
            json!({"retained":42})
        );
    }
    let mut payload = data(json!({}));
    payload["modelIdentifier"] = json!("forged-model");
    payload["modelVersion"] = json!("999");
    let result = run(workflow(json!([])), payload, false, json!({}));
    assert_eq!(result.status, "refused");
    assert!(
        result
            .provenance
            .as_ref()
            .unwrap()
            .iter()
            .all(|record| !forbidden.contains(&record.record_kind))
    );
    assert!(result.provenance.as_ref().unwrap().iter().any(|record| {
        record.data.as_ref().is_some_and(|v| {
            v["modelIdentifier"] == "admitted-rules-model"
                && v["modelVersion"] == "7.0.0"
                && v["executionReadinessAssessed"] == false
        })
    }));
}
#[test]
fn native_supplied_output_refuses_a_case_write_route_and_missing_action_context() {
    let mut candidate = workflow(json!([]));
    candidate["lifecycle"]["states"]["open"]["transitions"][0]["actions"] =
        json!([{"action":"setData","path":"caseFile.written","value":"unsafe"}]);
    let result = run(
        candidate,
        data(json!({"written":"unsafe"})),
        false,
        json!({"retained":42}),
    );
    assert_eq!(result.status, "refused");
    assert_eq!(
        result.final_snapshot.as_ref().unwrap().case_state,
        json!({"retained":42})
    );
    let mut candidate = workflow(json!([]));
    candidate["lifecycle"]["states"]["done"]["onEntry"] =
        json!([{"action":"emitEvent","event":"site"}]);
    candidate["agents"][0]["actionOverrides"] = json!([{"actionRef":"/lifecycle/states/done/onEntry/0","deonticConstraints":[{"id":"site-rule","kind":"prohibition","condition":"true","onViolation":"reject"}]}]);
    let mut payload = data(json!({}));
    payload["actionRef"] = json!("/lifecycle/states/done/onEntry/0");
    let result = run(candidate, payload, false, json!({"retained":42}));
    assert_eq!(result.status, "refused");
    assert_eq!(
        result.final_snapshot.as_ref().unwrap().configuration,
        vec!["open"]
    );
}

#[test]
fn shared_native_replay_corpus_matches_literal_expectations() {
    let corpus: Value =
        serde_json::from_str(include_str!("../fixtures/native-ai-replay/vectors.json")).unwrap();
    for vector in corpus["vectors"].as_array().unwrap() {
        assert_eq!(
            vector["dependencies"],
            json!([]),
            "this corpus declares no external resolver inputs"
        );
        let result = replay(
            &prepare_replay(ReplayRequest {
                candidate_bytes: serde_json::to_vec(&vector["candidate"]).unwrap(),
                dependencies: vec![],
                fixture_json: vector["fixture"].to_string(),
                clock_start_ms: 1234,
                required_observations: serde_json::from_value(
                    vector["requiredObservations"].clone(),
                )
                .unwrap(),
                expectations: serde_json::from_value(vector["expectations"].clone()).unwrap(),
            })
            .unwrap(),
        )
        .unwrap();
        assert!(
            result.execution["agentOutputs"]
                .as_str()
                .unwrap()
                .contains("native policy stage only")
        );
        assert!(result.attempts[0].drains[0].created_task_ids.is_empty());
        let summary = result
            .provenance
            .as_ref()
            .unwrap()
            .iter()
            .find_map(|record| {
                record
                    .data
                    .as_ref()
                    .filter(|data| data["exercise"] == "suppliedOutput")
            })
            .unwrap();
        assert_eq!(summary["executionReadinessAssessed"], false);
        assert_eq!(
            summary["unexecutedStages"],
            json!([
                "invocation",
                "inputDefinitionValidation",
                "outputDefinitionValidation",
                "humanReview",
                "governedCommit"
            ])
        );
        assert_eq!(
            result.status, vector["expected"]["status"],
            "{}: {result:?}",
            vector["id"]
        );
        assert_eq!(
            result.passed, vector["expected"]["passed"],
            "{}",
            vector["id"]
        );
        let snapshot = result.final_snapshot.as_ref().unwrap();
        assert_eq!(
            serde_json::to_value(&snapshot.configuration).unwrap(),
            vector["expected"]["configuration"],
            "{}",
            vector["id"]
        );
        assert_eq!(
            snapshot.case_state, vector["expected"]["caseState"],
            "{}",
            vector["id"]
        );
        assert_eq!(
            result.attempts[0].drains[0]
                .policy_evaluation
                .as_ref()
                .unwrap()
                .disposition(),
            vector["expected"]["policyDisposition"],
            "{}",
            vector["id"]
        );
    }
}

#[test]
fn reported_method_is_retained_but_cannot_establish_required_calibration() {
    // AI §7.1 makes the label untrusted reporting; §7.2 requires calibration
    // for modelNative and defines calibrated/conformal as calibration methods.
    for (method, expected) in [
        ("declared", "pass"),
        ("heuristic", "pass"),
        ("modelNative", "unsupported"),
        ("calibrated", "unsupported"),
        ("conformal", "unsupported"),
    ] {
        let mut candidate = workflow(json!([]));
        candidate["aiOversight"]["confidenceFloor"] =
            json!({"threshold":0.8,"onViolation":"reject"});
        let observed = run(
            candidate,
            json!({"output":{"amount":1},"confidenceReport":{"overall":0.9,"method":method}}),
            expected == "pass",
            json!({"retained":42}),
        );
        let stage = observed.attempts[0].drains[0]
            .policy_evaluation
            .as_ref()
            .unwrap();
        assert_eq!(stage.disposition(), expected, "{method}");
        assert_eq!(
            serde_json::to_value(stage.confidence_report.as_ref().unwrap()).unwrap(),
            json!({"overall":0.9,"method":method})
        );
        assert!(observed.attempts[0].drains[0].created_task_ids.is_empty());
        assert_eq!(
            observed.final_snapshot.as_ref().unwrap().case_state,
            json!({"retained":42})
        );
    }
}

#[test]
fn authored_typed_and_documented_stored_events_keep_the_same_actual_dispatch() {
    // Kernel §4.5 permits stored string coercion; authored documents use typed
    // events. This proves runtime dispatch, not complete Workflow admission.
    let pair: Value = serde_json::from_str(include_str!(
        "../fixtures/native-ai-replay/event-shapes.json"
    ))
    .unwrap();
    let mut identities = Vec::new();
    for key in ["candidate", "storedCandidate"] {
        let source = pair[key].clone();
        let admitted = wos_core::ai_policy::load_native_ai_policy(&source)
            .unwrap()
            .unwrap();
        assert_eq!(admitted.source(), &source);
        let result = replay(
            &prepare_replay(ReplayRequest {
                candidate_bytes: serde_json::to_vec(&source).unwrap(),
                dependencies: vec![],
                fixture_json: pair["fixture"].to_string(),
                clock_start_ms: 1234,
                required_observations: vec!["aiPolicy".into()],
                expectations: vec![],
            })
            .unwrap(),
        )
        .unwrap();
        assert!(result.passed, "{key}: {result:?}");
        let snapshot = result.final_snapshot.as_ref().unwrap();
        assert_eq!(
            serde_json::to_value(&snapshot.configuration).unwrap(),
            pair["expectedConfiguration"]
        );
        assert_eq!(snapshot.case_state, pair["expectedCaseState"]);
        assert_eq!(
            result.attempts[0].drains[0]
                .policy_evaluation
                .as_ref()
                .unwrap()
                .disposition(),
            "pass"
        );
        assert!(result.attempts[0].drains[0].created_task_ids.is_empty());
        identities.push(result.identity);
    }
    assert_ne!(
        identities[0], identities[1],
        "original source shape stays part of replay identity"
    );
}
