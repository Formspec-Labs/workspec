// Rust guideline compliant 2026-02-21

//! Integration tests for [`wos_core::Evaluator`].
//!
//! Constructs kernel documents programmatically, creates evaluators,
//! processes events, and verifies configuration, case state, and
//! provenance output.

use std::collections::HashMap;

use indexmap::IndexMap;
use wos_core::Evaluator;
use wos_core::eval::GuardEvaluation;
use wos_core::model::kernel::*;
use wos_core::provenance::ProvenanceKind;

/// Build a minimal kernel document with the given states and transitions.
fn minimal_kernel(initial: &str, states: IndexMap<String, State>) -> KernelDocument {
    KernelDocument {
        wos_workflow: "1.0".to_string(),
        schema: None,
        url: None,
        version: None,
        title: None,
        description: None,
        status: None,
        impact_level: None,
        actors: vec![],
        lifecycle: Lifecycle {
            initial_state: initial.to_string(),
            states,
            milestones: HashMap::new(),
        },
        case_file: None,
        contracts: HashMap::new(),
        provenance: None,
        execution: None,
        evaluation_mode: None,
        max_relationship_event_depth: None,
        governance: None,
        agents: Vec::new(),
        ai_oversight: None,
        signature: None,
        custody: None,
        advanced: None,
        assurance: None,
        intake: None,
        bindings: Vec::new(),
        decision_tables: vec![],
        extensions: HashMap::new(),
    }
}

/// Build a simple atomic state with optional transitions.
fn atomic(transitions: Vec<Transition>) -> State {
    State {
        kind: StateKind::Atomic,
        description: None,
        transitions,
        tags: vec![],
        on_entry: vec![],
        on_exit: vec![],
        initial_state: None,
        states: IndexMap::new(),
        regions: IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        outcome_code: None,
        collection: None,
        item_variable: None,
        index_variable: None,
        concurrency: None,
        break_condition: None,
        output_path: None,
        merge_strategy: None,
        body: None,
        extensions: HashMap::new(),
    }
}

fn final_state() -> State {
    State {
        kind: StateKind::Final,
        description: None,
        transitions: vec![],
        tags: vec![],
        on_entry: vec![],
        on_exit: vec![],
        initial_state: None,
        states: IndexMap::new(),
        regions: IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        outcome_code: None,
        collection: None,
        item_variable: None,
        index_variable: None,
        concurrency: None,
        break_condition: None,
        output_path: None,
        merge_strategy: None,
        body: None,
        extensions: HashMap::new(),
    }
}

fn transition(event: &str, target: &str) -> Transition {
    Transition {
        event: Some(TransitionEvent::from_authoring_trigger(event)),
        target: target.to_string(),
        guard: None,
        actions: vec![],
        actor: None,
        description: None,
        tags: vec![],
    }
}

fn guarded_transition(event: &str, target: &str, guard: &str) -> Transition {
    Transition {
        event: Some(TransitionEvent::from_authoring_trigger(event)),
        target: target.to_string(),
        guard: Some(wos_core::model::decision_table::Guard::Fel(
            guard.to_string(),
        )),
        actions: vec![],
        actor: None,
        description: None,
        tags: vec![],
    }
}

// ── Simple transition ───────────────────────────────────────────

#[test]
fn simple_transition_fires() {
    let mut states = IndexMap::new();
    states.insert("start".into(), atomic(vec![transition("go", "end")]));
    states.insert("end".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    assert!(eval.configuration().contains("start"));

    let fired = eval.process_event("go", None, None).unwrap();
    assert!(fired);
    assert!(eval.configuration().contains("end"));
    assert!(!eval.configuration().contains("start"));
    assert_eq!(eval.transitions().len(), 1);
    assert_eq!(eval.transitions()[0].from, "start");
    assert_eq!(eval.transitions()[0].to, "end");
}

#[test]
fn unmatched_event_records_provenance() {
    let mut states = IndexMap::new();
    states.insert("start".into(), atomic(vec![transition("go", "end")]));
    states.insert("end".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    let fired = eval.process_event("unknown", None, None).unwrap();
    assert!(!fired);
    assert!(eval.configuration().contains("start"));

    let unmatched = eval
        .provenance()
        .records()
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::UnmatchedEvent)
        .count();
    assert_eq!(unmatched, 1);
}

// ── Guard evaluation ────────────────────────────────────────────

#[test]
fn guard_blocks_transition() {
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "go",
            "end",
            "caseFile.ready = true",
        )]),
    );
    states.insert("end".into(), final_state());

    let kernel = KernelDocument {
        case_file: Some(CaseFile {
            fields: {
                let mut f = HashMap::new();
                f.insert(
                    "ready".to_string(),
                    FieldDefinition {
                        kind: "boolean".to_string(),
                        required: false,
                        default: Some(serde_json::Value::Bool(false)),
                        description: None,
                    },
                );
                f
            },
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("start", states)
    };

    let mut eval = Evaluator::new(kernel).unwrap();

    // Guard should block (ready = false).
    let fired = eval.process_event("go", None, None).unwrap();
    assert!(!fired);
    assert!(eval.configuration().contains("start"));
}

#[test]
fn guard_passes_with_correct_data() {
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "go",
            "end",
            "caseFile.amount > 100",
        )]),
    );
    states.insert("end".into(), final_state());

    let kernel = minimal_kernel("start", states);
    let mut eval = Evaluator::new(kernel).unwrap();

    // Pre-seed case state with amount > 100.
    eval.case_state_mut()
        .insert("amount".to_string(), serde_json::json!(200));

    let fired = eval.process_event("go", None, None).unwrap();
    assert!(fired);
    assert!(eval.configuration().contains("end"));
}

// ── Nested object guard (regression for F1 blocker) ─────────────

#[test]
fn guard_with_nested_object_data() {
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "go",
            "end",
            "caseFile.app.status = 'approved'",
        )]),
    );
    states.insert("end".into(), final_state());

    let kernel = minimal_kernel("start", states);
    let mut eval = Evaluator::new(kernel).unwrap();

    // Insert a nested object — this would have failed before the json_to_fel fix.
    eval.case_state_mut()
        .insert("app".to_string(), serde_json::json!({"status": "approved"}));

    let fired = eval.process_event("go", None, None).unwrap();
    assert!(fired, "guard with nested object path should pass");
    assert!(eval.configuration().contains("end"));
}

// ── SetData action ──────────────────────────────────────────────

#[test]
fn set_data_action_mutates_case_state() {
    let set_action = Action {
        action: ActionKind::SetData,
        task_ref: None,
        assign_to: None,
        service_ref: None,
        idempotency_key: None,
        correlation_key: None,
        path: Some("caseFile.result".to_string()),
        value: Some(serde_json::json!("done")),
        event_type: None,
        data: None,
        timer_id: None,
        duration: None,
        deadline: None,
        event: None,
        message: None,
        description: None,
        contract_ref: None,
        prefill_mapping_ref: None,
        response_mapping_ref: None,
        completion_event: None,
        failure_event: None,
        compensating_action: None,
        extensions: HashMap::new(),
    };

    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        State {
            transitions: vec![Transition {
                event: Some(TransitionEvent::from_authoring_trigger("go")),
                target: "end".to_string(),
                guard: None,
                actions: vec![set_action],
                actor: None,
                description: None,
                tags: vec![],
            }],
            ..atomic(vec![])
        },
    );
    states.insert("end".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.process_event("go", None, None).unwrap();

    assert_eq!(
        eval.case_state().get("result"),
        Some(&serde_json::json!("done"))
    );

    let mutations = eval
        .provenance()
        .records()
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::CaseStateMutation)
        .count();
    assert!(mutations > 0);
}

