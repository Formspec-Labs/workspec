// Rust guideline compliant 2026-02-21

//! Independently authored controls for the shared authoring replay API.

use serde_json::{Value, json};
use wos_conformance::studio_api::{ObservationExpectation, ReplayRequest, prepare_replay, replay};

fn candidate(guard: &str) -> Value {
    json!({"$wosWorkflow":"1.0","url":"urn:test:exact-replay","version":"1.0.0",
      "title":"Exact replay control","impactLevel":"operational","actors":[{"id":"reviewer","type":"human"}],
      "lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"decide"},"target":"done","guard":guard,
          "actions":[{"action":"setData","path":"caseFile.result","value":"accepted"}]}]},
        "done":{"type":"final"}}}})
}
fn fixture() -> Value {
    json!({"id":"independent-guard","rule":"control","description":"Reviewed success expectation",
      "documents":{},"event_sequence":[{"event":"decide","actor":"reviewer"}],
      "expected_transitions":[{"from":"open","target":"done","event":"decide"}]})
}
fn request(candidate: Value, fixture: Value) -> ReplayRequest {
    ReplayRequest {
        candidate_bytes: serde_json::to_vec(&candidate).unwrap(),
        dependencies: vec![],
        fixture_json: fixture.to_string(),
        clock_start_ms: 0,
        required_observations: vec![],
        expectations: vec![],
    }
}
#[test]
fn selected_candidate_mutation_changes_native_run_and_preserves_false_guard_attempt() {
    let good = replay(&prepare_replay(request(candidate("true"), fixture())).unwrap()).unwrap();
    let bad = replay(&prepare_replay(request(candidate("false"), fixture())).unwrap()).unwrap();
    assert!(good.passed, "{:?}", good.failures);
    assert!(!bad.passed);
    assert_ne!(good.identity, bad.identity);
    assert_eq!(bad.attempts.len(), 1);
    assert_eq!(bad.trace.steps.len(), 1);
    assert_eq!(bad.trace.steps[0].state_before, "open");
    assert_eq!(bad.trace.steps[0].state_after, "open");
    assert!(
        bad.trace.steps[0]
            .guards_evaluated
            .iter()
            .any(|g| !g.result)
    );
    assert_eq!(
        good.final_snapshot.as_ref().unwrap().case_state["result"],
        "accepted"
    );
    assert!(
        bad.final_snapshot
            .as_ref()
            .unwrap()
            .case_state
            .get("result")
            .is_none()
    );
}
#[test]
fn wrong_output_fails_and_unsupported_or_missing_observation_refuses() {
    let mut input = request(candidate("true"), fixture());
    input.expectations.push(ObservationExpectation {
        pointer: "/finalSnapshot/caseState/result".into(),
        expected: json!("wrong"),
    });
    let result = replay(&prepare_replay(input).unwrap()).unwrap();
    assert!(!result.passed);
    assert_eq!(result.differences[0].actual, Some(json!("accepted")));
    let mut input = request(candidate("true"), fixture());
    input.required_observations.push("durableRecovery".into());
    assert!(
        prepare_replay(input)
            .unwrap_err()
            .to_string()
            .contains("unsupported-observation")
    );
    let mut input = request(candidate("true"), fixture());
    input.expectations.push(ObservationExpectation {
        pointer: "/finalSnapshot/caseState/unavailable".into(),
        expected: json!("never actual"),
    });
    let result = replay(&prepare_replay(input).unwrap()).unwrap();
    assert!(!result.passed);
    assert!(
        result
            .refusals
            .iter()
            .any(|r| r.contains("missing-observation"))
    );
    assert!(
        result
            .final_snapshot
            .as_ref()
            .unwrap()
            .case_state
            .get("unavailable")
            .is_none()
    );
}
#[test]
fn same_prepared_inputs_repeat_semantically() {
    let prepared = prepare_replay(request(candidate("true"), fixture())).unwrap();
    let first = replay(&prepared).unwrap();
    let second = replay(&prepared).unwrap();
    assert_eq!(first.identity, second.identity);
    let mut semantic_traces = Vec::new();
    let mut firing_ids = Vec::new();
    for actual in [&first, &second] {
        assert!(actual.passed);
        let mut trace = actual.trace.clone();
        for guard in trace
            .steps
            .iter_mut()
            .flat_map(|step| &mut step.guards_evaluated)
        {
            // Each run retains its own actual firing identity. Verify that
            // association before comparing the repeatable semantic values.
            let firing_id = guard.firing_record_id.take().unwrap();
            let record = actual
                .provenance
                .as_ref()
                .unwrap()
                .iter()
                .find(|record| record.id == firing_id)
                .unwrap();
            assert_eq!(
                record.record_kind,
                wos_core::ProvenanceKind::StateTransition
            );
            assert_eq!(
                record.from_state.as_deref(),
                Some(guard.source_state.as_str())
            );
            assert_eq!(
                record.to_state.as_deref(),
                Some(guard.target_state.as_str())
            );
            firing_ids.push(firing_id);
        }
        semantic_traces.push(trace);
    }
    assert_eq!(firing_ids.len(), 2);
    assert_ne!(firing_ids[0], firing_ids[1]);
    assert_eq!(semantic_traces[0], semantic_traces[1]);
    assert_eq!(first.final_snapshot, second.final_snapshot);
}

