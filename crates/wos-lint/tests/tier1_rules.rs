// Tier 1 lint rule tests — one positive and one negative test per LINT-MATRIX rule.
//
// Each test constructs a minimal WOS document inline using `serde_json::json!`,
// passes it through `wos_lint::lint_document()`, and asserts on the diagnostics.
//
// Convention:
//   - `*_flagged` tests confirm the rule fires on a non-conformant document.
//   - `*_clean` tests confirm the rule does NOT fire on a conformant document.
//
// Rule IDs match LINT-MATRIX.md exactly (K-001, G-037, AI-041, etc.).

use serde_json::json;
use wos_lint::{Severity, lint_document};

// ── Helpers ────────────────────────────────────────────────────

fn lint(doc: serde_json::Value) -> Vec<wos_lint::Diagnostic> {
    let json_str = serde_json::to_string(&doc).expect("serialization failed");
    lint_document(&json_str).expect("lint_document returned Err")
}

fn has_rule(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> bool {
    diagnostics.iter().any(|d| d.rule_id == rule_id)
}

fn count_rule(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> usize {
    diagnostics.iter().filter(|d| d.rule_id == rule_id).count()
}

fn severity_of(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> Option<Severity> {
    diagnostics
        .iter()
        .find(|d| d.rule_id == rule_id)
        .map(|d| d.severity)
}

fn path_of(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> Option<String> {
    diagnostics
        .iter()
        .find(|d| d.rule_id == rule_id)
        .map(|d| d.path.clone())
}

const TEST_WORKFLOW_URL: &str = "https://example.com/wf";

/// Minimal valid kernel with a two-state flat lifecycle (no violations).
fn minimal_kernel_with_relationships(relationships: serde_json::Value) -> serde_json::Value {
    json!({
        "$wosKernel": "1.0",
        "url": TEST_WORKFLOW_URL,
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "transitions": [
                        { "event": "close", "target": "closed" }
                    ]
                },
                "closed": {
                    "type": "final"
                }
            }
        },
        "caseFile": {
            "fields": {},
            "relationships": relationships
        },
        "actors": []
    })
}

fn minimal_kernel() -> serde_json::Value {
    minimal_kernel_with_relationships(json!([]))
}

/// Schema-valid governance base so Tier 1 exercises typed deserialization.
fn minimal_governance_document() -> serde_json::Value {
    json!({
        "$wosWorkflowGovernance": "1.0",
        "targetWorkflow": TEST_WORKFLOW_URL
    })
}

/// Schema-valid AI integration base so Tier 1 exercises typed deserialization.
fn minimal_ai_integration_document() -> serde_json::Value {
    json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": TEST_WORKFLOW_URL,
        "agents": [
            {
                "id": "reviewAgent",
                "type": "agent",
                "agentType": "generative",
                "modelIdentifier": "gpt-4o",
                "modelVersion": "2025-03"
            }
        ]
    })
}

fn minimal_correspondence_metadata() -> serde_json::Value {
    json!({
        "$wosCorrespondenceMetadata": "1.0",
        "targetWorkflow": TEST_WORKFLOW_URL,
        "correspondenceField": "caseFile.correspondence",
        "entryTemplates": []
    })
}

// ========================================================================
// K-001: Final states MUST NOT have outgoing transitions.
// ========================================================================

#[test]
fn k001_final_state_with_transitions_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": { "type": "atomic" },
                "done": {
                    "type": "final",
                    "transitions": [
                        { "event": "reopen", "target": "open" }
                    ]
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-001"),
        "expected K-001 diagnostic, got: {diags:?}"
    );
    assert_eq!(severity_of(&diags, "K-001"), Some(Severity::Error));
}

#[test]
fn k001_final_state_without_transitions_clean() {
    let doc = minimal_kernel();
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-001"),
        "unexpected K-001 on valid kernel: {diags:?}"
    );
}

// ========================================================================
// K-002: Compound states MUST have initialState and states.
// ========================================================================

#[test]
fn k002_compound_without_initial_state_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "compound",
                    "states": {
                        "a": { "type": "atomic" }
                    }
                    // missing initialState
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-002"),
        "expected K-002 for missing initialState: {diags:?}"
    );
}

#[test]
fn k002_compound_without_states_map_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "compound",
                    "initialState": "sub1"
                    // missing states
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-002"),
        "expected K-002 for missing states: {diags:?}"
    );
}

#[test]
fn k002_compound_with_initial_state_and_states_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "compound",
                    "initialState": "sub1",
                    "states": {
                        "sub1": { "type": "atomic" },
                        "subDone": { "type": "final" }
                    }
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-002"),
        "unexpected K-002 on valid compound: {diags:?}"
    );
}

// ========================================================================
// K-003: Parallel states MUST have regions.
// ========================================================================

#[test]
fn k003_parallel_without_regions_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "parallel"
                    // missing regions
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-003"), "expected K-003: {diags:?}");
}

#[test]
fn k003_parallel_with_regions_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "parallel",
                    "regions": {
                        "regionA": {
                            "initialState": "r1",
                            "states": { "r1": { "type": "atomic" } }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-003"),
        "unexpected K-003 on valid parallel: {diags:?}"
    );
}

