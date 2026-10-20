// Rust guideline compliant 2026-02-21
//! Causal controls for selected native Formspec execution.

use serde_json::{Value, json};
use wos_conformance::studio_api::{ReplayRequest, ResolvedDependency, prepare_replay, replay};

fn workflow(mapping: bool) -> Value {
    let mut value = json!({"$wosWorkflow":"1.0","url":"urn:test:selected-workflow","version":"1.0.0",
      "title":"Selected Formspec","impactLevel":"operational","actors":[{"id":"reviewer","type":"human"}],
      "contracts":{"form":{"binding":"formspec","ref":"urn:test:selected-definition","definitionVersion":"1.0.0"}},
      "lifecycle":{"initialState":"review","states":{
        "review":{"type":"atomic","onEntry":[{"action":"createTask","taskRef":"reviewForm","assignTo":"reviewer","contractRef":"form","completionEvent":"completed"}],
          "transitions":[{"event":"completed","target":"done"}]},"done":{"type":"final"}}}});
    if mapping {
        value["lifecycle"]["states"]["review"]["onEntry"][0]["responseMappingRef"] =
            json!("urn:test:selected-mapping");
    }
    value
}

fn definition(required: bool) -> Value {
    json!({"$formspec":"1.0","url":"urn:test:selected-definition","version":"1.0.0","status":"draft",
      "title":"Selected Definition","items":[{"key":"answer","type":"field","dataType":"string","label":"Answer"}],
      "binds":[{"path":"answer","required":if required {"true"} else {"false"}}]})
}

fn dependency(role: &str, url: &str, document: Value, pair: Option<Value>) -> ResolvedDependency {
    let mut value = json!({"role":role,"url":url,"version":document["version"],"bytes":serde_json::to_vec(&document).unwrap()});
    if let Some(pair) = pair {
        value["definition"] = pair;
    }
    serde_json::from_value(value).unwrap()
}

fn request(required: bool, data: Value, mapping: Option<Value>) -> ReplayRequest {
    let has_mapping = mapping.is_some();
    let mut dependencies = vec![dependency(
        "definition",
        "urn:test:selected-definition",
        definition(required),
        None,
    )];
    if let Some(mapping) = mapping {
        dependencies.push(dependency(
            "mapping",
            "urn:test:selected-mapping",
            mapping,
            Some(json!({"url":"urn:test:selected-definition","version":"1.0.0"})),
        ));
    }
    let fixture = json!({"id":"selected-native-form","rule":"control","description":"Selected bytes own validation and mapping",
      "event_sequence":[{"event":"submit","actor":"reviewer","task_submission":{"task_ref":"reviewForm","response":{
        "$formspecResponse":"1.0","definitionUrl":"urn:test:selected-definition","definitionVersion":"1.0.0",
        "status":"completed","authored":"2026-01-01T00:00:00Z","data":data}}}],
        "expected_transitions":[{"from":"review","target":"done","event":"completed"}]});
    ReplayRequest {
        candidate_bytes: serde_json::to_vec(&workflow(has_mapping)).unwrap(),
        dependencies,
        fixture_json: fixture.to_string(),
        clock_start_ms: 0,
        required_observations: vec!["taskSubmission".into()],
        expectations: vec![],
    }
}

fn mapping(target: &str) -> Value {
    json!({"$formspecMapping":"1.0","version":"1.0.0","definitionRef":"urn:test:selected-definition",
      "definitionVersion":"^1.0.0","targetSchema":{"format":"json"},
      "rules":[{"sourcePath":"answer","targetPath":target,"transform":"preserve"}]})
}

#[test]
fn selected_definition_bytes_control_actual_task_validation() {
    let required = prepare_replay(request(true, json!({"answer":""}), None)).unwrap();
    let optional = prepare_replay(request(false, json!({"answer":""}), None)).unwrap();
    let rejected = replay(&required).unwrap();
    let accepted = replay(&optional).unwrap();
    assert_ne!(rejected.identity, accepted.identity);
    assert!(
        rejected.attempts[0]
            .submission_result
            .as_ref()
            .unwrap()
            .get("failed")
            .is_some()
    );
    assert!(
        accepted.attempts[0]
            .submission_result
            .as_ref()
            .unwrap()
            .get("completed")
            .is_some()
    );
    assert_eq!(accepted.execution["fixtureFormspecProcessor"], false);
    assert!(accepted.passed, "{:?}", accepted.failures);
    assert!(!rejected.passed);
    assert_eq!(accepted.final_snapshot.unwrap().case_state, json!({}));
}

