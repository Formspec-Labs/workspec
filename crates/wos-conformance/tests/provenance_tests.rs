// Provenance completeness and engine behavioral tests.
//
// These tests exercise the conformance engine programmatically (no fixture files)
// by constructing kernel documents inline and feeding events through the engine.
// They verify behavioral properties specified by the Lifecycle Detail Companion
// that are not covered by the fixture-based integration tests.

use serde_json::json;
use wos_conformance::{ConformanceFixture, WorkflowEngine};

// ── Helpers ────────────────────────────────────────────────────

/// Write a kernel JSON to a temp file and return the path.
fn write_kernel_to_temp(kernel: serde_json::Value) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let path = dir.path().join("kernel.json");
    std::fs::write(&path, serde_json::to_string_pretty(&kernel).unwrap()).unwrap();
    let path_str = path.to_str().unwrap().to_string();
    (dir, path_str)
}

/// Build a minimal fixture pointing to a kernel file.
fn fixture_for_kernel(kernel_path: &str) -> ConformanceFixture {
    serde_json::from_value(json!({
        "id": "test-fixture",
        "rule": "test",
        "description": "programmatic test",
        "documents": {
            "kernel": kernel_path
        },
        "event_sequence": [],
        "expected_transitions": []
    }))
    .expect("fixture deserialization failed")
}

/// A flat kernel with three states: open -> processing -> done,
/// with onEntry actions on processing that exercise setData.
fn flat_kernel_with_set_data() -> serde_json::Value {
    json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/flat",
        "actors": [
            { "id": "user", "type": "human" },
            { "id": "system", "type": "system" }
        ],
        "caseFile": {
            "fields": {
                "amount": { "type": "number", "default": 0 },
                "status": { "type": "string", "default": "new" },
                "processedBy": { "type": "string" }
            }
        },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "transitions": [
                        {
                            "event": "submit",
                            "target": "processing"
                        }
                    ]
                },
                "processing": {
                    "type": "atomic",
                    "onEntry": [
                        {
                            "action": "setData",
                            "path": "caseFile.status",
                            "value": "processing"
                        },
                        {
                            "action": "setData",
                            "path": "caseFile.processedBy",
                            "value": "system"
                        }
                    ],
                    "transitions": [
                        {
                            "event": "complete",
                            "target": "done"
                        }
                    ]
                },
                "done": {
                    "type": "final"
                }
            }
        }
    })
}

/// A kernel with a compound state containing an initialState and substates.
fn compound_kernel() -> serde_json::Value {
    json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/compound",
        "actors": [
            { "id": "user", "type": "human" }
        ],
        "caseFile": {
            "fields": {
                "step": { "type": "string", "default": "" }
            }
        },
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "compound",
                    "initialState": "step1",
                    "states": {
                        "step1": {
                            "type": "atomic",
                            "onEntry": [
                                { "action": "setData", "path": "caseFile.step", "value": "step1" }
                            ],
                            "transitions": [
                                { "event": "next", "target": "step2" }
                            ]
                        },
                        "step2": {
                            "type": "atomic",
                            "onEntry": [
                                { "action": "setData", "path": "caseFile.step", "value": "step2" }
                            ],
                            "transitions": [
                                { "event": "finish", "target": "completed" }
                            ]
                        }
                    },
                    "transitions": [
                        { "event": "abort", "target": "cancelled" }
                    ]
                },
                "completed": { "type": "final" },
                "cancelled": { "type": "final" }
            }
        }
    })
}

/// A kernel with a timer onEntry action.
fn timer_kernel() -> serde_json::Value {
    json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/timer",
        "actors": [
            { "id": "user", "type": "human" }
        ],
        "caseFile": {
            "fields": {
                "status": { "type": "string", "default": "pending" }
            }
        },
        "lifecycle": {
            "initialState": "waiting",
            "states": {
                "waiting": {
                    "type": "atomic",
                    "onEntry": [
                        {
                            "action": "startTimer",
                            "timerId": "deadline-timer",
                            "duration": "P30D",
                            "event": "timeout"
                        }
                    ],
                    "transitions": [
                        { "event": "respond", "target": "responded" },
                        { "event": "timeout", "target": "timedOut" }
                    ]
                },
                "responded": { "type": "final" },
                "timedOut": { "type": "final" }
            }
        }
    })
}

