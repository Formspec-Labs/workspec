// Rust guideline compliant 2026-02-21
//! Causal observations name actual selections, snapshots and native table rows.
use serde_json::json;
use wos_core::model::due_process::TransitionLocation;
use wos_core::model::kernel::KernelDocument;
use wos_core::{Evaluator, ProvenanceKind};

fn kernel() -> KernelDocument {
    serde_json::from_value(json!({
        "$wosWorkflow":"1.0", "url":"urn:test:causal", "version":"1.0.0",
        "governance":{"dueProcess":{"execution":{"profile":"referenceDueProcess/1"}}},
        "caseFile":{"fields":{"level":{"type":"number","default":100}}},
        "lifecycle":{"initialState":"open/a", "states":{
            "open/a":{"type":"atomic","onEntry":[{"action":"setData","path":"caseFile.level","value":99}],
                "transitions":[
                    {"event":"decide","guard":"false","target":"pending"},
                    {"event":"decide","guard":{"kind":"decisionTable","ref":"eligibility","outputColumn":"adverse","inputBindings":{"income":"caseFile.income"}},
                     "target":"pending","tags":["determination"],
                     "actions":[{"action":"setData","path":"caseFile.level","value":0},{"action":"setData","path":"caseFile.level","value":99}]}
                ]},
            "pending":{"type":"atomic"}
        }},
        "decisionTables":[{"id":"eligibility","hitPolicy":"first",
            "inputs":[{"name":"income","type":"number"}], "outputs":[{"name":"adverse","type":"boolean"}],
            "rows":[{"id":"high","inputCells":["income > 100"],"outputCells":["true"]},
                    {"id":"low","inputCells":["income <= 100"],"outputCells":["false"]}]}]
    })).unwrap()
}

#[test]
fn selected_location_rows_and_intermediate_effects_are_actual() {
    let mut evaluator =
        Evaluator::with_time_and_case_state(kernel(), 0, Some(&json!({"income":150}))).unwrap();
    assert_eq!(evaluator.initial_case_state()["level"], 100);
    evaluator
        .process_event("decide", Some("actor:human:reviewer"), None)
        .unwrap();
    let transition = &evaluator.transitions()[0];
    assert_eq!(
        transition.location,
        TransitionLocation {
            state_pointer: "/lifecycle/states/open~1a".into(),
            transition_index: 1
        }
    );
    let record = evaluator
        .provenance()
        .records()
        .iter()
        .find(|record| record.id == transition.provenance_record_id)
        .unwrap();
    assert_eq!(record.record_kind, ProvenanceKind::StateTransition);
    let guards = evaluator.guard_evaluations();
    assert!(guards[0].firing_record_id.is_none());
    assert_eq!(
        guards[1].firing_record_id.as_deref(),
        Some(record.id.as_str())
    );
    let row = guards[1].decision_table.as_ref().unwrap();
    assert_eq!(row.selected_row_id.as_deref(), Some("high"));
    assert_eq!(row.matched_row_ids, vec!["high"]);
    assert_eq!(row.output, Some(json!(true)));
    assert_eq!(
        evaluator
            .action_case_states()
            .iter()
            .map(|value| value["level"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![99, 0, 99]
    );
    assert_eq!(
        evaluator.entered_state_pointers(),
        &["/lifecycle/states/open~1a", "/lifecycle/states/pending"]
    );
}

#[test]
fn counterfactual_uses_same_native_table_without_entry_effects_or_firing() {
    let location = TransitionLocation {
        state_pointer: "/lifecycle/states/open~1a".into(),
        transition_index: 1,
    };
    let actual = Evaluator::evaluate_reference_alternative(
        &kernel(),
        &location,
        &json!({"income":50,"level":100}),
        0,
    )
    .unwrap();
    assert!(!actual.result);
    assert_eq!(
        actual.decision_table.unwrap().selected_row_id.as_deref(),
        Some("low")
    );
    assert!(actual.firing_record_id.is_none());
    assert_eq!(actual.inputs, json!({"income":50}));
    assert!(
        Evaluator::evaluate_reference_alternative(
            &kernel(),
            &TransitionLocation {
                transition_index: 5,
                ..location
            },
            &json!({"income":50}),
            0
        )
        .is_err()
    );
}

fn assert_firing_locations(raw: &serde_json::Value, evaluator: &Evaluator, expected: &[&str]) {
    let mut actual = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for transition in evaluator.transitions() {
        assert!(ids.insert(&transition.provenance_record_id));
        assert_eq!(transition.location.transition_index, 0);
        assert!(
            raw.pointer(&format!(
                "{}/transitions/0",
                transition.location.state_pointer
            ))
            .is_some()
        );
        let record = evaluator
            .provenance()
            .records()
            .iter()
            .find(|record| record.id == transition.provenance_record_id)
            .unwrap();
        assert_eq!(record.record_kind, ProvenanceKind::StateTransition);
        assert_eq!(record.from_state.as_deref(), Some(transition.from.as_str()));
        assert_eq!(record.to_state.as_deref(), Some(transition.to.as_str()));
        assert_eq!(
            record.data.as_ref().unwrap()["nativeTransitionLocation"],
            json!(transition.location)
        );
        let guards: Vec<_> = evaluator
            .guard_evaluations()
            .iter()
            .filter(|guard| guard.firing_record_id.as_deref() == Some(record.id.as_str()))
            .collect();
        assert_eq!(guards.len(), 1);
        assert_eq!(guards[0].location.as_ref(), Some(&transition.location));
        assert!(guards[0].result);
        actual.push(transition.location.state_pointer.as_str());
    }
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn compound_timer_and_repeated_self_firings_keep_actual_locations_and_distinct_ids() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"review","states":{
        "review":{"type":"compound","initialState":"waiting","states":{
            "waiting":{"type":"atomic","onEntry":[{"action":"startTimer","timerId":"review-timer","duration":"PT1S","event":"elapsed"}],
                "transitions":[{"event":"elapsed","target":"ready","guard":"true"}]},
            "ready":{"type":"atomic","transitions":[{"event":"repeat","target":"ready","guard":"true"}]}
        }}
    }}});
    let mut evaluator =
        Evaluator::with_time(serde_json::from_value(raw.clone()).unwrap(), 0).unwrap();
    evaluator.advance_time(1000, None).unwrap();
    for _ in 0..2 {
        assert!(evaluator.process_event("repeat", None, None).unwrap());
    }
    assert_firing_locations(
        &raw,
        &evaluator,
        &[
            "/lifecycle/states/review/states/waiting",
            "/lifecycle/states/review/states/ready",
            "/lifecycle/states/review/states/ready",
        ],
    );
}