#[test]
fn substituted_document_and_unresolved_or_wrong_identity_dependencies_refuse() {
    use wos_conformance::studio_api::ResolvedDependency;
    let mut scenario = fixture();
    scenario["documents"] = json!({"kernel":"inline"});
    scenario["inline_documents"] = json!({"kernel":candidate("false")});
    assert!(
        prepare_replay(request(candidate("true"), scenario))
            .unwrap_err()
            .to_string()
            .contains("substituted-fixture-document")
    );
    let mut input = request(candidate("true"), fixture());
    let mut alternate = candidate("true");
    alternate["url"] = json!("urn:test:wrong");
    input.dependencies.push(ResolvedDependency {
        role: "kernel@1.0.0".into(),
        url: "urn:test:wrong".into(),
        version: Some("1.0.0".into()),
        bytes: serde_json::to_vec(&alternate).unwrap(),
        definition: None,
    });
    assert!(
        prepare_replay(input)
            .unwrap_err()
            .to_string()
            .contains("dependency-workflow-identity-mismatch")
    );
    let mut scenario = fixture();
    scenario["event_sequence"] =
        json!([{"event":"$migrate","data":{"targetDefinitionVersion":"9.0.0"}}]);
    assert!(
        prepare_replay(request(candidate("true"), scenario))
            .unwrap_err()
            .to_string()
            .contains("unresolved-workflow-dependency")
    );
    let mut input = request(candidate("true"), fixture());
    input.dependencies.push(ResolvedDependency {
        role: "kernel@2.0.0".into(),
        url: "urn:test:exact-replay".into(),
        version: Some("2.0.0".into()),
        bytes: serde_json::to_vec(&candidate("true")).unwrap(),
        definition: None,
    });
    assert!(
        prepare_replay(input)
            .unwrap_err()
            .to_string()
            .contains("dependency-identity-mismatch")
    );
}

#[test]
fn embedded_governance_requirements_are_admitted_from_current_candidate() {
    let mut workflow = candidate("true");
    workflow["governance"] = json!({"dueProcess":{}});
    let first = replay(&prepare_replay(request(workflow.clone(), fixture())).unwrap()).unwrap();
    assert!(first.passed, "{:?}", first.failures);
    assert!(
        !first
            .provenance
            .unwrap()
            .iter()
            .any(|record| record.record_kind == wos_conformance::ProvenanceKind::SamplingDecision)
    );
    workflow["governance"]["dueProcess"]["adverseDecisionPolicy"] = json!({"noticeRequired":true});
    let error = prepare_replay(request(workflow, fixture()))
        .unwrap_err()
        .to_string();
    assert!(error.contains("noticeRequired"), "{error}");
}