/// A kernel with an explicit cancelTimer action on exit.
fn cancel_timer_kernel() -> serde_json::Value {
    json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/cancel-timer",
        "actors": [
            { "id": "user", "type": "human" }
        ],
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "waiting",
            "states": {
                "waiting": {
                    "type": "atomic",
                    "onEntry": [
                        {
                            "action": "startTimer",
                            "timerId": "deadline-timer",
                            "duration": "P30D",
                            "event": "timeout"
                        }
                    ],
                    "onExit": [
                        {
                            "action": "cancelTimer",
                            "timerId": "deadline-timer"
                        }
                    ],
                    "transitions": [
                        { "event": "respond", "target": "responded" },
                        { "event": "timeout", "target": "timedOut" }
                    ]
                },
                "responded": { "type": "final" },
                "timedOut": { "type": "final" }
            }
        }
    })
}

/// A kernel with a parallel state and two regions.
fn parallel_kernel() -> serde_json::Value {
    json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/parallel",
        "actors": [
            { "id": "reviewerA", "type": "human" },
            { "id": "reviewerB", "type": "human" }
        ],
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "dualReview",
            "states": {
                "dualReview": {
                    "type": "parallel",
                    "regions": {
                        "regionA": {
                            "initialState": "reviewA",
                            "states": {
                                "reviewA": {
                                    "type": "atomic",
                                    "transitions": [
                                        { "event": "completeA", "target": "doneA" }
                                    ]
                                },
                                "doneA": { "type": "final" }
                            }
                        },
                        "regionB": {
                            "initialState": "reviewB",
                            "states": {
                                "reviewB": {
                                    "type": "atomic",
                                    "transitions": [
                                        { "event": "completeB", "target": "doneB" }
                                    ]
                                },
                                "doneB": { "type": "final" }
                            }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "resolved" }
                    ]
                },
                "resolved": { "type": "final" }
            }
        }
    })
}

// ========================================================================
// Unmatched event produces provenance but no state change.
// ========================================================================

#[test]
fn unmatched_event_produces_provenance_without_state_change() {
    let kernel = flat_kernel_with_set_data();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture = fixture_for_kernel(&kernel_path);
    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");

    let fixture_with_events: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "unknownEvent", "actor": "user" }
        ],
        "expected_transitions": []
    }))
    .unwrap();
    // Re-resolve the path (it's already absolute from write_kernel_to_temp).

    let result = engine
        .execute(&fixture_with_events)
        .expect("execute failed");

    // No transitions should have fired.
    assert!(
        result.transitions.is_empty(),
        "unmatched event should not produce a transition, got: {:?}",
        result.transitions
    );

    // But provenance should record the unmatched event.
    let unmatched = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::UnmatchedEvent)
        .count();
    assert!(
        unmatched >= 1,
        "expected at least 1 unmatchedEvent provenance record, got {unmatched}"
    );
}

// ========================================================================
// setData action mutates case state and produces provenance.
// ========================================================================

#[test]
fn set_data_mutates_case_state_and_produces_provenance() {
    let kernel = flat_kernel_with_set_data();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "submit", "actor": "user" }
        ],
        "expected_transitions": [
            { "from": "open", "to": "processing", "event": "submit" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(result.passed, "fixture failed: {:?}", result.failures);

    // Verify caseStateMutation provenance records exist for the two setData actions.
    let mutations: Vec<_> = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::CaseStateMutation)
        .collect();
    assert!(
        mutations.len() >= 2,
        "expected at least 2 caseStateMutation records (status + processedBy), got {}",
        mutations.len()
    );

    // Verify the mutation data contains the expected paths.
    let paths: Vec<&str> = mutations
        .iter()
        .filter_map(|m| m.data.as_ref())
        .filter_map(|d| d.get("path").and_then(|p| p.as_str()))
        .collect();
    assert!(
        paths.contains(&"caseFile.status"),
        "expected caseFile.status mutation, got: {paths:?}"
    );
    assert!(
        paths.contains(&"caseFile.processedBy"),
        "expected caseFile.processedBy mutation, got: {paths:?}"
    );
}

