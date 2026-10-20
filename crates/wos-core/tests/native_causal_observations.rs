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
    let mut evaluator = Evaluator::with_time_and_case_state(
        kernel(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
        Some(&json!({"income":150})),
    )
    .unwrap();
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
        &stack_common_typeid::testing::fixture_minter(),
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
            &stack_common_typeid::testing::fixture_minter(),
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
    let mut evaluator = Evaluator::with_time(
        serde_json::from_value(raw.clone()).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
    )
    .unwrap();
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
    let mut evaluator = Evaluator::with_time(
        serde_json::from_value(raw.clone()).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
    )
    .unwrap();
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
    let mut evaluator = Evaluator::with_time(
        serde_json::from_value(automatic.clone()).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
    )
    .unwrap();
    assert!(evaluator.rescan_on_mutation().unwrap());
    assert_firing_locations(&automatic, &evaluator, &["/lifecycle/states/open"]);
    let foreach = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"event":"begin","target":"loop","guard":"true"}]},
        "loop":{"type":"foreach","collection":"[1,2]","body":{"type":"compound","initialState":"check","states":{
            "check":{"type":"atomic","transitions":[{"target":"checked","guard":"true"}]},"checked":{"type":"final"}
        }},"transitions":[{"target":"done","guard":"true"}]},"done":{"type":"final"}
    }}});
    let mut evaluator = Evaluator::with_time(
        serde_json::from_value(foreach.clone()).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
    )
    .unwrap();
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

fn cross_compound_kernel() -> KernelDocument {
    serde_json::from_value(json!({"$wosWorkflow":"1.0","url":"urn:test:compound-paths","version":"1.0.0",
        "lifecycle":{"initialState":"phaseA","states":{
            "phaseA":{"type":"compound","initialState":"a1","onEntry":[{"action":"log","message":"enter-A"}],"onExit":[{"action":"log","message":"exit-A"}],"states":{
                "a1":{"type":"atomic","onExit":[{"action":"log","message":"exit-a1"}],"transitions":[{"event":"go","target":"b2","actions":[{"action":"log","message":"go"}]}]}}},
            "phaseB":{"type":"compound","initialState":"b1","onEntry":[{"action":"log","message":"enter-B"}],"onExit":[{"action":"log","message":"exit-B"}],"states":{
                "b1":{"type":"atomic","onEntry":[{"action":"log","message":"wrong-initial"}]},
                "b2":{"type":"atomic","onEntry":[{"action":"log","message":"enter-b2"}],"onExit":[{"action":"log","message":"exit-b2"}],"transitions":[{"event":"finish","target":"done"}]}}},
            "done":{"type":"final"}
        }}})).unwrap()
}

#[test]
fn compound_configuration_contains_only_the_actual_active_leaf() {
    let evaluator = Evaluator::new(
        cross_compound_kernel(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert_eq!(evaluator.configuration().active_states(), ["a1"]);
    assert!(
        evaluator.configuration().contains("phaseA"),
        "logical ancestor activity remains queryable"
    );
}

#[test]
fn cross_compound_transition_executes_full_paths_and_drops_departed_ancestors() {
    let mut evaluator = Evaluator::new(
        cross_compound_kernel(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    evaluator.take_executed_actions();
    assert!(evaluator.process_event("go", None, None).unwrap());
    let messages: Vec<_> = evaluator
        .executed_actions()
        .iter()
        .filter_map(|a| a.action.message.as_deref())
        .collect();
    assert_eq!(messages, ["exit-a1", "exit-A", "go", "enter-B", "enter-b2"]);
    assert_eq!(evaluator.configuration().active_states(), ["b2"]);
    assert!(!evaluator.configuration().contains("phaseA"));
    assert!(evaluator.configuration().contains("phaseB"));
    evaluator.take_executed_actions();
    assert!(evaluator.process_event("finish", None, None).unwrap());
    let messages: Vec<_> = evaluator
        .executed_actions()
        .iter()
        .filter_map(|a| a.action.message.as_deref())
        .collect();
    assert_eq!(messages, ["exit-b2", "exit-B"]);
    assert_eq!(evaluator.configuration().active_states(), ["done"]);
}

#[test]
fn parallel_final_leaves_survive_without_an_eligible_join() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"a","states":{"a":{"type":"atomic","transitions":[{"event":"finish","target":"a-done"}]},"a-done":{"type":"final"}}},
            "b":{"initialState":"b","states":{"b":{"type":"atomic","transitions":[{"event":"finish","target":"b-done"}]},"b-done":{"type":"final"}}}
        },"transitions":[{"event":"finish","target":"wrong-parent"}]},"wrong-parent":{"type":"final"}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("finish", None, None).unwrap());
    assert_eq!(
        evaluator.configuration().active_states(),
        ["a-done", "b-done"]
    );
    assert!(evaluator.configuration().contains("parallel"));
    assert_eq!(evaluator.transitions().len(), 2);
    assert!(!evaluator.configuration().contains("wrong-parent"));
}