fn task_candidate() -> Value {
    json!({"$wosWorkflow":"1.0","url":"urn:test:task-replay","version":"1.0.0","title":"Task actor control",
      "impactLevel":"operational","actors":[{"id":"reviewer","type":"human"}],
      "contracts":{"review":{"binding":"formspec","ref":"urn:test:form","definitionVersion":"1.0.0"}},
      "lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"event":"start","target":"review"}]},
        "review":{"type":"atomic","onEntry":[{"action":"createTask","taskRef":"reviewForm","assignTo":"reviewer","contractRef":"review","responseMappingRef":"urn:test:mapping","completionEvent":"completed"}],
          "transitions":[{"event":"completed","target":"done"}]},"done":{"type":"final"}}}})
}

#[test]
fn submitted_actor_and_rejected_task_result_remain_native_observations() {
    let response = json!({"status":"completed","definitionUrl":"urn:test:form","definitionVersion":"1.0.0","data":{"answer":42}});
    let scenario = json!({"id":"task-actor-control","rule":"control","description":"Reject foreign actor, accept assigned actor",
      "documents":{},"event_sequence":[
        {"event":"start","actor":"reviewer"},
        {"event":"submit","actor":"intruder","task_submission":{"task_ref":"reviewForm","response":response}},
        {"event":"submit","actor":"reviewer","task_submission":{"task_ref":"reviewForm","response":response}}],
      "expected_errors":["actor unauthorized"],
      "expected_transitions":[{"from":"open","target":"review","event":"start"},{"from":"review","target":"done","event":"completed"}]});
    let result = replay(&prepare_replay(request(task_candidate(), scenario)).unwrap()).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.attempts[1].actor.as_deref(), Some("intruder"));
    assert!(
        result.attempts[1]
            .error
            .as_ref()
            .unwrap()
            .contains("actor unauthorized")
    );
    assert_eq!(result.attempts[1].before, result.attempts[1].after);
    assert!(
        result.attempts[2]
            .submission_result
            .as_ref()
            .unwrap()
            .get("completed")
            .is_some()
    );
    assert_eq!(
        result.final_snapshot.as_ref().unwrap().case_state["mapped"]["answer"],
        42
    );
    assert!(
        result.execution["formspec"]
            .as_str()
            .unwrap()
            .contains("FixtureFormspecProcessor")
    );

    let scenario = json!({"id":"task-rejection-control","rule":"control","description":"Native rejection must fail success expectation",
      "documents":{},"event_sequence":[{"event":"start","actor":"reviewer"},{"event":"submit","actor":"reviewer","task_submission":{"task_ref":"reviewForm","response":{"status":"draft"}}}],
      "expected_transitions":[{"from":"open","target":"review","event":"start"}]});
    let result = replay(&prepare_replay(request(task_candidate(), scenario)).unwrap()).unwrap();
    assert!(!result.passed);
    assert!(
        result.attempts[1]
            .submission_result
            .as_ref()
            .unwrap()
            .get("rejected")
            .is_some()
    );
}

fn timer_candidate() -> Value {
    let mut workflow = candidate("true");
    workflow["lifecycle"]["states"]["open"] = json!({"type":"atomic",
      "onEntry":[{"action":"startTimer","timerId":"deadline","duration":"PT10S","event":"expired"}],
      "transitions":[{"event":"expired","target":"done"}]});
    workflow
}
#[test]
fn clock_before_and_at_deadline_observes_actual_pending_and_fired_timer() {
    let scenario = json!({"id":"timer-boundary-control","rule":"control","description":"Before and at timer deadline",
      "documents":{},"event_sequence":[{"event":"noop","delay":"PT9S"},{"event":"noop","delay":"PT1S"}],
      "expected_transitions":[{"from":"open","target":"done","event":"expired"}]});
    let result = replay(&prepare_replay(request(timer_candidate(), scenario)).unwrap()).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.attempts[0].clock_ms, 9000);
    assert_eq!(
        result.attempts[0].after.as_ref().unwrap().configuration,
        ["open"]
    );
    assert_eq!(
        result.attempts[0]
            .after
            .as_ref()
            .unwrap()
            .timers
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(result.attempts[2].clock_ms, 10000);
    assert_eq!(
        result.attempts[2].after.as_ref().unwrap().configuration,
        ["done"]
    );
    assert!(
        result.attempts[2]
            .drains
            .iter()
            .any(|drain| drain.processed_event.as_deref() == Some("expired"))
    );
    assert_eq!(result.attempts[2].kind, "clock");
}