#[test]
fn selected_mapping_bytes_control_actual_case_mutation() {
    let first = replay(
        &prepare_replay(request(
            true,
            json!({"answer":"yes"}),
            Some(mapping("decision")),
        ))
        .unwrap(),
    )
    .unwrap();
    let second = replay(
        &prepare_replay(request(
            true,
            json!({"answer":"yes"}),
            Some(mapping("other")),
        ))
        .unwrap(),
    )
    .unwrap();
    assert_ne!(first.identity, second.identity);
    assert!(first.passed, "{:?}", first.failures);
    assert!(second.passed, "{:?}", second.failures);
    assert_eq!(
        first.final_snapshot.unwrap().case_state,
        json!({"decision":"yes"})
    );
    assert_eq!(
        second.final_snapshot.unwrap().case_state,
        json!({"other":"yes"})
    );
}

#[test]
fn moving_runtime_clock_controls_native_definition_constraint() {
    let run = |delay: Option<&str>| {
        let mut input = request(true, json!({"answer":"yes"}), None);
        let mut selected = definition(true);
        selected["binds"][0]["constraint"] = json!("today() < date('1970-01-02')");
        input.dependencies[0].bytes = serde_json::to_vec(&selected).unwrap();
        let mut fixture: Value = serde_json::from_str(&input.fixture_json).unwrap();
        if let Some(delay) = delay {
            fixture["event_sequence"][0]["delay"] = json!(delay);
        }
        input.fixture_json = fixture.to_string();
        let prepared = prepare_replay(input).unwrap();
        let first = replay(&prepared).unwrap();
        let second = replay(&prepared).unwrap();
        let mut semantic_snapshots = Vec::new();
        for actual in [&first, &second] {
            let mut snapshot = actual.final_snapshot.clone().unwrap();
            for task in snapshot.task_history.as_array_mut().unwrap() {
                for field in ["creation", "completion"] {
                    if let Some(witness) = task[field].as_object_mut() {
                        let record = actual
                            .provenance
                            .as_ref()
                            .unwrap()
                            .iter()
                            .find(|record| witness["id"] == record.id)
                            .unwrap();
                        assert_eq!(serde_json::to_value(record).unwrap(), json!(witness));
                        // Native record identities vary by run; preserve and
                        // verify them above, then compare all semantic fields.
                        witness.remove("id");
                    }
                }
            }
            semantic_snapshots.push(snapshot);
        }
        assert_eq!(semantic_snapshots[0], semantic_snapshots[1]);
        first
    };
    let timely = run(None);
    let late = run(Some("P2D"));
    assert!(timely.passed, "{:?}", timely.failures);
    assert!(!late.passed);
    assert!(late.attempts.iter().any(|attempt| {
        attempt
            .submission_result
            .as_ref()
            .is_some_and(|result| result.get("failed").is_some())
    }));
}

#[test]
fn every_selected_artifact_and_exact_pair_remain_retained() {
    let mut input = request(true, json!({"answer":"yes"}), Some(mapping("decision")));
    let mut extra = definition(false);
    extra["url"] = json!("urn:test:other-definition");
    input.dependencies.push(dependency(
        "definition",
        "urn:test:other-definition",
        extra,
        None,
    ));
    let original: Vec<_> = input
        .dependencies
        .iter()
        .map(|item| {
            (
                item.role.clone(),
                item.url.clone(),
                item.version.clone(),
                item.bytes.clone(),
            )
        })
        .collect();
    let prepared = prepare_replay(input).unwrap();
    assert!(
        prepared.input_bytes("definition").is_none(),
        "role alone must not choose between Definitions"
    );
    for (role, url, version, bytes) in original {
        assert_eq!(
            prepared.input_bytes_for(&role, &url, version.as_deref()),
            Some(bytes.as_slice())
        );
    }
    let result = replay(&prepared).unwrap();
    let pairing = result
        .inputs
        .iter()
        .find(|item| item.role == "mapping")
        .unwrap()
        .definition
        .as_ref()
        .unwrap();
    assert_eq!(pairing.url, "urn:test:selected-definition");
    assert_eq!(pairing.version, "1.0.0");
    assert!(result.runtime_identity["compiledSource"]["externalPathDependencySourceFingerprints"]["files"].as_object().unwrap().keys().any(|path| path.starts_with("formspec/crates/formspec-eval/src/")));
}