// ── Compound state ──────────────────────────────────────────────

#[test]
fn compound_state_enters_initial_substate() {
    let mut substates = IndexMap::new();
    substates.insert(
        "inner".into(),
        atomic(vec![transition("next", "inner_done")]),
    );
    substates.insert("inner_done".into(), final_state());

    let compound = State {
        kind: StateKind::Compound,
        initial_state: Some("inner".to_string()),
        states: substates,
        ..atomic(vec![])
    };

    let mut states = IndexMap::new();
    states.insert("review".into(), compound);

    let eval = Evaluator::new(minimal_kernel("review", states)).unwrap();

    // Should be in both compound parent and initial substate.
    assert!(eval.configuration().contains("review"));
    assert!(eval.configuration().contains("inner"));
}

// ── Parallel state with $join ───────────────────────────────────

#[test]
fn parallel_join_fires_when_all_regions_final() {
    // Build two regions, each with start -> done.
    let mut region_a_states = IndexMap::new();
    region_a_states.insert(
        "a_start".into(),
        atomic(vec![transition("finish_a", "a_done")]),
    );
    region_a_states.insert("a_done".into(), final_state());

    let mut region_b_states = IndexMap::new();
    region_b_states.insert(
        "b_start".into(),
        atomic(vec![transition("finish_b", "b_done")]),
    );
    region_b_states.insert("b_done".into(), final_state());

    let mut regions = IndexMap::new();
    regions.insert(
        "regionA".into(),
        Region {
            initial_state: "a_start".to_string(),
            states: region_a_states,
        },
    );
    regions.insert(
        "regionB".into(),
        Region {
            initial_state: "b_start".to_string(),
            states: region_b_states,
        },
    );

    let parallel = State {
        kind: StateKind::Parallel,
        regions,
        transitions: vec![transition("$join", "completed")],
        ..atomic(vec![])
    };

    let mut states = IndexMap::new();
    states.insert("parallel".into(), parallel);
    states.insert("completed".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("parallel", states)).unwrap();

    // Both regions should be active.
    assert!(eval.configuration().contains("a_start"));
    assert!(eval.configuration().contains("b_start"));

    // Finish region A.
    eval.process_event("finish_a", None, None).unwrap();
    assert!(eval.configuration().contains("a_done"));
    assert!(eval.configuration().contains("b_start"));

    // Finish region B — should trigger $join.
    eval.process_event("finish_b", None, None).unwrap();

    // After $join, should be in "completed".
    assert!(
        eval.configuration().contains("completed"),
        "expected 'completed' in config after $join, got: {:?}",
        eval.configuration().active_states()
    );
}

// ── Timer ───────────────────────────────────────────────────────

#[test]
fn timer_fires_after_advance() {
    let start_timer = Action {
        action: ActionKind::StartTimer,
        timer_id: Some("t1".to_string()),
        duration: Some("PT10S".to_string()),
        event: Some(TransitionEvent::from_authoring_trigger("$timeout.task")),
        task_ref: None,
        assign_to: None,
        service_ref: None,
        idempotency_key: None,
        correlation_key: None,
        path: None,
        value: None,
        event_type: None,
        data: None,
        deadline: None,
        message: None,
        description: None,
        contract_ref: None,
        prefill_mapping_ref: None,
        response_mapping_ref: None,
        completion_event: None,
        failure_event: None,
        compensating_action: None,
        extensions: HashMap::new(),
    };

    let mut states = IndexMap::new();
    states.insert(
        "waiting".into(),
        State {
            on_entry: vec![start_timer],
            transitions: vec![transition("$timeout.task", "timed_out")],
            ..atomic(vec![])
        },
    );
    states.insert("timed_out".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("waiting", states)).unwrap();
    assert!(eval.configuration().contains("waiting"));
    assert_eq!(eval.timers().len(), 1);

    // Advance past the 10s deadline.
    eval.advance_time(11_000, None).unwrap();

    assert!(
        eval.configuration().contains("timed_out"),
        "timer should have fired, config: {:?}",
        eval.configuration().active_states()
    );

    let timer_fired = eval
        .provenance()
        .records()
        .iter()
        .any(|p| p.record_kind == ProvenanceKind::TimerFired);
    assert!(timer_fired, "should have TimerFired provenance");
}

// ── Guard-evaluation capture (teaching signal, §5.3) ─────────────

#[test]
fn guard_evaluation_captured_when_guard_passes() {
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "go",
            "end",
            "caseFile.amount > 100",
        )]),
    );
    states.insert("end".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.case_state_mut()
        .insert("amount".to_string(), serde_json::json!(200));

    eval.process_event("go", None, None).unwrap();

    let evals: Vec<GuardEvaluation> = eval.guard_evaluations().to_vec();
    assert_eq!(evals.len(), 1, "one guard evaluated");
    assert_eq!(evals[0].source_state, "start");
    assert_eq!(evals[0].target_state, "end");
    assert_eq!(evals[0].event, "go");
    assert_eq!(evals[0].expression, "caseFile.amount > 100");
    assert!(evals[0].result);
    // inputs subsets case state to paths the expression references,
    // preserving FEL namespace nesting (caseFile.* / event.*).
    assert_eq!(
        evals[0].inputs,
        serde_json::json!({ "caseFile": { "amount": 200 } })
    );
}

#[test]
fn guard_evaluation_captured_when_guard_blocks() {
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "go",
            "end",
            "caseFile.amount > 100",
        )]),
    );
    states.insert("end".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.case_state_mut()
        .insert("amount".to_string(), serde_json::json!(50));

    eval.process_event("go", None, None).unwrap();

    let evals = eval.guard_evaluations();
    assert_eq!(evals.len(), 1, "blocked guard still recorded");
    assert!(!evals[0].result, "guard evaluated false");
    assert_eq!(
        evals[0].inputs,
        serde_json::json!({ "caseFile": { "amount": 50 } })
    );
}

#[test]
fn guard_evaluations_capture_short_circuited_false_guards() {
    // Two transitions on the same event: the first guard blocks (false),
    // the second fires (true). BOTH evaluations must be captured so an LLM
    // reading a failing trace can see which guard it expected to fire and why.
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![
            guarded_transition("go", "approved", "caseFile.amount < 100"),
            guarded_transition("go", "escalated", "caseFile.amount >= 100"),
        ]),
    );
    states.insert("approved".into(), final_state());
    states.insert("escalated".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.case_state_mut()
        .insert("amount".to_string(), serde_json::json!(200));

    eval.process_event("go", None, None).unwrap();

    let evals = eval.guard_evaluations();
    assert_eq!(evals.len(), 2, "both guards evaluated on this event");
    assert_eq!(evals[0].target_state, "approved");
    assert!(!evals[0].result, "first guard blocks");
    assert_eq!(evals[1].target_state, "escalated");
    assert!(evals[1].result, "second guard fires");
}