#[test]
fn strict_empty_transition_expectations_and_unsupported_governed_output_never_pass() {
    let mut scenario = fixture();
    scenario["expected_transitions"] = json!([]);
    assert!(
        !replay(&prepare_replay(request(candidate("true"), scenario)).unwrap())
            .unwrap()
            .passed
    );
    let mut workflow = candidate("true");
    workflow["bindings"] = json!([{"on":"state:done","contractRef":"urn:test:form","projection":{"answer":"caseFile.answer"},"writeScope":["caseFile.answer"],"mutationSource":"system","verificationLevel":"validated"}]);
    assert!(
        prepare_replay(request(workflow, fixture()))
            .unwrap_err()
            .to_string()
            .contains("unsupported-governed-output-binding")
    );
    let mut scenario = fixture();
    scenario["expected_output"] = json!({"answer":42});
    assert!(
        prepare_replay(request(candidate("true"), scenario))
            .unwrap_err()
            .to_string()
            .contains("unsupported-scenario-field")
    );
}

#[test]
fn repeated_same_name_events_keep_false_and_true_guard_attempts_separate() {
    let mut workflow = candidate("event.allow = true");
    workflow["lifecycle"]["states"]["open"]["transitions"][0]["guard"] =
        json!("event.allow = true");
    let mut scenario = fixture();
    scenario["event_sequence"] = json!([{"event":"decide","actor":"reviewer","data":{"allow":false}},{"event":"decide","actor":"reviewer","data":{"allow":true}}]);
    let result = replay(&prepare_replay(request(workflow, scenario)).unwrap()).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.trace.steps.len(), 2);
    assert!(
        result.trace.steps[0]
            .guards_evaluated
            .iter()
            .all(|guard| !guard.result)
    );
    assert!(
        result.trace.steps[1]
            .guards_evaluated
            .iter()
            .any(|guard| guard.result)
    );
    assert_eq!(result.trace.steps[0].state_after, "open");
    assert_eq!(result.trace.steps[1].state_after, "done");
}

#[test]
fn legacy_file_resolution_is_frozen_before_execution() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("candidate.json");
    std::fs::write(&path, serde_json::to_vec(&candidate("true")).unwrap()).unwrap();
    let mut scenario = fixture();
    scenario["documents"] = json!({"kernel":path.to_str().unwrap()});
    let fixture = serde_json::from_value(scenario).unwrap();
    let mut engine = wos_conformance::WorkflowEngine::new(&fixture).unwrap();
    std::fs::write(&path, serde_json::to_vec(&candidate("false")).unwrap()).unwrap();
    assert!(engine.execute(&fixture).unwrap().passed);
}