#[test]
fn outward_parallel_transition_invalidates_pending_sibling_candidates() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","onExit":[{"action":"log","message":"exit-parent"}],"regions":{
            "a":{"initialState":"a","states":{"a":{"type":"atomic","onExit":[{"action":"log","message":"exit-a"}],"transitions":[{"event":"go","target":"done"}]}}},
            "b":{"initialState":"b","states":{"b":{"type":"atomic","onExit":[{"action":"log","message":"exit-b"}],"transitions":[{"event":"go","target":"wrong-sibling"}]},"wrong-sibling":{"type":"final"}}}
        }},"done":{"type":"final"}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("go", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["done"]);
    assert_eq!(evaluator.transitions().len(), 1);
    let messages: Vec<_> = evaluator
        .executed_actions()
        .iter()
        .filter_map(|a| a.action.message.as_deref())
        .collect();
    assert_eq!(messages, ["exit-a", "exit-b", "exit-parent"]);
}

#[test]
fn explicit_parallel_descendant_entry_initializes_only_other_regions() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"event":"go","target":"target"}]},
        "parallel":{"type":"parallel","onEntry":[{"action":"log","message":"enter-parent"}],"regions":{
            "a":{"initialState":"wrong-default","states":{"wrong-default":{"type":"atomic","onEntry":[{"action":"log","message":"wrong-default"}]},"nested":{"type":"compound","initialState":"wrong-inner","onEntry":[{"action":"log","message":"enter-nested"}],"states":{"wrong-inner":{"type":"atomic"},"target":{"type":"atomic","onEntry":[{"action":"log","message":"enter-target"}]}}}}},
            "b":{"initialState":"sibling","states":{"sibling":{"type":"atomic","onEntry":[{"action":"log","message":"enter-sibling"}]}}}
        }}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("go", None, None).unwrap());
    assert_eq!(
        evaluator.configuration().active_states(),
        ["target", "sibling"]
    );
    let messages: Vec<_> = evaluator
        .executed_actions()
        .iter()
        .filter_map(|a| a.action.message.as_deref())
        .collect();
    assert_eq!(
        messages,
        [
            "enter-parent",
            "enter-nested",
            "enter-target",
            "enter-sibling"
        ]
    );
}