// ========================================================================
// K-004: cancellationPolicy MUST only appear on parallel states.
// ========================================================================

#[test]
fn k004_cancellation_policy_on_atomic_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "cancellationPolicy": "fail-fast"
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-004"), "expected K-004: {diags:?}");
}

#[test]
fn k004_cancellation_policy_on_parallel_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "parallel",
                    "cancellationPolicy": "fail-fast",
                    "regions": {
                        "r1": {
                            "initialState": "a",
                            "states": { "a": { "type": "atomic" } }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-004"),
        "unexpected K-004 on parallel: {diags:?}"
    );
}

// ========================================================================
// K-005: historyState MUST only appear on compound states.
// ========================================================================

#[test]
fn k005_history_state_on_atomic_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "historyState": "shallow"
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-005"), "expected K-005: {diags:?}");
}

#[test]
fn k005_history_state_on_compound_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "compound",
                    "initialState": "sub1",
                    "historyState": "shallow",
                    "states": {
                        "sub1": { "type": "atomic" }
                    }
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-005"),
        "unexpected K-005 on compound: {diags:?}"
    );
}

// ========================================================================
// K-006: Transition target MUST reference an existing state.
// ========================================================================

#[test]
fn k006_transition_target_nonexistent_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "transitions": [
                        { "event": "go", "target": "doesNotExist" }
                    ]
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-006"), "expected K-006: {diags:?}");
}

#[test]
fn k006_cross_scope_target_is_valid_clean() {
    // A substate targeting a top-level state should NOT produce K-006.
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "compound",
                    "initialState": "sub1",
                    "states": {
                        "sub1": {
                            "type": "atomic",
                            "transitions": [
                                { "event": "done", "target": "completed" }
                            ]
                        }
                    }
                },
                "completed": { "type": "final" }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-006"),
        "unexpected K-006 on cross-scope target: {diags:?}"
    );
}

#[test]
fn k006_same_scope_valid_target_clean() {
    let doc = minimal_kernel();
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-006"),
        "unexpected K-006 on valid target: {diags:?}"
    );
}

// ========================================================================
// K-007: Event names MUST NOT use the $ prefix.
// ========================================================================

#[test]
fn k007_dollar_prefix_event_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "transitions": [
                        { "event": "$custom", "target": "closed" }
                    ]
                },
                "closed": { "type": "final" }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-007"),
        "expected K-007 for $custom event: {diags:?}"
    );
}

#[test]
fn k007_join_event_is_exempt_clean() {
    // $join is the one allowed $ event (used on parallel states).
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "parallel",
                    "regions": {
                        "r1": {
                            "initialState": "a",
                            "states": { "a": { "type": "atomic" } }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-007"),
        "unexpected K-007 for $join event: {diags:?}"
    );
}

#[test]
fn k007_normal_event_name_clean() {
    let doc = minimal_kernel();
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-007"),
        "unexpected K-007 on normal event names: {diags:?}"
    );
}

// ========================================================================
// K-008: Parallel outgoing transition MUST use $join as event.
// ========================================================================

#[test]
fn k008_parallel_outgoing_non_join_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "parallel",
                    "regions": {
                        "r1": {
                            "initialState": "a",
                            "states": { "a": { "type": "atomic" } }
                        }
                    },
                    "transitions": [
                        { "event": "completed", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-008"), "expected K-008: {diags:?}");
}

#[test]
fn k008_parallel_outgoing_join_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "parallel",
                    "regions": {
                        "r1": {
                            "initialState": "a",
                            "states": { "a": { "type": "atomic" } }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "K-008"), "unexpected K-008: {diags:?}");
}

// ========================================================================
// K-009: Actor identifiers MUST be unique.
// ========================================================================

#[test]
fn k009_duplicate_actor_ids_flagged() {
    let mut doc = minimal_kernel();
    doc["actors"] = json!([
        { "id": "worker", "type": "human" },
        { "id": "worker", "type": "system" }
    ]);

    let diags = lint(doc);
    assert!(has_rule(&diags, "K-009"), "expected K-009: {diags:?}");
    assert_eq!(path_of(&diags, "K-009"), Some("/actors/1/id".to_string()));
}

#[test]
fn k009_unique_actor_ids_clean() {
    let mut doc = minimal_kernel();
    doc["actors"] = json!([
        { "id": "worker", "type": "human" },
        { "id": "system-worker", "type": "system" }
    ]);

    let diags = lint(doc);
    assert!(!has_rule(&diags, "K-009"), "unexpected K-009: {diags:?}");
}

// ========================================================================
// K-014: Milestone id values MUST be unique (via map keys).
// ========================================================================

#[test]
fn k014_empty_milestone_id_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } },
            "milestones": {
                "": { "condition": "true" }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-014"),
        "expected K-014 for empty milestone id: {diags:?}"
    );
}

#[test]
fn k014_non_empty_milestone_ids_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } },
            "milestones": {
                "milestone1": { "condition": "true" },
                "milestone2": { "condition": "false" }
            }
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "K-014"), "unexpected K-014: {diags:?}");
}

// ========================================================================
// K-015: setData path MUST reference a declared caseFile.fields entry.
// ========================================================================