// ========================================================================
// setData action result appears in subsequent guard evaluation.
// ========================================================================

#[test]
fn set_data_result_visible_in_guard_evaluation() {
    // Kernel where a setData on entry sets amount to 100, then a guard
    // checks caseFile.amount > 50.
    let kernel = json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/guard-after-setdata",
        "actors": [{ "id": "user", "type": "human" }],
        "caseFile": {
            "fields": {
                "amount": { "type": "number", "default": 0 }
            }
        },
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "setData", "path": "caseFile.amount", "value": 100 }
                    ],
                    "transitions": [
                        {
                            "event": "check",
                            "target": "highValue",
                            "guard": "caseFile.amount > 50"
                        },
                        {
                            "event": "check",
                            "target": "lowValue"
                        }
                    ]
                },
                "highValue": { "type": "final" },
                "lowValue": { "type": "final" }
            }
        }
    });

    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "check", "actor": "user" }
        ],
        "expected_transitions": [
            { "from": "start", "to": "highValue", "event": "check" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(
        result.passed,
        "guard should have seen amount=100 from setData; failures: {:?}",
        result.failures
    );
}

// ========================================================================
// Timer creation on state entry produces provenance.
// ========================================================================

#[test]
fn timer_creation_on_state_entry_produces_provenance() {
    let kernel = timer_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [],
        "expected_transitions": []
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    let timer_created = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::TimerCreated)
        .count();
    assert!(
        timer_created >= 1,
        "expected timerCreated provenance on state entry, got {timer_created}"
    );

    // Verify the timer ID is in the provenance data.
    let timer_record = result
        .provenance
        .iter()
        .find(|p| p.record_kind == wos_conformance::ProvenanceKind::TimerCreated)
        .expect("timerCreated record missing");
    let timer_id = timer_record
        .data
        .as_ref()
        .and_then(|d| d.get("timerId"))
        .and_then(|v| v.as_str());
    assert_eq!(
        timer_id,
        Some("deadline-timer"),
        "timer ID should be 'deadline-timer'"
    );
}

// ========================================================================
// Timer fires after simulated delay.
// ========================================================================

#[test]
fn timer_fires_after_simulated_delay() {
    let kernel = timer_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "noop", "delay": "P31D" }
        ],
        "expected_transitions": [
            { "from": "waiting", "to": "timedOut", "event": "timeout" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(
        result.passed,
        "timer should have fired after 31 days: {:?}",
        result.failures
    );

    let timer_fired = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::TimerFired)
        .count();
    assert!(
        timer_fired >= 1,
        "expected timerFired provenance, got {timer_fired}"
    );
}

// ========================================================================
// Timer cancellation on explicit cancelTimer action.
// ========================================================================

#[test]
fn timer_cancelled_on_explicit_cancel_action() {
    let kernel = cancel_timer_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "respond", "actor": "user" }
        ],
        "expected_transitions": [
            { "from": "waiting", "to": "responded", "event": "respond" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(result.passed, "fixture failed: {:?}", result.failures);

    // The onExit cancelTimer should produce a timerCancelled provenance record.
    let cancelled = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::TimerCancelled)
        .count();
    assert!(
        cancelled >= 1,
        "expected timerCancelled provenance from onExit cancelTimer, got {cancelled}"
    );
}

// ========================================================================
// Compound state entry enters initialState recursively.
// ========================================================================

#[test]
fn compound_state_enters_initial_state_recursively() {
    let kernel = compound_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [],
        "expected_transitions": []
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    // On initialization, the engine should enter review -> step1 (the compound's initialState).
    // The step1 onEntry sets caseFile.step to "step1", so we should see that mutation.
    let step_mutations: Vec<_> = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::CaseStateMutation)
        .filter(|p| {
            p.data
                .as_ref()
                .and_then(|d| d.get("path"))
                .and_then(|v| v.as_str())
                == Some("caseFile.step")
        })
        .collect();

    assert!(
        !step_mutations.is_empty(),
        "compound state should have entered step1 and triggered setData for caseFile.step"
    );
}

// ========================================================================
// Compound state substate transitions work correctly.
// ========================================================================