#[test]
fn selected_calendar_bytes_change_timer_behavior_without_candidate_changes() {
    use wos_conformance::studio_api::ResolvedDependency;
    let mut workflow = timer_candidate();
    workflow["lifecycle"]["states"]["open"]["onEntry"][0]["duration"] = json!("P1D");
    let scenario = json!({"id":"calendar-dependency-control","rule":"control","description":"Selected holiday changes native deadline",
      "documents":{},"event_sequence":[{"event":"noop","delay":"P1DT9H"}],
      "expected_transitions":[{"from":"open","target":"done","event":"expired"}]});
    let calendar = json!({"$wosDelivery":"1.0","url":"urn:test:calendar","version":"1.0.0","targetWorkflow":"urn:test:exact-replay",
      "calendar":{"version":"1.0.0","timezone":"UTC","workWeek":["monday","tuesday","wednesday","thursday","friday"],"holidays":[],"operatingHours":{"start":"09:00","end":"17:00"}}});
    let run = |calendar: Value| {
        let mut input = request(workflow.clone(), scenario.clone());
        input.dependencies.push(ResolvedDependency {
            role: "businessCalendar".into(),
            url: "urn:test:calendar".into(),
            version: Some("1.0.0".into()),
            bytes: serde_json::to_vec(&calendar).unwrap(),
            definition: None,
        });
        replay(&prepare_replay(input).unwrap()).unwrap()
    };
    let first = run(calendar.clone());
    let mut holiday = calendar;
    holiday["calendar"]["holidays"] =
        json!([{"name":"Controlled Friday holiday","date":"1970-01-02"}]);
    let second = run(holiday);
    assert!(first.passed, "{:?}", first.failures);
    assert!(!second.passed);
    assert_eq!(
        second.final_snapshot.as_ref().unwrap().configuration,
        ["open"]
    );
    assert_ne!(first.identity, second.identity);
    assert_eq!(
        first.inputs.iter().find(|input| input.role == "kernel"),
        second.inputs.iter().find(|input| input.role == "kernel")
    );
}

#[test]
fn canonical_top_level_agents_activate_native_ai_policy() {
    let mut workflow = candidate("true");
    workflow["actors"] = json!([{"id":"reviewer","type":"agent"}]);
    workflow["agents"] = json!([{"id":"reviewer","agentType":"generative","modelIdentifier":"fixture-model","modelVersion":"1.0",
      "deonticConstraints":[{"id":"positive-answer","kind":"permission","bounds":"output.answer > 0","onViolation":"reject"}]}]);
    workflow["aiOversight"] = json!({"defaultAutonomy":"assistive"});
    workflow["lifecycle"]["states"]["open"]["transitions"][0]
        .as_object_mut()
        .unwrap()
        .remove("actions");
    let mut scenario = fixture();
    scenario["event_sequence"][0]["data"] =
        json!({"output":{"answer":-1},"confidenceReport":{"overall":1.0,"method":"declared"}});
    let rejected =
        replay(&prepare_replay(request(workflow.clone(), scenario.clone())).unwrap()).unwrap();
    assert!(!rejected.passed);
    assert!(rejected.provenance.as_ref().unwrap().iter().any(|record| {
        record
            .data
            .as_ref()
            .and_then(|data| data.get("constraintId"))
            .and_then(Value::as_str)
            == Some("positive-answer")
    }));
    workflow["agents"][0]["deonticConstraints"][0]["bounds"] = json!("output.answer >= -1");
    let accepted = replay(&prepare_replay(request(workflow, scenario)).unwrap()).unwrap();
    assert!(accepted.passed, "{:?}", accepted.failures);
    assert_ne!(rejected.identity, accepted.identity);
}

#[test]
fn required_notice_delivery_refuses_before_initialization() {
    let mut workflow = candidate("true");
    workflow["governance"] = json!({"dueProcess":{"adverseDecisionPolicy":{"noticeRequired":true,"noticeTiming":"beforeEffective","noticeGracePeriod":"P1D"}}});
    workflow["lifecycle"]["states"]["open"]["transitions"][0]["tags"] = json!(["adverse-decision"]);
    let error = prepare_replay(request(workflow.clone(), fixture()))
        .unwrap_err()
        .to_string();
    assert!(error.contains("noticeRequired"), "{error}");
    let mut input = request(workflow, fixture());
    input.required_observations.push("deliveredNotice".into());
    assert!(
        prepare_replay(input)
            .unwrap_err()
            .to_string()
            .contains("unsupported-observation")
    );
}