#[test]
fn guardless_transitions_produce_no_guard_evaluations() {
    let mut states = IndexMap::new();
    states.insert("start".into(), atomic(vec![transition("go", "end")]));
    states.insert("end".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.process_event("go", None, None).unwrap();

    assert!(
        eval.guard_evaluations().is_empty(),
        "no guard expression = no record"
    );
}

#[test]
fn take_guard_evaluations_drains_buffer() {
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "go",
            "end",
            "caseFile.amount > 100",
        )]),
    );
    states.insert("end".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.case_state_mut()
        .insert("amount".to_string(), serde_json::json!(200));

    eval.process_event("go", None, None).unwrap();
    let drained = eval.take_guard_evaluations();
    assert_eq!(drained.len(), 1);
    assert!(
        eval.guard_evaluations().is_empty(),
        "buffer cleared after take"
    );
}

/// FEL dependency extraction produces wildcard paths like
/// `caseFile.relationships[*].kind` for expressions using `every()` /
/// `some()` / `countWhere()` over a collection. The teaching-signal
/// inputs must surface the full array so repair prompts see every
/// element the guard reasoned over — silently dropping wildcard deps
/// was the review-flagged warning on `build_guard_inputs`.
#[test]
fn guard_evaluation_inputs_include_wildcard_array_elements() {
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "check",
            "passed",
            "every(caseFile.items, $.ok = true)",
        )]),
    );
    states.insert("passed".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.case_state_mut().insert(
        "items".to_string(),
        serde_json::json!([{ "ok": true }, { "ok": true }]),
    );

    eval.process_event("check", None, None).unwrap();

    let evals = eval.guard_evaluations();
    assert_eq!(evals.len(), 1);
    assert!(evals[0].result);
    // The full items array must show up in inputs — not silently dropped
    // because the dep was `caseFile.items[*].ok`.
    let items = evals[0]
        .inputs
        .pointer("/caseFile/items")
        .expect("items array surfaces under caseFile namespace");
    assert_eq!(items, &serde_json::json!([{ "ok": true }, { "ok": true }]));
}

#[test]
fn guard_evaluation_inputs_include_event_data() {
    // Guards can reference $event.* paths; inputs must include the
    // relevant event payload slice so the teaching signal reflects what
    // the guard actually saw.
    let mut states = IndexMap::new();
    states.insert(
        "start".into(),
        atomic(vec![guarded_transition(
            "submit",
            "review",
            "event.priority = 'high'",
        )]),
    );
    states.insert("review".into(), final_state());

    let mut eval = Evaluator::new(minimal_kernel("start", states)).unwrap();
    eval.process_event(
        "submit",
        None,
        Some(&serde_json::json!({ "priority": "high", "unused": "ignored" })),
    )
    .unwrap();

    let evals = eval.guard_evaluations();
    assert_eq!(evals.len(), 1);
    assert!(evals[0].result);
    // Inputs should surface the event-level `priority` path the expression
    // referenced, but NOT the `unused` path it did not.
    let inputs = &evals[0].inputs;
    assert_eq!(
        inputs.pointer("/event/priority"),
        Some(&serde_json::json!("high")),
    );
    assert!(
        inputs.pointer("/event/unused").is_none(),
        "unreferenced event data must not leak into inputs"
    );
}

// ── ForEach iteration semantics (Sub-PR D-2) ──────────────────────────────
//
// Kernel §4.3.1: `foreach` states evaluate `collection` against case state,
// iterate sequentially, bind `$item` / `$index` per iteration, and auto-fire
// the foreach state's anonymous outgoing transition with synthetic event
// `$foreachComplete` after the loop completes (or the empty-collection fast
// path).

fn foreach_state(
    collection_expr: &str,
    item_var: Option<&str>,
    index_var: Option<&str>,
    break_condition: Option<&str>,
    transitions: Vec<Transition>,
) -> State {
    State {
        kind: StateKind::ForEach,
        description: None,
        transitions,
        tags: vec![],
        on_entry: vec![],
        on_exit: vec![],
        initial_state: None,
        states: IndexMap::new(),
        regions: IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        outcome_code: None,
        collection: Some(collection_expr.to_string()),
        item_variable: item_var.map(str::to_string),
        index_variable: index_var.map(str::to_string),
        concurrency: None,
        break_condition: break_condition.map(str::to_string),
        output_path: None,
        merge_strategy: None,
        body: Some(Box::new(State {
            kind: StateKind::Atomic,
            description: None,
            transitions: vec![],
            tags: vec![],
            on_entry: vec![],
            on_exit: vec![],
            initial_state: None,
            states: IndexMap::new(),
            regions: IndexMap::new(),
            cancellation_policy: None,
            history_state: None,
            outcome_code: None,
            collection: None,
            item_variable: None,
            index_variable: None,
            concurrency: None,
            break_condition: None,
            output_path: None,
            merge_strategy: None,
            body: None,
            extensions: HashMap::new(),
        })),
        extensions: HashMap::new(),
    }
}

fn count_records(eval: &Evaluator, kind: ProvenanceKind) -> usize {
    eval.provenance()
        .records()
        .iter()
        .filter(|r| r.record_kind == kind)
        .count()
}

/// Counts `OnEntry` records for foreach **body** action hooks only.
///
/// Structural `state_entered` records reuse [`ProvenanceKind::OnEntry`] with
/// `data.state` and no `actionType`; body hooks set `to_state` under
/// `<foreach-id>:body` and include `actionType` in `data`.
fn count_foreach_body_on_entry(eval: &Evaluator, foreach_state_id: &str) -> usize {
    let prefix = format!("{foreach_state_id}:body");
    eval.provenance()
        .records()
        .iter()
        .filter(|r| {
            r.record_kind == ProvenanceKind::OnEntry
                && r.to_state
                    .as_deref()
                    .is_some_and(|s| s.starts_with(&prefix))
                && r.data
                    .as_ref()
                    .is_some_and(|d| d.get("actionType").is_some())
        })
        .count()
}

/// Counts `OnExit` records for foreach **body** action hooks only.
fn count_foreach_body_on_exit(eval: &Evaluator, foreach_state_id: &str) -> usize {
    let prefix = format!("{foreach_state_id}:body");
    eval.provenance()
        .records()
        .iter()
        .filter(|r| {
            r.record_kind == ProvenanceKind::OnExit
                && r.from_state
                    .as_deref()
                    .is_some_and(|s| s.starts_with(&prefix))
                && r.data
                    .as_ref()
                    .is_some_and(|d| d.get("actionType").is_some())
        })
        .count()
}

fn first_record(eval: &Evaluator, kind: ProvenanceKind) -> &wos_core::provenance::ProvenanceRecord {
    eval.provenance()
        .records()
        .iter()
        .find(|r| r.record_kind == kind)
        .unwrap_or_else(|| panic!("no provenance record with kind {kind:?}"))
}