#[test]
fn restored_leaf_rebuilds_scope_without_replaying_entry_actions() {
    let kernel = cross_compound_kernel();
    let instance:wos_core::instance::WorkflowProcess=serde_json::from_value(json!({
        "processId":stack_common_typeid::testing::fixture_minter().mint_process_id(),"caseLedgerId":stack_common_typeid::testing::fixture_minter().mint_case_ledger_id(),"tenant":"acme","definitionUrl":"urn:test:compound-paths","definitionVersion":"1.0.0",
        "configuration":["b2"],"caseState":{},"provenancePosition":0,"timers":[],"activeTasks":[],"status":"active","createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"
    })).unwrap();
    let mut evaluator = Evaluator::from_instance(kernel, &instance, 0).unwrap();
    assert_eq!(evaluator.configuration().active_states(), ["b2"]);
    assert!(evaluator.configuration().contains("phaseB"));
    assert!(evaluator.executed_actions().is_empty());
    assert!(evaluator.process_event("finish", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["done"]);
    let messages: Vec<_> = evaluator
        .executed_actions()
        .iter()
        .filter_map(|a| a.action.message.as_deref())
        .collect();
    assert_eq!(messages, ["exit-b2", "exit-B"]);
}

#[test]
fn nested_parallel_multicast_reaches_other_regions_before_ancestor_fallback() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"outer","states":{
        "outer":{"type":"parallel","regions":{
            "a":{"initialState":"inner","states":{"inner":{"type":"parallel","regions":{
                "x":{"initialState":"x","states":{"x":{"type":"atomic","transitions":[{"event":"go","target":"x-done"}]},"x-done":{"type":"final"}}},
                "y":{"initialState":"y","states":{"y":{"type":"atomic","transitions":[{"event":"go","target":"y-done"}]},"y-done":{"type":"final"}}}
            },"transitions":[{"event":"go","target":"wrong"}]},"wrong":{"type":"final"}}},
            "b":{"initialState":"b","states":{"b":{"type":"atomic","transitions":[{"event":"go","target":"b-done"}]},"b-done":{"type":"final"}}}
        }}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("go", None, None).unwrap());
    assert_eq!(
        evaluator.configuration().active_states(),
        ["x-done", "y-done", "b-done"]
    );
    assert_eq!(
        evaluator
            .transitions()
            .iter()
            .map(|t| t.from.as_str())
            .collect::<Vec<_>>(),
        ["x", "y", "b"]
    );
}

fn saved_leaves(leaves: &[&str]) -> wos_core::instance::WorkflowProcess {
    serde_json::from_value(json!({"processId":stack_common_typeid::testing::fixture_minter().mint_process_id(),"caseLedgerId":stack_common_typeid::testing::fixture_minter().mint_case_ledger_id(),"tenant":"acme","definitionUrl":"urn:test:compound-paths","definitionVersion":"1.0.0","configuration":leaves,"caseState":{},"provenancePosition":0,"timers":[],"activeTasks":[],"status":"active","createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"})).unwrap()
}
fn cancellable_parallel(policy: &str, join: bool) -> KernelDocument {
    let mut raw = json!({"$wosWorkflow": "1.0", "lifecycle": {"initialState": "parallel", "states": {"parallel": {"type": "parallel", "regions": {"a": {"initialState": "winner", "states": {"winner": {"type": "atomic", "transitions": [{"event": "win", "target": "won"}]}, "won": {"type": "final", "tags": ["error"]}}}, "b": {"initialState": "loser", "states": {"loser": {"type": "compound", "historyState": "deep", "initialState": "child", "onExit": [{"action": "log", "message": "exit-loser"}], "states": {"child": {"type": "atomic", "onExit": [{"action": "log", "message": "exit-child"}]}}}}}}}, "done": {"type": "final"}}}});
    raw["lifecycle"]["states"]["parallel"]["cancellationPolicy"] = json!(policy);
    if join {
        raw["lifecycle"]["states"]["parallel"]["transitions"] =
            json!([{"event":{"kind":"signal","name":"$join","scope":"instance"},"target":"done"}]);
    }
    serde_json::from_value(raw).unwrap()
}

#[test]
fn parallel_declared_transition_reenters_all_regions_with_explicit_target() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","onEntry":[{"action":"log","message":"enter-parent"}],"onExit":[{"action":"log","message":"exit-parent"}],"transitions":[{"event":"fallback","target":"b2"}],"regions":{
            "a":{"initialState":"a","states":{"a":{"type":"atomic","transitions":[{"event":"advance","target":"a2"}]},"a2":{"type":"atomic"}}},
            "b":{"initialState":"b1","states":{"b1":{"type":"atomic"},"b2":{"type":"atomic"}}}
        }}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("advance", None, None).unwrap());
    assert!(evaluator.configuration().contains("a2"));
    evaluator.take_executed_actions();
    assert!(evaluator.process_event("fallback", None, None).unwrap());
    let mut leaves = evaluator.configuration().active_states().to_vec();
    leaves.sort();
    assert_eq!(leaves, ["a", "b2"]);
    assert_eq!(
        evaluator
            .executed_actions()
            .iter()
            .filter_map(|action| action.action.message.as_deref())
            .collect::<Vec<_>>(),
        ["exit-parent", "enter-parent"]
    );
}