#[test]
fn compound_state_substate_transitions() {
    let kernel = compound_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "next", "actor": "user" },
            { "event": "finish", "actor": "user" }
        ],
        "expected_transitions": [
            { "from": "step1", "to": "step2", "event": "next" },
            { "from": "step2", "to": "completed", "event": "finish" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(
        result.passed,
        "compound transitions failed: {:?}",
        result.failures
    );
}

// ========================================================================
// Parallel state $join fires only when ALL regions reach final.
// ========================================================================

#[test]
fn parallel_join_fires_only_when_all_regions_final() {
    let kernel = parallel_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);

    // Only complete region A. $join should NOT fire.
    let fixture_partial: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "completeA", "actor": "reviewerA" }
        ],
        "expected_transitions": [
            { "from": "reviewA", "to": "doneA", "event": "completeA" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture_partial).expect("engine init failed");
    let result = engine.execute(&fixture_partial).expect("execute failed");

    // There should be exactly 1 transition (region A), no $join.
    assert!(
        result.passed,
        "partial execution failed: {:?}",
        result.failures
    );
    let join_transitions = result
        .transitions
        .iter()
        .filter(|t| t.event == "$join")
        .count();
    assert_eq!(
        join_transitions, 0,
        "expected 0 $join transitions when only one region is final, got {join_transitions}"
    );
}

#[test]
fn parallel_join_fires_when_all_regions_final() {
    let kernel = parallel_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);

    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "completeA", "actor": "reviewerA" },
            { "event": "completeB", "actor": "reviewerB" }
        ],
        "expected_transitions": [
            { "from": "reviewA", "to": "doneA", "event": "completeA" },
            { "from": "reviewB", "to": "doneB", "event": "completeB" },
            { "from": "dualReview", "to": "resolved", "event": "$join" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(result.passed, "$join should fire: {:?}", result.failures);
}

// ========================================================================
// Every state transition produces a provenance record.
// ========================================================================

#[test]
fn every_state_transition_produces_provenance() {
    let kernel = flat_kernel_with_set_data();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "test",
        "description": "test",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "submit", "actor": "user" },
            { "event": "complete", "actor": "system" }
        ],
        "expected_transitions": [
            { "from": "open", "to": "processing", "event": "submit" },
            { "from": "processing", "to": "done", "event": "complete" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(result.passed, "transitions failed: {:?}", result.failures);

    // For each actual transition, verify a matching provenance record exists.
    for t in &result.transitions {
        let matching_provenance = result.provenance.iter().any(|p| {
            p.record_kind == wos_conformance::ProvenanceKind::StateTransition
                && p.from_state.as_deref() == Some(&t.from)
                && p.to_state.as_deref() == Some(&t.to)
                && p.event.as_deref() == Some(&t.event)
        });
        assert!(
            matching_provenance,
            "no stateTransition provenance for {}->{} on '{}'; provenance: {:?}",
            t.from, t.to, t.event, result.provenance
        );
    }
}

// ========================================================================
// Determinism: same events produce same transitions.
// ========================================================================

#[test]
fn determinism_same_events_produce_same_transitions() {
    let kernel = flat_kernel_with_set_data();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "K-011",
        "description": "determinism check",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "submit", "actor": "user" },
            { "event": "complete", "actor": "system" }
        ],
        "expected_transitions": [
            { "from": "open", "to": "processing", "event": "submit" },
            { "from": "processing", "to": "done", "event": "complete" }
        ]
    }))
    .unwrap();

    // Run the same fixture twice and verify identical transition sequences.
    let mut engine1 = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result1 = engine1.execute(&fixture).expect("execute failed");

    let mut engine2 = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result2 = engine2.execute(&fixture).expect("execute failed");

    assert_eq!(
        result1.transitions.len(),
        result2.transitions.len(),
        "determinism violation: different number of transitions"
    );

    for (i, (t1, t2)) in result1
        .transitions
        .iter()
        .zip(&result2.transitions)
        .enumerate()
    {
        assert_eq!(
            t1.from, t2.from,
            "determinism violation at transition {i}: from differs"
        );
        assert_eq!(
            t1.to, t2.to,
            "determinism violation at transition {i}: to differs"
        );
        assert_eq!(
            t1.event, t2.event,
            "determinism violation at transition {i}: event differs"
        );
    }
}

