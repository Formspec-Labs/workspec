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
        extensions: HashMap::new(),
    }
}

fn transition(event: &str, target: &str) -> Transition {
    Transition {
        event: Some(TransitionEvent::from_authoring_trigger(event)),
        target: target.to_string(),
        guard: None,
        actions: vec![],
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
                        default: Some(serde_json::Value::Bool(false)),
                        description: None,
                    },
                );
                f
            },
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