#[test]
fn original_candidate_bytes_are_retained_and_distinct_from_canonical_identity() {
    let mut input = request(candidate("true"), fixture());
    let original = serde_json::to_vec_pretty(&candidate("true")).unwrap();
    input.candidate_bytes = original.clone();
    let prepared = prepare_replay(input).unwrap();
    assert_eq!(prepared.input_bytes("kernel"), Some(original.as_slice()));
    let result = replay(&prepared).unwrap();
    let candidate = result
        .inputs
        .iter()
        .find(|input| input.role == "kernel")
        .unwrap();
    assert_ne!(candidate.raw_sha256, candidate.canonical_sha256);
    assert_eq!(
        result.runtime_identity["compiledSource"]["completeCompiledGraph"],
        false
    );
    assert!(
        result.runtime_identity["compiledSource"]["nativeReplaySourceSnapshot"]
            .as_str()
            .unwrap()
            .len()
            == 64
    );
}

#[test]
fn common_literal_vectors_match_independent_expected_outcomes() {
    use wos_conformance::studio_api::ResolvedDependency;
    let corpus: Value =
        serde_json::from_str(include_str!("../fixtures/authoring-replay/vectors.json")).unwrap();
    for vector in corpus["vectors"].as_array().unwrap() {
        let mut input = request(vector["candidate"].clone(), vector["fixture"].clone());
        input.required_observations = serde_json::from_value(
            vector
                .get("requiredObservations")
                .cloned()
                .unwrap_or(json!([])),
        )
        .unwrap();
        input.expectations =
            serde_json::from_value(vector.get("expectations").cloned().unwrap_or(json!([])))
                .unwrap();
        for dependency in vector
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            input.dependencies.push(ResolvedDependency {
                role: dependency["role"].as_str().unwrap().into(),
                url: dependency["url"].as_str().unwrap().into(),
                version: dependency["version"].as_str().map(str::to_string),
                bytes: serde_json::to_vec(&dependency["document"]).unwrap(),
                definition: None,
            });
        }
        let actual = match prepare_replay(input) {
            Err(error) => {
                assert!(error.to_string().contains("replay-refused"));
                "refused".to_string()
            }
            Ok(prepared) => replay(&prepared).unwrap().status,
        };
        assert_eq!(
            actual,
            vector["expected"].as_str().unwrap(),
            "shared vector {}",
            vector["id"]
        );
    }
}

#[test]
fn process_creation_failure_retains_request_and_error_without_fabricated_state() {
    let mut workflow = task_candidate();
    workflow["contracts"]["review"]
        .as_object_mut()
        .unwrap()
        .remove("definitionVersion");
    let prepared = prepare_replay(request(workflow, fixture())).unwrap();
    let result = replay(&prepared).unwrap();
    assert!(!result.passed);
    assert_eq!(result.status, "refused");
    assert_eq!(
        result.initialization["request"]["definitionUrl"],
        "urn:test:task-replay"
    );
    assert!(
        result.initialization["error"]
            .as_str()
            .unwrap()
            .contains("requires definitionVersion")
    );
    assert!(result.initialization["result"].is_null());
    assert!(result.final_snapshot.is_none());
    assert!(result.provenance.is_none());
    assert!(result.attempts.is_empty());
}