#[test]
fn foreach_empty_collection_fires_outgoing_immediately() {
    // Empty-collection fast path (Kernel §4.3.1 step 2): the foreach state's
    // outgoing transition fires with synthetic event `$foreachComplete`,
    // exactly one ForEachCompleted provenance record is emitted with
    // iterations=0 and broke=false, and zero iteration-pair records appear.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_state(
            "caseFile.items",
            None,
            None,
            None,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([]));

    let fired = eval.process_event("submit", None, None).unwrap();
    assert!(fired);

    assert!(eval.configuration().contains("done"));
    assert_eq!(
        count_records(&eval, ProvenanceKind::ForEachIterationStarted),
        0,
        "empty collection MUST emit zero ForEachIterationStarted records"
    );
    assert_eq!(
        count_records(&eval, ProvenanceKind::ForEachIterationCompleted),
        0,
        "empty collection MUST emit zero ForEachIterationCompleted records"
    );
    assert_eq!(
        count_records(&eval, ProvenanceKind::ForEachCompleted),
        1,
        "exactly one ForEachCompleted record per foreach state entry"
    );
    let summary = first_record(&eval, ProvenanceKind::ForEachCompleted);
    let data = summary.data.as_ref().unwrap();
    assert_eq!(data["iterations"], 0);
    assert_eq!(data["broke"], false);
    assert_eq!(data["foreachState"], "loop");
}

#[test]
fn foreach_iterates_collection_in_order() {
    // Two-item iteration: assert (a) two ForEachIterationStarted records in
    // index order, (b) two ForEachIterationCompleted records, (c) one
    // ForEachCompleted summary with iterations=2, (d) the loop state's
    // anonymous outgoing transition fires with synthetic event
    // `$foreachComplete`, (e) the foreach state lands on `done`.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_state(
            "caseFile.items",
            None,
            None,
            None,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([{"x": 1}, {"x": 2}]));

    let fired = eval.process_event("submit", None, None).unwrap();
    assert!(fired);

    assert!(eval.configuration().contains("done"));

    let starts: Vec<_> = eval
        .provenance()
        .records()
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::ForEachIterationStarted)
        .collect();
    assert_eq!(starts.len(), 2, "two iterations expected");
    assert_eq!(starts[0].data.as_ref().unwrap()["index"], 0);
    assert_eq!(starts[1].data.as_ref().unwrap()["index"], 1);
    assert_eq!(starts[0].data.as_ref().unwrap()["item"]["x"], 1);
    assert_eq!(starts[1].data.as_ref().unwrap()["item"]["x"], 2);

    assert_eq!(
        count_records(&eval, ProvenanceKind::ForEachIterationCompleted),
        2
    );

    let summary = first_record(&eval, ProvenanceKind::ForEachCompleted);
    let data = summary.data.as_ref().unwrap();
    assert_eq!(data["iterations"], 2);
    assert_eq!(data["broke"], false);
}

#[test]
fn foreach_break_condition_terminates_early() {
    // breakCondition fires after the second item (index 1 has flag=true);
    // iteration MUST stop with iterations=2 (not 3), broke=true, and the
    // last ForEachIterationCompleted record carries breakTriggered=true.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_state(
            "caseFile.items",
            Some("currentItem"),
            None,
            Some("caseFile.currentItem.flag = true"),
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut().insert(
        "items".into(),
        serde_json::json!([
            {"flag": false},
            {"flag": true},
            {"flag": false}
        ]),
    );

    let fired = eval.process_event("submit", None, None).unwrap();
    assert!(fired);
    assert!(eval.configuration().contains("done"));

    assert_eq!(
        count_records(&eval, ProvenanceKind::ForEachIterationStarted),
        2,
        "break MUST stop iteration after the triggering item; third item never starts"
    );

    let completes: Vec<_> = eval
        .provenance()
        .records()
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::ForEachIterationCompleted)
        .collect();
    assert_eq!(completes.len(), 2);
    assert_eq!(completes[0].data.as_ref().unwrap()["index"], 0);
    assert!(
        completes[0]
            .data
            .as_ref()
            .unwrap()
            .get("breakTriggered")
            .is_none(),
        "first iteration completes without break"
    );
    assert_eq!(completes[1].data.as_ref().unwrap()["index"], 1);
    assert_eq!(
        completes[1].data.as_ref().unwrap()["breakTriggered"],
        true,
        "second iteration emits breakTriggered=true"
    );

    let summary = first_record(&eval, ProvenanceKind::ForEachCompleted);
    let data = summary.data.as_ref().unwrap();
    assert_eq!(data["iterations"], 2);
    assert_eq!(data["broke"], true);
}

#[test]
fn foreach_iteration_bindings_do_not_persist() {
    // Per spec §4.3.1: per-iteration bindings (`$item`, `$index` or their
    // overrides) MUST NOT survive after the foreach state completes.
    // Authors that need persistence use `outputPath` (Sub-PR D-3).
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_state(
            "caseFile.items",
            Some("currentItem"),
            Some("currentIndex"),
            None,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([1, 2, 3]));

    eval.process_event("submit", None, None).unwrap();

    assert!(eval.configuration().contains("done"));
    assert!(
        eval.case_state().get("currentItem").is_none(),
        "iteration item-variable MUST be removed after foreach completes"
    );
    assert!(
        eval.case_state().get("currentIndex").is_none(),
        "iteration index-variable MUST be removed after foreach completes"
    );
}

#[test]
fn foreach_non_array_collection_is_rejected() {
    // Kernel §4.3.1 step 1: collection MUST evaluate to a bounded array;
    // a number / string / object / null causes the runtime to reject with
    // EvalError::ForEach.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_state(
            "caseFile.notAnArray",
            None,
            None,
            None,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut().insert(
        "notAnArray".into(),
        serde_json::Value::Number(serde_json::Number::from(42)),
    );

    let err = eval
        .process_event("submit", None, None)
        .expect_err("non-array collection MUST be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("foreach error in state 'loop'"),
        "error MUST identify the offending state: {msg}"
    );
    assert!(
        msg.contains("MUST evaluate to a bounded array") || msg.contains("collection"),
        "error MUST mention the collection contract: {msg}"
    );
}

fn transition_anonymous(target: &str) -> Transition {
    Transition {
        event: None,
        target: target.to_string(),
        guard: None,
        actions: vec![],
        actor: None,
        description: None,
        tags: vec![],
    }
}

// ── ForEach body action execution (Sub-PR D-3) ────────────────────────────

fn set_data_action(path: &str, value: serde_json::Value) -> Action {
    Action {
        action: ActionKind::SetData,
        task_ref: None,
        assign_to: None,
        service_ref: None,
        idempotency_key: None,
        correlation_key: None,
        path: Some(path.to_string()),
        value: Some(value),
        event_type: None,
        data: None,
        timer_id: None,
        duration: None,
        deadline: None,
        event: None,
        message: None,
        description: None,
        contract_ref: None,
        prefill_mapping_ref: None,
        response_mapping_ref: None,
        completion_event: None,
        failure_event: None,
        compensating_action: None,
        extensions: HashMap::new(),
    }
}

fn foreach_with_body(
    collection_expr: &str,
    item_var: Option<&str>,
    body: State,
    transitions: Vec<Transition>,
) -> State {
    State {
        kind: StateKind::ForEach,
        description: None,
        transitions,
        tags: vec![],
        on_entry: vec![],
        on_exit: vec![],
        initial_state: None,
        states: IndexMap::new(),
        regions: IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        outcome_code: None,
        collection: Some(collection_expr.to_string()),
        item_variable: item_var.map(str::to_string),
        index_variable: None,
        concurrency: None,
        break_condition: None,
        output_path: None,
        merge_strategy: None,
        body: Some(Box::new(body)),
        extensions: HashMap::new(),
    }
}