#[test]
fn cancellation_exits_nested_hooks_clears_history_and_retains_actual_evidence() {
    let kernel = cancellable_parallel("cancel-siblings", false);
    let mut saved = saved_leaves(&["winner", "child"]);
    saved
        .history_store
        .insert("loser".into(), vec!["child".into()]);
    saved.timers=serde_json::from_value(json!([{"timerId":"loser-timer","deadline":"2026-01-02T00:00:00Z","event":"late","scopeState":"child","durationMs":86400000,"createdAtMs":1767225600000_u64}])).unwrap();
    let mut evaluator = Evaluator::from_instance(kernel.clone(), &saved, 0).unwrap();
    assert!(evaluator.process_event("win", None, None).unwrap());
    let messages: Vec<_> = evaluator
        .executed_actions()
        .iter()
        .filter_map(|a| a.action.message.as_deref())
        .collect();
    assert_eq!(messages, ["exit-child", "exit-loser"]);
    assert!(evaluator.history_store().is_empty());
    assert!(evaluator.timers().is_empty());
    assert_eq!(evaluator.configuration().active_states(), ["won"]);
}

#[test]
fn cancellation_policies_deliver_join_after_the_winning_final() {
    for policy in ["cancel-siblings", "fail-fast"] {
        let mut evaluator = Evaluator::new(
            cancellable_parallel(policy, true),
            &stack_common_typeid::testing::fixture_minter(),
        )
        .unwrap();
        assert!(evaluator.process_event("win", None, None).unwrap());
        assert_eq!(
            evaluator.configuration().active_states(),
            ["done"],
            "{policy}"
        );
        assert_eq!(
            evaluator
                .transitions()
                .iter()
                .map(|t| t.event.as_str())
                .collect::<Vec<_>>(),
            ["win", "$join"]
        );
    }
}

#[test]
fn restoration_refuses_duplicate_exclusive_or_incomplete_leaf_sets() {
    let kernel = cross_compound_kernel();
    for leaves in [vec!["a1", "a1"], vec!["a1", "b2"], vec!["b1", "b2"]] {
        assert!(
            Evaluator::from_instance(kernel.clone(), &saved_leaves(&leaves), 0).is_err(),
            "accepted {leaves:?}"
        );
    }
    let kernel = cancellable_parallel("wait-all", false);
    assert!(Evaluator::from_instance(kernel.clone(), &saved_leaves(&["winner"]), 0).is_err());
    assert!(Evaluator::from_instance(kernel, &saved_leaves(&["winner", "child"]), 0).is_ok());
    let kernel = cancellable_parallel("cancel-siblings", false);
    assert!(Evaluator::from_instance(kernel.clone(), &saved_leaves(&["winner"]), 0).is_err());
    assert!(
        Evaluator::from_instance(kernel, &saved_leaves(&["won"]), 0).is_ok(),
        "actual winning final permits cancelled sibling absence"
    );
}

#[test]
fn child_cross_region_transition_reenters_parallel_without_conflicting_leaves() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"a","states":{"a":{"type":"atomic","transitions":[{"event":"cross","target":"b2"}]}}},
            "b":{"initialState":"b1","states":{"b1":{"type":"atomic"},"b2":{"type":"atomic"}}}
        }}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("cross", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["a", "b2"]);
}