#[test]
fn k015_set_data_undeclared_field_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "caseFile": {
            "fields": {
                "amount": { "type": "number" }
            }
        },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "setData", "path": "caseFile.nonExistent", "value": 42 }
                    ]
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-015"),
        "expected K-015 for undeclared field: {diags:?}"
    );
}

#[test]
fn k015_set_data_declared_field_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "caseFile": {
            "fields": {
                "amount": { "type": "number" }
            }
        },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "setData", "path": "caseFile.amount", "value": 42 }
                    ]
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-015"),
        "unexpected K-015 on declared field: {diags:?}"
    );
}

// ========================================================================
// K-022: Digest present implies algorithm in extensions.
// ========================================================================

#[test]
fn k022_digest_without_algorithm_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } }
        },
        "provenance": [
            {
                "digest": "sha256:abc123"
                // extensions missing or no algorithm key
            }
        ]
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-022"), "expected K-022: {diags:?}");
}

#[test]
fn k022_digest_with_algorithm_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } }
        },
        "provenance": [
            {
                "digest": "sha256:abc123",
                "extensions": { "algorithm": "sha256" }
            }
        ]
    });
    // Note: extensions keys don't need x- prefix here because K-030 only checks
    // keys inside "extensions" objects — but wait, "algorithm" doesn't start with x-.
    // However, looking at the rule implementation, K-022 and K-030 are separate.
    // K-022 checks for "algorithm" key in extensions. K-030 checks all extension keys
    // must be x-prefixed. So this doc will also get K-030.
    // For this test, we only assert K-022 is absent.
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-022"),
        "unexpected K-022 when algorithm present: {diags:?}"
    );
}

// ========================================================================
// K-029: startTimer MUST specify exactly one of duration or deadline.
// ========================================================================

#[test]
fn k029_start_timer_both_duration_and_deadline_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "onEntry": [
                        {
                            "action": "startTimer",
                            "timerId": "t1",
                            "duration": "PT30S",
                            "deadline": "2026-01-01T00:00:00Z",
                            "event": "timeout"
                        }
                    ]
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-029"),
        "expected K-029 for both duration+deadline: {diags:?}"
    );
}

#[test]
fn k029_start_timer_neither_duration_nor_deadline_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "onEntry": [
                        {
                            "action": "startTimer",
                            "timerId": "t1",
                            "event": "timeout"
                            // neither duration nor deadline
                        }
                    ]
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-029"),
        "expected K-029 for missing duration/deadline: {diags:?}"
    );
}

#[test]
fn k029_start_timer_with_duration_only_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "onEntry": [
                        {
                            "action": "startTimer",
                            "timerId": "t1",
                            "duration": "PT30S",
                            "event": "timeout"
                        }
                    ]
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "K-029"), "unexpected K-029: {diags:?}");
}

// ========================================================================
// K-030: Extension keys MUST be x- prefixed.
// ========================================================================

#[test]
fn k030_extension_key_without_x_prefix_flagged() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } }
        },
        "extensions": {
            "customField": "value"
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-030"), "expected K-030: {diags:?}");
}

#[test]
fn k030_extension_key_with_x_prefix_clean() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } }
        },
        "extensions": {
            "x-customField": "value"
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "K-030"), "unexpected K-030: {diags:?}");
}

#[test]
fn k030_nested_extension_key_without_prefix_flagged() {
    // Extensions can appear at any nesting level.
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {
                    "type": "atomic",
                    "extensions": { "badKey": true }
                }
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-030"),
        "expected K-030 for nested extensions: {diags:?}"
    );
}

// ========================================================================
// K-048: Non-standard case relationship type MUST use x- prefix.
// ========================================================================

#[test]
fn k048_non_standard_relationship_type_flagged() {
    let doc = minimal_kernel_with_relationships(json!([
        { "type": "derived-from", "targetCase": "https://example.com/cases/123" }
    ]));
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-048"), "expected K-048: {diags:?}");
}

#[test]
fn k048_standard_relationship_type_clean() {
    let doc = minimal_kernel_with_relationships(json!([
        { "type": "parent", "targetCase": "https://example.com/cases/123" },
        { "type": "supersedes", "targetCase": "https://example.com/cases/456" }
    ]));
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-048"),
        "unexpected K-048 on standard types: {diags:?}"
    );
}

#[test]
fn k048_extension_relationship_type_clean() {
    let doc = minimal_kernel_with_relationships(json!([
        { "type": "x-custom", "targetCase": "https://example.com/cases/123" }
    ]));
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "K-048"),
        "unexpected K-048 on x- prefixed type: {diags:?}"
    );
}

// ========================================================================
// K-021: Provenance actorId MUST reference a declared actor.
// ========================================================================

#[test]
fn k021_provenance_actor_not_declared_flagged() {
    let mut doc = minimal_kernel();
    doc.as_object_mut().unwrap().insert(
        "provenance".to_string(),
        json!([
            { "actorId": "unknownActor", "recordKind": "stateTransition" }
        ]),
    );
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-021"), "expected K-021: {diags:?}");
    assert_eq!(severity_of(&diags, "K-021"), Some(Severity::Error));
}