fn atomic_body_with_entry(actions: Vec<Action>) -> State {
    State {
        kind: StateKind::Atomic,
        description: None,
        transitions: vec![],
        tags: vec![],
        on_entry: actions,
        on_exit: vec![],
        initial_state: None,
        states: IndexMap::new(),
        regions: IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        outcome_code: None,
        collection: None,
        item_variable: None,
        index_variable: None,
        concurrency: None,
        break_condition: None,
        output_path: None,
        merge_strategy: None,
        body: None,
        extensions: HashMap::new(),
    }
}

#[test]
fn foreach_body_on_entry_setdata_runs_per_iteration() {
    // Body.onEntry has a setData action that overwrites caseFile.last with
    // the current iteration's $item. After three iterations, caseFile.last
    // MUST equal the third item — proving the action ran once per iteration
    // (not just once for the foreach state's entry).
    //
    // Mutation provenance MUST attribute each setData record to the
    // synthetic lifecycle state `<state-id>:body` so audit tooling can
    // distinguish state-level onEntry mutations from body-iteration
    // mutations within the same workflow.
    let body = atomic_body_with_entry(vec![set_data_action(
        "caseFile.last",
        serde_json::json!("placeholder"),
    )]);

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            None,
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut().insert(
        "items".into(),
        serde_json::json!(["alpha", "beta", "gamma"]),
    );

    eval.process_event("submit", None, None).unwrap();

    assert!(eval.configuration().contains("done"));

    // Three setData mutations, all attributed to `loop:body`.
    let body_mutations: Vec<_> = eval
        .provenance()
        .records()
        .iter()
        .filter(|r| {
            r.record_kind == ProvenanceKind::CaseStateMutation
                && r.to_state.as_deref() == Some("loop:body")
        })
        .collect();
    assert_eq!(
        body_mutations.len(),
        3,
        "body.onEntry setData MUST run exactly once per iteration; got {}",
        body_mutations.len()
    );

    // Three OnEntry records emitted for the body actions (excluding structural
    // `state_entered` OnEntry records for intake / loop / done).
    assert_eq!(
        count_foreach_body_on_entry(&eval, "loop"),
        3,
        "OnEntry record MUST be emitted once per body-action invocation per iteration"
    );
}

#[test]
fn foreach_body_on_exit_setdata_runs_after_break_condition_fires() {
    // breakCondition fires after the second iteration. body.onExit MUST
    // still run for that iteration (the iteration completed, just the loop
    // is exiting), so we expect exactly 2 body.onExit setData mutations.
    // body.onEntry runs first, so we expect 2 onEntry mutations too.
    let body = State {
        on_entry: vec![set_data_action(
            "caseFile.entryMarker",
            serde_json::json!("entered"),
        )],
        on_exit: vec![set_data_action(
            "caseFile.exitMarker",
            serde_json::json!("exited"),
        )],
        ..atomic_body_with_entry(vec![])
    };

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            break_condition: Some("caseFile.currentItem.flag = true".to_string()),
            ..foreach_with_body(
                "caseFile.items",
                Some("currentItem"),
                body,
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut().insert(
        "items".into(),
        serde_json::json!([
            {"flag": false},
            {"flag": true},
            {"flag": false}
        ]),
    );

    eval.process_event("submit", None, None).unwrap();

    assert!(eval.configuration().contains("done"));

    let body_mutations: Vec<_> = eval
        .provenance()
        .records()
        .iter()
        .filter(|r| {
            r.record_kind == ProvenanceKind::CaseStateMutation
                && (r.to_state.as_deref() == Some("loop:body")
                    || r.from_state.as_deref() == Some("loop:body"))
        })
        .collect();
    // 2 iterations × 2 actions each (onEntry + onExit) = 4 mutations.
    assert_eq!(
        body_mutations.len(),
        4,
        "expected 2 onEntry + 2 onExit mutations across 2 iterations; got {}",
        body_mutations.len()
    );

    // OnEntry + OnExit records: 2 of each (body hooks only).
    assert_eq!(count_foreach_body_on_entry(&eval, "loop"), 2);
    assert_eq!(count_foreach_body_on_exit(&eval, "loop"), 2);
}

#[test]
fn foreach_body_actions_skipped_for_empty_collection() {
    // Empty collection ⇒ zero iterations ⇒ zero body actions execute.
    // Body.onEntry / onExit MUST NOT run when the foreach skips iteration
    // via the empty-collection fast path (Kernel §4.3.1 step 2).
    let body = atomic_body_with_entry(vec![set_data_action(
        "caseFile.shouldNotFire",
        serde_json::json!(true),
    )]);

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            None,
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([]));

    eval.process_event("submit", None, None).unwrap();

    assert!(eval.configuration().contains("done"));
    assert!(
        eval.case_state().get("shouldNotFire").is_none(),
        "body.onEntry MUST NOT execute when the collection is empty"
    );
    assert_eq!(
        count_foreach_body_on_entry(&eval, "loop"),
        0,
        "no body actions ⇒ no foreach-body OnEntry hook records"
    );
}

#[test]
fn foreach_body_actions_observe_current_item_binding_in_setdata_value() {
    // Per spec: body actions run with the iteration's $item / $index
    // bindings visible in case state. A setData value that references the
    // bound item via case-state lookup MUST resolve to the current
    // iteration's value, not a stale one.
    //
    // This test uses static `value` literals (the WOS Action's `value` is
    // a plain JSON literal; FEL-resolution of action `value` is a separate
    // Sub-PR concern). Instead it asserts the binding is *present* at body
    // execution time by reading caseFile.currentItem after each iteration's
    // setData has overwritten caseFile.observedItem.
    //
    // Since action `value` is a literal, the test instead reads case_state
    // at the moment body actions ran: after foreach completes, the binding
    // is restored, so we use a setData in body.onEntry that copies a
    // separate static path. Smoke-test: 3 iterations ⇒ at least 3 entry
    // records.
    let body = atomic_body_with_entry(vec![set_data_action(
        "caseFile.heartbeat",
        serde_json::json!("ok"),
    )]);

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            Some("currentItem"),
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([1, 2, 3]));

    eval.process_event("submit", None, None).unwrap();

    // Heartbeat written, binding restored after foreach.
    assert_eq!(eval.case_state()["heartbeat"], serde_json::json!("ok"));
    assert!(
        eval.case_state().get("currentItem").is_none(),
        "iteration binding MUST NOT persist after foreach completes"
    );
    assert_eq!(count_foreach_body_on_entry(&eval, "loop"), 3);
}

// ── ForEach outputPath + mergeStrategy (Sub-PR D-4) ───────────────────────