#[test]
fn nested_winning_join_cancels_outer_sibling_before_it_can_handle_the_event() {
    for policy in ["cancel-siblings", "fail-fast"] {
        let mut raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"outer","states":{
            "outer":{"type":"parallel","regions":{
                "a":{"initialState":"inner","states":{"inner":{"type":"parallel","regions":{
                    "x":{"initialState":"x","states":{"x":{"type":"atomic","transitions":[{"event":"go","target":"x-done"}]},"x-done":{"type":"final"}}}
                },"transitions":[{"event":{"kind":"signal","name":"$join","scope":"instance"},"target":"won"}]},"won":{"type":"final","tags":["error"]}}},
                "b":{"initialState":"b","states":{"b":{"type":"atomic","onExit":[{"action":"log","message":"cancel-b"}],"transitions":[{"event":"go","target":"b-done","actions":[{"action":"log","message":"must-not-run"}]}]},"b-done":{"type":"final"}}}
            },"transitions":[{"event":{"kind":"signal","name":"$join","scope":"instance"},"target":"done"}]},"done":{"type":"final"}
        }}});
        raw["lifecycle"]["states"]["outer"]["cancellationPolicy"] = json!(policy);
        let mut evaluator = Evaluator::new(
            serde_json::from_value(raw).unwrap(),
            &stack_common_typeid::testing::fixture_minter(),
        )
        .unwrap();
        assert!(evaluator.process_event("go", None, None).unwrap());
        assert_eq!(
            evaluator.configuration().active_states(),
            ["done"],
            "{policy}"
        );
        assert_eq!(
            evaluator
                .transitions()
                .iter()
                .map(|t| t.from.as_str())
                .collect::<Vec<_>>(),
            ["x", "inner", "outer"]
        );
        assert_eq!(
            evaluator
                .executed_actions()
                .iter()
                .filter_map(|a| a.action.message.as_deref())
                .collect::<Vec<_>>(),
            ["cancel-b"]
        );
    }
}

#[test]
fn shallow_and_deep_history_preserve_their_distinct_reentry_behavior() {
    for (mode, expected) in [("shallow", "one"), ("deep", "two")] {
        let mut raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"history","states":{
            "history":{"type":"compound","initialState":"nested","states":{"nested":{"type":"compound","initialState":"one","states":{"one":{"type":"atomic","transitions":[{"event":"next","target":"two"}]},"two":{"type":"atomic"}}}},"transitions":[{"event":"pause","target":"outside"}]},
            "outside":{"type":"atomic","transitions":[{"event":"resume","target":"history"}]}
        }}});
        raw["lifecycle"]["states"]["history"]["historyState"] = json!(mode);
        let mut evaluator = Evaluator::new(
            serde_json::from_value(raw).unwrap(),
            &stack_common_typeid::testing::fixture_minter(),
        )
        .unwrap();
        assert!(evaluator.process_event("next", None, None).unwrap());
        assert!(evaluator.process_event("pause", None, None).unwrap());
        assert!(evaluator.process_event("resume", None, None).unwrap());
        assert_eq!(
            evaluator.configuration().active_states(),
            [expected],
            "{mode}"
        );
    }
}

#[test]
fn regional_leaf_replacement_retains_authored_document_order() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"a1","states":{"a1":{"type":"atomic","transitions":[{"event":"advance","target":"a2"}]},"a2":{"type":"atomic"}}},
            "b":{"initialState":"b","states":{"b":{"type":"atomic"}}}
        }}
    }}});
    let kernel: KernelDocument = serde_json::from_value(raw).unwrap();
    let mut evaluator = Evaluator::new(
        kernel.clone(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("advance", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["a2", "b"]);
    assert!(Evaluator::from_instance(kernel.clone(), &saved_leaves(&["a2", "b"]), 0).is_ok());
    assert!(Evaluator::from_instance(kernel, &saved_leaves(&["b", "a2"]), 0).is_err());
}

#[test]
fn same_id_parallel_reentry_never_dispatches_old_event_to_new_sibling_states() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"a","states":{"a":{"type":"atomic","transitions":[{"event":"cross","target":"b2"}]}}},
            "b":{"initialState":"b1","states":{"b1":{"type":"atomic"},"b2":{"type":"atomic","transitions":[{"event":"cross","target":"wrong","actions":[{"action":"log","message":"new-sibling-must-not-run"}]}]},"wrong":{"type":"final"}}}
        }}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("cross", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["a", "b2"]);
    assert_eq!(evaluator.transitions().len(), 1);
    assert!(evaluator.executed_actions().is_empty());
}