#[test]
fn successful_native_drains_survive_a_later_drain_failure() {
    let mut workflow = candidate("true");
    workflow["lifecycle"]["states"] = json!({
      "open":{"type":"atomic","transitions":[{"event":"kick","target":"waiting","actions":[
        {"action":"emitEvent","eventType":"good"},{"action":"emitEvent","eventType":"bad"}]}]},
      "waiting":{"type":"atomic","transitions":[{"event":"good","guard":"true","target":"review"}]},
      "review":{"type":"atomic","transitions":[{"event":"bad","target":"done","actions":[{"action":"invokeService"}]}]},
      "done":{"type":"final"}});
    let scenario = json!({"id":"partial-drain-control","rule":"control","description":"Retain committed drain before later action fails",
      "documents":{},"event_sequence":[{"event":"kick","actor":"reviewer"},{"event":"noop","delay":"PT0S"}],
      "expected_transitions":[{"from":"open","target":"waiting","event":"kick"},{"from":"waiting","target":"review","event":"good"}]});
    let result = replay(&prepare_replay(request(workflow, scenario)).unwrap()).unwrap();
    assert!(!result.passed);
    let clock = &result.attempts[1];
    assert_eq!(clock.kind, "clock");
    assert!(
        clock
            .error
            .as_ref()
            .unwrap()
            .contains("invokeService missing serviceRef")
    );
    let committed = clock
        .drains
        .iter()
        .find(|drain| drain.processed_event.as_deref() == Some("good"))
        .unwrap();
    assert_eq!(committed.transitions[0].from, "waiting");
    assert_eq!(committed.transitions[0].to, "review");
    assert!(committed.guard_evaluations.iter().any(|guard| guard.result));
    assert_eq!(clock.after.as_ref().unwrap().configuration, ["review"]);
    assert!(
        clock
            .provenance
            .as_ref()
            .unwrap()
            .iter()
            .any(|record| record.to_state.as_deref() == Some("review"))
    );
}

#[test]
fn deployment_posture_binds_native_signature_policy_without_workflow_target() {
    use wos_conformance::studio_api::ResolvedDependency;
    let vector: Value = serde_json::from_str(include_str!(
        "../fixtures/authoring-replay/posture-policy.json"
    ))
    .unwrap();
    let input = || {
        let mut input = request(vector["candidate"].clone(), vector["fixture"].clone());
        input.dependencies.push(ResolvedDependency {
            role: "postureDeclaration".into(),
            url: vector["dependency"]["url"].as_str().unwrap().into(),
            version: Some("1.0.0".into()),
            bytes: serde_json::to_vec(&vector["dependency"]["document"]).unwrap(),
            definition: None,
        });
        input
    };
    let result = replay(&prepare_replay(input()).unwrap()).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(
        serde_json::to_value(&result.final_snapshot.as_ref().unwrap().configuration).unwrap(),
        vector["expectedConfiguration"]
    );
    assert!(result.provenance.as_ref().unwrap().iter().any(|record| {
        record.data.as_ref().is_some_and(|data| {
            data["primitiveVerification"]["status"] == vector["expectedPrimitiveVerification"]
        })
    }));
    let mut wrong = input();
    let mut document = vector["dependency"]["document"].clone();
    document["url"] = json!("https://example.gov/posture/alternate.json");
    wrong.dependencies[0].url = document["url"].as_str().unwrap().into();
    wrong.dependencies[0].bytes = serde_json::to_vec(&document).unwrap();
    assert!(
        prepare_replay(wrong)
            .unwrap_err()
            .to_string()
            .contains("dependency-posture-policy-mismatch")
    );
    let mut missing = input();
    missing.dependencies.clear();
    assert!(
        prepare_replay(missing)
            .unwrap_err()
            .to_string()
            .contains("unresolved-posture-policy-dependency")
    );
}

#[test]
fn retired_agent_shapes_refuse_without_rewriting_source() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../fixtures/authoring-replay/unsupported-agent-shapes.json"
    ))
    .unwrap();
    for vector in corpus["vectors"].as_array().unwrap() {
        let mut input = request(vector["candidate"].clone(), vector["fixture"].clone());
        let original = serde_json::to_vec_pretty(&vector["candidate"]).unwrap();
        input.candidate_bytes = original.clone();
        let error = prepare_replay(input.clone()).unwrap_err().to_string();
        assert!(
            error.contains(vector["expectedReason"].as_str().unwrap()),
            "{}: {error}",
            vector["id"]
        );
        assert!(
            error.contains("native AI policy"),
            "{}: {error}",
            vector["id"]
        );
        assert_eq!(input.candidate_bytes, original);
        assert_eq!(
            serde_json::from_slice::<Value>(&input.candidate_bytes).unwrap(),
            vector["candidate"]
        );
    }
}