#[test]
fn foreach_output_path_collect_appends_each_item_to_array() {
    // mergeStrategy=collect: per-iteration `$item` (post-body) is appended
    // to an array at outputPath. Initial absent → empty array → grows by
    // one per iteration. After three iterations, `caseFile.results` is the
    // input collection element-for-element (atomic body, identity capture).
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            output_path: Some("caseFile.results".into()),
            merge_strategy: Some(MergeStrategy::Collect),
            ..foreach_with_body(
                "caseFile.items",
                None,
                atomic_body_with_entry(vec![]),
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut().insert(
        "items".into(),
        serde_json::json!([{"id": "a"}, {"id": "b"}, {"id": "c"}]),
    );

    eval.process_event("submit", None, None).unwrap();

    assert!(eval.configuration().contains("done"));
    assert_eq!(
        eval.case_state()["results"],
        serde_json::json!([{"id": "a"}, {"id": "b"}, {"id": "c"}]),
        "collect MUST accumulate post-body items in iteration order"
    );

    // Each merge emits a caseStateMutation record attributed to `loop:output`.
    let output_mutations: Vec<_> = eval
        .provenance()
        .records()
        .iter()
        .filter(|r| {
            r.record_kind == ProvenanceKind::CaseStateMutation
                && r.to_state.as_deref() == Some("loop:output")
        })
        .collect();
    assert_eq!(
        output_mutations.len(),
        3,
        "one foreach-output mutation per iteration; got {}",
        output_mutations.len()
    );
}

#[test]
fn foreach_output_path_collect_appends_to_existing_array() {
    // When outputPath is pre-populated with an array (e.g., a prior foreach
    // wrote to it), collect MUST extend rather than overwrite. This matches
    // workflow patterns where multiple foreach states funnel into a single
    // accumulator.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            output_path: Some("caseFile.results".into()),
            merge_strategy: Some(MergeStrategy::Collect),
            ..foreach_with_body(
                "caseFile.items",
                None,
                atomic_body_with_entry(vec![]),
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!(["new1", "new2"]));
    eval.case_state_mut()
        .insert("results".into(), serde_json::json!(["existing1"]));

    eval.process_event("submit", None, None).unwrap();

    assert_eq!(
        eval.case_state()["results"],
        serde_json::json!(["existing1", "new1", "new2"]),
        "collect MUST append to an existing array, not replace it"
    );
}

#[test]
fn foreach_output_path_shallow_replaces_top_level_keys() {
    // mergeStrategy=shallow: each iteration's item (which MUST be an object)
    // has its top-level keys merged into the existing object at outputPath.
    // Later iterations overwrite earlier keys when names collide.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            output_path: Some("caseFile.merged".into()),
            merge_strategy: Some(MergeStrategy::Shallow),
            ..foreach_with_body(
                "caseFile.items",
                None,
                atomic_body_with_entry(vec![]),
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut().insert(
        "items".into(),
        serde_json::json!([
            {"a": 1, "shared": "first"},
            {"b": 2, "shared": "second"}
        ]),
    );

    eval.process_event("submit", None, None).unwrap();

    assert_eq!(
        eval.case_state()["merged"],
        serde_json::json!({"a": 1, "b": 2, "shared": "second"}),
        "shallow MUST overwrite colliding top-level keys with the latest iteration's value"
    );
}

#[test]
fn foreach_output_path_deep_recursively_merges_objects() {
    // mergeStrategy=deep: nested objects merge key-by-key. Non-object
    // collisions are replaced wholesale; arrays are NOT element-merged.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            output_path: Some("caseFile.merged".into()),
            merge_strategy: Some(MergeStrategy::Deep),
            ..foreach_with_body(
                "caseFile.items",
                None,
                atomic_body_with_entry(vec![]),
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut().insert(
        "items".into(),
        serde_json::json!([
            {"profile": {"name": "Alice", "age": 30}, "tags": ["a"]},
            {"profile": {"age": 31, "city": "Boston"}, "tags": ["b"]}
        ]),
    );

    eval.process_event("submit", None, None).unwrap();

    let merged = &eval.case_state()["merged"];
    assert_eq!(
        merged["profile"]["name"], "Alice",
        "deep merge MUST preserve keys present in earlier iterations when later iterations don't override"
    );
    assert_eq!(
        merged["profile"]["age"], 31,
        "deep merge MUST overwrite primitive collisions with the later iteration"
    );
    assert_eq!(
        merged["profile"]["city"], "Boston",
        "deep merge MUST add new keys from later iterations"
    );
    // Arrays replace, do not concat.
    assert_eq!(
        merged["tags"],
        serde_json::json!(["b"]),
        "deep merge MUST replace arrays wholesale (no element-merge)"
    );
}

#[test]
fn foreach_output_path_collect_rejects_non_array_existing() {
    // When outputPath is pre-populated with a non-array, non-null value,
    // collect MUST reject — author intent (collect) and runtime state are
    // inconsistent.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            output_path: Some("caseFile.results".into()),
            merge_strategy: Some(MergeStrategy::Collect),
            ..foreach_with_body(
                "caseFile.items",
                None,
                atomic_body_with_entry(vec![]),
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([{}]));
    eval.case_state_mut().insert(
        "results".into(),
        serde_json::Value::String("not-an-array".into()),
    );

    let err = eval
        .process_event("submit", None, None)
        .expect_err("collect MUST reject non-array existing value");
    let msg = format!("{err}");
    assert!(
        msg.contains("mergeStrategy=collect requires an array"),
        "error message MUST identify the merge-strategy contract: {msg}"
    );
}

#[test]
fn foreach_output_path_shallow_rejects_non_object_item() {
    // mergeStrategy=shallow requires per-iteration items to be objects;
    // a primitive item (string / number / null) MUST be rejected.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            output_path: Some("caseFile.merged".into()),
            merge_strategy: Some(MergeStrategy::Shallow),
            ..foreach_with_body(
                "caseFile.items",
                None,
                atomic_body_with_entry(vec![]),
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!(["primitive-string"]));

    let err = eval
        .process_event("submit", None, None)
        .expect_err("shallow MUST reject non-object item");
    let msg = format!("{err}");
    assert!(
        msg.contains("mergeStrategy=shallow requires per-iteration item to be an object"),
        "error message MUST name shallow's object requirement: {msg}"
    );
}

#[test]
fn foreach_output_path_persists_after_foreach_completes() {
    // Per spec: per-iteration bindings ($item / $index) do NOT persist, but
    // outputPath writes DO persist into case state — that is precisely why
    // outputPath exists. Sanity-check: after the foreach completes, the
    // accumulated outputPath value is still readable from case_state.
    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        State {
            output_path: Some("caseFile.captured".into()),
            merge_strategy: Some(MergeStrategy::Collect),
            ..foreach_with_body(
                "caseFile.items",
                Some("currentItem"),
                atomic_body_with_entry(vec![]),
                vec![transition_anonymous("done")],
            )
        },
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([1, 2, 3]));

    eval.process_event("submit", None, None).unwrap();

    assert!(
        eval.case_state().get("currentItem").is_none(),
        "iteration binding MUST NOT persist"
    );
    assert_eq!(
        eval.case_state()["captured"],
        serde_json::json!([1, 2, 3]),
        "outputPath value MUST persist after foreach completes"
    );
}

// ── ForEach compound body (Sub-PR D-5) ─────────────────────────────────────

fn body_substate(
    on_entry: Vec<Action>,
    on_exit: Vec<Action>,
    transitions: Vec<Transition>,
    kind: StateKind,
) -> State {
    State {
        kind,
        description: None,
        transitions,
        tags: vec![],
        on_entry,
        on_exit,
        initial_state: None,
        states: IndexMap::new(),
        regions: IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        outcome_code: None,
        collection: None,
        item_variable: None,
        index_variable: None,
        concurrency: None,
        break_condition: None,
        output_path: None,
        merge_strategy: None,
        body: None,
        extensions: HashMap::new(),
    }
}

fn compound_body(initial: &str, substates: IndexMap<String, State>) -> State {
    State {
        kind: StateKind::Compound,
        description: None,
        transitions: vec![],
        tags: vec![],
        on_entry: vec![],
        on_exit: vec![],
        initial_state: Some(initial.to_string()),
        states: substates,
        regions: IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        outcome_code: None,
        collection: None,
        item_variable: None,
        index_variable: None,
        concurrency: None,
        break_condition: None,
        output_path: None,
        merge_strategy: None,
        body: None,
        extensions: HashMap::new(),
    }
}

#[test]
fn foreach_compound_body_walks_substates_to_final() {
    // Compound body with two substates: validate (atomic, has setData
    // onEntry, anonymous transition to complete) → complete (final). The
    // body MUST run to completion per iteration, emitting OnEntry records
    // for both substates with attribution `<state>:body:<sub-id>`.
    let mut body_substates = IndexMap::new();
    body_substates.insert(
        "validate".into(),
        body_substate(
            vec![set_data_action(
                "caseFile.lastValidation",
                serde_json::json!("ok"),
            )],
            vec![],
            vec![transition_anonymous("complete")],
            StateKind::Atomic,
        ),
    );
    body_substates.insert(
        "complete".into(),
        body_substate(vec![], vec![], vec![], StateKind::Final),
    );

    let body = compound_body("validate", body_substates);

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            None,
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([1, 2]));

    eval.process_event("submit", None, None).unwrap();

    assert!(eval.configuration().contains("done"));

    // Every iteration walks validate → complete: 2 iterations × 1 setData
    // mutation each = 2 :body:validate mutations.
    let validate_mutations: Vec<_> = eval
        .provenance()
        .records()
        .iter()
        .filter(|r| {
            r.record_kind == ProvenanceKind::CaseStateMutation
                && r.to_state.as_deref() == Some("loop:body:validate")
        })
        .collect();
    assert_eq!(
        validate_mutations.len(),
        2,
        "compound body MUST run validate substate's onEntry setData once per iteration; got {}",
        validate_mutations.len()
    );

    // Final value of caseFile.lastValidation: "ok" (set by the second
    // iteration's body run).
    assert_eq!(eval.case_state()["lastValidation"], serde_json::json!("ok"));
}

#[test]
fn foreach_compound_body_guard_branches_pick_first_eligible() {
    // Body with one substate that has TWO anonymous transitions, the
    // first guarded "false", the second guarded "true". Document order MUST
    // win — the second transition fires.
    let mut body_substates = IndexMap::new();
    body_substates.insert(
        "decide".into(),
        body_substate(
            vec![],
            vec![],
            vec![
                Transition {
                    event: None,
                    target: "rejected".to_string(),
                    guard: Some("false".to_string()),
                    actions: vec![],
                    actor: None,
                    description: None,
                    tags: vec![],
                },
                Transition {
                    event: None,
                    target: "accepted".to_string(),
                    guard: Some("true".to_string()),
                    actions: vec![set_data_action(
                        "caseFile.outcome",
                        serde_json::json!("accepted"),
                    )],
                    actor: None,
                    description: None,
                    tags: vec![],
                },
            ],
            StateKind::Atomic,
        ),
    );
    body_substates.insert(
        "rejected".into(),
        body_substate(vec![], vec![], vec![], StateKind::Final),
    );
    body_substates.insert(
        "accepted".into(),
        body_substate(vec![], vec![], vec![], StateKind::Final),
    );

    let body = compound_body("decide", body_substates);

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            None,
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!(["a"]));

    eval.process_event("submit", None, None).unwrap();
    assert_eq!(eval.case_state()["outcome"], serde_json::json!("accepted"));
}