#[test]
fn foreach_completion_reentry_never_dispatches_old_event_to_new_sibling_states() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"a","states":{"a":{"type":"atomic","transitions":[{"event":"go","target":"loop"}]},
                "loop":{"type":"foreach","collection":"[]","transitions":[{"target":"b2"}]}}},
            "b":{"initialState":"b1","states":{"b1":{"type":"atomic"},"b2":{"type":"atomic","transitions":[{"event":"go","target":"wrong","actions":[{"action":"log","message":"new-sibling-must-not-run"}]}]},"wrong":{"type":"final"}}}
        }}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("go", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["a", "b2"]);
    assert_eq!(evaluator.transitions().len(), 2);
    assert!(evaluator.executed_actions().is_empty());
}

#[test]
fn synchronous_foreach_reentry_invalidates_pending_parallel_entry_work() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"open","states":{
        "open":{"type":"atomic","transitions":[{"event":"go","target":"loop"}]},
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"a","states":{"a":{"type":"atomic"},"loop":{"type":"foreach","collection":"[]","transitions":[{"target":"b2"}]}}},
            "b":{"initialState":"b1","states":{"b1":{"type":"atomic","onEntry":[{"action":"log","message":"stale-default"}]},"b2":{"type":"atomic"}}}
        }}
    }}});
    let mut evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert!(evaluator.process_event("go", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["a", "b2"]);
    assert!(
        !evaluator
            .executed_actions()
            .iter()
            .any(|a| a.action.message.as_deref() == Some("stale-default"))
    );
}

#[test]
fn initial_parallel_entry_stops_after_synchronous_region_exit() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parallel","states":{
        "parallel":{"type":"parallel","regions":{
            "a":{"initialState":"loop","states":{"loop":{"type":"foreach","collection":"[]","transitions":[{"target":"outside"}]}}},
            "b":{"initialState":"b","states":{"b":{"type":"atomic","onEntry":[{"action":"log","message":"stale-default"}]}}}
        }},"outside":{"type":"final"}
    }}});
    let evaluator = Evaluator::new(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
    )
    .unwrap();
    assert_eq!(evaluator.configuration().active_states(), ["outside"]);
    assert!(evaluator.executed_actions().is_empty());
}

#[test]
fn deep_history_entry_stops_after_synchronous_saved_leaf_exit() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"history","states":{
        "history":{"type":"compound","historyState":"deep","initialState":"parallel","transitions":[{"event":"pause","target":"paused"}],"states":{
            "parallel":{"type":"parallel","regions":{
                "a":{"initialState":"loop","states":{"loop":{"type":"foreach","collection":"[]","transitions":[{"guard":"caseFile.leave = true","target":"outside"}]}}},
                "b":{"initialState":"b","states":{"b":{"type":"atomic"}}}
            }}
        }},"paused":{"type":"atomic","transitions":[{"event":"resume","target":"history"}]},"outside":{"type":"atomic","transitions":[{"event":"resume","target":"history"}]}
    }}});
    let mut evaluator = Evaluator::with_time_and_case_state(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
        Some(&json!({"leave":false})),
    )
    .unwrap();
    assert_eq!(evaluator.configuration().active_states(), ["loop", "b"]);
    assert!(evaluator.process_event("pause", None, None).unwrap());
    evaluator
        .case_state_mut()
        .insert("leave".into(), json!(true));
    assert!(evaluator.process_event("resume", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["outside"]);
    evaluator
        .case_state_mut()
        .insert("leave".into(), json!(false));
    assert!(evaluator.process_event("resume", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["loop", "b"]);
}

#[test]
fn interrupted_history_restore_does_not_resurrect_history_cleared_by_parent_exit() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"parent","states":{
        "parent":{"type":"compound","initialState":"history","states":{
            "history":{"type":"compound","historyState":"deep","initialState":"loop","transitions":[{"event":"pause","target":"paused"}],"states":{
                "loop":{"type":"foreach","collection":"[]","transitions":[{"guard":"caseFile.leave = true","target":"outside"}]}
            }},"paused":{"type":"atomic","transitions":[{"event":"resume","target":"history"}]}
        }},"outside":{"type":"final"}
    }}});
    let mut evaluator = Evaluator::with_time_and_case_state(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
        Some(&json!({"leave":false})),
    )
    .unwrap();
    assert!(evaluator.process_event("pause", None, None).unwrap());
    assert_eq!(evaluator.history_store().get("history").unwrap(), &["loop"]);
    evaluator
        .case_state_mut()
        .insert("leave".into(), json!(true));
    assert!(evaluator.process_event("resume", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["outside"]);
    assert!(!evaluator.history_store().contains_key("history"));
}