#[test]
fn invalid_selection_and_task_pairing_refuse_before_process_creation() {
    let base = || request(true, json!({"answer":"yes"}), Some(mapping("decision")));
    let mut bad_pair = base();
    bad_pair.dependencies[1]
        .definition
        .as_mut()
        .unwrap()
        .version = "2.0.0".into();
    assert!(
        prepare_replay(bad_pair)
            .unwrap_err()
            .to_string()
            .contains("Definition is not selected")
    );

    let mut missing = base();
    missing.dependencies.pop();
    assert!(
        prepare_replay(missing)
            .unwrap_err()
            .to_string()
            .contains("Mapping is not selected")
    );

    let mut duplicate = base();
    duplicate
        .dependencies
        .push(duplicate.dependencies[0].clone());
    assert!(
        prepare_replay(duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicate-dependency-identity")
    );

    let mut wrong_range = base();
    let mut document = mapping("decision");
    document["definitionVersion"] = json!(">=2.0.0");
    wrong_range.dependencies[1].bytes = serde_json::to_vec(&document).unwrap();
    assert!(
        prepare_replay(wrong_range)
            .unwrap_err()
            .to_string()
            .contains("version range differ")
    );

    let mut reverse_only = base();
    let mut document = mapping("decision");
    document["direction"] = json!("reverse");
    reverse_only.dependencies[1].bytes = serde_json::to_vec(&document).unwrap();
    assert!(
        prepare_replay(reverse_only)
            .unwrap_err()
            .to_string()
            .contains("direction is not permitted")
    );

    let mut ambiguous = base();
    let mut second = ambiguous.dependencies[1].clone();
    second.version = Some("2.0.0".into());
    let mut document = mapping("other");
    document["version"] = json!("2.0.0");
    second.bytes = serde_json::to_vec(&document).unwrap();
    ambiguous.dependencies.push(second);
    assert!(
        prepare_replay(ambiguous)
            .unwrap_err()
            .to_string()
            .contains("ambiguous selected Mapping locator")
    );
}

#[test]
fn selected_processing_refuses_canned_outcomes_and_unqualified_instances() {
    let mut mixed = request(true, json!({"answer":"yes"}), None);
    let mut fixture: Value = serde_json::from_str(&mixed.fixture_json).unwrap();
    fixture["definition_errors"] = json!([{"code":"fixtureFlag"}]);
    mixed.fixture_json = fixture.to_string();
    assert!(
        prepare_replay(mixed)
            .unwrap_err()
            .to_string()
            .contains("cannot use canned")
    );

    let mut external = request(true, json!({"answer":"yes"}), None);
    let mut document = definition(true);
    document["instances"] = json!({"source":{"source":"https://example.test/unqualified","schema":{"answer":"string"}}});
    external.dependencies[0].bytes = serde_json::to_vec(&document).unwrap();
    let error = prepare_replay(external).unwrap_err().to_string();
    assert!(
        error.contains("instances") || error.contains("rejected"),
        "{error}"
    );
}

#[test]
fn reverse_prefill_executes_selected_mapping_and_nested_task_pairing_is_checked() {
    let mut input = request(true, json!({"answer":"yes"}), Some(mapping("decision")));
    let mut document = mapping("decision");
    document["direction"] = json!("both");
    input.dependencies[1].bytes = serde_json::to_vec(&document).unwrap();
    let mut workflow: Value = serde_json::from_slice(&input.candidate_bytes).unwrap();
    workflow["lifecycle"]["states"]["review"]["onEntry"][0]["prefillMappingRef"] =
        json!("urn:test:selected-mapping");
    input.candidate_bytes = serde_json::to_vec(&workflow).unwrap();
    let mut fixture: Value = serde_json::from_str(&input.fixture_json).unwrap();
    fixture["initial_case_state"] = json!({"decision":"prefilled"});
    input.fixture_json = fixture.to_string();
    let result = replay(&prepare_replay(input.clone()).unwrap()).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(
        result.attempts[0].before.as_ref().unwrap().active_tasks[0]["context"]["prefillData"]["answer"],
        "prefilled"
    );
    assert!(
        result
            .provenance
            .as_ref()
            .unwrap()
            .iter()
            .any(|record| record.record_kind == wos_conformance::ProvenanceKind::TaskPresented)
    );

    workflow["lifecycle"]["states"]["unreachable"] = json!({"type":"compound","initialState":"nested","states":{
        "nested":{"type":"atomic","onEntry":[{"action":"createTask","taskRef":"nested","contractRef":"form","prefillMappingRef":"urn:test:missing"}]}}});
    input.candidate_bytes = serde_json::to_vec(&workflow).unwrap();
    assert!(
        prepare_replay(input)
            .unwrap_err()
            .to_string()
            .contains("Mapping is not selected")
    );
}