#[test]
fn parallel_region_and_join_firings_keep_own_locations() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"a-open","states":{"a-open":{"type":"atomic","transitions":[{"event":"finish","target":"a-done","guard":"true"}]},"a-done":{"type":"final"}}},
            "b":{"initialState":"b-open","states":{"b-open":{"type":"atomic","transitions":[{"event":"finish","target":"b-done","guard":"true"}]},"b-done":{"type":"final"}}}
        },"transitions":[{"event":"$join","target":"done","guard":"true"}]},"done":{"type":"final"}
    }}});
    let mut evaluator =
        Evaluator::with_time(serde_json::from_value(raw.clone()).unwrap(), 0).unwrap();
    assert!(evaluator.process_event("finish", None, None).unwrap());
    assert_firing_locations(
        &raw,
        &evaluator,
        &[
            "/lifecycle/states/parallel/regions/a/states/a-open",
            "/lifecycle/states/parallel/regions/b/states/b-open",
            "/lifecycle/states/parallel",
        ],
    );
}

#[test]
fn automatic_and_foreach_body_and_completion_firings_keep_actual_locations() {
    let automatic = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"target":"done","guard":"true"}]},"done":{"type":"final"}
    }}});
    let mut evaluator =
        Evaluator::with_time(serde_json::from_value(automatic.clone()).unwrap(), 0).unwrap();
    assert!(evaluator.rescan_on_mutation().unwrap());
    assert_firing_locations(&automatic, &evaluator, &["/lifecycle/states/open"]);
    let foreach = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"event":"begin","target":"loop","guard":"true"}]},
        "loop":{"type":"foreach","collection":"[1,2]","body":{"type":"compound","initialState":"check","states":{
            "check":{"type":"atomic","transitions":[{"target":"checked","guard":"true"}]},"checked":{"type":"final"}
        }},"transitions":[{"target":"done","guard":"true"}]},"done":{"type":"final"}
    }}});
    let mut evaluator =
        Evaluator::with_time(serde_json::from_value(foreach.clone()).unwrap(), 0).unwrap();
    assert!(evaluator.process_event("begin", None, None).unwrap());
    assert_firing_locations(
        &foreach,
        &evaluator,
        &[
            "/lifecycle/states/open",
            "/lifecycle/states/loop",
            "/lifecycle/states/loop/body/states/check",
            "/lifecycle/states/loop/body/states/check",
        ],
    );
}