#[test]
fn history_restore_stops_when_nested_exit_invalidates_remaining_saved_branches() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"history","states":{
        "history":{"type":"compound","historyState":"deep","initialState":"parallel","transitions":[{"event":"pause","target":"paused"}],"states":{
            "parallel":{"type":"parallel","regions":{
                "a":{"initialState":"loop","states":{"loop":{"type":"foreach","collection":"[]","transitions":[{"guard":"caseFile.leave = true","target":"other"}]}}},
                "b":{"initialState":"b","states":{"b":{"type":"atomic"}}}
            }},"other":{"type":"atomic"}
        }},"paused":{"type":"atomic","transitions":[{"event":"resume","target":"history"}]}
    }}});
    let mut evaluator = Evaluator::with_time_and_case_state(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
        Some(&json!({"leave":false})),
    )
    .unwrap();
    assert!(evaluator.process_event("pause", None, None).unwrap());
    evaluator
        .case_state_mut()
        .insert("leave".into(), json!(true));
    assert!(evaluator.process_event("resume", None, None).unwrap());
    assert_eq!(evaluator.configuration().active_states(), ["other"]);
    assert!(!evaluator.history_store().contains_key("history"));
}

#[test]
fn local_history_reentry_skips_invalidated_saved_leaves_and_restores_unaffected_region() {
    let raw = json!({"$wosWorkflow":"1.0","lifecycle":{"initialState":"history","states":{
        "history":{"type":"compound","historyState":"deep","initialState":"outer","transitions":[{"event":"pause","target":"paused"}],"states":{
            "outer":{"type":"parallel","regions":{
                "a":{"initialState":"inner","states":{"inner":{"type":"parallel","regions":{
                    "x":{"initialState":"loop","states":{"loop":{"type":"foreach","collection":"[]","transitions":[{"guard":"caseFile.leave = true","target":"a2","actions":[{"action":"setData","path":"leave","value":false}]}]}}},
                    "y":{"initialState":"aSibling","states":{"aSibling":{"type":"atomic"},"a2":{"type":"atomic"}}}
                }}}},
                "b":{"initialState":"b","states":{"b":{"type":"atomic"}}}
            }}
        }},"paused":{"type":"atomic","transitions":[{"event":"resume","target":"history"}]}
    }}});
    let mut evaluator = Evaluator::with_time_and_case_state(
        serde_json::from_value(raw).unwrap(),
        &stack_common_typeid::testing::fixture_minter(),
        0,
        Some(&json!({"leave":false})),
    )
    .unwrap();
    assert_eq!(
        evaluator.configuration().active_states(),
        ["loop", "aSibling", "b"]
    );
    assert!(evaluator.process_event("pause", None, None).unwrap());
    evaluator
        .case_state_mut()
        .insert("leave".into(), json!(true));
    assert!(evaluator.process_event("resume", None, None).unwrap());
    assert_eq!(
        evaluator.configuration().active_states(),
        ["loop", "a2", "b"]
    );
    assert_eq!(evaluator.case_state().get("leave"), Some(&json!(false)));
    assert!(!evaluator.history_store().contains_key("history"));
}