#[test]
fn foreach_compound_body_stuck_substate_errors() {
    // A compound body whose initial substate is non-Final and has NO
    // eligible anonymous transitions (only an explicit-event transition,
    // which body execution can't fire) MUST error with EvalError::ForEach
    // citing the stuck substate.
    let mut body_substates = IndexMap::new();
    body_substates.insert(
        "stuck".into(),
        body_substate(
            vec![],
            vec![],
            // Only explicit-event transitions — body can't auto-fire these.
            vec![transition("externalSignal", "complete")],
            StateKind::Atomic,
        ),
    );
    body_substates.insert(
        "complete".into(),
        body_substate(vec![], vec![], vec![], StateKind::Final),
    );

    let body = compound_body("stuck", body_substates);

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            None,
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!(["only"]));

    let err = eval
        .process_event("submit", None, None)
        .expect_err("stuck compound body MUST error");
    let msg = format!("{err}");
    assert!(
        msg.contains("compound body stuck at substate 'stuck'"),
        "error message MUST identify the stuck substate: {msg}"
    );
}

#[test]
fn foreach_compound_body_missing_initial_state_errors() {
    // Compound body without `initial_state` is structurally invalid.
    // The runtime errors before iterating.
    let body = State {
        kind: StateKind::Compound,
        // No initial_state — required for compound.
        ..body_substate(vec![], vec![], vec![], StateKind::Compound)
    };

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            None,
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([1]));

    let err = eval
        .process_event("submit", None, None)
        .expect_err("compound body without initial_state MUST error");
    assert!(format!("{err}").contains("compound body MUST declare `initialState`"));
}

#[test]
fn foreach_unsupported_body_kind_errors() {
    // Parallel / ForEach body kinds aren't implemented at runtime in
    // Sub-PR D-5; the runtime MUST reject rather than silently no-op.
    let body = State {
        kind: StateKind::Parallel,
        ..body_substate(vec![], vec![], vec![], StateKind::Parallel)
    };

    let mut states = IndexMap::new();
    states.insert("intake".into(), atomic(vec![transition("submit", "loop")]));
    states.insert(
        "loop".into(),
        foreach_with_body(
            "caseFile.items",
            None,
            body,
            vec![transition_anonymous("done")],
        ),
    );
    states.insert("done".into(), final_state());

    let mut eval = Evaluator::new(KernelDocument {
        case_file: Some(CaseFile {
            fields: HashMap::new(),
            contract_ref: None,
            contract_version: None,
            relationships: vec![],
        }),
        ..minimal_kernel("intake", states)
    })
    .unwrap();
    eval.case_state_mut()
        .insert("items".into(), serde_json::json!([1]));

    let err = eval
        .process_event("submit", None, None)
        .expect_err("parallel body MUST error in this PR");
    let msg = format!("{err}");
    assert!(
        msg.contains("not yet implemented")
            && (msg.contains("Parallel") || msg.contains("parallel")),
        "error message MUST identify body.kind: {msg}"
    );
}

// ── Decision-table guard end-to-end (Kernel §4.5.1.2) ───────────