// ========================================================================
// Guard evaluation: first match wins (document order).
// ========================================================================

#[test]
fn guard_evaluation_first_match_wins_document_order() {
    // Two transitions on the same event, first guard passes.
    let kernel = json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/first-match",
        "actors": [{ "id": "user", "type": "human" }],
        "caseFile": {
            "fields": {
                "amount": { "type": "number", "default": 100 }
            }
        },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "transitions": [
                        {
                            "event": "evaluate",
                            "target": "approved",
                            "guard": "caseFile.amount > 50"
                        },
                        {
                            "event": "evaluate",
                            "target": "rejected"
                        }
                    ]
                },
                "approved": { "type": "final" },
                "rejected": { "type": "final" }
            }
        }
    });

    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "K-033",
        "description": "first match wins",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "evaluate", "actor": "user" }
        ],
        "expected_transitions": [
            { "from": "open", "to": "approved", "event": "evaluate" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(
        result.passed,
        "first-match-wins failed (amount=100 should match guard > 50): {:?}",
        result.failures
    );
}

// ========================================================================
// Guard evaluation: second match when first guard fails.
// ========================================================================

#[test]
fn guard_evaluation_fallthrough_when_first_guard_fails() {
    let kernel = json!({
        "$wosKernel": "1.0",
        "url": "https://test.example.com/fallthrough",
        "actors": [{ "id": "user", "type": "human" }],
        "caseFile": {
            "fields": {
                "amount": { "type": "number", "default": 10 }
            }
        },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "transitions": [
                        {
                            "event": "evaluate",
                            "target": "approved",
                            "guard": "caseFile.amount > 50"
                        },
                        {
                            "event": "evaluate",
                            "target": "rejected"
                        }
                    ]
                },
                "approved": { "type": "final" },
                "rejected": { "type": "final" }
            }
        }
    });

    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test",
        "rule": "K-033",
        "description": "fallthrough to second",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "evaluate", "actor": "user" }
        ],
        "expected_transitions": [
            { "from": "open", "to": "rejected", "event": "evaluate" }
        ]
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    assert!(
        result.passed,
        "fallthrough failed (amount=10 should fail guard > 50, fall to rejected): {:?}",
        result.failures
    );
}

// ========================================================================
// Auxiliary provenance (invalid fixture delay) is stamped by the engine.
// ========================================================================
//
// Regression: the engine constructs ProvenanceRecord::invalid_duration records
// with an empty timestamp and previously merged them into the final log via
// `.extend(...)` without stamping. Downstream exporters and consumers expect
// every record surfaced from a fixture run to carry a non-empty RFC-3339
// timestamp. This test pins that expectation by forcing the invalid-duration
// path with a malformed ISO-8601 duration string.
#[test]
fn invalid_fixture_delay_produces_stamped_auxiliary_provenance() {
    let kernel = timer_kernel();
    let (_dir, kernel_path) = write_kernel_to_temp(kernel);
    let fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": "test-invalid-delay",
        "rule": "test",
        "description": "malformed ISO-8601 duration triggers invalidDuration provenance",
        "documents": { "kernel": kernel_path },
        "event_sequence": [
            { "event": "noop", "delay": "NOT-AN-ISO-DURATION" }
        ],
        "expected_transitions": []
    }))
    .unwrap();

    let mut engine = WorkflowEngine::new(&fixture).expect("engine init failed");
    let result = engine.execute(&fixture).expect("execute failed");

    let invalid_records: Vec<_> = result
        .provenance
        .iter()
        .filter(|record| record.record_kind == wos_conformance::ProvenanceKind::InvalidDuration)
        .collect();

    assert_eq!(
        invalid_records.len(),
        1,
        "expected exactly one invalidDuration record, got {} (all records: {:?})",
        invalid_records.len(),
        result.provenance
    );

    // Every record — runtime-emitted and auxiliary alike — must carry a
    // non-empty timestamp. If the engine ever regresses to unstamped auxiliary
    // provenance, the specific failure below names the record.
    for record in &result.provenance {
        assert!(
            !record.timestamp.is_empty(),
            "every provenance record must be stamped; got unstamped record: {record:?}"
        );
    }
}