#[test]
fn k021_provenance_actor_declared_clean() {
    let mut doc = minimal_kernel();
    doc.as_object_mut().unwrap().insert(
        "actors".to_string(),
        json!([{ "id": "alice", "type": "human" }]),
    );
    doc.as_object_mut().unwrap().insert(
        "provenance".to_string(),
        json!([{ "actorId": "alice", "recordKind": "stateTransition" }]),
    );
    let diags = lint(doc);
    assert!(!has_rule(&diags, "K-021"), "unexpected K-021: {diags:?}");
}

#[test]
fn k021_no_provenance_skips_check() {
    let doc = minimal_kernel();
    let diags = lint(doc);
    assert!(!has_rule(&diags, "K-021"), "unexpected K-021: {diags:?}");
}

// ========================================================================
// G-037: Assertion id values MUST be unique.
// ========================================================================

#[test]
fn g037_duplicate_assertion_id_flagged() {
    let doc = json!({
        "$wosAssertionLibrary": "1.0",
        "assertions": [
            { "id": "a1", "type": "arithmetic", "expression": "1 + 1" },
            { "id": "a1", "type": "range", "expression": "x > 0" }
        ]
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-037"), "expected G-037: {diags:?}");
    assert_eq!(severity_of(&diags, "G-037"), Some(Severity::Error));
}

#[test]
fn g037_unique_assertion_ids_clean() {
    let doc = json!({
        "$wosAssertionLibrary": "1.0",
        "assertions": [
            { "id": "a1", "type": "arithmetic", "expression": "1 + 1" },
            { "id": "a2", "type": "range", "expression": "x > 0" }
        ]
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-037"), "unexpected G-037: {diags:?}");
}

// ========================================================================
// G-038: Arithmetic/range/temporal assertion without expression -> warning.
// ========================================================================

#[test]
fn g038_arithmetic_without_expression_flagged() {
    let doc = json!({
        "$wosAssertionLibrary": "1.0",
        "assertions": [
            { "id": "a1", "type": "arithmetic" }
        ]
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-038"), "expected G-038: {diags:?}");
    assert_eq!(severity_of(&diags, "G-038"), Some(Severity::Warning));
}

#[test]
fn g038_range_with_expression_clean() {
    let doc = json!({
        "$wosAssertionLibrary": "1.0",
        "assertions": [
            { "id": "a1", "type": "range", "expression": "value >= 0 && value <= 100" }
        ]
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-038"), "unexpected G-038: {diags:?}");
}

#[test]
fn g038_temporal_without_expression_flagged() {
    let doc = json!({
        "$wosAssertionLibrary": "1.0",
        "assertions": [
            { "id": "a1", "type": "temporal" }
        ]
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "G-038"),
        "expected G-038 for temporal type: {diags:?}"
    );
}

// ========================================================================
// G-039: source-grounded/consistency assertion without fields -> warning.
// ========================================================================

#[test]
fn g039_source_grounded_without_fields_flagged() {
    let doc = json!({
        "$wosAssertionLibrary": "1.0",
        "assertions": [
            { "id": "a1", "type": "source-grounded" }
        ]
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-039"), "expected G-039: {diags:?}");
    assert_eq!(severity_of(&diags, "G-039"), Some(Severity::Warning));
}

#[test]
fn g039_consistency_with_fields_clean() {
    let doc = json!({
        "$wosAssertionLibrary": "1.0",
        "assertions": [
            { "id": "a1", "type": "consistency", "fields": ["income", "deductions"] }
        ]
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-039"), "unexpected G-039: {diags:?}");
}

// ========================================================================
// G-044: Delegation expirationDate MUST be strictly after effectiveDate.
// ========================================================================

#[test]
fn g044_expiration_before_effective_flagged() {
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "delegations".to_string(),
        json!([
            {
                "id": "delegation-1",
                "delegator": "alice",
                "delegate": "bob",
                "scope": {},
                "authority": "determination",
                "effectiveDate": "2026-06-01",
                "expirationDate": "2026-01-01"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-044"), "expected G-044: {diags:?}");
    assert_eq!(severity_of(&diags, "G-044"), Some(Severity::Error));
}

#[test]
fn g044_expiration_equal_to_effective_flagged() {
    // Spec says "strictly after" so equal dates should fail.
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "delegations".to_string(),
        json!([
            {
                "id": "delegation-1",
                "delegator": "alice",
                "delegate": "bob",
                "scope": {},
                "authority": "determination",
                "effectiveDate": "2026-06-01",
                "expirationDate": "2026-06-01"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "G-044"),
        "expected G-044 for equal dates: {diags:?}"
    );
}

#[test]
fn g044_expiration_after_effective_clean() {
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "delegations".to_string(),
        json!([
            {
                "id": "delegation-1",
                "delegator": "alice",
                "delegate": "bob",
                "scope": {},
                "authority": "determination",
                "effectiveDate": "2026-01-01",
                "expirationDate": "2026-12-31"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-044"), "unexpected G-044: {diags:?}");
}

// ========================================================================
// G-045: revokedDate MUST be on or after effectiveDate.
// ========================================================================

#[test]
fn g045_revoked_before_effective_flagged() {
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "delegations".to_string(),
        json!([
            {
                "id": "delegation-1",
                "delegator": "alice",
                "delegate": "bob",
                "scope": {},
                "authority": "determination",
                "effectiveDate": "2026-06-01",
                "revokedDate": "2026-01-01"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-045"), "expected G-045: {diags:?}");
}

#[test]
fn g045_revoked_same_as_effective_clean() {
    // Spec says "on or after" so same date is OK.
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "delegations".to_string(),
        json!([
            {
                "id": "delegation-1",
                "delegator": "alice",
                "delegate": "bob",
                "scope": {},
                "authority": "determination",
                "effectiveDate": "2026-06-01",
                "revokedDate": "2026-06-01"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "G-045"),
        "unexpected G-045 for same-date revocation: {diags:?}"
    );
}

// ========================================================================
// G-047: Parameter values MUST be in ascending effectiveDate order.
// ========================================================================

#[test]
fn g047_values_out_of_order_flagged() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "parameters": {
            "threshold": {
                "type": "number",
                "values": [
                    { "effectiveDate": "2026-06-01", "value": 100 },
                    { "effectiveDate": "2026-01-01", "value": 50 }
                ]
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-047"), "expected G-047: {diags:?}");
}

#[test]
fn g047_values_in_order_clean() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "parameters": {
            "threshold": {
                "type": "number",
                "values": [
                    { "effectiveDate": "2026-01-01", "value": 50 },
                    { "effectiveDate": "2026-06-01", "value": 100 }
                ]
            }
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-047"), "unexpected G-047: {diags:?}");
}

// ========================================================================
// G-048: Binding id MUST match the key under which it appears.
// ========================================================================

#[test]
fn g048_binding_id_mismatch_flagged() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "bindings": {
            "myBinding": {
                "id": "wrongId",
                "parameterId": "threshold"
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-048"), "expected G-048: {diags:?}");
}

#[test]
fn g048_binding_id_matches_key_clean() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "bindings": {
            "myBinding": {
                "id": "myBinding",
                "parameterId": "threshold"
            }
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-048"), "unexpected G-048: {diags:?}");
}

// ========================================================================
// G-050: Parameter value type mismatch.
// ========================================================================

#[test]
fn g050_number_parameter_with_string_value_flagged() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "parameters": {
            "threshold": {
                "type": "number",
                "values": [
                    { "effectiveDate": "2026-01-01", "value": "not a number" }
                ]
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-050"), "expected G-050: {diags:?}");
}

#[test]
fn g050_boolean_parameter_with_boolean_value_clean() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "parameters": {
            "enabled": {
                "type": "boolean",
                "values": [
                    { "effectiveDate": "2026-01-01", "value": true }
                ]
            }
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-050"), "unexpected G-050: {diags:?}");
}

#[test]
fn g050_string_parameter_with_number_value_flagged() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "parameters": {
            "label": {
                "type": "string",
                "values": [
                    { "effectiveDate": "2026-01-01", "value": 42 }
                ]
            }
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "G-050"),
        "expected G-050 for string/number mismatch: {diags:?}"
    );
}

// ========================================================================
// G-055: Hold expectedDuration invalid format.
// ========================================================================

#[test]
fn g055_invalid_duration_format_flagged() {
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "holdPolicies".to_string(),
        json!([
            {
                "holdType": "pending-applicant-response",
                "expectedDuration": "30 days",
                "resumeTrigger": "applicantResponse",
                "timeoutAction": "escalate"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-055"), "expected G-055: {diags:?}");
}

#[test]
fn g055_valid_iso_duration_clean() {
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "holdPolicies".to_string(),
        json!([
            {
                "holdType": "pending-applicant-response",
                "expectedDuration": "P30D",
                "resumeTrigger": "applicantResponse",
                "timeoutAction": "escalate"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-055"), "unexpected G-055: {diags:?}");
}

#[test]
fn g055_indefinite_literal_clean() {
    let mut doc = minimal_governance_document();
    doc.as_object_mut().unwrap().insert(
        "holdPolicies".to_string(),
        json!([
            {
                "holdType": "pending-applicant-response",
                "expectedDuration": "indefinite",
                "resumeTrigger": "applicantResponse",
                "timeoutAction": "escalate"
            }
        ]),
    );
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "G-055"),
        "unexpected G-055 for 'indefinite': {diags:?}"
    );
}

// ========================================================================
// G-057: Binding values MUST be in ascending effectiveDate order.
// ========================================================================

#[test]
fn g057_binding_values_out_of_order_flagged() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "bindings": {
            "b1": {
                "id": "b1",
                "values": [
                    { "effectiveDate": "2026-12-01", "value": "late" },
                    { "effectiveDate": "2026-01-01", "value": "early" }
                ]
            }
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-057"), "expected G-057: {diags:?}");
}

#[test]
fn g057_binding_values_in_order_clean() {
    let doc = json!({
        "$wosPolicyParameters": "1.0",
        "bindings": {
            "b1": {
                "id": "b1",
                "values": [
                    { "effectiveDate": "2026-01-01", "value": "early" },
                    { "effectiveDate": "2026-12-01", "value": "late" }
                ]
            }
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-057"), "unexpected G-057: {diags:?}");
}

// ========================================================================
// G-058: Holiday entry MUST have exactly one of date or rule.
// ========================================================================

fn minimal_business_calendar() -> serde_json::Value {
    json!({
        "$wosBusinessCalendar": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "timezone": "America/New_York",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"]
    })
}

#[test]
fn g058_holiday_both_date_and_rule_flagged() {
    let mut doc = minimal_business_calendar();
    doc.as_object_mut().unwrap().insert(
        "holidays".to_string(),
        json!([{ "name": "Bad", "date": "2026-01-01", "rule": "nthWeekday(3, monday, january)" }]),
    );
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-058"), "expected G-058: {diags:?}");
}

#[test]
fn g058_holiday_neither_date_nor_rule_flagged() {
    let mut doc = minimal_business_calendar();
    doc.as_object_mut()
        .unwrap()
        .insert("holidays".to_string(), json!([{ "name": "Incomplete" }]));
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-058"), "expected G-058: {diags:?}");
}

#[test]
fn g058_holiday_date_only_clean() {
    let mut doc = minimal_business_calendar();
    doc.as_object_mut().unwrap().insert(
        "holidays".to_string(),
        json!([{ "name": "New Year", "date": "2026-01-01" }]),
    );
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-058"), "unexpected G-058: {diags:?}");
}

#[test]
fn g058_no_holidays_skips_check() {
    let doc = minimal_business_calendar();
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-058"), "unexpected G-058: {diags:?}");
}

// ========================================================================
// G-059: Operating hours end MUST be strictly after start.
// ========================================================================

#[test]
fn g059_operating_hours_end_not_after_start_flagged() {
    let mut doc = minimal_business_calendar();
    doc.as_object_mut().unwrap().insert(
        "operatingHours".to_string(),
        json!({ "start": "17:00", "end": "08:00" }),
    );
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-059"), "expected G-059: {diags:?}");
}

#[test]
fn g059_operating_hours_invalid_hhmm_flagged() {
    let mut doc = minimal_business_calendar();
    doc.as_object_mut().unwrap().insert(
        "operatingHours".to_string(),
        json!({ "start": "08:00", "end": "not-a-time" }),
    );
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "G-059"),
        "expected G-059 for invalid time: {diags:?}"
    );
    assert!(
        path_of(&diags, "G-059").is_some_and(|p| p == "/operatingHours"),
        "expected path /operatingHours, got: {diags:?}"
    );
}

#[test]
fn g059_operating_hours_end_after_start_clean() {
    let mut doc = minimal_business_calendar();
    doc.as_object_mut().unwrap().insert(
        "operatingHours".to_string(),
        json!({ "start": "08:00", "end": "17:00" }),
    );
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-059"), "unexpected G-059: {diags:?}");
}

#[test]
fn g059_no_operating_hours_skips_check() {
    let doc = minimal_business_calendar();
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-059"), "unexpected G-059: {diags:?}");
}

// ========================================================================
// G-062: Adverse-decision templates MUST cover required sections.
// ========================================================================

fn minimal_notification_sidecar() -> serde_json::Value {
    json!({
        "$wosNotificationTemplate": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "templates": {}
    })
}

#[test]
fn g062_adverse_template_missing_sections_flagged() {
    let mut doc = minimal_notification_sidecar();
    doc["templates"] = json!({
        "badAdverse": {
            "category": "adverse-decision",
            "sections": [
                { "id": "determination", "contentType": "structured", "content": "x" }
            ]
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-062"), "expected G-062: {diags:?}");
    assert!(
        count_rule(&diags, "G-062") >= 2,
        "expected multiple G-062: {diags:?}"
    );
}

#[test]
fn g062_adverse_template_complete_clean() {
    let mut doc = minimal_notification_sidecar();
    doc["templates"] = json!({
        "fullAdverse": {
            "category": "adverse-decision",
            "sections": [
                { "id": "determination", "contentType": "structured", "content": "d" },
                { "id": "reasons", "contentType": "structured", "content": "r" },
                { "id": "appealRights", "contentType": "appeal-rights", "content": "a" },
                { "id": "appealInstructions", "contentType": "action-required", "content": "i" }
            ]
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-062"), "unexpected G-062: {diags:?}");
}

#[test]
fn g062_non_adverse_category_skips_section_rules() {
    let mut doc = minimal_notification_sidecar();
    doc["templates"] = json!({
        "hold": {
            "category": "hold-notification",
            "sections": [{ "id": "only", "contentType": "text", "content": "x" }]
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-062"), "unexpected G-062: {diags:?}");
}

// ========================================================================
// G-065: Section ids MUST be unique within a template.
// ========================================================================

#[test]
fn g065_duplicate_section_id_flagged() {
    let mut doc = minimal_notification_sidecar();
    doc["templates"] = json!({
        "dup": {
            "category": "case-status-update",
            "sections": [
                { "id": "same", "contentType": "text", "content": "a" },
                { "id": "same", "contentType": "text", "content": "b" }
            ]
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "G-065"), "expected G-065: {diags:?}");
}

#[test]
fn g065_unique_section_ids_clean() {
    let mut doc = minimal_notification_sidecar();
    doc["templates"] = json!({
        "ok": {
            "category": "case-status-update",
            "sections": [
                { "id": "a", "contentType": "text", "content": "1" },
                { "id": "b", "contentType": "text", "content": "2" }
            ]
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-065"), "unexpected G-065: {diags:?}");
}

#[test]
fn g065_single_section_skips_uniqueness_violation() {
    let mut doc = minimal_notification_sidecar();
    doc["templates"] = json!({
        "one": {
            "category": "case-status-update",
            "sections": [{ "id": "only", "contentType": "text", "content": "x" }]
        }
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "G-065"), "unexpected G-065: {diags:?}");
}

// ========================================================================
// CM-001: Correspondence entry template ids MUST be unique.
// ========================================================================

#[test]
fn cm001_duplicate_entry_template_ids_flagged() {
    let mut doc = minimal_correspondence_metadata();
    doc["entryTemplates"] = json!([
        {
            "id": "inboundMail",
            "channel": "mail",
            "direction": "inbound",
            "actorType": "applicant"
        },
        {
            "id": "inboundMail",
            "channel": "email",
            "direction": "inbound",
            "actorType": "representative"
        }
    ]);

    let diags = lint(doc);
    assert!(has_rule(&diags, "CM-001"), "expected CM-001: {diags:?}");
    assert_eq!(
        path_of(&diags, "CM-001"),
        Some("/entryTemplates/1/id".to_string())
    );
}

#[test]
fn cm001_unique_entry_template_ids_clean() {
    let mut doc = minimal_correspondence_metadata();
    doc["entryTemplates"] = json!([
        {
            "id": "inboundMail",
            "channel": "mail",
            "direction": "inbound",
            "actorType": "applicant"
        },
        {
            "id": "phoneContact",
            "channel": "phone",
            "direction": "inbound",
            "actorType": "applicant"
        }
    ]);

    let diags = lint(doc);
    assert!(!has_rule(&diags, "CM-001"), "unexpected CM-001: {diags:?}");
}

// ========================================================================
// AI-041: Fallback chain without terminal action -> error.
// ========================================================================

#[test]
fn ai041_fallback_chain_without_terminal_action_flagged() {
    let mut doc = minimal_ai_integration_document();
    doc.as_object_mut().unwrap().insert(
        "fallbackChain".to_string(),
        json!([
            { "action": "retry" },
            { "action": "alternateAgent", "alternateAgentRef": "agent-b" }
        ]),
    );
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "AI-041"),
        "expected AI-041 for non-terminal chain: {diags:?}"
    );
}

#[test]
fn ai041_fallback_chain_terminates_with_escalate_clean() {
    let mut doc = minimal_ai_integration_document();
    doc.as_object_mut().unwrap().insert(
        "fallbackChain".to_string(),
        json!([
            { "action": "retry" },
            { "action": "escalateToHuman" }
        ]),
    );
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "AI-041"),
        "unexpected AI-041 for escalateToHuman: {diags:?}"
    );
}

#[test]
fn ai041_fallback_chain_terminates_with_fail_clean() {
    let mut doc = minimal_ai_integration_document();
    doc.as_object_mut().unwrap().insert(
        "fallbackChain".to_string(),
        json!([
            { "action": "retry" },
            { "action": "fail" }
        ]),
    );
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "AI-041"),
        "unexpected AI-041 for fail: {diags:?}"
    );
}

#[test]
fn ai041_fallback_chain_with_cycle_flagged() {
    let mut doc = minimal_ai_integration_document();
    doc.as_object_mut().unwrap().insert(
        "fallbackChain".to_string(),
        json!([
            { "action": "alternateAgent", "alternateAgentRef": "agent-b" },
            { "action": "alternateAgent", "alternateAgentRef": "agent-b" },
            { "action": "escalateToHuman" }
        ]),
    );
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "AI-041"),
        "expected AI-041 for cycle: {diags:?}"
    );
}

// ========================================================================
// AI-049: Narrative tier authoritative MUST be false.
// ========================================================================

#[test]
fn ai049_narrative_authoritative_true_flagged() {
    let doc = json!({
        "$wosAIIntegration": "1.0",
        "narrativeProvenance": [
            { "authoritative": true, "text": "This is a narrative." }
        ]
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "AI-049"), "expected AI-049: {diags:?}");
    assert_eq!(severity_of(&diags, "AI-049"), Some(Severity::Error));
}

#[test]
fn ai049_narrative_authoritative_missing_warns() {
    let doc = json!({
        "$wosAIIntegration": "1.0",
        "narrativeProvenance": [
            { "text": "Narrative with no authoritative field." }
        ]
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "AI-049"),
        "expected AI-049 warning: {diags:?}"
    );
    assert_eq!(severity_of(&diags, "AI-049"), Some(Severity::Warning));
}

#[test]
fn ai049_narrative_authoritative_false_clean() {
    let doc = json!({
        "$wosAIIntegration": "1.0",
        "narrativeProvenance": [
            { "authoritative": false, "text": "This is a narrative." }
        ]
    });
    let diags = lint(doc);
    assert!(!has_rule(&diags, "AI-049"), "unexpected AI-049: {diags:?}");
}

#[test]
fn ai049_provenance_tier_narrative_authoritative_true_flagged() {
    // The rule also checks generic provenance entries with tier="narrative".
    let doc = json!({
        "$wosAIIntegration": "1.0",
        "provenance": [
            { "tier": "narrative", "authoritative": true, "text": "Narrative." }
        ]
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "AI-049"),
        "expected AI-049 for provenance tier=narrative: {diags:?}"
    );
}

// ========================================================================
// DM-001 / EQ-001: Extension keys must use x- prefix.
// (These reuse the K-030 check mechanism for different document types.)
// ========================================================================

#[test]
fn dm001_drift_monitor_extension_key_without_prefix() {
    // DriftMonitor is detected by $wosDriftMonitor marker.
    // But the tier1 check_ai_integration dispatches to AI docs only.
    // DM-001/EQ-001 are actually K-030 re-applied to non-kernel docs.
    // The implementation applies check_extension_prefixes to all doc types
    // that reach their check_* function. Let's verify.
    let doc = json!({
        "$wosAIIntegration": "1.0",
        "extensions": {
            "noPrefix": "bad"
        }
    });
    let diags = lint(doc);
    assert!(
        has_rule(&diags, "K-030"),
        "expected K-030 on AI doc extensions: {diags:?}"
    );
}

// ========================================================================
// Edge case: valid minimal kernel produces zero diagnostics.
// ========================================================================

#[test]
fn minimal_valid_kernel_has_zero_diagnostics() {
    let doc = minimal_kernel();
    let diags = lint(doc);
    assert!(
        diags.is_empty(),
        "minimal valid kernel should have 0 diagnostics, got: {diags:?}"
    );
}

// ========================================================================
// Edge case: multiple violations in a single document.
// ========================================================================

#[test]
fn multiple_violations_reported_together() {
    let doc = json!({
        "$wosKernel": "1.0",
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {
                    "type": "parallel"
                    // K-003: missing regions
                    // No transitions, so no K-008 on this one
                },
                "done": {
                    "type": "final",
                    "transitions": [
                        { "event": "$badEvent", "target": "review" }
                    ]
                    // K-001: final with transitions
                    // K-007: $badEvent prefix
                }
            }
        },
        "extensions": {
            "noPrefix": "bad"
            // K-030: missing x- prefix
        }
    });
    let diags = lint(doc);
    assert!(has_rule(&diags, "K-001"), "expected K-001");
    assert!(has_rule(&diags, "K-003"), "expected K-003");
    assert!(has_rule(&diags, "K-007"), "expected K-007");
    assert!(has_rule(&diags, "K-030"), "expected K-030");
    assert!(count_rule(&diags, "K-001") >= 1);
}

// ========================================================================
// I-001: outputBinding JSONPath MUST NOT use unsupported RFC 9535 features.
// ========================================================================

fn minimal_integration_profile(output_binding: serde_json::Value) -> serde_json::Value {
    json!({
        "$wosIntegrationProfile": "1.0",
        "targetWorkflow": {
            "url": "https://example.com/wf"
        },
        "bindings": {
            "myService": {
                "type": "request-response",
                "outputBinding": output_binding
            }
        }
    })
}

#[test]
fn i001_filter_expression_in_output_binding_flagged() {
    let doc = minimal_integration_profile(json!({
        "caseFile.result": "$[?(@.ok)]"
    }));
    let diags = lint(doc);
    assert!(has_rule(&diags, "I-001"), "expected I-001: {diags:?}");
    assert_eq!(
        severity_of(&diags, "I-001"),
        Some(Severity::Error),
        "I-001 must be Error: {diags:?}"
    );
}

#[test]
fn i001_recursive_descent_in_output_binding_flagged() {
    let doc = minimal_integration_profile(json!({
        "caseFile.deep": "$..value"
    }));
    let diags = lint(doc);
    assert!(has_rule(&diags, "I-001"), "expected I-001: {diags:?}");
    assert_eq!(severity_of(&diags, "I-001"), Some(Severity::Error));
}

#[test]
fn i001_simple_member_access_clean() {
    let doc = minimal_integration_profile(json!({
        "caseFile.result": "$.result",
        "caseFile.items": "$.data.items"
    }));
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "I-001"),
        "unexpected I-001 on plain paths: {diags:?}"
    );
}

#[test]
fn i001_wildcard_and_slice_clean() {
    let doc = minimal_integration_profile(json!({
        "caseFile.names": "$.items[*].name",
        "caseFile.first_two": "$.items[0:2]"
    }));
    let diags = lint(doc);
    assert!(
        !has_rule(&diags, "I-001"),
        "unexpected I-001 on wildcard/slice: {diags:?}"
    );
}

#[test]
fn i001_path_in_lint_diagnostic_points_to_binding_key() {
    let doc = minimal_integration_profile(json!({
        "caseFile.x": "$[?(@.bad)]"
    }));
    let diags = lint(doc);
    let path = path_of(&diags, "I-001").expect("expected I-001");
    assert!(
        path.contains("myService"),
        "diagnostic path should name the binding key 'myService', got: {path}"
    );
    assert!(
        path.contains("outputBinding"),
        "diagnostic path should contain 'outputBinding', got: {path}"
    );
}