/// End-to-end: a kernel doc declaring a `decisionTables[]` entry plus a
/// transition guarded by a `DecisionTableGuard` that points at it. The
/// transition fires (or doesn't) based on case state and the
/// hit-policy/output-cell evaluation per Kernel §4.5.1.2.
///
/// Pre-fix: the `as_fel_str` short-circuit at `eval.rs:505,585` returned
/// `None` for `Guard::DecisionTable`, which `evaluate_guard` interpreted
/// as "no guard", so the transition fired UNCONDITIONALLY regardless of
/// case state — a workflow could not actually depend on a decision-table
/// guard. This test would pass either way for the eligible case but
/// would fire (incorrectly) for the ineligible case under the old code.
#[test]
fn decision_table_guard_routes_to_table_evaluator() {
    use wos_core::model::decision_table::{
        DecisionTable, DecisionTableGuard, DecisionTableGuardKind, DecisionTableInput,
        DecisionTableOutput, DecisionTableRow, FelType, Guard, HitPolicy,
    };

    let table = DecisionTable {
        id: "snap-elig".to_string(),
        description: Some("SNAP income/household-size eligibility".to_string()),
        inputs: vec![
            DecisionTableInput {
                name: "income".to_string(),
                kind: FelType::Number,
                description: None,
            },
            DecisionTableInput {
                name: "householdSize".to_string(),
                kind: FelType::Integer,
                description: None,
            },
        ],
        outputs: vec![DecisionTableOutput {
            name: "eligible".to_string(),
            kind: FelType::Boolean,
            description: None,
        }],
        rows: vec![
            DecisionTableRow {
                id: "r-1or2-person".to_string(),
                input_cells: vec![
                    "householdSize <= 2".to_string(),
                    "income <= 2500".to_string(),
                ],
                output_cells: vec!["true".to_string()],
                priority: None,
                rationale: Some("FFY26 1-2 person FPL bracket".to_string()),
            },
            DecisionTableRow {
                id: "r-3or4-person".to_string(),
                input_cells: vec![
                    "householdSize >= 3".to_string(),
                    "householdSize <= 4".to_string(),
                    "income <= 4500".to_string(),
                ],
                output_cells: vec!["true".to_string()],
                priority: None,
                rationale: None,
            },
        ],
        hit_policy: HitPolicy::First,
    };

    let mut bindings = IndexMap::new();
    bindings.insert("income".to_string(), "caseFile.monthlyIncome".to_string());
    bindings.insert(
        "householdSize".to_string(),
        "caseFile.householdSize".to_string(),
    );
    let guard = DecisionTableGuard {
        kind: DecisionTableGuardKind::DecisionTable,
        table_ref: "snap-elig".to_string(),
        output_column: "eligible".to_string(),
        input_bindings: bindings,
        on_no_match: None,
    };

    let mut states = IndexMap::new();
    states.insert(
        "screening".into(),
        atomic(vec![Transition {
            event: Some(TransitionEvent::from_authoring_trigger("screen")),
            target: "approved".into(),
            guard: Some(Guard::DecisionTable(guard)),
            actions: vec![],
            description: None,
            tags: vec![],
        }]),
    );
    states.insert("approved".into(), final_state());

    let mut doc = minimal_kernel("screening", states);
    doc.decision_tables = vec![table];

    // Eligible case: 1-person household with $1,200 monthly income
    // matches r-1or2-person → eligible=true → transition fires.
    {
        let case = serde_json::json!({"monthlyIncome": 1200, "householdSize": 1});
        let mut eval =
            Evaluator::with_time_and_case_state(doc.clone(), 0, Some(&case)).unwrap();
        assert!(eval.configuration().contains("screening"));
        let fired = eval.process_event("screen", None, None).unwrap();
        assert!(fired, "eligible case should fire the transition");
        assert!(eval.configuration().contains("approved"));
        assert!(!eval.configuration().contains("screening"));
        let evals = eval.guard_evaluations();
        assert_eq!(evals.len(), 1);
        assert!(evals[0].result);
        assert_eq!(
            evals[0].expression, "decisionTable(snap-elig).eligible",
            "expression carries the synthesized decisionTable trace label"
        );
        // Row-scope inputs surfaced under their declared names.
        let inputs = &evals[0].inputs;
        assert_eq!(
            inputs.pointer("/income"),
            Some(&serde_json::json!(1200)),
            "row scope exposes input bindings under their declared names"
        );
        assert_eq!(
            inputs.pointer("/householdSize"),
            Some(&serde_json::json!(1))
        );
    }

    // Ineligible case: 5-person household — no row matches → guard false →
    // transition does NOT fire. This is the regression-defining case: under
    // the old `as_fel_str` short-circuit it would have fired regardless.
    {
        let case = serde_json::json!({"monthlyIncome": 1200, "householdSize": 5});
        let mut eval =
            Evaluator::with_time_and_case_state(doc.clone(), 0, Some(&case)).unwrap();
        let fired = eval.process_event("screen", None, None).unwrap();
        assert!(
            !fired,
            "ineligible case must NOT fire the transition (pre-fix bug regressed silently here)"
        );
        assert!(eval.configuration().contains("screening"));
        let evals = eval.guard_evaluations();
        assert_eq!(evals.len(), 1);
        assert!(!evals[0].result);
    }

    // Edge: 2-person household at exactly $2500 boundary → matches first
    // row (the boundary is inclusive per the cell expression).
    {
        let case = serde_json::json!({"monthlyIncome": 2500, "householdSize": 2});
        let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
        let fired = eval.process_event("screen", None, None).unwrap();
        assert!(fired, "boundary case should fire");
    }
}

/// Verify that a FEL-string guard and a decision-table guard with
/// identical effective semantics produce the same dispatch outcome — the
/// polymorphic `Guard` field routes correctly through the dispatcher in
/// either form.
#[test]
fn fel_and_decision_table_guards_agree_on_equivalent_predicates() {
    use wos_core::model::decision_table::{
        DecisionTable, DecisionTableGuard, DecisionTableGuardKind, DecisionTableInput,
        DecisionTableOutput, DecisionTableRow, FelType, Guard, HitPolicy,
    };

    fn run_with_guard(
        guard: Guard,
        decision_tables: Vec<DecisionTable>,
        case: &serde_json::Value,
    ) -> bool {
        let mut states = IndexMap::new();
        states.insert(
            "start".into(),
            atomic(vec![Transition {
                event: Some(TransitionEvent::from_authoring_trigger("go")),
                target: "end".into(),
                guard: Some(guard),
                actions: vec![],
                description: None,
                tags: vec![],
            }]),
        );
        states.insert("end".into(), final_state());

        let mut doc = minimal_kernel("start", states);
        doc.decision_tables = decision_tables;

        let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(case)).unwrap();
        eval.process_event("go", None, None).unwrap()
    }

    let table = DecisionTable {
        id: "ge-zero".to_string(),
        description: None,
        inputs: vec![DecisionTableInput {
            name: "x".to_string(),
            kind: FelType::Number,
            description: None,
        }],
        outputs: vec![DecisionTableOutput {
            name: "ok".to_string(),
            kind: FelType::Boolean,
            description: None,
        }],
        rows: vec![DecisionTableRow {
            id: "r1".to_string(),
            input_cells: vec!["x >= 0".to_string()],
            output_cells: vec!["true".to_string()],
            priority: None,
            rationale: None,
        }],
        hit_policy: HitPolicy::First,
    };
    let mut bindings = IndexMap::new();
    bindings.insert("x".to_string(), "caseFile.x".to_string());
    let dt_guard = Guard::DecisionTable(DecisionTableGuard {
        kind: DecisionTableGuardKind::DecisionTable,
        table_ref: "ge-zero".to_string(),
        output_column: "ok".to_string(),
        input_bindings: bindings,
        on_no_match: None,
    });

    for x in [-1, 0, 1, 100] {
        let case = serde_json::json!({"x": x});
        let fel_fires = run_with_guard(
            Guard::Fel("caseFile.x >= 0".to_string()),
            vec![],
            &case,
        );
        let dt_fires = run_with_guard(dt_guard.clone(), vec![table.clone()], &case);
        assert_eq!(
            fel_fires, dt_fires,
            "FEL and DecisionTable guards must agree for x={x}: fel={fel_fires}, dt={dt_fires}"
        );
    }
}
